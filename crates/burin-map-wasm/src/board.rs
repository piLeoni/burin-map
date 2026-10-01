//! The running draw's progress, where the page can read it while the draw
//! blocks the worker: in the threaded build this module's memory is a
//! `SharedArrayBuffer`. Each slot holds a [`Stage`] (0: not begun); the
//! layout is mirrored in web/src/lib/board.js.

use std::sync::atomic::{AtomicU32, Ordering};

use burin_map_core::{Progress, Stage, Step};
use js_sys::{Uint32Array, WebAssembly};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

/// Draws begun so far, so the page knows when the board is the new draw's.
const DRAWS: usize = 0;
pub const WATER_MASK: usize = 1;
pub const TERRAIN: usize = 2;
const SHORE: usize = 3;
const SHORELINE: usize = 4;
const ORDER: usize = 5;
pub const SVG: usize = 6;
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

pub fn mark(slot: usize, stage: Stage) {
    BOARD[slot].store(stage as u32, Ordering::Release);
}

pub struct Board;

impl Progress for Board {
    fn reach(&self, step: Step, stage: Stage) {
        let slot = match step {
            Step::Batch(i) if i < MAX_BATCHES => LAND + i,
            Step::Water(i) if i < MAX_BATCHES => WATER + i,
            Step::Shore => SHORE,
            Step::Shoreline => SHORELINE,
            Step::Order => ORDER,
            _ => return,
        };
        mark(slot, stage);
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
