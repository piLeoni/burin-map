//! Building a drawing from Rust: batches, in order, then `Atlas::draw`.
//!
//! The JavaScript front end builds exactly this: `atlas.batch()` appends one
//! [`Batch`] run, `atlas.water()` one pass on the sea, and `draw` runs the
//! engine. Nothing here is a second way of saying what a document says — the
//! engine's own [`Batch`], [`WaterBatch`] and [`Shoreline`] are what is built,
//! just with trees for the colour and width instead of strings, and with the
//! layers stacked in the order they are written.

use crate::water::{Shoreline, WaterBatch};
use crate::{Batch, Drawing};

pub use crate::water::Draw as WaterDraw;

/// Water the drawing sits on: the passes, and the shore itself.
#[derive(Clone, Debug, Default)]
pub struct Water {
    pub batches: Vec<WaterBatch>,
    pub shoreline: Option<Shoreline>,
}

/// A drawing being put together: land batches and water passes, in the order
/// they are drawn.
#[derive(Clone, Debug, Default)]
pub struct Atlas {
    pub batches: Vec<Batch>,
    pub water: Option<Water>,
}

impl Atlas {
    pub fn new() -> Atlas {
        Atlas::default()
    }

    /// Append a batch. Fills and lines may be mixed; they are drawn in this
    /// order, each fill over the strokes beneath it.
    pub fn batch(&mut self, b: impl Into<Batch>) -> &mut Self {
        self.batches.push(b.into());
        self
    }

    /// Append a pass on the sea. The first one makes the sea appear; without
    /// any, no water is drawn however the elevation reads.
    pub fn water(&mut self, b: impl Into<WaterBatch>) -> &mut Self {
        self.water.get_or_insert_with(Water::default).batches.push(b.into());
        self
    }

    /// Draw the shoreline. `lines` counts how many lines follow the coast out
    /// to sea; the first is the coast itself.
    pub fn shore(&mut self, s: impl Into<Shoreline>) -> &mut Self {
        self.water.get_or_insert_with(Water::default).shoreline = Some(s.into());
        self
    }

    /// Draw it over an elevation grid, `width` × `height` px at `px_m` metres
    /// per pixel. The sea comes from the elevation, as `draw_batches` does.
    pub fn draw(&self, elev: &[f32], width: usize, height: usize, px_m: f32) -> Drawing {
        self.draw_with(elev, None, width, height, px_m)
    }

    /// As [`Atlas::draw`], on a water coverage grid given from outside.
    pub fn draw_with(
        &self,
        elev: &[f32],
        coverage: Option<&[f32]>,
        width: usize,
        height: usize,
        px_m: f32,
    ) -> Drawing {
        crate::draw_atlas(self, elev, coverage, width, height, px_m)
    }
}
