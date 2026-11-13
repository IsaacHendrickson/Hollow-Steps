//! The words next to the arrival markers: which room an inbound arrival comes from (and
//! what is wrong with it, if anything), and the arrival of the link being made.
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::exit_arrival::arrival_error;
use macroquad::prelude::*;

const LABEL_SIZE: f32 = 16.0;

/// What the label of `selection` says, if it has one.
pub(crate) fn label(editor: &EditorSession, selection: Selection, center: Vec2) -> Option<String> {
    match selection {
        Selection::Inbound { room, .. } => {
            let from = editor.title_of(room);
            Some(match arrival_error(&editor.draft.layout, center) {
                Some(error) => format!("from {from}: {error}"),
                None => format!("from {from}"),
            })
        }
        Selection::LinkArrival(_) => Some("Arrival".into()),
        _ => None,
    }
}

/// Writes the labels above the markers of `markers` that have one.
pub(crate) fn draw(
    markers: &[(Selection, Rect, Color)],
    editor: &EditorSession,
    canvas: &CanvasView,
) {
    for (selection, rect, color) in markers {
        if let Some(text) = label(editor, *selection, rect.center()) {
            let r = canvas.rect(*rect);
            draw_text(&text, r.x, r.y - 6.0, LABEL_SIZE, *color);
        }
    }
}
