//! Command line: `burin-map --terrain-at "Twin Peaks, San Francisco" --terrain-size 20km --out-file sf.png`.
//! Lives in the library so the Python package can run the very same command.
//!
//! Ground size, frame size and scale are tied together: give any two and
//! the third follows (the log always reports the resulting scale).

use std::ffi::OsString;
use std::io::{IsTerminal, Write};
use std::path::PathBuf;
use std::time::Instant;

use clap::parser::ValueSource;
use clap::{value_parser, Arg, ArgAction, ArgGroup, ArgMatches, Command};

use crate::geo::{parse_length_km, parse_scale};
use crate::progress::latlon;
use crate::document::size_pair;
use crate::{
    bounds_around, dem::projected_size, draw_batches, locate, Batch, Bounds, Document, Error, Frame,
    Progress, Style, TileSource,
};

const PLACES: &[(&str, Bounds)] = &[
    ("bern", [7.10, 46.70, 7.80, 47.20]),
    ("matterhorn", [7.40, 45.80, 8.00, 46.20]),
    ("san-francisco", [-122.75, 37.45, -122.05, 38.05]),
];

/// Values taken by a terrain / frame / out flag.
#[derive(Clone, Copy)]
enum Takes {
    One,
    Four,
    OneOrTwo,
}

/// Flags that describe the whole run rather than one batch: (long, short, values).
/// They go to the shared prefix wherever they appear on the command line.
const GLOBAL: &[(&str, &str, Takes)] = &[
    ("terrain-place", "tp", Takes::One),
    ("terrain-at", "tat", Takes::One),
    ("terrain-bounds", "tbo", Takes::Four),
    ("terrain-size", "ts", Takes::OneOrTwo),
    ("frame-size", "fs", Takes::OneOrTwo),
    ("export-scale", "esc", Takes::One),
    ("pen-width", "pen", Takes::One),
    ("out-file", "of", Takes::One),
    ("out-format", "fmt", Takes::One),
    ("out-geojson", "", Takes::One),
    ("style-file", "sf", Takes::One),
    ("style-json", "sj", Takes::One),
];

fn short_of(long: &str) -> &'static str {
    GLOBAL.iter().find(|g| g.0 == long).unwrap().1
}

pub fn command() -> Command {
    let defaults = Style::default();
    let global = |long: &'static str| Arg::new(long).long(long).visible_alias(short_of(long));
    let mut cmd = Command::new("burin-map")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Pen-and-ink terrain map from SRTM elevation data.")
        .after_help("Examples:\n  \
            burin-map --terrain-at \"Twin Peaks, San Francisco\" --terrain-size 20km --out-file sf.png\n  \
            burin-map --tp bern --s 5 --dl 0.3\n  \
            burin-map --terrain-place bern --batch --space 6 --mod-breaks 0.1 \\\n  \
            \x20   --batch --batch-angle 90 --mod-breaks 0.85 --batch-floor 0.45\n  \
            burin-map --tat Zermatt --fs 297 420 --esc 1:50000 --fmt svg\n  \
            burin-map --style-json '{\"area\":{\"place\":\"bern\"},\"batches\":[{\"angle\":90}]}'")
        .next_help_heading("Terrain — the ground to draw")
        .arg(global("terrain-place").value_name("NAME").default_value("san-francisco")
            .value_parser(PLACES.iter().map(|p| p.0).collect::<Vec<_>>()).help("named area"))
        .arg(global("terrain-at").value_name("WHERE").allow_hyphen_values(true)
            .help("centre: an address or place name, or \"lat,lon\""))
        .arg(global("terrain-bounds").num_args(4).value_names(["W", "S", "E", "N"])
            .allow_negative_numbers(true).value_parser(value_parser!(f64))
            .help("area in degrees: west south east north"))
        .group(ArgGroup::new("terrain").args(["terrain-place", "terrain-at", "terrain-bounds"]))
        .arg(global("terrain-size").num_args(1..=2).value_names(["WIDTH", "HEIGHT"])
            .help("with --terrain-at: ground width [and height], e.g. 20km, 800m, 5mi; a bare number is km. \
                   One value is a square, or takes the paper's proportions. Default 30km"))
        .next_help_heading("Frame — the paper it is drawn on (optional)")
        .arg(global("frame-size").num_args(1..=2).value_names(["WIDTH", "HEIGHT"])
            .value_parser(value_parser!(f64))
            .help("paper width [and height] in mm; one value is a square. \
                   Give the ground size or the scale with it, not both"))
        .arg(global("export-scale").value_name("1:N")
            .help("map scale, e.g. 1:25000; with --terrain-size it sets the paper, with --frame-size the ground"))
        .arg(global("pen-width").value_name("MM").default_value("0.3").value_parser(value_parser!(f64))
            .help("pen width on paper, mm (stroke-width in SVG, does not affect line density)"))
        .next_help_heading("Out — what gets written")
        .arg(global("out-file").value_name("PATH").default_value("burin-map.png")
            .help("output path"))
        .arg(global("out-format").value_name("FMT").value_parser(["png", "svg", "both", "geojson"])
            .help("png, svg, both, or geojson; default from the --out-file extension"))
        .arg(global("out-geojson").value_name("PATH").value_parser(value_parser!(PathBuf))
            .help("write a GeoJSON FeatureCollection with geographic coordinates"))
        .next_help_heading("Batch — parallel drawing passes")
        .arg(Arg::new("batch").long("batch").short('b').action(ArgAction::Append).num_args(0)
            .help("start a pass (may repeat). Flags after each --batch apply to that pass until the \
                   next one; flags before the first --batch are shared. Without --batch there is one pass"))
        .arg(Arg::new("batch-angle").long("batch-angle").visible_alias("ba").value_name("DEG")
            .default_value("0").allow_negative_numbers(true).value_parser(value_parser!(f64))
            .help("line direction: 0 follows the contour lines, 90 runs straight downhill"))
        .arg(Arg::new("batch-name").long("batch-name").visible_alias("bn").value_name("NAME")
            .help("label for this batch, shown in the log"))
        .arg(global("style-file").value_name("PATH").value_parser(value_parser!(PathBuf))
            .help("JSON or JSON5 document (comments allowed) with an optional area block and the batch passes"))
        .arg(global("style-json").value_name("JSON")
            .help("inline JSON or JSON5 document (or a bare list of batches)"));

    let mut last_group = "";
    for p in Style::PARAMS {
        if p.group != last_group {
            let heading = match p.group {
                "space" => "Space — distance between lines",
                "dash" => "Dash — ink along each line",
                "mod" => "Mod — hand-drawn character",
                "input" => "Input — 0…1 field from terrain + sun",
                "sun" => "Sun",
                "engine" => "Engine — streamline tracer",
                "batch" => "Batch style — per-pass floor, stroke, seed",
                other => other,
            };
            cmd = cmd.next_help_heading(heading);
            last_group = p.group;
        }
        let default = (defaults.get(p.name).unwrap() as f32).to_string();
        cmd = cmd.arg(
            Arg::new(p.name)
                .long(p.name.replace('_', "-"))
                .visible_alias(p.short)
                .value_name("X")
                .default_value(default)
                .allow_negative_numbers(true)
                .value_parser(value_parser!(f64))
                .help(p.help),
        );
    }
    cmd
}

/// Area to draw, frame to draw it in and a name for the area, from the
/// `san-francisco` → `San Francisco`.
fn place_title(name: &str) -> String {
    name.split('-').map(|w| w[..1].to_uppercase() + &w[1..]).collect::<Vec<_>>().join(" ")
}

/// Area to draw, frame to draw it in and a name for the area.
///
/// Explicit command-line flags always win. Anything left unset falls back to
/// the area block of the JSON document (`--style-json` / `--style-file`).
fn area_and_frame(m: &ArgMatches, doc: &Document) -> Result<(Bounds, Frame, String), Error> {
    let area = doc.area.clone().unwrap_or_default();
    let given = |name: &str| {
        matches!(m.value_source(name), Some(ValueSource::CommandLine))
    };

    let paper = if given("frame-size") {
        m.get_many::<f64>("frame-size").map(|v| v.copied().collect::<Vec<_>>())
    } else {
        area.size_mm.clone()
    };
    let paper = match paper {
        Some(v) => Some(size_pair(&v).filter(|p| p.iter().all(|x| *x > 0.0)).ok_or_else(|| {
            Error::Input("the paper size needs 1 or 2 positive numbers in mm".into())
        })?),
        None => None,
    };
    let mut frame = Frame {
        size_mm: paper,
        pen_width: if given("pen-width") { *m.get_one("pen-width").unwrap() } else { area.pen_mm.unwrap_or(0.3) },
        ..Frame::default()
    };
    let scale = if given("export-scale") {
        m.get_one::<String>("export-scale").map(|s| parse_scale(s)).transpose()?
    } else {
        area.scale
    };
    let size = if given("terrain-size") {
        match m.get_many::<String>("terrain-size") {
            Some(v) => {
                let v: Vec<f64> = v.map(|s| parse_length_km(s)).collect::<Result<_, _>>()?;
                Some((v[0], v.get(1).copied()))
            }
            None => None,
        }
    } else {
        area.size_km.as_ref().filter(|s| !s.is_empty()).map(|s| (s[0], s.get(1).copied()))
    };
    let frame_aspect = frame.size_mm.map(|[w, h]| h / w);
    // Paper for a ground of the given projected size at 1:n.
    let paper_for = |b: &Bounds, n: f64| {
        let (pw, ph) = projected_size(b);
        [pw * 1000.0 / n, ph * 1000.0 / n]
    };

    // Where: explicit bounds, a named place, or a centre to look up. CLI first,
    // then the document's area block.
    let at = if given("terrain-at") {
        m.get_one::<String>("terrain-at").cloned()
    } else {
        area.at.clone()
    };
    let cli_bounds = m.get_many::<f64>("terrain-bounds").map(|b| {
        let b: Vec<f64> = b.copied().collect();
        [b[0], b[1], b[2], b[3]]
    });
    let place = if given("terrain-place") {
        m.get_one::<String>("terrain-place").cloned()
    } else {
        area.place.clone().or_else(|| m.get_one::<String>("terrain-place").cloned())
    };

    // A fixed area (bounds or place) needs no size at all: the ground is known.
    let fixed: Option<Bounds> = cli_bounds.or(area.bounds).or_else(|| {
        // Only treat the place as fixed when the CLI default wasn't overridden
        // by an explicit --terrain-at.
        if at.is_none() {
            place.as_ref().and_then(|name| PLACES.iter().find(|p| p.0 == name).map(|p| p.1))
        } else {
            None
        }
    });

    let Some(q) = at else {
        if size.is_some() {
            return Err(Error::Input("--terrain-size only applies with --terrain-at".into()));
        }
        let (bounds, label) = match fixed {
            Some(b) => {
                if cli_bounds.is_some() {
                    (b, format!("{} to {}", latlon(b[1], b[0]), latlon(b[3], b[2])))
                } else {
                    let name = place.as_deref().unwrap_or("san-francisco");
                    (b, place_title(name))
                }
            }
            None => {
                let name = place.as_deref().unwrap_or("san-francisco");
                (PLACES.iter().find(|p| p.0 == name).map(|p| p.1).unwrap_or(PLACES[0].1), place_title(name))
            }
        };
        if let Some(n) = scale {
            if frame.size_mm.is_some() {
                return Err(Error::Input(
                    "the area, --frame-size and --export-scale can't all be fixed: drop one".into()));
            }
            frame.size_mm = Some(paper_for(&bounds, n));
        }
        return Ok((bounds, frame, label));
    };

    let (w_km, h_km) = match (size, scale, frame.size_mm) {
        (Some(_), Some(_), Some(_)) => {
            return Err(Error::Input(
                "--terrain-size, --frame-size and --export-scale can't all be given: drop one".into()));
        }
        (Some((w, h)), _, _) => (w, h.unwrap_or(w * frame_aspect.unwrap_or(1.0))),
        (None, Some(n), Some([fw, fh])) => (fw * n / 1e6, fh * n / 1e6),
        (None, Some(_), None) => {
            return Err(Error::Input(
                "with --terrain-at, --export-scale needs --terrain-size or --frame-size".into()));
        }
        (None, None, _) => (30.0, 30.0 * frame_aspect.unwrap_or(1.0)),
    };
    let (lat, lon, label) = locate(&q)?;
    // Degrees per km is approximate; correct against the projection so the
    // area (and any scale derived from it) comes out as asked.
    let mut bounds = bounds_around(lat, lon, w_km, h_km);
    for _ in 0..2 {
        let (pw, ph) = projected_size(&bounds);
        let (w0, s0, e0, n0) = (bounds[0], bounds[1], bounds[2], bounds[3]);
        let (fx, fy) = (w_km * 1000.0 / pw, h_km * 1000.0 / ph);
        let (cx, cy, hx, hy) = ((w0 + e0) / 2.0, (s0 + n0) / 2.0, (e0 - w0) / 2.0 * fx, (n0 - s0) / 2.0 * fy);
        bounds = [cx - hx, cy - hy, cx + hx, cy + hy];
    }
    if let (Some(n), None) = (scale, frame.size_mm) {
        frame.size_mm = Some(paper_for(&bounds, n));
    }
    Ok((bounds, frame, label))
}

/// Coloured, aligned progress on stdout. Colour is dropped automatically
/// when stdout isn't a terminal or NO_COLOR is set.
struct Report {
    live: bool,
}

const LABEL: Style_ = Style_::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)));
const DONE: Style_ = Style_::new().bold().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
const DIM: Style_ = Style_::new().dimmed();
const BOLD: Style_ = Style_::new().bold();
use anstyle::Style as Style_;

impl Report {
    fn new() -> Self {
        Report { live: std::io::stdout().is_terminal() }
    }

    /// One step: label column, message, optional time on the right.
    fn step(&self, label: &str, msg: &str, secs: Option<f64>) {
        let clear = if self.live { "\r\x1b[2K" } else { "" };
        let time = secs.map(|s| format!("{DIM}{s:>5.1} s{DIM:#}")).unwrap_or_default();
        let mut out = anstream::stdout();
        let _ = writeln!(out, "{clear}  {LABEL}{label:<10}{LABEL:#} {msg:<62}{time}");
    }

    /// Continuation line under the previous step, dimmed.
    fn detail(&self, msg: &str) {
        let _ = writeln!(anstream::stdout(), "  {:<10} {DIM}{msg}{DIM:#}", "");
    }

    /// Overwritten in place on a terminal, skipped otherwise.
    fn transient(&self, label: &str, msg: &str) {
        if self.live {
            let mut out = anstream::stdout();
            let _ = write!(out, "\r\x1b[2K  {LABEL}{label:<10}{LABEL:#} {DIM}{msg}{DIM:#}");
            let _ = out.flush();
        }
    }

    fn progress(&self, p: &Progress) {
        use crate::progress::{megabytes, thousands};
        match *p {
            Progress::Tile { name, source, done, total } => {
                let what = match source {
                    TileSource::Cache => "from cache".to_string(),
                    TileSource::Download(b) => format!("downloaded ({})", megabytes(b)),
                    TileSource::Sea => "open sea".to_string(),
                };
                self.transient("elevation", &format!("SRTM tile {done} of {total}: {name} {what}"));
            }
            Progress::Elevation { tiles, downloaded, bytes, width, height, px_m, highest_m, secs, .. } => {
                let plural = if tiles == 1 { "" } else { "s" };
                let src = match downloaded {
                    0 => format!("{tiles} SRTM tile{plural}, all from cache"),
                    d if d == tiles => format!("{tiles} SRTM tile{plural} downloaded ({})", megabytes(bytes)),
                    d => format!("{tiles} SRTM tiles: {d} downloaded ({}), {} from cache", megabytes(bytes), tiles - d),
                };
                self.step("elevation", &src, Some(secs));
                self.detail(&format!("projected onto {width} × {height} px, {px_m:.0} m per pixel, \
                                      highest point {highest_m:.0} m"));
            }
            Progress::Frame { width_mm, height_mm, scale, pen_width, pitch_mm } => {
                self.step("frame", &format!("{width_mm:.0} × {height_mm:.0} mm at {}",
                                            crate::progress::scale(scale)), None);
                self.detail(&format!("stroke {pen_width} mm, lines {pitch_mm:.2} mm apart"));
            }
            Progress::Relief { secs } => self.step("relief", "light and shadow, slope, line directions", Some(secs)),
            Progress::Lines { count, secs } => {
                self.step("lines", &format!("{} lines traced along the contours", thousands(count as u64)), Some(secs))
            }
            Progress::Dashes { count, secs } => {
                self.step("dashes", &format!("{} dashes", thousands(count as u64)), Some(secs))
            }
        }
    }
}

/// Run the command with `args` (program name first), printing progress.
pub fn run<I, T>(args: I) -> Result<(), Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: Vec<OsString> = args.into_iter().map(Into::into).collect();
    let (prefix, sections) = split_batch_argv(&args)?;
    let m = command().try_get_matches_from(prefix).unwrap_or_else(|e| e.exit());

    // Parse the document (from --style-json / --style-file) once. A bare
    // batch array is accepted too, so older invocations keep working.
    let doc: Document = if let Some(json) = m.get_one::<String>("style-json") {
        Document::parse(json).map_err(|e| Error::Input(format!("invalid --style-json: {e}")))?
    } else if let Some(path) = m.get_one::<PathBuf>("style-file") {
        let s = std::fs::read_to_string(path)
            .map_err(|e| Error::Input(format!("reading {}: {e}", path.display())))?;
        Document::parse(&s).map_err(|e| Error::Input(format!("invalid --style-file: {e}")))?
    } else {
        Document::default()
    };

    let batches: Vec<Batch> = if !doc.batches.is_empty() {
        doc.batches.clone()
    } else if let Some(sections) = sections {
        let base = shared_style(&m);
        sections
            .iter()
            .map(|sec| parse_batch_section(&base, sec))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let mut st = Style::default();
        for p in Style::PARAMS {
            st.set(p.name, *m.get_one::<f64>(p.name).unwrap());
        }
        vec![Batch {
            name: m.get_one::<String>("batch-name").cloned(),
            angle: *m.get_one::<f64>("batch-angle").unwrap() as f32,
            style: st,
        }]
    };

    let r = Report::new();
    if m.get_one::<String>("terrain-at").is_some() || doc.area.as_ref().is_some_and(|a| a.at.is_some()) {
        r.transient("where", "looking up the place");
    }
    let (bounds, frame, label) = area_and_frame(&m, &doc)?;
    let (wm, hm) = projected_size(&bounds);
    let centre = latlon((bounds[1] + bounds[3]) / 2.0, (bounds[0] + bounds[2]) / 2.0);
    r.step("where", &format!("{BOLD}{label}{BOLD:#}"), None);
    r.detail(&format!("{:.1} × {:.1} km around {centre}", wm / 1000.0, hm / 1000.0));
    if batches.len() > 1 || batches[0].angle != 0.0 || batches[0].name.is_some() {
        let desc: Vec<String> = batches
            .iter()
            .map(|b| match &b.name {
                Some(n) => format!("{n} {:.0}°", b.angle),
                None => format!("{:.0}°", b.angle),
            })
            .collect();
        r.detail(&format!("{} batch passes ({})", batches.len(), desc.join(", ")));
    }

    let t = Instant::now();
    let map = draw_batches(&bounds, &batches, &frame, |p| r.progress(p))?;
    let out = PathBuf::from(m.get_one::<String>("out-file").unwrap());
    let d = &map.drawing;
    let is_svg = out.extension().map(|e| e == "svg").unwrap_or(false);
    let (want_png, want_svg) = match m.get_one::<String>("out-format").map(|s| s.as_str()) {
        Some("png") => (true, false),
        Some("svg") => (false, true),
        Some("geojson") => (false, false),
        Some(_) => (true, true),
        None => (!is_svg, is_svg),
    };
    let mut files = Vec::new();
    if want_png {
        let png = out.with_extension("png");
        std::fs::write(&png, map.to_png()?)?;
        files.push((png, format!("{} × {} px", d.width, d.height)));
    }
    if want_svg {
        let svg = out.with_extension("svg");
        std::fs::write(&svg, map.to_svg())?;
        let size = match map.mm_per_px {
            Some(k) => format!("{:.0} × {:.0} mm", d.width as f64 * k, d.height as f64 * k),
            None => format!("{} × {} px", d.width, d.height),
        };
        files.push((svg, size));
    }
    if want_png || want_svg {
        // already handled above
    }
    if let Some(geojson_path) = m.get_one::<PathBuf>("out-geojson") {
        let text = map.to_geojson(&bounds);
        std::fs::write(geojson_path, text)?;
        r.detail(&format!("GeoJSON written to {}", geojson_path.display()));
    }
    let mut out = anstream::stdout();
    for (i, (path, size)) in files.iter().enumerate() {
        let label = if i == 0 { "wrote" } else { "" };
        let _ = writeln!(
            out,
            "  {DONE}{label:<10}{DONE:#} {BOLD}{}{BOLD:#}  {DIM}{size}{DIM:#}",
            path.display()
        );
    }
    let _ = writeln!(out, "  {:<10} {DIM}done in {:.1} s{DIM:#}", "", t.elapsed().as_secs_f64());
    Ok(())
}

/// Style from the flags before the first `--batch`: defaults, overridden by
/// whatever was given on the command line.
fn shared_style(m: &ArgMatches) -> Style {
    let mut st = Style::default();
    for p in Style::PARAMS {
        if m.value_source(p.name) == Some(ValueSource::CommandLine) {
            st.set(p.name, *m.get_one::<f64>(p.name).unwrap());
        }
    }
    st
}

/// Split `argv` on `--batch` / `-b`. Terrain, frame and out flags always go
/// to the clap prefix, even after a `--batch`; style flags stay in the
/// current batch section.
fn split_batch_argv(args: &[OsString]) -> Result<(Vec<OsString>, Option<Vec<Vec<OsString>>>), Error> {
    let mut prefix = Vec::new();
    let mut sections: Option<Vec<Vec<OsString>>> = None;
    let mut cur: Option<Vec<OsString>> = None;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if i == 0 {
            prefix.push(a.clone());
            i += 1;
            continue;
        }
        if a == "--batch" || a == "-b" {
            let secs = sections.get_or_insert_with(Vec::new);
            if let Some(c) = cur.take() {
                secs.push(c);
            }
            cur = Some(Vec::new());
            i += 1;
            continue;
        }
        let flag = a.to_string_lossy();
        if cur.is_some() {
            if let Some(n) = global_value_count(&flag, &args[i + 1..]) {
                let end = (i + 1 + n).min(args.len());
                prefix.extend_from_slice(&args[i..end]);
                i = end;
                continue;
            }
        }
        match cur {
            Some(ref mut c) => c.push(a.clone()),
            None => prefix.push(a.clone()),
        }
        i += 1;
    }
    if let Some(c) = cur {
        sections.get_or_insert_with(Vec::new).push(c);
    }
    Ok((prefix, sections))
}

/// For a run-wide flag, how many value tokens follow it; `None` for anything else.
fn global_value_count(flag: &str, rest: &[OsString]) -> Option<usize> {
    if matches!(flag, "--help" | "-h" | "--version" | "-V") {
        return Some(0);
    }
    let name = flag.strip_prefix("--")?;
    let takes = GLOBAL.iter().find(|g| g.0 == name || g.1 == name)?.2;
    Some(match takes {
        Takes::One => 1,
        Takes::Four => 4,
        Takes::OneOrTwo => {
            let n = rest.iter().take(2).take_while(|a| !a.to_string_lossy().starts_with("--")).count();
            n.max(1)
        }
    })
}

fn parse_batch_section(base: &Style, args: &[OsString]) -> Result<Batch, Error> {
    let mut st = base.clone();
    let mut angle = 0.0f64;
    let mut label = None;
    let mut i = 0;
    while i < args.len() {
        let flag = args[i].to_string_lossy();
        let Some(name) = flag.strip_prefix("--") else {
            return Err(Error::Input(format!("unexpected {flag:?} in --batch section")));
        };
        let name = name.replace('-', "_");
        let raw = args.get(i + 1).ok_or_else(|| Error::Input(format!("missing value for {flag}")))?;
        let raw = raw.to_string_lossy();
        i += 2;
        if name == "batch_name" || name == "bn" {
            label = Some(raw.into_owned());
            continue;
        }
        let v: f64 = raw.parse().map_err(|_| Error::Input(format!("bad value {raw:?} for {flag}")))?;
        if name == "batch_angle" || name == "ba" {
            angle = v;
        } else if !st.set(&name, v) {
            return Err(Error::Input(format!("unknown batch parameter {flag}")));
        }
    }
    Ok(Batch { name: label, angle: angle as f32, style: st })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(args: &[&str]) -> Result<(Bounds, Frame), Error> {
        let m = command().try_get_matches_from(std::iter::once("burin-map").chain(args.iter().copied())).unwrap();
        area_and_frame(&m, &Document::default()).map(|(b, f, _)| (b, f))
    }

    fn argv(args: &[&str]) -> Vec<OsString> {
        std::iter::once("burin-map").chain(args.iter().copied()).map(Into::into).collect()
    }

    #[test]
    fn cli_is_well_formed() {
        command().debug_assert();
    }

    #[test]
    fn scale_sets_the_missing_side() {
        let (b, f) = resolve(&["--terrain-place", "bern", "--export-scale", "1:100000"]).unwrap();
        let [w_mm, h_mm] = f.size_mm.unwrap();
        let (pw, ph) = projected_size(&b);
        assert!((w_mm - pw / 100.0).abs() < 1e-6 && (h_mm - ph / 100.0).abs() < 1e-6);
        let (b, _) = resolve(&["--tat", "46,7.7", "--frame-size", "297", "420", "--export-scale", "1:50000"]).unwrap();
        let (pw, ph) = projected_size(&b);
        assert!((pw / 1000.0 - 14.85).abs() < 0.1 && (ph / 1000.0 - 21.0).abs() < 0.1, "{pw} {ph}");
    }

    #[test]
    fn one_paper_value_is_a_square() {
        let (b, f) = resolve(&["--tat", "46,7.7", "--frame-size", "200", "--export-scale", "1:10000"]).unwrap();
        assert_eq!(f.size_mm, Some([200.0, 200.0]));
        let (pw, ph) = projected_size(&b);
        assert!((pw - 2000.0).abs() < 10.0 && (ph - 2000.0).abs() < 10.0, "{pw} {ph}");
    }

    #[test]
    fn overdetermined_is_an_error() {
        assert!(resolve(&["--tp", "bern", "--export-scale", "1:1000", "--frame-size", "200"]).is_err());
        assert!(resolve(&["--tat", "46,7", "--ts", "5km", "--frame-size", "200", "--export-scale", "1:1000"]).is_err());
        assert!(resolve(&["--tp", "bern", "--ts", "5km"]).is_err());
        assert!(resolve(&["--tat", "46,7", "--export-scale", "1:1000"]).is_err());
    }

    #[test]
    fn old_frame_flags_are_gone() {
        for old in ["--frame-px", "--frame-width", "--frame-height", "--fpx", "--fw", "--fh",
                    "--frame-scale", "--fsc", "--frame-pen", "--fpen"] {
            assert!(command().try_get_matches_from(argv(&[old, "100"])).is_err(), "{old}");
        }
    }

    #[test]
    fn size_takes_units_and_two_values() {
        let (b, _) = resolve(&["--terrain-at", "46,7", "--terrain-size", "800m", "1.2km"]).unwrap();
        let (w, h) = projected_size(&b);
        assert!((w - 800.0).abs() < 5.0 && (h - 1200.0).abs() < 5.0, "{w} {h}");
    }

    #[test]
    fn short_and_long_style_flags_agree() {
        let m = command().try_get_matches_from(argv(&["--s", "6", "--dash-light", "0.3"])).unwrap();
        let st = shared_style(&m);
        assert!((st.space - 6.0).abs() < 1e-6);
        assert!((st.dash_light - 0.3).abs() < 1e-6);
    }

    #[test]
    fn old_names_are_rejected() {
        for old in ["--spacing", "--duty-min", "--place", "--svg", "--hatch", "--shade-gap"] {
            assert!(command().try_get_matches_from(argv(&[old, "1"])).is_err(), "{old}");
        }
    }

    #[test]
    fn batch_sections_parse() {
        let args = argv(&[
            "--tp", "bern", "--space", "6",
            "--batch", "--mod-breaks", "0.1", "--of", "x.png",
            "-b", "--batch-name", "Steep valleys", "--ba", "90", "--mb", "0.9", "--batch-floor", "0.4",
        ]);
        let (prefix, secs) = split_batch_argv(&args).unwrap();
        let secs = secs.unwrap();
        assert_eq!(secs.len(), 2);
        let m = command().try_get_matches_from(prefix).unwrap();
        assert_eq!(m.get_one::<String>("out-file").unwrap(), "x.png");
        let base = shared_style(&m);
        assert!((base.space - 6.0).abs() < 1e-6);
        let b0 = parse_batch_section(&base, &secs[0]).unwrap();
        let b1 = parse_batch_section(&base, &secs[1]).unwrap();
        assert_eq!(b0.angle, 0.0);
        assert_eq!(b0.name, None);
        assert_eq!(b1.name.as_deref(), Some("Steep valleys"));
        assert!((b0.style.mod_breaks - 0.1).abs() < 1e-6);
        assert!((b0.style.space - 6.0).abs() < 1e-6);
        assert_eq!(b1.angle, 90.0);
        assert!((b1.style.mod_breaks - 0.9).abs() < 1e-6);
        assert!((b1.style.batch_floor - 0.4).abs() < 1e-6);
        assert!(parse_batch_section(&base, &["--breaks".into(), "0.2".into()]).is_err());
    }
}
