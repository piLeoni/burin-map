//! The running draw's progress, where the page can read it while the draw
//! blocks the worker: in the threaded build this module's memory is a
//! `SharedArrayBuffer`. Each slot holds a [`Stage`] (0: not begun), or for
//! [`SHARE`] the water's share. The page hands what it reads to a [`Panel`],
//! the engine's progress panel, which also knows the slots to read.

use std::sync::atomic::{AtomicU32, Ordering};

use burin_map_core::panel::{self, Event};
use burin_map_core::{Batch, Progress, Row, Shoreline, Stage, Step, WaterBatch};
use js_sys::{Uint32Array, WebAssembly};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Draws begun so far, so the page knows when the board is the new draw's.
const DRAWS: usize = 0;
/// The water's share of the grid in thousandths, plus one (0: not known yet).
const SHARE: usize = 7;
/// Batches beyond this many still draw, unreported.
const MAX_BATCHES: usize = 120;
const LAND: usize = 16;
const WATER: usize = LAND + MAX_BATCHES;
const LEN: usize = WATER + MAX_BATCHES;

static BOARD: [AtomicU32; LEN] = [const { AtomicU32::new(0) }; LEN];

/// Clears the board for a new draw.
pub fn begin() {
    for slot in &BOARD[1..] {
        slot.store(0, Ordering::Relaxed);
    }
    BOARD[DRAWS].fetch_add(1, Ordering::Release);
}

/// `share` of the grid is water.
pub fn share(share: f64) {
    BOARD[SHARE].store(
        (share.clamp(0.0, 1.0) * 1000.0).round() as u32 + 1,
        Ordering::Release,
    );
}

fn slot(step: Step) -> Option<usize> {
    Some(match step {
        Step::WaterMask => 1,
        Step::Terrain => 2,
        Step::Shore => 3,
        Step::Shoreline => 4,
        Step::Order => 5,
        Step::Svg => 6,
        Step::Batch(i) if i < MAX_BATCHES => LAND + i,
        Step::Water(i) if i < MAX_BATCHES => WATER + i,
        _ => return None,
    })
}

pub struct Board;

impl Progress for Board {
    fn reach(&self, step: Step, stage: Stage) {
        if let Some(s) = slot(step) {
            BOARD[s].store(stage as u32, Ordering::Release);
        }
    }
}

/// `progressBoard()`: the board, as a view on this module's memory.
#[wasm_bindgen(js_name = progressBoard)]
pub fn progress_board() -> Uint32Array {
    let memory: WebAssembly::Memory = wasm_bindgen::memory().unchecked_into();
    Uint32Array::new_with_byte_offset_and_length(
        &memory.buffer(),
        BOARD.as_ptr() as u32,
        LEN as u32,
    )
}

/// The rows of the draw of these batches (JSON, as for `draw_batches`) and
/// this water spec (JSON, as `water.spec`), in the order to show them.
fn rows(json: &str, water_spec: Option<String>) -> Result<Vec<Row>, JsValue> {
    #[derive(serde::Deserialize)]
    struct Water {
        #[serde(
            default = "burin_map_core::water::shoreline_option::default",
            with = "burin_map_core::water::shoreline_option"
        )]
        shoreline: Option<Shoreline>,
        #[serde(default)]
        batches: Vec<WaterBatch>,
    }
    let err = |e: serde_json::Error| JsValue::from_str(&e.to_string());
    let batches: Vec<Batch> = burin_map_core::from_json_batches(json).map_err(err)?;
    let water: Option<Water> = water_spec
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(err)?;
    Ok(burin_map_core::rows(
        &batches,
        water
            .as_ref()
            .map(|w| (&w.batches[..], w.shoreline.as_ref())),
    ))
}

/// The engine's progress panel (`burin_map_core::panel`), the one the command
/// line shows: the page tells it what happens, with the seconds since the
/// draw began, and shows what [`Panel::view`] returns.
#[wasm_bindgen]
pub struct Panel {
    inner: panel::Panel,
    /// Each board slot the draw reports on, and its step.
    steps: Vec<(usize, Step)>,
}

#[wasm_bindgen]
impl Panel {
    /// A panel for drawing `title`: these batches (JSON, as for
    /// `draw_batches`) and, with water, this water spec (JSON, as `water.spec`).
    #[wasm_bindgen(constructor)]
    pub fn new(title: &str, json: &str, water_spec: Option<String>) -> Result<Panel, JsValue> {
        let water = water_spec.is_some();
        let rows = rows(json, water_spec)?;
        let steps = rows
            .iter()
            .filter_map(|r| Some((slot(r.step)?, r.step)))
            .collect();
        Ok(Panel {
            inner: panel::Panel::new(title, water, &rows),
            steps,
        })
    }

    /// The board slots to read while the draw runs.
    pub fn slots(&self) -> Vec<u32> {
        self.steps
            .iter()
            .map(|&(s, _)| s as u32)
            .chain([SHARE as u32])
            .collect()
    }

    /// What the board says in `slot`.
    pub fn board(&mut self, slot: u32, value: u32, now: f64) {
        let slot = slot as usize;
        if slot == SHARE {
            if value > 0 {
                let share = (value - 1) as f64 / 1000.0;
                self.inner.event(&Event::WaterShare { share }, now);
            }
            return;
        }
        let stage = Stage::ALL.into_iter().find(|s| *s as u32 == value);
        let step = self
            .steps
            .iter()
            .find(|&&(s, _)| s == slot)
            .map(|&(_, s)| s);
        if let (Some(step), Some(stage)) = (step, stage) {
            self.inner.event(&Event::Step { step, stage }, now);
        }
    }

    /// An elevation tile in; `done` of `total`.
    pub fn tile(&mut self, done: u32, total: u32, now: f64) {
        let (done, total) = (done as usize, total as usize);
        self.inner.event(&Event::Tile { done, total }, now);
    }

    /// The elevation grid, from `tiles` tiles at `zoom`.
    #[allow(clippy::too_many_arguments)]
    pub fn elevation(
        &mut self,
        elev: &[f32],
        tiles: u32,
        zoom: u8,
        width: u32,
        height: u32,
        px_m: f64,
        now: f64,
    ) {
        let e = Event::Elevation {
            tiles: tiles as usize,
            zoom,
            width: width as usize,
            height: height as usize,
            px_m,
            highest_m: elev.iter().copied().fold(0.0, f32::max),
            deepest_m: elev.iter().copied().fold(0.0, f32::min),
        };
        self.inner.event(&e, now);
    }

    /// A water tile in; `done` of `total`.
    #[wasm_bindgen(js_name = waterTile)]
    pub fn water_tile(&mut self, done: u32, total: u32, now: f64) {
        let (done, total) = (done as usize, total as usize);
        self.inner.event(&Event::WaterTile { done, total }, now);
    }

    /// Every water tile in.
    pub fn water(&mut self, now: f64) {
        self.inner.event(&Event::Water, now);
    }

    /// The area drawn, `[west, south, east, north]` in degrees.
    pub fn area(&mut self, west: f64, south: f64, east: f64, north: f64, now: f64) {
        const KM_PER_DEG: f64 = 6378.137 * std::f64::consts::PI / 180.0;
        let lat = (south + north) / 2.0;
        let e = Event::Area {
            width_km: (east - west) * KM_PER_DEG * lat.to_radians().cos(),
            height_km: (north - south) * KM_PER_DEG,
            lat,
            lon: (west + east) / 2.0,
        };
        self.inner.event(&e, now);
    }

    /// The drawing holds `count` dashes.
    pub fn dashes(&mut self, count: u32, now: f64) {
        let count = count as usize;
        self.inner.event(&Event::Dashes { count }, now);
    }

    /// A line for the messages; `warn` when something went wrong on the way.
    pub fn message(&mut self, text: String, warn: bool, now: f64) {
        self.inner.message(text, warn, now);
    }

    /// The drawing, and whatever still runs, as done (`ok`) or failed.
    pub fn finish(&mut self, ok: bool, now: f64) {
        self.inner.finish(ok, now);
    }

    /// What to show, as JSON: `{ head, title, rows: [{ state, mark, name,
    /// child, bar: [lead, full, rest] | null, detail }], name_width,
    /// bar_width, messages: [{ at, text, warn }], earlier }`.
    pub fn view(&self, now: f64) -> String {
        serde_json::to_string(&self.inner.view(now)).unwrap_or_default()
    }
}
