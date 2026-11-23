//! The room editor canvas's picture of enemies, the boss and hazards: the body (or plain box)
//! drawn at its visual size, with the hitbox outlined over it, so both can be judged together;
//! a hazard is drawn in its own look (shape, facing, color), which is its hitbox.
//!
//! `visual_previews` decides what to draw from the editor's current (possibly unsaved)
//! definitions and is tested without a window; the `draw_*` functions only paint it.

use crate::body_draw::draw_body;
use crate::terrain_draw;
use hollow_steps::body::BodyInstance;
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_view::CanvasView;
use hollow_steps::hazard::Hazard;
use hollow_steps::player_look_editor::PreviewSettings;
use hollow_steps::terrain_style::Shape;
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

/// The plain box drawn for the player when its look cannot be drawn.
const PLAYER_BOX: Color = Color::new(0.36, 0.52, 0.78, 0.9);
/// The outline of the player's hitbox in the Player panel's preview.
const PLAYER_HITBOX: Color = Color::new(0.45, 0.9, 1.0, 0.95);

/// What to draw for one placed enemy or the boss.
pub(crate) struct VisualPreview {
    pub(crate) selection: Selection,
    /// The drawn box, in world coordinates.
    pub(crate) rect: Rect,
    /// The plain box's color, used when there is no body.
    pub(crate) color: Color,
    /// A fresh body at rest, for an enemy or boss whose definition has one.
    pub(crate) body: Option<BodyInstance>,
    /// A hazard's draw list (lava at rest); empty for everything else.
    pub(crate) shapes: Vec<Shape>,
}

/// A preview for each enemy, boss and hazard marker in `markers` (whose rectangles are the
/// hitboxes). Other markers, and enemies or bosses whose definition is missing, get none.
pub(crate) fn visual_previews(
    editor: &EditorSession,
    markers: &[(Selection, Rect, Color)],
) -> Vec<VisualPreview> {
    markers
        .iter()
        .filter_map(|(selection, hitbox, _)| {
            let (visual, color, body) = match *selection {
                Selection::Hazard(index) => {
                    let hazard = Hazard::from(editor.draft.hazard(index)?);
                    return Some(VisualPreview {
                        selection: *selection,
                        rect: *hitbox,
                        color: MAROON,
                        body: None,
                        shapes: hazard.look(0.0),
                    });
                }
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
                        definition.body.clone().and_then(|visual| {
                            BodyInstance::new(visual, &editor.content.bodies).ok()
                        }),
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
                shapes: Vec::new(),
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
        if !preview.shapes.is_empty() {
            // A hazard: its look through the canvas zoom and pan. Its marker outlines it.
            let everywhere = Rect::new(-1.0e6, -1.0e6, 2.0e6, 2.0e6);
            terrain_draw::draw_shapes_mapped(&preview.shapes, everywhere, canvas.scale, |p| {
                canvas.screen(p)
            });
            continue;
        }
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

/// Paints the player at the room's spawn point as the Player panel has it: the unsaved look in
/// the chosen pose and facing, at the unsaved size, with the hitbox outlined if asked.
pub(crate) fn draw_player_preview(
    editor: &EditorSession,
    settings: PreviewSettings,
    canvas: &CanvasView,
) {
    let preview = editor.player_preview(settings);
    let screen = canvas.rect(Rect::new(
        preview.center.x - preview.size.x / 2.0,
        preview.center.y - preview.size.y / 2.0,
        preview.size.x,
        preview.size.y,
    ));
    match &preview.body {
        Some(body) => draw_body(
            body,
            screen.center(),
            screen.size(),
            preview.face_left,
            |c| c,
        ),
        None => draw_rectangle(screen.x, screen.y, screen.w, screen.h, PLAYER_BOX),
    }
    if preview.show_hitbox {
        draw_rectangle_lines(screen.x, screen.y, screen.w, screen.h, 2.0, PLAYER_HITBOX);
    }
}

/// Paints the markers: filled boxes, except that an enemy's, the boss's or a hazard's box is
/// only faintly filled so the picture drawn under it shows, and a pickup slot is a dashed ring
/// with its kind's letter. The selected marker has a white outline.
pub(crate) fn draw_markers(
    editor: &EditorSession,
    markers: &[(Selection, Rect, Color)],
    selected: Option<Selection>,
    canvas: &CanvasView,
) {
    for (selection, rect, color) in markers {
        let r = canvas.rect(*rect);
        if let Selection::Slot(index) = selection {
            if let Some(slot) = editor.draft.slot(*index) {
                slot_mark::draw(r, slot.kind, *color, selected == Some(*selection));
            }
            continue;
        }
        let alpha = if matches!(
            selection,
            Selection::Enemy(_) | Selection::Boss | Selection::Hazard(_)
        ) {
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

mod slot_mark;
#[cfg(test)]
mod tests;
