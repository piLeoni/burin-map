//! Command line: `burin --at "Twin Peaks, San Francisco" --size 20km -o sf.png`.
//! Lives in the library so the Python package can run the very same command.
//!
//! Ground size, paper size and scale are tied together: give any two and
//! the third follows (the log always reports the resulting scale).

use std::path::PathBuf;

use clap::{value_parser, Arg, ArgAction, ArgGroup, ArgMatches, Command};

use crate::geo::{parse_length_km, parse_scale};
use crate::{bounds_around, dem::projected_size, draw, locate, Bounds, Error, Page, Style};

const PLACES: &[(&str, Bounds)] = &[
    ("bern", [7.10, 46.70, 7.80, 47.20]),
    ("matterhorn", [7.40, 45.80, 8.00, 46.20]),
    ("san-francisco", [-122.75, 37.45, -122.05, 38.05]),
];

pub fn command() -> Command {
    let defaults = Style::default();
    let mut cmd = Command::new("burin")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Pen-and-ink terrain map from SRTM elevation data.")
        .after_help("Examples:\n  \
            burin --at \"Twin Peaks, San Francisco\" --size 20km -o sf.png --svg\n  \
            burin --at 45.976,7.658 --size 12km 16km\n  \
            burin --at \"Zermatt\" --page-width 297 --page-height 420 --scale 1:50000 --svg\n  \
            burin --place bern --scale 1:100000 --svg     (page size follows)")
        .next_help_heading("Where")
        .arg(Arg::new("place").long("place").default_value("san-francisco")
            .value_parser(PLACES.iter().map(|p| p.0).collect::<Vec<_>>()).help("named area"))
        .arg(Arg::new("bounds").long("bounds").num_args(4).value_names(["W", "S", "E", "N"])
            .allow_negative_numbers(true).value_parser(value_parser!(f64))
            .help("area in degrees: west south east north"))
        .arg(Arg::new("at").long("at").value_name("WHERE").allow_hyphen_values(true)
            .help("centre: an address or place name, or \"lat,lon\""))
        .group(ArgGroup::new("where").args(["place", "bounds", "at"]))
        .arg(Arg::new("size").long("size").num_args(1..=2).value_names(["WIDTH", "HEIGHT"])
            .help("with --at: ground width [and height], e.g. 20km, 800m, 5mi; a bare number is km. \
                   Height defaults to the page's proportions, else square. Default 30km"))
        .next_help_heading("Output")
        .arg(Arg::new("output").short('o').long("output").value_name("PNG").default_value("burin.png")
            .help("PNG path"))
        .arg(Arg::new("svg").long("svg").action(ArgAction::SetTrue).help("also write an SVG next to the PNG"))
        .arg(Arg::new("width").long("width").value_name("PX").default_value("2400")
            .value_parser(value_parser!(usize))
            .help("image width in px, when no page is given"))
        .next_help_heading("Page (plotter / print): sizes the SVG in mm")
        .arg(Arg::new("page-width").long("page-width").value_name("MM").value_parser(value_parser!(f64))
            .help("drawing width on paper, mm"))
        .arg(Arg::new("page-height").long("page-height").value_name("MM").value_parser(value_parser!(f64))
            .help("max drawing height, mm; the drawing shrinks to fit"))
        .arg(Arg::new("scale").long("scale").value_name("1:N")
            .help("map scale; with --page-width sets the ground size, otherwise the page size"))
        .arg(Arg::new("pen").long("pen").value_name("MM").default_value("0.3").value_parser(value_parser!(f64))
            .help("pen tip width, mm"))
        .next_help_heading("Style");
    for &(name, help) in Style::PARAMS {
        // Through f32 so 4.8 prints as 4.8, not 4.800000190734863.
        let default = (defaults.get(name).unwrap() as f32).to_string();
        cmd = cmd.arg(Arg::new(name).long(name.replace('_', "-")).value_name("X")
            .default_value(default).allow_negative_numbers(true)
            .value_parser(value_parser!(f64)).help(help));
    }
    cmd
}

/// Area to draw and page to draw it on, from the where / size / page / scale options.
fn area_and_page(m: &ArgMatches) -> Result<(Bounds, Page), Error> {
    let mut page = Page {
        width: *m.get_one("width").unwrap(),
        page_width_mm: m.get_one("page-width").copied(),
        page_height_mm: m.get_one("page-height").copied(),
        pen_mm: *m.get_one("pen").unwrap(),
    };
    let scale = m.get_one::<String>("scale").map(|s| parse_scale(s)).transpose()?;
    let size = match m.get_many::<String>("size") {
        Some(v) => {
            let v: Vec<f64> = v.map(|s| parse_length_km(s)).collect::<Result<_, _>>()?;
            Some((v[0], v.get(1).copied()))
        }
        None => None,
    };
    let page_aspect = match (page.page_width_mm, page.page_height_mm) {
        (Some(w), Some(h)) => Some(h / w),
        _ => None,
    };

    let Some(q) = m.get_one::<String>("at") else {
        if size.is_some() {
            return Err(Error::Input("--size only applies with --at".into()));
        }
        let bounds = match m.get_many::<f64>("bounds") {
            Some(b) => {
                let b: Vec<f64> = b.copied().collect();
                [b[0], b[1], b[2], b[3]]
            }
            None => {
                let name = m.get_one::<String>("place").unwrap();
                PLACES.iter().find(|p| p.0 == name).unwrap().1
            }
        };
        if let Some(n) = scale {
            if page.page_width_mm.is_some() {
                return Err(Error::Input(
                    "the area, --page-width and --scale can't all be fixed: drop one".into()));
            }
            page.page_width_mm = Some(projected_size(&bounds).0 * 1000.0 / n);
        }
        return Ok((bounds, page));
    };

    let (w_km, h_km) = match (size, scale, page.page_width_mm) {
        (Some(_), Some(_), Some(_)) => {
            return Err(Error::Input("--size, --page-width and --scale can't all be given: drop one".into()));
        }
        (Some((w, h)), _, _) => (w, h.unwrap_or(w * page_aspect.unwrap_or(1.0))),
        (None, Some(n), Some(pw)) => (pw * n / 1e6, page.page_height_mm.unwrap_or(pw) * n / 1e6),
        (None, Some(_), None) => {
            return Err(Error::Input("with --at, --scale needs --size or --page-width".into()));
        }
        (None, None, _) => (30.0, 30.0 * page_aspect.unwrap_or(1.0)),
    };
    let (lat, lon, label) = locate(q)?;
    println!("{label}\n  {} × {} km", round2(w_km), round2(h_km));
    let bounds = bounds_around(lat, lon, w_km, h_km);
    if let (Some(n), None) = (scale, page.page_width_mm) {
        page.page_width_mm = Some(projected_size(&bounds).0 * 1000.0 / n);
    }
    Ok((bounds, page))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// Run the command with `args` (program name first), printing progress.
pub fn run<I, T>(args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let m = command().try_get_matches_from(args).unwrap_or_else(|e| e.exit());
    let mut st = Style::default();
    for &(name, _) in Style::PARAMS {
        st.set(name, *m.get_one::<f64>(name).unwrap());
    }
    let (bounds, page) = area_and_page(&m)?;
    let map = draw(&bounds, &st, &page, |msg| println!("{msg}"))?;
    let out = PathBuf::from(m.get_one::<String>("output").unwrap());
    std::fs::write(&out, map.to_png()?)?;
    println!("wrote {}", out.display());
    if m.get_flag("svg") {
        let svg = out.with_extension("svg");
        std::fs::write(&svg, map.to_svg())?;
        println!("wrote {}", svg.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(args: &[&str]) -> Result<(Bounds, Page), Error> {
        area_and_page(&command().try_get_matches_from(std::iter::once("burin").chain(args.iter().copied())).unwrap())
    }

    #[test]
    fn cli_is_well_formed() {
        command().debug_assert();
    }

    #[test]
    fn scale_sets_the_missing_side() {
        // Place + scale → page width from the projected area.
        let (b, p) = resolve(&["--place", "bern", "--scale", "1:100000"]).unwrap();
        let w_mm = p.page_width_mm.unwrap();
        assert!((w_mm - projected_size(&b).0 / 100.0).abs() < 1e-6);
        // Coordinates + page + scale → ground from the paper: 297 mm at 1:50,000 = 14.85 km.
        let (b, _) = resolve(&["--at", "46,7.7", "--page-width", "297", "--scale", "1:50000"]).unwrap();
        assert!((projected_size(&b).0 / 1000.0 - 14.85).abs() < 0.1);
    }

    #[test]
    fn overdetermined_is_an_error() {
        assert!(resolve(&["--place", "bern", "--scale", "1:1000", "--page-width", "200"]).is_err());
        assert!(resolve(&["--at", "46,7", "--size", "5km", "--page-width", "200", "--scale", "1:1000"]).is_err());
        assert!(resolve(&["--place", "bern", "--size", "5km"]).is_err());
    }

    #[test]
    fn size_takes_units_and_two_values() {
        let (b, _) = resolve(&["--at", "46,7", "--size", "800m", "1.2km"]).unwrap();
        let (w, h) = projected_size(&b);
        assert!((w - 800.0).abs() < 5.0 && (h - 1200.0).abs() < 5.0, "{w} {h}");
    }
}
