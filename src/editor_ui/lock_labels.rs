//! The key numbers the editor canvas writes: "Key 3" next to the band of an exit locked by key 3,
//! and "3" in the middle of a key's marker. `labels` decides what to write from the markers
//! drawn this frame and is tested without a window; `draw` only paints it. Where the text goes
//! is `exit_lock::label_rect`, the rule the game uses too.
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::exit_lock::{key_marker_label, label_rect, lock_label};
use hollow_steps::room::RoomEdge;
use macroquad::prelude::*;

const LABEL_SIZE: f32 = 16.0;
/// Beside an exit's band, on the dark canvas.
const EXIT_LABEL: Color = Color::new(0.96, 0.78, 0.30, 1.0);
/// In the middle of a key's gold marker.
const KEY_LABEL: Color = BLACK;

/// One piece of text to write on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LockLabel {
    pub(crate) text: String,
    /// The marker the text belongs to, in world coordinates.
    pub(crate) marker: Rect,
    /// The edge of an edge exit, which decides which side of its band the text goes on.
    pub(crate) edge: RoomEdge,
    /// In the middle of the marker (a key, or a door) instead of beside a band.
    pub(crate) centered: bool,
    pub(crate) color: Color,
}

/// The labels for the lock markers among `markers`: one for each locked exit and each key.
pub(crate) fn labels(
    editor: &EditorSession,
    markers: &[(Selection, Rect, Color)],
) -> Vec<LockLabel> {
    let layout = &editor.draft.layout;
    markers
        .iter()
        .filter_map(|(selection, marker, _)| match *selection {
            Selection::Exit(i) => {
                let exit = layout.exits.get(i)?;
                Some(LockLabel {
                    text: lock_label(exit.locked_by)?,
                    marker: *marker,
                    edge: exit.edge,
                    centered: exit.is_door(),
                    color: EXIT_LABEL,
                })
            }
            Selection::Item(i) => Some(LockLabel {
                text: key_marker_label(layout.items.get(i)?.item)?,
                marker: *marker,
                edge: RoomEdge::Left,
                centered: true,
                color: KEY_LABEL,
            }),
            _ => None,
        })
        .collect()
}

/// Writes the lock labels of `markers`.
pub(crate) fn draw(
    markers: &[(Selection, Rect, Color)],
    editor: &EditorSession,
    canvas: &CanvasView,
) {
    for label in labels(editor, markers) {
        let measured = measure_text(&label.text, None, LABEL_SIZE as u16, 1.0);
        let text = vec2(measured.width, LABEL_SIZE);
        let at = label_rect(label.edge, label.centered, canvas.rect(label.marker), text);
        draw_text(
            &label.text,
            at.x,
            at.y + measured.offset_y,
            LABEL_SIZE,
            label.color,
        );
    }
}
