//! Small deterministic PRNG (PCG32), identical on every platform including WASM.

const MULT: u64 = 6_364_136_223_846_793_005;

/// A fixed value in [0, 1) for `x` (SplitMix64): noise that stays put where a
/// stream of draws would depend on the order things are visited in.
pub fn hash01(x: u64) -> f64 {
    let mut z = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^= z >> 31;
    (z >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
}

#[derive(Clone)]
pub struct Rng {
    state: u64,
    inc: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut r = Rng {
            state: 0,
            inc: (seed << 1) | 1,
        };
        r.next_u32();
        r.state = r.state.wrapping_add(0x853c_49e6_748f_ea9b ^ seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(MULT).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        xorshifted.rotate_right((old >> 59) as u32)
    }

    /// Uniform in [0, 1).
    pub fn f64(&mut self) -> f64 {
        let v = ((self.next_u32() as u64) << 32) | self.next_u32() as u64;
        (v >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Skip `delta` outputs of `next_u32` in O(log delta) (Brown, 1994).
    pub fn advance(&mut self, mut delta: u64) {
        let (mut mult, mut plus) = (MULT, self.inc);
        let (mut acc_mult, mut acc_plus) = (1u64, 0u64);
        while delta > 0 {
            if delta & 1 == 1 {
                acc_mult = acc_mult.wrapping_mul(mult);
                acc_plus = acc_plus.wrapping_mul(mult).wrapping_add(plus);
            }
            plus = mult.wrapping_add(1).wrapping_mul(plus);
            mult = mult.wrapping_mul(mult);
            delta >>= 1;
        }
        self.state = acc_mult.wrapping_mul(self.state).wrapping_add(acc_plus);
    }

    /// Fill `out` with standard normals, in parallel chunks but exactly as
    /// successive `normal()` calls would (each consumes 4 × `next_u32`).
    pub fn fill_normal(&mut self, out: &mut [f32]) {
        use rayon::prelude::*;
        const CHUNK: usize = 1 << 16;
        let start = self.clone();
        out.par_chunks_mut(CHUNK).enumerate().for_each(|(c, part)| {
            let mut r = start.clone();
            r.advance(4 * (c * CHUNK) as u64);
            for v in part {
                *v = r.normal() as f32;
            }
        });
        self.advance(4 * out.len() as u64);
    }

    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f64()
    }

    /// Standard normal (Box–Muller).
    pub fn normal(&mut self) -> f64 {
        let u1 = self.f64().max(1e-300);
        let u2 = self.f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }

    pub fn shuffle<T>(&mut self, v: &mut [T]) {
        for i in (1..v.len()).rev() {
            let j = (self.f64() * (i + 1) as f64) as usize;
            v.swap(i, j.min(i));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_matches_stepping() {
        let mut a = Rng::new(7);
        let mut b = a.clone();
        for _ in 0..12_345 {
            a.next_u32();
        }
        b.advance(12_345);
        assert_eq!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn fill_normal_matches_sequential() {
        let n = 200_003;
        let mut a = Rng::new(3);
        let seq: Vec<f32> = (0..n).map(|_| a.normal() as f32).collect();
        let mut b = Rng::new(3);
        let mut par = vec![0.0f32; n];
        b.fill_normal(&mut par);
        assert!(seq
            .iter()
            .zip(&par)
            .all(|(x, y)| x.to_bits() == y.to_bits()));
        assert_eq!(a.next_u32(), b.next_u32());
    }
}
