//! Transverse Mercator on the WGS84 ellipsoid (Krüger series to n⁶, as in
//! PROJ's `tmerc`; sub-millimetre within a few thousand km of the centre).

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_223_563;

pub struct TMerc {
    lon0: f64,
    y0: f64,
    /// Rectifying radius A of the series.
    ra: f64,
    e: f64,
    alpha: [f64; 6],
    beta: [f64; 6],
}

impl TMerc {
    /// Centred on (lon0, lat0) in degrees, scale 1 on the central meridian,
    /// origin (0, 0) at the centre.
    pub fn new(lon0: f64, lat0: f64) -> Self {
        let n = F / (2.0 - F);
        let (n2, n3, n4, n5, n6) = (n * n, n.powi(3), n.powi(4), n.powi(5), n.powi(6));
        let ra = A / (1.0 + n) * (1.0 + n2 / 4.0 + n4 / 64.0 + n6 / 256.0);
        let alpha = [
            n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0 + 41.0 * n4 / 180.0 - 127.0 * n5 / 288.0 + 7891.0 * n6 / 37800.0,
            13.0 * n2 / 48.0 - 3.0 * n3 / 5.0 + 557.0 * n4 / 1440.0 + 281.0 * n5 / 630.0 - 1983433.0 * n6 / 1935360.0,
            61.0 * n3 / 240.0 - 103.0 * n4 / 140.0 + 15061.0 * n5 / 26880.0 + 167603.0 * n6 / 181440.0,
            49561.0 * n4 / 161280.0 - 179.0 * n5 / 168.0 + 6601661.0 * n6 / 7257600.0,
            34729.0 * n5 / 80640.0 - 3418889.0 * n6 / 1995840.0,
            212378941.0 * n6 / 319334400.0,
        ];
        let beta = [
            n / 2.0 - 2.0 * n2 / 3.0 + 37.0 * n3 / 96.0 - n4 / 360.0 - 81.0 * n5 / 512.0 + 96199.0 * n6 / 604800.0,
            n2 / 48.0 + n3 / 15.0 - 437.0 * n4 / 1440.0 + 46.0 * n5 / 105.0 - 1118711.0 * n6 / 3870720.0,
            17.0 * n3 / 480.0 - 37.0 * n4 / 840.0 - 209.0 * n5 / 4480.0 + 5569.0 * n6 / 90720.0,
            4397.0 * n4 / 161280.0 - 11.0 * n5 / 504.0 - 830251.0 * n6 / 7257600.0,
            4583.0 * n5 / 161280.0 - 108847.0 * n6 / 3991680.0,
            20648693.0 * n6 / 638668800.0,
        ];
        let mut t = TMerc { lon0, y0: 0.0, ra, e: (F * (2.0 - F)).sqrt(), alpha, beta };
        t.y0 = t.forward(lon0, lat0).1;
        t
    }

    /// Conformal latitude tangent τ' from geodetic τ = tan φ.
    fn tau_prime(&self, tau: f64) -> f64 {
        let s = (self.e * (self.e * tau / (1.0 + tau * tau).sqrt()).atanh()).sinh();
        tau * (1.0 + s * s).sqrt() - s * (1.0 + tau * tau).sqrt()
    }

    /// Degrees → metres.
    pub fn forward(&self, lon: f64, lat: f64) -> (f64, f64) {
        let lam = (lon - self.lon0).to_radians();
        let tp = self.tau_prime(lat.to_radians().tan());
        let xi_p = tp.atan2(lam.cos());
        let eta_p = (lam.sin() / (tp * tp + lam.cos().powi(2)).sqrt()).asinh();
        let (mut xi, mut eta) = (xi_p, eta_p);
        for (j, a) in self.alpha.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi += a * (k * xi_p).sin() * (k * eta_p).cosh();
            eta += a * (k * xi_p).cos() * (k * eta_p).sinh();
        }
        (self.ra * eta, self.ra * xi - self.y0)
    }

    /// Metres → degrees.
    pub fn inverse(&self, x: f64, y: f64) -> (f64, f64) {
        let (xi, eta) = ((y + self.y0) / self.ra, x / self.ra);
        let (mut xi_p, mut eta_p) = (xi, eta);
        for (j, b) in self.beta.iter().enumerate() {
            let k = 2.0 * (j + 1) as f64;
            xi_p -= b * (k * xi).sin() * (k * eta).cosh();
            eta_p -= b * (k * xi).cos() * (k * eta).sinh();
        }
        let tp = xi_p.sin() / (eta_p.sinh().powi(2) + xi_p.cos().powi(2)).sqrt();
        let lam = eta_p.sinh().atan2(xi_p.cos());
        // Newton on τ' (τ) = tp.
        let e2 = self.e * self.e;
        let mut tau = tp;
        for _ in 0..5 {
            let tpi = self.tau_prime(tau);
            let d = (tp - tpi) / (1.0 + tpi * tpi).sqrt() * (1.0 + (1.0 - e2) * tau * tau)
                / ((1.0 - e2) * (1.0 + tau * tau).sqrt());
            tau += d;
            if d.abs() < 1e-14 {
                break;
            }
        }
        (self.lon0 + lam.to_degrees(), tau.atan().to_degrees())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centre_is_origin_and_round_trips() {
        let t = TMerc::new(-122.4, 37.75);
        let (x, y) = t.forward(-122.4, 37.75);
        assert!(x.abs() < 1e-6 && y.abs() < 1e-6);
        for &(lon, lat) in &[(-122.75, 37.45), (-122.05, 38.05), (-121.0, 36.0)] {
            let (x, y) = t.forward(lon, lat);
            let (lo, la) = t.inverse(x, y);
            assert!((lo - lon).abs() < 1e-9 && (la - lat).abs() < 1e-9, "{lo} {la}");
        }
    }

    #[test]
    fn one_degree_of_latitude() {
        // ~110.99 km per degree at 37.75°N on WGS84.
        let t = TMerc::new(0.0, 37.25);
        let d = t.forward(0.0, 38.25).1 - t.forward(0.0, 37.25).1;
        assert!((d - 110_990.0).abs() < 30.0, "{d}");
    }
}
