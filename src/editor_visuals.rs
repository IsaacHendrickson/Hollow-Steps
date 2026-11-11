//! The room editor canvas's picture of enemies and the boss: the body (or plain box) drawn at
//! its visual size, with the hitbox outlined over it, so both can be judged together.
//!
//! `visual_previews` decides what to draw from the editor's current (possibly unsaved)
//! definitions and is tested without a window; the `draw_*` functions only paint it.

use crate::body_draw::draw_body;
use hollow_steps::body::BodyInstance;
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::visual_box;
use macroquad::prelude::*;

/// The thin outline around a drawn box. It shows the visual size even where a body leaves part
/// of its box empty, and stays distinct from the colored hitbox outline.
const VISUAL_OUTLINE: Color = Color::new(0.8, 0.9, 1.0, 0.55);
/// The same outline for the selected enemy or boss.
const SELECTED_VISUAL_OUTLINE: Color = Color::new(1.0, 1.0, 1.0, 0.95);
/// The fill of an enemy's or boss's hitbox: faint, so the picture under it stays visible.
const HITBOX_FILL_ALPHA: f32 = 0.15;
/// The fill of every other marker.
const MARKER_FILL_ALPHA: f32 = 0.45;

/// What to draw for one placed enemy or the boss.
pub(crate) struct VisualPreview {
    pub(crate) selection: Selection,
    /// The drawn box, in world coordinates.
    pub(crate) rect: Rect,
    /// The plain box's color, used when there is no body.
    pub(crate) color: Color,
    /// A fresh body at rest, for an enemy whose definition has one.
    pub(crate) body: Option<BodyInstance>,
}

/// A preview for each enemy and boss marker in `markers` (whose rectangles are the hitboxes).
/// Other markers, and enemies or bosses whose definition is missing, get none.
pub(crate) fn visual_previews(
    editor: &EditorSession,
    markers: &[(Selection, Rect, Color)],
) -> Vec<VisualPreview> {
    markers
        .iter()
        .filter_map(|(selection, hitbox, _)| {
            let (visual, color, body) = match *selection {
                Selection::Enemy(index) => {
                    let name = &editor.draft.enemies.get(index)?.definition;
                    (
                        editor.definitions.visual_size(name).ok()?,
                        MAROON,
                        editor.definitions.body_instance(name).ok()?,
                    )
                }
                Selection::Boss => {
                    let boss = editor.draft.boss.as_ref()?;
                    let definition = editor
                        .content
                        .boss_path(&boss.definition)
                        .and_then(|path| editor.content.bosses.get(&path))?;
                    (
                        visual_box::size(hitbox.size(), definition.visual_size),
                        PURPLE,
                        None,
                    )
                }
                _ => return None,
            };
            let center = visual_box::center(hitbox.center(), hitbox.size(), visual);
            Some(VisualPreview {
                selection: *selection,
                rect: Rect::new(
                    center.x - visual.x / 2.0,
                    center.y - visual.y / 2.0,
                    visual.x,
                    visual.y,
                ),
                color,
                body,
            })
        })
        .collect()
}

/// Paints the previews under the markers: the body scaled with the canvas zoom, or the plain
/// box, each with a thin outline of the visual box (brighter for the `selected` one).
pub(crate) fn draw_visual_previews(
    previews: &[VisualPreview],
    selected: Option<Selection>,
    canvas: &CanvasView,
) {
    for preview in previews {
        let screen = canvas.rect(preview.rect);
        match &preview.body {
            Some(body) => draw_body(body, screen.center(), screen.size(), false, |color| color),
            None => draw_rectangle(screen.x, screen.y, screen.w, screen.h, preview.color),
        }
        let outline = if selected == Some(preview.selection) {
            SELECTED_VISUAL_OUTLINE
        } else {
            VISUAL_OUTLINE
        };
        draw_rectangle_lines(screen.x, screen.y, screen.w, screen.h, 1.0, outline);
    }
}

/// Paints the markers: filled boxes, except that an enemy's or the boss's hitbox is only
/// faintly filled so the picture drawn under it shows. The selected marker has a white outline.
pub(crate) fn draw_markers(
    markers: &[(Selection, Rect, Color)],
    selected: Option<Selection>,
    canvas: &CanvasView,
) {
    for (selection, rect, color) in markers {
        let r = canvas.rect(*rect);
        let alpha = if matches!(selection, Selection::Enemy(_) | Selection::Boss) {
            HITBOX_FILL_ALPHA
        } else {
            MARKER_FILL_ALPHA
        };
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            Color::new(color.r, color.g, color.b, alpha),
        );
        let is_selected = selected == Some(*selection);
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            if is_selected { 3.0 } else { 1.0 },
            if is_selected { WHITE } else { *color },
        );
    }
}

#[cfg(test)]
mod tests;
