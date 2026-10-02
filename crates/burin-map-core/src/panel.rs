//! The draw's progress as a display shows it, the same on a terminal and in
//! the browser: told what happens, it keeps a tree of tasks (download, then
//! the drawing with each step beneath it as the engine names them) and the
//! messages below, and hands back what to show. It has no clock: every call
//! says when it is, in seconds since the panel began.

use serde::Serialize;

use crate::progress::{Row, Stage, Step};

const WIDTH: usize = 20;
const SPIN: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
/// Seconds per spinner frame.
const FRAME: f64 = 0.08;
/// Messages kept on show; older ones scroll away.
const MESSAGES_SHOWN: usize = 8;

/// What happened, as a display is told it.
#[derive(Clone, Debug)]
pub enum Event {
    /// An elevation tile in; `done` of `total`.
    Tile { done: usize, total: usize },
    /// Elevation on the grid: `tiles` tiles at `zoom`.
    Elevation {
        tiles: usize,
        zoom: u8,
        width: usize,
        height: usize,
        px_m: f64,
        highest_m: f32,
        deepest_m: f32,
    },
    /// A water tile in; `done` of `total`.
    WaterTile { done: usize, total: usize },
    /// Every water tile in.
    Water,
    /// `share` of the grid is water, once the mask is made.
    WaterShare { share: f64 },
    /// The area on the ground.
    Area {
        width_km: f64,
        height_km: f64,
        lat: f64,
        lon: f64,
    },
    /// A frame with a physical size.
    Frame {
        width_mm: f64,
        height_mm: f64,
        scale: f64,
        pen_width: f64,
        pitch_mm: f64,
    },
    /// The drawing holds `count` dashes.
    Dashes { count: usize },
    /// A step of the drawing reached a stage, as the engine tells it.
    Step { step: Step, stage: Stage },
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Wait,
    Run,
    Ok,
    Fail,
}

#[derive(Clone, Copy, PartialEq)]
enum Key {
    Download,
    Elevation,
    Water,
    Drawing,
    Step(Step),
}

struct Task {
    key: Key,
    label: String,
    child: bool,
    state: State,
    done: usize,
    total: usize,
    unit: &'static str,
    note: String,
    /// The names of a drawing step's stages before done.
    stages: Vec<&'static str>,
    t0: f64,
    secs: f64,
}

impl Task {
    fn new(key: Key, label: &str) -> Self {
        Task {
            key,
            label: label.into(),
            child: !matches!(key, Key::Download | Key::Drawing),
            state: State::Wait,
            done: 0,
            total: 0,
            unit: "",
            note: String::new(),
            stages: Vec::new(),
            t0: 0.0,
            secs: 0.0,
        }
    }

    fn run(&mut self, now: f64) {
        if self.state == State::Wait {
            self.state = State::Run;
            self.t0 = now;
        }
    }

    fn end(&mut self, state: State, now: f64) {
        self.run(now);
        self.secs = now - self.t0;
        self.state = state;
    }

    fn ran(&self, now: f64) -> f64 {
        match self.state {
            State::Run => (now - self.t0).max(0.0),
            _ => self.secs,
        }
    }
}

/// A message below the tree, `at` seconds in.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Message {
    pub at: f64,
    pub text: String,
    /// Something went wrong on the way (a retried download).
    pub warn: bool,
}

/// One row of the tree as shown.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Line {
    pub state: State,
    /// Spinner frame, ✓, ✗ or ·.
    pub mark: char,
    /// The label, behind its branch (`├ `, `└ `) when the row is a child.
    pub name: String,
    pub child: bool,
    /// A child's bar as dimmed lead, full and dimmed rest; a parent has
    /// none, its children's would only repeat.
    pub bar: Option<[String; 3]>,
    pub detail: String,
}

/// What to show now.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct View {
    /// "please wait", "done" or "failed".
    pub head: &'static str,
    pub title: String,
    /// Only the rows that have started: the tree grows as work begins.
    pub rows: Vec<Line>,
    /// Characters the widest name takes, for lining up the bars.
    pub name_width: usize,
    /// Cells in a bar.
    pub bar_width: usize,
    /// The latest messages, oldest first.
    pub messages: Vec<Message>,
    /// Messages before those, no longer shown.
    pub earlier: usize,
}

pub struct Panel {
    title: String,
    tasks: Vec<Task>,
    log: Vec<Message>,
    finished: Option<bool>,
}

impl Panel {
    /// A panel for drawing `title`, fetching water when `water`, with a row
    /// for each of the engine's `rows`. The download starts at once.
    pub fn new(title: &str, water: bool, rows: &[Row]) -> Self {
        let mut download = Task::new(Key::Download, "download");
        download.run(0.0);
        let mut tasks = vec![download, Task::new(Key::Elevation, "elevation")];
        if water {
            tasks.push(Task::new(Key::Water, "water"));
        }
        tasks.push(Task::new(Key::Drawing, "drawing"));
        for r in rows {
            let mut t = Task::new(Key::Step(r.step), &r.label);
            t.stages = r.stages();
            if !t.stages.is_empty() {
                t.total = Stage::Done as usize;
            }
            tasks.push(t);
        }
        Panel {
            title: title.into(),
            tasks,
            log: Vec::new(),
            finished: None,
        }
    }

    pub fn is_finished(&self) -> bool {
        self.finished.is_some()
    }

    fn task(&mut self, key: Key) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.key == key)
    }

    pub fn event(&mut self, e: &Event, now: f64) {
        match *e {
            Event::Tile { done, total } => {
                if let Some(t) = self.task(Key::Elevation) {
                    t.run(now);
                    (t.done, t.total, t.unit) = (done, total, "tiles");
                }
            }
            Event::Elevation {
                tiles,
                zoom,
                width,
                height,
                px_m,
                highest_m,
                deepest_m,
            } => {
                if let Some(t) = self.task(Key::Elevation) {
                    (t.done, t.total, t.unit) = (tiles, tiles, "tiles");
                    t.note = format!("zoom {zoom}");
                    t.end(State::Ok, now);
                }
                self.message(
                    format!(
                        "{width} × {height} px, {px_m:.0} m per pixel, {}",
                        relief(highest_m, deepest_m)
                    ),
                    false,
                    now,
                );
                self.settle_download(now);
            }
            Event::WaterTile { done, total } => {
                if let Some(t) = self.task(Key::Water) {
                    t.run(now);
                    (t.done, t.total, t.unit) = (done, total, "tiles");
                }
            }
            Event::Water => {
                if let Some(t) = self.task(Key::Water) {
                    t.done = t.total;
                    t.end(State::Ok, now);
                }
                self.settle_download(now);
            }
            Event::WaterShare { share } => {
                let note = format!("{:.0}% water", share * 100.0);
                if let Some(t) = self.task(Key::Step(Step::WaterMask)) {
                    t.note = note;
                } else if let Some(t) = self.task(Key::Water) {
                    t.note = note;
                }
            }
            Event::Area {
                width_km,
                height_km,
                lat,
                lon,
            } => self.message(ground(width_km, height_km, lat, lon), false, now),
            Event::Frame {
                width_mm,
                height_mm,
                scale: k,
                pen_width,
                pitch_mm,
            } => self.message(
                format!(
                    "{width_mm:.0} × {height_mm:.0} mm at {}, stroke {pen_width} mm, lines {pitch_mm:.2} mm apart",
                    scale(k)
                ),
                false,
                now,
            ),
            Event::Dashes { count } => {
                if let Some(t) = self.task(Key::Drawing) {
                    t.note = format!("{} dashes", thousands(count as u64));
                }
            }
            Event::Step { step, stage } => {
                if let Some(t) = self.task(Key::Drawing) {
                    t.run(now);
                }
                if let Some(t) = self.task(Key::Step(step)) {
                    t.run(now);
                    if stage == Stage::Done {
                        if t.state == State::Run {
                            t.done = t.total;
                            if t.key != Key::Step(Step::WaterMask) {
                                t.note.clear();
                            }
                            t.end(State::Ok, now);
                        }
                    } else if !t.stages.is_empty() {
                        t.done = stage as usize;
                        t.note = t.stages[stage as usize - 1].into();
                    }
                }
            }
        }
    }

    /// The download done once everything under it is, with the tiles it took.
    fn settle_download(&mut self, now: f64) {
        let under = |t: &&Task| matches!(t.key, Key::Elevation | Key::Water);
        if !self
            .tasks
            .iter()
            .filter(under)
            .all(|t| t.state == State::Ok)
        {
            return;
        }
        let tiles: usize = self.tasks.iter().filter(under).map(|t| t.total).sum();
        if let Some(t) = self.task(Key::Download) {
            if t.state == State::Run {
                t.note = format!("{tiles} tiles");
                t.end(State::Ok, now);
            }
        }
    }

    /// A line for the messages below the tree.
    pub fn message(&mut self, text: String, warn: bool, now: f64) {
        self.log.push(Message {
            at: now,
            text,
            warn,
        });
    }

    /// The drawing, and whatever still runs, as done (`ok`) or failed.
    pub fn finish(&mut self, ok: bool, now: f64) {
        for t in &mut self.tasks {
            if t.state == State::Run || (t.key == Key::Drawing && ok) {
                if ok {
                    t.done = t.total;
                    t.end(State::Ok, now);
                } else {
                    t.end(State::Fail, now);
                }
            }
        }
        self.finished = Some(ok);
    }

    pub fn view(&self, now: f64) -> View {
        let spin = SPIN[(now.max(0.0) / FRAME) as usize % SPIN.len()];
        let name_width = self
            .tasks
            .iter()
            .map(|t| t.label.chars().count() + if t.child { 2 } else { 0 })
            .max()
            .unwrap_or(0)
            .max(9);
        let shown: Vec<&Task> = self
            .tasks
            .iter()
            .filter(|t| t.state != State::Wait)
            .collect();
        let rows = shown
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let last = t.child && !shown.get(i + 1).is_some_and(|n| n.child);
                let name = match (t.child, last) {
                    (true, true) => format!("└ {}", t.label),
                    (true, false) => format!("├ {}", t.label),
                    _ => t.label.clone(),
                };
                let (bar, detail) = if t.child {
                    let (bar, detail) = meter(t, now);
                    (Some(bar), detail)
                } else {
                    (None, summary(t, now))
                };
                Line {
                    state: t.state,
                    mark: match t.state {
                        State::Run => spin,
                        State::Ok => '✓',
                        State::Fail => '✗',
                        State::Wait => '·',
                    },
                    name,
                    child: t.child,
                    bar,
                    detail,
                }
            })
            .collect();
        let earlier = self.log.len().saturating_sub(MESSAGES_SHOWN);
        View {
            head: match self.finished {
                None => "please wait",
                Some(true) => "done",
                Some(false) => "failed",
            },
            title: self.title.clone(),
            rows,
            name_width,
            bar_width: WIDTH,
            messages: self.log[earlier..].to_vec(),
            earlier,
        }
    }
}

fn secs(s: f64) -> String {
    format!("{s:.1} s")
}

/// What a parent row says beside its spinner: the time, and once done its note.
fn summary(t: &Task, now: f64) -> String {
    match t.state {
        State::Ok if !t.note.is_empty() => format!("{} · {}", secs(t.ran(now)), t.note),
        _ => secs(t.ran(now)),
    }
}

/// A child's bar and the text after it.
fn meter(t: &Task, now: f64) -> ([String; 3], String) {
    let ran = t.ran(now);
    let frac = |d: usize, n: usize| if n == 0 { 0.0 } else { d as f64 / n as f64 };
    match t.state {
        State::Wait => (bar(0.0), String::new()),
        State::Ok | State::Fail => {
            let f = if t.state == State::Ok {
                1.0
            } else {
                frac(t.done, t.total)
            };
            let count = if t.total > 0 && !t.unit.is_empty() {
                format!("{}/{} {} · ", t.done, t.total, t.unit)
            } else {
                String::new()
            };
            let note = if t.note.is_empty() {
                String::new()
            } else {
                format!(" · {}", t.note)
            };
            (bar(f), format!("{count}{}{note}", secs(ran)))
        }
        State::Run if t.total > 0 => {
            let what = if t.note.is_empty() {
                format!("{}/{} {}", t.done, t.total, t.unit)
            } else {
                t.note.clone()
            };
            (
                bar(frac(t.done, t.total)),
                format!("{what} · {}", secs(ran)),
            )
        }
        State::Run => (bounce(ran), secs(ran)),
    }
}

/// `frac` of the bar filled.
fn bar(frac: f64) -> [String; 3] {
    let n = (frac.clamp(0.0, 1.0) * WIDTH as f64).round() as usize;
    [String::new(), "█".repeat(n), "░".repeat(WIDTH - n)]
}

/// A block sliding to and fro, for work with no count.
fn bounce(secs: f64) -> [String; 3] {
    let w = (WIDTH / 4).max(2);
    let span = WIDTH - w;
    let step = (secs * 1000.0 / 70.0) as usize % (2 * span);
    let at = if step < span { step } else { 2 * span - step };
    ["░".repeat(at), "█".repeat(w), "░".repeat(span - at)]
}

/// "1,234,567".
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// "1:100,000": three significant figures, since whole pixels make the exact
/// value drift a little from the one asked for (1:100,021).
pub fn scale(n: f64) -> String {
    let mag = 10f64.powi(n.log10().floor() as i32 - 2).max(1.0);
    format!("1:{}", thousands(((n / mag).round() * mag) as u64))
}

/// "46.95°N 7.45°E".
pub fn latlon(lat: f64, lon: f64) -> String {
    format!(
        "{:.2}°{} {:.2}°{}",
        lat.abs(),
        if lat >= 0.0 { 'N' } else { 'S' },
        lon.abs(),
        if lon >= 0.0 { 'E' } else { 'W' }
    )
}

/// "highest point 925 m, sea floor down to 60 m".
pub fn relief(highest_m: f32, deepest_m: f32) -> String {
    let mut s = format!("highest point {highest_m:.0} m");
    if deepest_m < 0.0 {
        s += &format!(", sea floor down to {:.0} m", -deepest_m);
    }
    s
}

/// "20.0 × 20.0 km around 37.75°N 122.45°W".
pub fn ground(width_km: f64, height_km: f64, lat: f64, lon: f64) -> String {
    format!(
        "{width_km:.1} × {height_km:.1} km around {}",
        latlon(lat, lon)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Batch, Style};

    #[test]
    fn formats() {
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(999), "999");
        assert_eq!(latlon(46.95, -7.45), "46.95°N 7.45°W");
        assert_eq!(scale(100_021.0), "1:100,000");
        assert_eq!(scale(67_540.0), "1:67,500");
        assert_eq!(scale(250.0), "1:250");
    }

    #[test]
    fn the_tree_grows_as_work_begins() {
        let rows = crate::rows(&[Batch::along(Style::default())], None);
        let mut p = Panel::new("Bern", false, &rows);
        let names = |p: &Panel, now| {
            p.view(now)
                .rows
                .into_iter()
                .map(|r| r.name)
                .collect::<Vec<_>>()
        };
        assert_eq!(names(&p, 0.0), ["download"]);
        p.event(&Event::Tile { done: 1, total: 4 }, 0.5);
        assert_eq!(names(&p, 0.5), ["download", "└ elevation"]);
        let elevation = Event::Elevation {
            tiles: 4,
            zoom: 12,
            width: 100,
            height: 80,
            px_m: 30.0,
            highest_m: 900.0,
            deepest_m: -20.0,
        };
        p.event(&elevation, 1.0);
        let v = p.view(1.0);
        assert_eq!(v.rows[0].detail, "1.0 s · 4 tiles");
        assert!(v.rows[0].bar.is_none());
        assert_eq!(v.rows[1].detail, "4/4 tiles · 0.5 s · zoom 12");
        assert_eq!(
            v.messages[0].text,
            "100 × 80 px, 30 m per pixel, highest point 900 m, sea floor down to 20 m"
        );
        p.event(
            &Event::Step {
                step: Step::Batch(0),
                stage: Stage::Trace,
            },
            1.2,
        );
        assert_eq!(
            names(&p, 1.2),
            ["download", "└ elevation", "drawing", "└ 1 lines"]
        );
        assert_eq!(p.view(1.2).rows[3].detail, "trace · 0.0 s");
        p.event(&Event::Dashes { count: 1234 }, 1.5);
        p.finish(true, 2.0);
        let v = p.view(2.0);
        assert_eq!(v.head, "done");
        assert_eq!(v.rows[2].detail, "0.8 s · 1,234 dashes");
        assert_eq!(v.rows[3].mark, '✓');
    }
}
