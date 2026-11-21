//! The editor window's two checks beside the room: what the last Check dungeon found
//! (`docs/dungeon-check.md`) and the Reach preview (`docs/reach.md`). The preview's interval is
//! measured on the real player, which takes a moment, so it is measured only while the Reach
//! toggle is on and again only when the numbers behind it change (`ReachKey`).
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::reach::{outline_labels, outlines, Interval, ReachKey};
use macroquad::prelude::*;

const LABEL_SIZE: f32 = 16.0;
/// Without Dash: a cool grey-blue that stands apart from the entrance markers.
const WITHOUT_DASH: Color = Color::new(0.62, 0.86, 0.95, 1.0);
/// With Dash: a warm pink, the same hue the Dash pickup is not.
const WITH_DASH: Color = Color::new(1.0, 0.45, 0.65, 1.0);

#[derive(Default)]
pub(crate) struct ChecksUi {
    /// What the last Check dungeon found, one sentence each; `None` before the first press.
    pub(crate) dungeon: Option<Vec<String>>,
    /// The Reach toggle of the View section.
    pub(crate) reach: bool,
    /// The interval and the numbers it was measured from.
    pub(super) measured: Option<(ReachKey, Interval)>,
}

impl ChecksUi {
    /// The interval for the numbers the editor holds now, measured again only when they differ
    /// from the numbers of the last measurement.
    pub(crate) fn interval(&mut self, editor: &EditorSession) -> Interval {
        let key = editor.reach_key();
        match &self.measured {
            Some((measured, interval)) if *measured == key => *interval,
            _ => {
                let interval = editor.reach_interval();
                self.measured = Some((key, interval));
                interval
            }
        }
    }
}

/// Draws the two outlines (and what each says) around the selected entrance while Reach is on,
/// and nothing while the interval is empty: the panel says why then.
pub(crate) fn draw_reach(
    checks: &mut ChecksUi,
    selected: Option<Selection>,
    editor: &EditorSession,
    canvas: &CanvasView,
) {
    if !checks.reach {
        return;
    }
    let interval = checks.interval(editor);
    if !interval.usable() {
        return;
    }
    let origin = editor.reach_origin(selected);
    let found = outlines(origin, editor.player.edited().player.size, &interval);
    let labels = outline_labels(&interval);
    for (world, label, color) in [
        (found.with, &labels[1], WITH_DASH),
        (found.without, &labels[0], WITHOUT_DASH),
    ] {
        let rect = canvas.rect(world);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
        let measured = measure_text(label, None, LABEL_SIZE as u16, 1.0);
        draw_text(
            label,
            rect.x + 4.0,
            rect.y + measured.offset_y + 2.0,
            LABEL_SIZE,
            color,
        );
    }
}

#[cfg(test)]
mod tests;
