//! What a draw is doing while it runs, for a progress display: the steps it
//! goes through, what each is called, and how far each has got.

use crate::water::{Draw, Shoreline, WaterBatch};
use crate::Batch;

/// A part of a draw that reports how far it has got.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Water coverage from the water tiles; reported by the caller.
    WaterMask,
    /// The terrain fields, before any batch; reported by the caller.
    Terrain,
    /// A land batch, by its index in the batch list.
    Batch(usize),
    /// Distance from the shore, which the water batches start from.
    Shore,
    /// A water batch, by its index in the water batches.
    Water(usize),
    /// The shoreline.
    Shoreline,
    /// Every pen's dashes put in plotting order.
    Order,
    /// The drawing written out as SVG; reported by the caller.
    Svg,
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

impl Stage {
    pub const ALL: [Stage; 5] = [
        Stage::Start,
        Stage::Trace,
        Stage::Order,
        Stage::Dashes,
        Stage::Done,
    ];

    /// What this stage is called for a batch that draws `draw`.
    pub fn name(self, draw: Draw) -> &'static str {
        match (self, draw) {
            (Stage::Start, _) => "fields",
            (Stage::Trace, Draw::Fill) => "rings",
            (Stage::Trace, Draw::Lines) => "trace",
            (Stage::Trace, d) => d.name(),
            (Stage::Order, _) => "order",
            (Stage::Dashes, _) => "dashes",
            (Stage::Done, _) => "done",
        }
    }
}

/// Told of each stage a step reaches, from whichever thread runs it.
pub trait Progress: Sync {
    fn reach(&self, step: Step, stage: Stage);
}

/// Nobody watching.
impl Progress for () {
    fn reach(&self, _: Step, _: Stage) {}
}

/// One line of a progress display.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub step: Step,
    /// What the step is called; for a batch, its [`Batch::label`].
    pub label: String,
    /// What a batch draws, which names its stages; `None` for a step done in
    /// one piece.
    pub draw: Option<Draw>,
}

impl Row {
    /// The names of the stages this row goes through before `Done`, in order;
    /// empty for a step done in one piece.
    pub fn stages(&self) -> Vec<&'static str> {
        match self.draw {
            Some(d) => Stage::ALL[..4].iter().map(|s| s.name(d)).collect(),
            None => Vec::new(),
        }
    }
}

/// Every step a draw of `batches`, with `water` when it has some, reports,
/// in the order a display should list them.
pub fn rows(batches: &[Batch], water: Option<(&[WaterBatch], Option<&Shoreline>)>) -> Vec<Row> {
    let row = |step, label: &str, draw| Row {
        step,
        label: label.into(),
        draw,
    };
    let mut out = Vec::new();
    if water.is_some() {
        out.push(row(Step::WaterMask, "water mask", None));
    }
    out.push(row(Step::Terrain, "terrain", None));
    for (i, b) in batches.iter().enumerate() {
        out.push(row(Step::Batch(i), &b.label(i), Some(b.draw)));
    }
    if let Some((wet, shoreline)) = water {
        out.push(row(Step::Shore, "shore", None));
        for (i, b) in wet.iter().enumerate() {
            out.push(row(Step::Water(i), &b.label(i), Some(b.draw)));
        }
        if shoreline.is_some() {
            out.push(row(Step::Shoreline, SHORELINE, None));
        }
    }
    out.push(row(Step::Order, "plot order", None));
    out.push(row(Step::Svg, "svg", None));
    out
}

/// What the shoreline is called, and its layer unless it sets one.
pub const SHORELINE: &str = "shoreline";

/// A batch's label: its `name`, else its `layer`, else its number from 1 and
/// what it draws (`what`), as "2 dots".
pub(crate) fn label(name: Option<&str>, layer: Option<&str>, i: usize, what: &str) -> String {
    [name, layer]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|s| !s.is_empty())
        .map_or_else(|| format!("{} {what}", i + 1), str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Style;

    #[test]
    fn rows_follow_the_document() {
        let batches = [
            Batch::along(Style::default()),
            Batch {
                name: Some("Valleys".into()),
                draw: Draw::Dots,
                ..Batch::along(Style::default())
            },
            Batch {
                layer: Some("paper".into()),
                draw: Draw::Fill,
                ..Batch::along(Style::default())
            },
        ];
        let sea = [WaterBatch {
            draw: Draw::Ripples,
            ..WaterBatch::default()
        }];
        let labels = |r: Vec<Row>| r.into_iter().map(|r| r.label).collect::<Vec<_>>();
        assert_eq!(
            labels(rows(&batches, None)),
            [
                "terrain",
                "1 lines",
                "Valleys",
                "paper",
                "plot order",
                "svg"
            ]
        );
        let shore = Shoreline::default();
        let all = rows(&batches[..1], Some((&sea, Some(&shore))));
        assert_eq!(
            labels(all.clone()),
            [
                "water mask",
                "terrain",
                "1 lines",
                "shore",
                "1 sea ripples",
                "shoreline",
                "plot order",
                "svg"
            ]
        );
        assert_eq!(all[4].stages(), ["fields", "ripples", "order", "dashes"]);
        assert!(all[0].stages().is_empty());
    }
}
