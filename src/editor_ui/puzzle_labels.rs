//! The text on the canvas markers of the puzzle pieces: "3 hits" on a breakable wall, "S2" on
//! switch 2 and on the barrier it opens. `labels` decides what to write from the markers drawn
//! this frame (the text itself is `RoomDraft::marker_label`) and is tested without a window;
//! `draw` only paints it, in the middle of the marker like a key's number.
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::exit_lock::label_rect;
use hollow_steps::room::RoomEdge;
use macroquad::prelude::*;

const LABEL_SIZE: f32 = 16.0;
const LABEL_COLOR: Color = WHITE;

/// One piece of text to write on the canvas.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PuzzleLabel {
    pub(crate) text: String,
    /// The marker the text belongs to, in world coordinates.
    pub(crate) marker: Rect,
}

/// The labels for the puzzle markers among `markers`.
pub(crate) fn labels(
    editor: &EditorSession,
    markers: &[(Selection, Rect, Color)],
) -> Vec<PuzzleLabel> {
    markers
        .iter()
        .filter_map(|(selection, marker, _)| {
            Some(PuzzleLabel {
                text: editor.draft.marker_label(*selection)?,
                marker: *marker,
            })
        })
        .collect()
}

/// Writes the puzzle labels of `markers`.
pub(crate) fn draw(
    markers: &[(Selection, Rect, Color)],
    editor: &EditorSession,
    canvas: &CanvasView,
) {
    for label in labels(editor, markers) {
        let measured = measure_text(&label.text, None, LABEL_SIZE as u16, 1.0);
        let text = vec2(measured.width, LABEL_SIZE);
        let zone = canvas.rect(label.marker);
        // Centered: the edge only matters for a label beside an exit's band.
        let at = label_rect(RoomEdge::Left, true, zone, text);
        draw_text(
            &label.text,
            at.x,
            at.y + measured.offset_y,
            LABEL_SIZE,
            LABEL_COLOR,
        );
    }
}
