//! The engine's progress panel (`burin_map_core::panel`), redrawn in place
//! on a terminal: the tree on top, growing as tasks start, and the messages
//! below it, pushed down as it grows. The browser shows the same panel.

use std::io::Write;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anstyle::Style;
use burin_map_core::panel::{self, Event, State, View};
use burin_map_core::Row;

use crate::progress::Progress;

const FRAME: Duration = Duration::from_millis(80);

struct Inner {
    panel: panel::Panel,
    /// Lines drawn last time, to go back over.
    lines: usize,
}

pub struct Panel {
    color: bool,
    start: Instant,
    inner: Mutex<Inner>,
}

impl Panel {
    /// A panel for drawing `title`, fetching water when `water`, with a row
    /// for each of the engine's `rows`.
    pub fn new(title: &str, water: bool, rows: &[Row]) -> Self {
        Panel {
            color: std::env::var_os("NO_COLOR").is_none(),
            start: Instant::now(),
            inner: Mutex::new(Inner {
                panel: panel::Panel::new(title, water, rows),
                lines: 0,
            }),
        }
    }

    /// Seconds since the panel began.
    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Redraws every frame until [`Panel::finish`], for the spinners and the
    /// bars of work that has no count.
    pub fn tick(&self) {
        loop {
            {
                let mut inner = self.inner.lock().unwrap();
                if inner.panel.is_finished() {
                    return;
                }
                self.render(&mut inner);
            }
            std::thread::sleep(FRAME);
        }
    }

    pub fn event(&self, e: Event) {
        let mut inner = self.inner.lock().unwrap();
        inner.panel.event(&e, self.now());
        self.render(&mut inner);
    }

    pub fn progress(&self, p: &Progress) {
        let events = match *p {
            Progress::Tile { done, total } => vec![Event::Tile { done, total }],
            Progress::Elevation {
                tiles,
                zoom,
                width,
                height,
                px_m,
                highest_m,
                deepest_m,
                ..
            } => vec![Event::Elevation {
                tiles,
                zoom,
                width,
                height,
                px_m,
                highest_m,
                deepest_m,
            }],
            Progress::WaterTile { done, total } => vec![Event::WaterTile { done, total }],
            Progress::Water { share, .. } => vec![Event::Water, Event::WaterShare { share }],
            Progress::Frame {
                width_mm,
                height_mm,
                scale,
                pen_width,
                pitch_mm,
            } => vec![Event::Frame {
                width_mm,
                height_mm,
                scale,
                pen_width,
                pitch_mm,
            }],
            Progress::Relief { .. } | Progress::Lines { .. } => vec![],
            Progress::Dashes { count, .. } => vec![Event::Dashes { count }],
            Progress::Step { step, stage } => vec![Event::Step { step, stage }],
        };
        let mut inner = self.inner.lock().unwrap();
        let now = self.now();
        for e in &events {
            inner.panel.event(e, now);
        }
        self.render(&mut inner);
    }

    /// A line for the messages below the tree.
    pub fn message(&self, text: String) {
        let mut inner = self.inner.lock().unwrap();
        inner.panel.message(text, false, self.now());
        self.render(&mut inner);
    }

    /// The drawing, and whatever still runs, as done (`ok`) or failed; the
    /// panel stays on screen as it last was.
    pub fn finish(&self, ok: bool) {
        let mut inner = self.inner.lock().unwrap();
        inner.panel.finish(ok, self.now());
        self.render(&mut inner);
    }

    fn style(&self, s: Style) -> Style {
        if self.color {
            s
        } else {
            Style::new()
        }
    }

    fn render(&self, inner: &mut Inner) {
        let lines = self.paint(&inner.panel.view(self.now()));
        let mut out = String::new();
        if inner.lines > 0 {
            out.push_str(&format!("\x1b[{}A", inner.lines));
        }
        for l in &lines {
            out.push_str("\r\x1b[2K");
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("\x1b[J");
        inner.lines = lines.len();
        let mut stdout = std::io::stdout().lock();
        let _ = stdout.write_all(out.as_bytes());
        let _ = stdout.flush();
    }

    /// The view as terminal lines, fitted to the screen.
    fn paint(&self, v: &View) -> Vec<String> {
        let dim = self.style(Style::new().dimmed());
        let bold = self.style(Style::new().bold());
        let plain = Style::new();
        let mut lines = vec![format!(
            "{bold}{}{bold:#}{dim} ▸ {}{dim:#}",
            v.head, v.title
        )];
        for r in &v.rows {
            let (row, note) = match r.state {
                State::Wait => (dim, dim),
                State::Fail => (bold, bold),
                State::Ok => (plain, dim),
                State::Run => (plain, plain),
            };
            let pad = v.name_width.saturating_sub(r.name.chars().count());
            let (mark, name, detail) = (r.mark, &r.name, &r.detail);
            lines.push(match &r.bar {
                Some([lead, full, rest]) => format!(
                    "{row}{mark} {name}{row:#}{:pad$}  {dim}{lead}{dim:#}{full}{dim}{rest}{dim:#}  {note}{detail}{note:#}",
                    ""
                ),
                None => format!("{row}{mark} {name}{row:#}{:pad$}  {note}{detail}{note:#}", ""),
            });
        }
        let mut messages = Vec::new();
        if !v.messages.is_empty() {
            let title = match v.earlier {
                0 => " messages ".to_string(),
                n => format!(" messages · {n} earlier "),
            };
            let rule = "─".repeat(
                (v.name_width + v.bar_width + 30).saturating_sub(title.chars().count() + 2),
            );
            messages.push(format!("{dim}──{title}{rule}{dim:#}"));
            for m in &v.messages {
                let s = if m.warn { bold } else { dim };
                messages.push(format!("{s}{:>6.1} s  {}{s:#}", m.at, m.text));
            }
        }
        let (cols, rows) = screen();
        // Too short a screen loses the top of the tree first.
        let room = rows.saturating_sub(messages.len()).max(1);
        let drop = lines.len().saturating_sub(room);
        lines.drain(..drop);
        lines.extend(messages);
        lines
            .iter()
            .map(|l| clip(l, cols.saturating_sub(1)))
            .collect()
    }
}

/// `line` cut to `cols` visible characters, escape codes kept whole: a line
/// that wrapped would throw off the climb back up on the next redraw.
fn clip(line: &str, cols: usize) -> String {
    let mut out = String::with_capacity(line.len());
    let (mut seen, mut chars) = (0, line.chars());
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            out.push(c);
            for c in chars.by_ref() {
                out.push(c);
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
            continue;
        }
        if seen == cols {
            out.push('…');
            out.push_str("\x1b[0m");
            break;
        }
        out.push(c);
        seen += 1;
    }
    out
}

/// Columns and rows of the terminal behind stdout.
fn screen() -> (usize, usize) {
    #[cfg(unix)]
    // SAFETY: TIOCGWINSZ only fills in the zeroed struct it is given.
    unsafe {
        let mut ws: libc::winsize = std::mem::zeroed();
        if libc::ioctl(1, libc::TIOCGWINSZ, &mut ws) == 0 && ws.ws_col > 0 && ws.ws_row > 0 {
            return (ws.ws_col as usize, ws.ws_row as usize - 1);
        }
    }
    (100, 40)
}
