//! The editor window's two checks beside the room: what the last Check dungeon found
//! (`docs/dungeon-check.md`) and the Reach preview (`docs/reach.md`), and whether the Dungeon map
//! window is open (`docs/dungeon-map.md`) and the generated dungeon preview (`docs/dungeon-gen.md`).
//! The Reach preview's interval is
//! measured on the real player, which takes a moment, so it is measured only while the Reach
//! toggle is on and again only when the numbers behind it change (`ReachKey`).
use hollow_steps::ability::Ability;
use hollow_steps::dungeon_preview::DungeonPreview;
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::reach::{outline_label, outlines, Interval, ReachKey};
use macroquad::prelude::*;

const LABEL_SIZE: f32 = 16.0;
/// With no ability: a cool grey-blue that stands apart from the entrance markers.
const WITHOUT_ABILITY: Color = Color::new(0.62, 0.86, 0.95, 1.0);

/// The outline color of an ability, each its own hue; a new ability has to pick one here.
fn color_of(ability: Ability) -> Color {
    match ability {
        // A warm pink, the same hue the Dash pickup is not.
        Ability::Dash => Color::new(1.0, 0.45, 0.65, 1.0),
        // A fresh yellow-green.
        Ability::DoubleJump => Color::new(0.74, 0.93, 0.42, 1.0),
    }
}

#[derive(Default)]
pub(crate) struct ChecksUi {
    /// What the last Check dungeon found, one sentence each; `None` before the first press.
    pub(crate) dungeon: Option<Vec<String>>,
    /// The Reach toggle of the View section.
    pub(crate) reach: bool,
    /// The Dungeon map window is open. Opening another room closes it.
    pub(crate) map_window: bool,
    /// The Preview generated dungeon window is open (`editor_panel::preview_window`).
    pub(crate) preview_window: bool,
    /// Its seed box and the plan it last built; it keeps them while the window is closed.
    pub(crate) preview: DungeonPreview,
    /// The interval and the numbers it was measured from.
    pub(crate) measured: Option<(ReachKey, Interval)>,
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

/// Draws the outline of a jump with no ability and one for each ability some gap or ledge can
/// need (and what each says), around the selected entrance while Reach is on. Draws nothing while
/// no ability has such a gap or ledge: the panel says why then.
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
    let origin = editor.reach_origin(selected);
    let found = outlines(origin, editor.player.edited().player.size, &interval);
    if found.with.is_empty() {
        return;
    }
    // The plain jump last, so its box and label lie on top of the others.
    let drawn = found
        .with
        .iter()
        .map(|(ability, rect)| (Some(*ability), *rect, color_of(*ability)))
        .chain([(None, found.without, WITHOUT_ABILITY)]);
    for (ability, world, color) in drawn {
        let rect = canvas.rect(world);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, color);
        let label = outline_label(&interval, ability);
        let measured = measure_text(&label, None, LABEL_SIZE as u16, 1.0);
        draw_text(
            &label,
            rect.x + 4.0,
            rect.y + measured.offset_y + 2.0,
            LABEL_SIZE,
            color,
        );
    }
}

#[cfg(test)]
mod tests;
