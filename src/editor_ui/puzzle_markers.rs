//! The canvas markers of the room's puzzle pieces: breakable walls, switches and barriers. Their
//! colors differ from the hazards' magenta and from every other marker's; the panel's legend
//! (`puzzle_panel::draw_legend`) names them.
use hollow_steps::editor::{EditorSession, Selection};
use macroquad::prelude::*;

/// Brown, like the stone of a wall.
const BREAKABLE: Color = Color::new(0.72, 0.50, 0.28, 1.0);
/// Teal, a plate to strike.
const SWITCH: Color = Color::new(0.10, 0.75, 0.72, 1.0);
/// Slate, iron bars.
const BARRIER: Color = Color::new(0.55, 0.62, 0.78, 1.0);

/// One marker for each wall, switch and barrier of the draft, over the piece's own rectangle.
pub(crate) fn markers(editor: &EditorSession) -> Vec<(Selection, Rect, Color)> {
    let layout = &editor.draft.layout;
    let walls = layout
        .breakables
        .iter()
        .enumerate()
        .map(|(i, wall)| (Selection::Breakable(i), wall.rect(), BREAKABLE));
    let switches = layout
        .switches
        .iter()
        .enumerate()
        .map(|(i, switch)| (Selection::Switch(i), switch.rect(), SWITCH));
    let barriers = layout
        .barriers
        .iter()
        .enumerate()
        .map(|(i, barrier)| (Selection::Barrier(i), barrier.rect(), BARRIER));
    walls.chain(switches).chain(barriers).collect()
}
