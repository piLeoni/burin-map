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

use burin_map_core::panel::Event;
use burin_map_core::{Stage, Step};

use crate::document::PLACES;
use crate::geo::{parse_length_km, parse_scale};
use crate::panel::Panel;
use crate::progress::relief;
use crate::{dem::projected_size, Batch, Bounds, Document, Error, Frame, Progress, Style};

/// Values taken by a terrain / frame / out flag.
#[derive(Clone, Copy)]
enum Takes {
    Zero,
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
    ("script", "", Takes::One),
    ("style-file", "sf", Takes::One),
    ("style-json", "sj", Takes::One),
    ("water", "", Takes::One),
    ("force-cpu", "", Takes::Zero),
];

fn short_of(long: &str) -> &'static str {
    GLOBAL.iter().find(|g| g.0 == long).unwrap().1
}

pub fn command() -> Command {
    let defaults = Style::default();
    let global = |long: &'static str| {
        let arg = Arg::new(long).long(long);
        match short_of(long) {
            "" => arg,
            short => arg.visible_alias(short),
        }
    };
    let mut cmd = Command::new("burin-map")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Pen-and-ink terrain map from elevation tiles.")
        .after_help("Examples:\n  \
            burin-map --script presets/contours.js --out-file sf.png\n  \
            burin-map --terrain-at \"Twin Peaks, San Francisco\" --terrain-size 20km --out-file sf.png\n  \
            burin-map --tp bern --sl 5 --dl 0.3\n  \
            burin-map --terrain-place bern --batch --space-light 6 --mod-breaks 0.1 \\\n  \
            \x20   --batch --batch-angle 90 --mod-breaks 0.85 --batch-floor 0.45\n  \
            burin-map --tat Zermatt --fs 297 420 --esc 1:50000 --fmt svg")
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
            .help("pen width on paper, mm; with a paper size, a finer pen means more lines on the sheet"))
        .next_help_heading("Out — what gets written")
        .arg(global("out-file").value_name("PATH").default_value("burin-map.png")
            .help("output path"))
        .arg(global("out-format").value_name("FMT").value_parser(["png", "svg", "both", "geojson"])
            .help("png, svg, both, or geojson; default from the --out-file extension"))
        .arg(global("out-geojson").value_name("PATH").value_parser(value_parser!(PathBuf))
            .help("write a GeoJSON FeatureCollection with geographic coordinates"))
        .arg(global("force-cpu").action(ArgAction::SetTrue)
            .help("build the terrain fields on the CPU even when a GPU is available. Slower, but \
                   bit-for-bit reproducible, unlike the GPU"))
        .next_help_heading("Document — what to draw")
        .arg(global("script").value_name("PATH").value_parser(value_parser!(PathBuf))
            .help("a JavaScript document (`.js`): it builds the batches and water passes the engine \
                   draws, and may set the area like a JSON document. The main path"))
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
            .help("JSON or JSON5 document (comments allowed) with an optional area block and the batch \
                   passes; an escape hatch for a saved atlas, not the main path — see --script"))
        .arg(global("style-json").value_name("JSON")
            .help("inline JSON or JSON5 document (or a bare list of batches)"))
        .next_help_heading("Water — sea, lakes and rivers from OpenStreetMap")
        .arg(global("water").value_name("JSON")
            .help("the document's water block, as JSON or JSON5: '{}' for the shoreline alone, or \
                   '{\"shoreline\": {\"lines\": 2}, \"batches\": [{\"follow\": \"shore\"}]}'. \
                   It also decides where the land is, instead of the height"));

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
                "water" => "Water style — only in water batches (inside --water)",
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

/// Area to draw, frame to draw it in and a name for the area.
///
/// Explicit command-line flags always win. Anything left unset falls back to
/// the document's area block (`burin.area(...)` in a script).
fn area_and_frame(m: &ArgMatches, doc: &Document) -> Result<(Bounds, Frame, String), Error> {
    let mut area = doc.area.clone().unwrap_or_default();
    let given = |name: &str| matches!(m.value_source(name), Some(ValueSource::CommandLine));
    // Where: one of bounds, a centre or a named place replaces the others. A
    // fixed area also drops the document's ground size, which is the size
    // around its centre.
    if let Some(b) = m.get_many::<f64>("terrain-bounds") {
        let b: Vec<f64> = b.copied().collect();
        area.bounds = Some([b[0], b[1], b[2], b[3]]);
        (area.at, area.place, area.size_km) = (None, None, None);
    }
    if given("terrain-at") {
        area.at = m.get_one::<String>("terrain-at").cloned();
        (area.bounds, area.place) = (None, None);
    }
    if given("terrain-place") {
        area.place = m.get_one::<String>("terrain-place").cloned();
        (area.at, area.bounds, area.size_km) = (None, None, None);
    }
    if given("terrain-size") {
        let v = m.get_many::<String>("terrain-size").unwrap();
        area.size_km = Some(v.map(|s| parse_length_km(s)).collect::<Result<_, _>>()?);
    }
    if given("frame-size") {
        area.size_mm = m.get_many::<f64>("frame-size").map(|v| v.copied().collect());
    }
    if given("export-scale") {
        area.scale = Some(parse_scale(m.get_one::<String>("export-scale").unwrap())?);
    }
    if given("pen-width") {
        area.pen_mm = m.get_one::<f64>("pen-width").copied();
    }
    area.resolve()
}

/// Coloured, aligned progress on stdout. Colour is dropped automatically
/// when stdout isn't a terminal or NO_COLOR is set.
struct Report {
    live: bool,
}

const LABEL: Style_ = Style_::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Cyan)));
const DONE: Style_ = Style_::new()
    .bold()
    .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green)));
const DIM: Style_ = Style_::new().dimmed();
const BOLD: Style_ = Style_::new().bold();
use anstyle::Style as Style_;

impl Report {
    fn new() -> Self {
        Report {
            live: std::io::stdout().is_terminal(),
        }
    }

    /// One step: label column, message, optional time on the right.
    fn step(&self, label: &str, msg: &str, secs: Option<f64>) {
        let clear = if self.live { "\r\x1b[2K" } else { "" };
        let time = secs
            .map(|s| format!("{DIM}{s:>5.1} s{DIM:#}"))
            .unwrap_or_default();
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
            let _ = write!(
                out,
                "\r\x1b[2K  {LABEL}{label:<10}{LABEL:#} {DIM}{msg}{DIM:#}"
            );
            let _ = out.flush();
        }
    }

    fn progress(&self, p: &Progress) {
        use crate::progress::thousands;
        match *p {
            Progress::Tile { done, total } => {
                self.transient("elevation", &format!("terrain tile {done} of {total}"));
            }
            Progress::Elevation {
                tiles,
                zoom,
                width,
                height,
                px_m,
                highest_m,
                deepest_m,
                secs,
                ..
            } => {
                let plural = if tiles == 1 { "" } else { "s" };
                self.step(
                    "elevation",
                    &format!("{tiles} terrain tile{plural} at zoom {zoom}"),
                    Some(secs),
                );
                self.detail(&format!(
                    "projected onto {width} × {height} px, {px_m:.0} m per pixel, {}",
                    relief(highest_m, deepest_m)
                ));
            }
            Progress::WaterTile { done, total } => {
                self.transient("water", &format!("OpenStreetMap tile {done} of {total}"));
            }
            Progress::Water { share, secs } => self.step(
                "water",
                &format!("OpenStreetMap, {:.0}% of the area", share * 100.0),
                Some(secs),
            ),
            Progress::Frame {
                width_mm,
                height_mm,
                scale,
                pen_width,
                pitch_mm,
            } => {
                self.step(
                    "frame",
                    &format!(
                        "{width_mm:.0} × {height_mm:.0} mm at {}",
                        crate::progress::scale(scale)
                    ),
                    None,
                );
                self.detail(&format!(
                    "stroke {pen_width} mm, lines {pitch_mm:.2} mm apart"
                ));
            }
            Progress::Relief { secs } => self.step(
                "relief",
                "light and shadow, slope, line directions",
                Some(secs),
            ),
            Progress::Lines { count, secs } => self.step(
                "lines",
                &format!(
                    "{} lines traced along the contours",
                    thousands(count as u64)
                ),
                Some(secs),
            ),
            Progress::Dashes { count, secs } => self.step(
                "dashes",
                &format!("{} dashes", thousands(count as u64)),
                Some(secs),
            ),
            Progress::Step { .. } => {}
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
    let m = command()
        .try_get_matches_from(prefix)
        .unwrap_or_else(|e| e.exit());

    // A script is a front end: run it first, and the atlas it builds becomes
    // the document. The other flags stay overridable, exactly as with JSON.
    let mut doc: Document = if let Some(path) = m.get_one::<PathBuf>("script") {
        let source = std::fs::read_to_string(path)
            .map_err(|e| Error::Input(format!("reading {}: {e}", path.display())))?;
        Document::from_script(&source)
            .map_err(|e| Error::Input(format!("{}: {e}", path.display())))?
    } else if let Some(json) = m.get_one::<String>("style-json") {
        Document::parse(json).map_err(|e| Error::Input(format!("invalid --style-json: {e}")))?
    } else if let Some(path) = m.get_one::<PathBuf>("style-file") {
        let s = std::fs::read_to_string(path)
            .map_err(|e| Error::Input(format!("reading {}: {e}", path.display())))?;
        Document::parse(&s).map_err(|e| Error::Input(format!("invalid --style-file: {e}")))?
    } else {
        Document::default()
    };
    if let Some(json) = m.get_one::<String>("water") {
        let water =
            json5::from_str(json).map_err(|e| Error::Input(format!("invalid --water: {e}")))?;
        doc.water = Some(water);
    }

    let batches: Vec<Batch> = if !doc.batches.is_empty() {
        doc.batches.clone()
    } else if let Some(sections) = sections {
        let base = shared_style(&m);
        let mut out: Vec<Batch> = Vec::new();
        for sec in sections {
            // Terrain flags left out carry over from the pass before.
            let mut start = base.clone();
            if let Some(prev) = out.last() {
                start.inherit_terrain(&prev.style);
            }
            out.push(parse_batch_section(&start, &sec)?);
        }
        out
    } else {
        let mut st = Style::default();
        for p in Style::PARAMS {
            st.set(p.name, *m.get_one::<f64>(p.name).unwrap());
        }
        vec![Batch {
            name: m.get_one::<String>("batch-name").cloned(),
            layer: None,
            color: None,
            color_expr: None,
            angle: *m.get_one::<f64>("batch-angle").unwrap() as f32,
            ground: Default::default(),
            draw: Default::default(),
            style: st,
            ..Default::default()
        }]
    };

    let r = Report::new();
    #[cfg(feature = "gpu")]
    if m.get_flag("force-cpu") {
        crate::engine::force_cpu();
        r.detail(&format!(
            "terrain fields on the {}",
            crate::engine::describe()
        ));
    }
    if m.get_one::<String>("terrain-at").is_some()
        || doc.area.as_ref().is_some_and(|a| a.at.is_some())
    {
        r.transient("where", "looking up the place");
    }
    let (bounds, frame, label) = area_and_frame(&m, &doc)?;
    let (wm, hm) = projected_size(&bounds);
    let (width_km, height_km) = (wm / 1000.0, hm / 1000.0);
    let (lat, lon) = ((bounds[1] + bounds[3]) / 2.0, (bounds[0] + bounds[2]) / 2.0);
    let ground = burin_map_core::panel::ground(width_km, height_km, lat, lon);
    let area = Event::Area {
        width_km,
        height_km,
        lat,
        lon,
    };
    if !r.live {
        r.step("where", &format!("{BOLD}{label}{BOLD:#}"), None);
        r.detail(&ground);
    }
    if !r.live && (batches.len() > 1 || batches[0].angle != 0.0 || batches[0].name.is_some()) {
        let desc: Vec<String> = batches
            .iter()
            .map(|b| match &b.name {
                Some(n) => format!("{n} {:.0}°", b.angle),
                None => format!("{:.0}°", b.angle),
            })
            .collect();
        r.detail(&format!(
            "{} batch passes ({})",
            batches.len(),
            desc.join(", ")
        ));
    }

    let out = PathBuf::from(m.get_one::<String>("out-file").unwrap());
    let is_svg = out.extension().map(|e| e == "svg").unwrap_or(false);
    let (want_png, want_svg) = match m.get_one::<String>("out-format").map(|s| s.as_str()) {
        Some("png") => (true, false),
        Some("svg") => (false, true),
        Some("geojson") => (false, false),
        Some(_) => (true, true),
        None => (!is_svg, is_svg),
    };

    let t = Instant::now();
    let water = doc.water.as_ref();
    let panel = r.live.then(|| {
        let rows: Vec<_> = burin_map_core::rows(
            &batches,
            water.map(|w| (&w.batches[..], w.shoreline.as_ref())),
        )
        .into_iter()
        // The water row above builds the mask, while it fetches.
        .filter(|row| row.step != Step::WaterMask && (want_svg || row.step != Step::Svg))
        .collect();
        let p = Panel::new(&label, water.is_some(), &rows);
        p.event(area);
        p
    });
    let svg_step = |stage| {
        if let Some(p) = &panel {
            p.progress(&Progress::Step {
                step: Step::Svg,
                stage,
            });
        }
    };
    let files = std::thread::scope(|s| {
        if let Some(p) = &panel {
            s.spawn(|| p.tick());
        }
        let done = (|| -> Result<_, Box<dyn std::error::Error>> {
            let map = crate::draw_with_water(&bounds, &batches, water, &frame, |p| match &panel {
                Some(panel) => panel.progress(p),
                None => r.progress(p),
            })?;
            let d = &map.drawing;
            let mut files = Vec::new();
            if want_png {
                let png = out.with_extension("png");
                std::fs::write(&png, map.to_png()?)?;
                files.push((png, format!("{} × {} px", d.width, d.height)));
            }
            if want_svg {
                svg_step(Stage::Start);
                let svg = out.with_extension("svg");
                std::fs::write(&svg, map.to_svg())?;
                svg_step(Stage::Done);
                let size = match map.mm_per_px {
                    Some(k) => format!("{:.0} × {:.0} mm", d.width as f64 * k, d.height as f64 * k),
                    None => format!("{} × {} px", d.width, d.height),
                };
                files.push((svg, size));
            }
            if let Some(path) = m.get_one::<PathBuf>("out-geojson") {
                std::fs::write(path, map.to_geojson(&bounds))?;
                files.push((path.clone(), "GeoJSON".into()));
            }
            Ok(files)
        })();
        if let Some(p) = &panel {
            for (path, size) in done.iter().flatten() {
                p.message(format!("wrote {}  {size}", path.display()));
            }
            p.finish(done.is_ok());
        }
        done
    })?;
    if panel.is_some() {
        return Ok(());
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
    let _ = writeln!(
        out,
        "  {:<10} {DIM}done in {:.1} s{DIM:#}",
        "",
        t.elapsed().as_secs_f64()
    );
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

/// The clap prefix, then one entry per `--batch` section (if any).
type BatchArgv = (Vec<OsString>, Option<Vec<Vec<OsString>>>);

/// Split `argv` on `--batch` / `-b`. Terrain, frame and out flags always go
/// to the clap prefix, even after a `--batch`; style flags stay in the
/// current batch section.
fn split_batch_argv(args: &[OsString]) -> Result<BatchArgv, Error> {
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
        Takes::Zero => 0,
        Takes::One => 1,
        Takes::Four => 4,
        Takes::OneOrTwo => {
            let n = rest
                .iter()
                .take(2)
                .take_while(|a| !a.to_string_lossy().starts_with("--"))
                .count();
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
            return Err(Error::Input(format!(
                "unexpected {flag:?} in --batch section"
            )));
        };
        let name = name.replace('-', "_");
        let raw = args
            .get(i + 1)
            .ok_or_else(|| Error::Input(format!("missing value for {flag}")))?;
        let raw = raw.to_string_lossy();
        i += 2;
        if name == "batch_name" || name == "bn" {
            label = Some(raw.into_owned());
            continue;
        }
        let v: f64 = raw
            .parse()
            .map_err(|_| Error::Input(format!("bad value {raw:?} for {flag}")))?;
        if name == "batch_angle" || name == "ba" {
            angle = v;
        } else if !st.set(&name, v) {
            return Err(Error::Input(format!("unknown batch parameter {flag}")));
        }
    }
    Ok(Batch {
        name: label,
        layer: None,
        color: None,
        color_expr: None,
        angle: angle as f32,
        ground: Default::default(),
        draw: Default::default(),
        style: st,
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(args: &[&str]) -> Result<(Bounds, Frame), Error> {
        let m = command()
            .try_get_matches_from(std::iter::once("burin-map").chain(args.iter().copied()))
            .unwrap();
        area_and_frame(&m, &Document::default()).map(|(b, f, _)| (b, f))
    }

    fn argv(args: &[&str]) -> Vec<OsString> {
        std::iter::once("burin-map")
            .chain(args.iter().copied())
            .map(Into::into)
            .collect()
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
        let (b, _) = resolve(&[
            "--tat",
            "46,7.7",
            "--frame-size",
            "297",
            "420",
            "--export-scale",
            "1:50000",
        ])
        .unwrap();
        let (pw, ph) = projected_size(&b);
        assert!(
            (pw / 1000.0 - 14.85).abs() < 0.1 && (ph / 1000.0 - 21.0).abs() < 0.1,
            "{pw} {ph}"
        );
    }

    #[test]
    fn one_paper_value_is_a_square() {
        let (b, f) = resolve(&[
            "--tat",
            "46,7.7",
            "--frame-size",
            "200",
            "--export-scale",
            "1:10000",
        ])
        .unwrap();
        assert_eq!(f.size_mm, Some([200.0, 200.0]));
        let (pw, ph) = projected_size(&b);
        assert!(
            (pw - 2000.0).abs() < 10.0 && (ph - 2000.0).abs() < 10.0,
            "{pw} {ph}"
        );
    }

    #[test]
    fn overdetermined_is_an_error() {
        assert!(resolve(&[
            "--tp",
            "bern",
            "--export-scale",
            "1:1000",
            "--frame-size",
            "200"
        ])
        .is_err());
        assert!(resolve(&[
            "--tat",
            "46,7",
            "--ts",
            "5km",
            "--frame-size",
            "200",
            "--export-scale",
            "1:1000"
        ])
        .is_err());
        assert!(resolve(&["--tp", "bern", "--ts", "5km"]).is_err());
        assert!(resolve(&["--tat", "46,7", "--export-scale", "1:1000"]).is_err());
    }

    #[test]
    fn old_frame_flags_are_gone() {
        for old in [
            "--frame-px",
            "--frame-width",
            "--frame-height",
            "--fpx",
            "--fw",
            "--fh",
            "--frame-scale",
            "--fsc",
            "--frame-pen",
            "--fpen",
        ] {
            assert!(
                command().try_get_matches_from(argv(&[old, "100"])).is_err(),
                "{old}"
            );
        }
    }

    #[test]
    fn flags_win_over_the_scripts_area() {
        let doc = Document::from_script(
            r#"burin.area({ at: "46,7.7", size_km: [8, 6], pen_mm: 0.5 }); burin.batch({}); burin.draw()"#,
        )
        .unwrap();
        let resolve_with = |args: &[&str]| {
            let m = command().try_get_matches_from(argv(args)).unwrap();
            area_and_frame(&m, &doc).unwrap()
        };
        let (b, f, _) = resolve_with(&[]);
        let (w, h) = projected_size(&b);
        assert!((w - 8000.0).abs() < 10.0 && (h - 6000.0).abs() < 10.0, "{w} {h}");
        assert_eq!(f.pen_width, 0.5);
        let (b, f, label) = resolve_with(&["--tp", "bern", "--pen-width", "0.2"]);
        assert_eq!((b, label.as_str(), f.pen_width), (PLACES[0].1, "Bern", 0.2));
        let (b, _, _) = resolve_with(&["--terrain-size", "4km"]);
        let (w, h) = projected_size(&b);
        assert!((w - 4000.0).abs() < 10.0 && (h - 4000.0).abs() < 10.0, "{w} {h}");
    }

    #[test]
    fn size_takes_units_and_two_values() {
        let (b, _) = resolve(&["--terrain-at", "46,7", "--terrain-size", "800m", "1.2km"]).unwrap();
        let (w, h) = projected_size(&b);
        assert!(
            (w - 800.0).abs() < 5.0 && (h - 1200.0).abs() < 5.0,
            "{w} {h}"
        );
    }

    #[test]
    fn short_and_long_style_flags_agree() {
        let m = command()
            .try_get_matches_from(argv(&["--sl", "6", "--dash-light", "0.3"]))
            .unwrap();
        let st = shared_style(&m);
        assert!((st.space_light - 6.0).abs() < 1e-6);
        assert!((st.dash_light - 0.3).abs() < 1e-6);
    }

    #[test]
    fn old_names_are_rejected() {
        for old in [
            "--spacing",
            "--duty-min",
            "--place",
            "--svg",
            "--hatch",
            "--shade-gap",
            "--space",
            "--dash-drop",
        ] {
            assert!(
                command().try_get_matches_from(argv(&[old, "1"])).is_err(),
                "{old}"
            );
        }
    }

    #[test]
    fn batch_sections_parse() {
        let args = argv(&[
            "--tp",
            "bern",
            "--space-light",
            "6",
            "--batch",
            "--mod-breaks",
            "0.1",
            "--of",
            "x.png",
            "-b",
            "--batch-name",
            "Steep valleys",
            "--ba",
            "90",
            "--mb",
            "0.9",
            "--batch-floor",
            "0.4",
        ]);
        let (prefix, secs) = split_batch_argv(&args).unwrap();
        let secs = secs.unwrap();
        assert_eq!(secs.len(), 2);
        let m = command().try_get_matches_from(prefix).unwrap();
        assert_eq!(m.get_one::<String>("out-file").unwrap(), "x.png");
        let base = shared_style(&m);
        assert!((base.space_light - 6.0).abs() < 1e-6);
        let b0 = parse_batch_section(&base, &secs[0]).unwrap();
        let b1 = parse_batch_section(&base, &secs[1]).unwrap();
        assert_eq!(b0.angle, 0.0);
        assert_eq!(b0.name, None);
        assert_eq!(b1.name.as_deref(), Some("Steep valleys"));
        assert!((b0.style.mod_breaks - 0.1).abs() < 1e-6);
        assert!((b0.style.space_light - 6.0).abs() < 1e-6);
        assert_eq!(b1.angle, 90.0);
        assert!((b1.style.mod_breaks - 0.9).abs() < 1e-6);
        assert!((b1.style.batch_floor - 0.4).abs() < 1e-6);
        assert!(parse_batch_section(&base, &["--breaks".into(), "0.2".into()]).is_err());
    }
}
