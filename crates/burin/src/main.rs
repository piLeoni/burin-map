//! Command line: `burin --at "Twin Peaks, San Francisco" --size 20 -o sf.png`.

use std::path::PathBuf;
use std::process::ExitCode;

use burin::{bounds_around, draw, locate, Bounds, Page, Style};
use clap::{value_parser, Arg, ArgAction, ArgGroup, Command};

const PLACES: &[(&str, Bounds)] = &[
    ("bern", [7.10, 46.70, 7.80, 47.20]),
    ("matterhorn", [7.40, 45.80, 8.00, 46.20]),
    ("san-francisco", [-122.75, 37.45, -122.05, 38.05]),
];

fn command() -> Command {
    let defaults = Style::default();
    let mut cmd = Command::new("burin")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Pen-and-ink terrain map from SRTM elevation data.")
        .arg(Arg::new("place").long("place").default_value("san-francisco")
            .value_parser(PLACES.iter().map(|p| p.0).collect::<Vec<_>>()).help("named area"))
        .arg(Arg::new("bounds").long("bounds").num_args(4).value_names(["W", "S", "E", "N"])
            .allow_negative_numbers(true).value_parser(value_parser!(f64))
            .help("area in degrees: west south east north"))
        .arg(Arg::new("at").long("at").value_name("WHERE").allow_hyphen_values(true)
            .help("centre: an address or place name, or \"lat,lon\""))
        .group(ArgGroup::new("where").args(["place", "bounds", "at"]))
        .arg(Arg::new("size").long("size").value_name("KM").default_value("30")
            .value_parser(value_parser!(f64))
            .help("with --at: area width in km (height follows the page, else square)"))
        .arg(Arg::new("output").short('o').long("output").default_value("burin.png").help("PNG path"))
        .arg(Arg::new("svg").long("svg").action(ArgAction::SetTrue).help("also write an SVG next to the PNG"))
        .arg(Arg::new("width").long("width").default_value("2400").value_parser(value_parser!(usize))
            .help("output width in px (height follows the area's aspect)"))
        .next_help_heading("Page (plotter / print); overrides --width and sizes the SVG in mm")
        .arg(Arg::new("page-width").long("page-width").value_name("MM").value_parser(value_parser!(f64))
            .help("drawing width on paper, mm"))
        .arg(Arg::new("page-height").long("page-height").value_name("MM").value_parser(value_parser!(f64))
            .help("max drawing height, mm; the drawing shrinks to fit"))
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

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let m = command().get_matches();
    let mut st = Style::default();
    for &(name, _) in Style::PARAMS {
        st.set(name, *m.get_one::<f64>(name).unwrap());
    }
    let page = Page {
        width: *m.get_one("width").unwrap(),
        page_width_mm: m.get_one("page-width").copied(),
        page_height_mm: m.get_one("page-height").copied(),
        pen_mm: *m.get_one("pen").unwrap(),
    };

    let bounds: Bounds = if let Some(q) = m.get_one::<String>("at") {
        let (lat, lon, label) = locate(q)?;
        let size: f64 = *m.get_one("size").unwrap();
        let aspect = match (page.page_width_mm, page.page_height_mm) {
            (Some(w), Some(h)) => h / w,
            _ => 1.0,
        };
        println!("{label}\n  {size} × {} km", (size * aspect * 100.0).round() / 100.0);
        bounds_around(lat, lon, size, size * aspect)
    } else if let Some(b) = m.get_many::<f64>("bounds") {
        let b: Vec<f64> = b.copied().collect();
        [b[0], b[1], b[2], b[3]]
    } else {
        let name = m.get_one::<String>("place").unwrap();
        PLACES.iter().find(|p| p.0 == name).unwrap().1
    };

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

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("burin: {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn cli_is_well_formed() {
        super::command().debug_assert();
    }
}
