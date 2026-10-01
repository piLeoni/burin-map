//! What a draw is doing while it runs, for a progress display.

/// A part of a draw that reports how far it has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// A land batch, by its index in the batch list.
    Batch(usize),
    /// A water batch, by its index in the water batches.
    Water(usize),
    /// Distance from the shore, which the water batches start from.
    Shore,
    /// The shoreline.
    Shoreline,
    /// Every pen's dashes put in plotting order.
    Order,
}

/// How far a step has got. A batch of lines goes through all of them in
/// order; dots and fills skip some, and a step done in one piece goes from
/// `Start` straight to `Done`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Stage {
    /// Begun: for a batch, building its fields.
    Start = 1,
    /// Tracing the lines, or placing the dots or the rings of a fill.
    Trace = 2,
    /// Putting the lines in plotting order.
    Order = 3,
    /// Cutting the lines into dashes.
    Dashes = 4,
    Done = 5,
}

/// Told of each stage a step reaches, from whichever thread runs it.
pub trait Progress: Sync {
    fn reach(&self, step: Step, stage: Stage);
}

/// Nobody watching.
impl Progress for () {
    fn reach(&self, _: Step, _: Stage) {}
}
