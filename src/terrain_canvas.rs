//! Macroquad side of the terrain tools: draws solids, their handles and the rectangle being
//! drawn, and turns mouse state into `TerrainTool` calls. The rules live in
//! `hollow_steps::editor_terrain` and `hollow_steps::editor_tools`.
use hollow_steps::editor::{EditorSession, RoomLayout, Selection};
use hollow_steps::editor_history::tracked;
use hollow_steps::editor_terrain::Handle;
use hollow_steps::editor_tools::{EditorTool, TerrainTool};
use hollow_steps::editor_view::CanvasView;
use macroquad::prelude::*;

use crate::EditorUi;

/// Handle squares are this many screen pixels wide at any zoom.
const HANDLE_PIXELS: f32 = 9.0;
/// How close, in screen pixels, a click must be to grab a handle.
const HANDLE_REACH_PIXELS: f32 = 8.0;

type Markers = [(Selection, Rect, Color)];

/// Terrain tools work on the authored room, not the arrival-spawn view or the properties
/// panel.
fn enabled(state: &EditorUi) -> bool {
    state.destination.is_none() && !state.properties
}

fn report(editor: &mut EditorSession, result: Result<(), String>) {
    if let Err(error) = result {
        editor.message = error;
    }
}

/// How far a handle reaches, in world units, at the current zoom.
pub fn handle_reach(canvas: &CanvasView) -> f32 {
    HANDLE_REACH_PIXELS / canvas.scale
}

/// Select tool press. A solid takes the click when a handle of the selected solid is under
/// the cursor, or (with no marker there) the solid itself; it is selected and a move or
/// resize begins. Otherwise the solid selection is dropped and `false` is returned, so the
/// caller goes on to the markers.
pub fn press_select(
    state: &mut EditorUi,
    editor: &mut EditorSession,
    over_marker: bool,
    world: Vec2,
    reach: f32,
) -> bool {
    if !enabled(state) {
        return false;
    }
    let hit = state
        .terrain
        .hit_handle(&editor.draft, world, reach)
        .or_else(|| {
            if over_marker {
                None
            } else {
                TerrainTool::hit_body(&editor.draft, world)
            }
        });
    match hit {
        Some(hit) => {
            state.selected = None;
            state.dragging = None;
            state.terrain.grab(editor, &mut state.history, hit, world);
            true
        }
        None => {
            state.terrain.select(None);
            false
        }
    }
}

/// Terrain tool press: starts drawing a solid.
pub fn press_draw(state: &mut EditorUi, editor: &mut EditorSession, world: Vec2) {
    if enabled(state) {
        state.selected = None;
        state.dragging = None;
        state.terrain.begin_draw(editor, world, &state.grid);
    }
}

/// The left button is held: moves, resizes or sizes the solid being worked on.
pub fn drag(state: &mut EditorUi, editor: &mut EditorSession, world: Vec2) {
    let result = state.terrain.drag(editor, world, &state.grid);
    report(editor, result);
}

/// The left button was released: finishes the drawing, move or resize.
pub fn release(state: &mut EditorUi, editor: &mut EditorSession) {
    let result = state.terrain.release(editor, &mut state.history);
    report(editor, result);
}

/// Deletes one marker (enemy, item, exit, boss) as one undo step.
fn delete_marker(state: &mut EditorUi, editor: &mut EditorSession, selection: Selection) {
    match tracked(&mut state.history, editor, |e| e.draft.delete(selection)) {
        Ok(()) => {
            state.selected = None;
            state.dragging = None;
            state.destination = None;
            editor.dirty = true;
        }
        Err(error) => editor.message = error,
    }
}

/// Deletes the selected solid or marker (the Delete key and the panel button).
pub fn delete_selected(state: &mut EditorUi, editor: &mut EditorSession) {
    if state.terrain.selected().is_some() {
        let result = state
            .terrain
            .delete_selected(editor, &mut state.history)
            .map(|_| ());
        report(editor, result);
    } else if let Some(selection) = state.selected {
        delete_marker(state, editor, selection);
    }
}

/// Markers that deleting makes sense for; the spawn, the patrol handles and the arrivals of
/// other rooms' exits and of a link stay.
fn erasable(selection: Selection) -> bool {
    !matches!(
        selection,
        Selection::Spawn
            | Selection::PatrolLeft(_)
            | Selection::PatrolRight(_)
            | Selection::Inbound { .. }
            | Selection::LinkArrival(_)
    )
}

/// The marker drawn on top at `world`, if it can be deleted.
fn marker_at(markers: &Markers, world: Vec2) -> Option<Selection> {
    markers
        .iter()
        .rev()
        .find(|(selection, rect, _)| erasable(*selection) && rect.contains(world))
        .map(|(selection, _, _)| *selection)
}

/// Erase tool click and right-click: deletes what is under `world`. Markers sit above
/// solids, so they go first.
pub fn erase(state: &mut EditorUi, editor: &mut EditorSession, markers: &Markers, world: Vec2) {
    if let Some(selection) = marker_at(markers, world) {
        delete_marker(state, editor, selection);
    } else if enabled(state) {
        let result = state
            .terrain
            .erase_at(editor, &mut state.history, world)
            .map(|_| ());
        report(editor, result);
    }
}

/// The solid the Erase tool would delete at `world`, for the hover highlight.
pub fn erase_target(
    state: &EditorUi,
    editor: &EditorSession,
    markers: &Markers,
    world: Vec2,
) -> Option<usize> {
    (state.tool == EditorTool::Erase && enabled(state) && marker_at(markers, world).is_none())
        .then(|| editor.draft.terrain_at(world))
        .flatten()
}

fn solid_rect(&(x, y, w, h): &(f32, f32, f32, f32)) -> Rect {
    Rect::new(x, y, w, h)
}

fn size_label(rect: &Rect) -> String {
    format!("{:.0} x {:.0}", rect.w, rect.h)
}

/// Draws the outlines of the solids of `layout`, then the selection handles, the erase
/// highlight and the rectangle being drawn. Only the authored room is editable, so only it
/// shows a selection. `styled` says the solids are already drawn with the room's look (the
/// canvas preview): they are then left unfilled, and only the selected solid gets an outline,
/// so the picture is what the game shows. Without it each solid is a plain filled rectangle.
pub fn draw(
    state: &EditorUi,
    layout: &RoomLayout,
    canvas: &CanvasView,
    erase_target: Option<usize>,
    styled: bool,
) {
    let editable = enabled(state);
    let selected = state.terrain.selected().filter(|_| editable);
    for (index, solid) in layout.terrain.iter().enumerate() {
        let r = canvas.rect(solid_rect(solid));
        if !styled {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::from_rgba(70, 77, 112, 235));
        }
        let (width, color) = if selected == Some(index) {
            (3.0, WHITE)
        } else {
            (1.0, Color::from_rgba(150, 160, 205, 255))
        };
        if !styled || selected == Some(index) {
            draw_rectangle_lines(r.x, r.y, r.w, r.h, width, color);
        }
        if erase_target == Some(index) {
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(1.0, 0.2, 0.2, 0.45));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, RED);
        }
    }
    if let Some(rect) = selected.and_then(|i| layout.terrain.get(i)).map(solid_rect) {
        let r = canvas.rect(rect);
        if state.tool == EditorTool::Select {
            for handle in Handle::ALL {
                let at = canvas.screen(handle.position(&rect));
                draw_rectangle(
                    at.x - HANDLE_PIXELS / 2.0,
                    at.y - HANDLE_PIXELS / 2.0,
                    HANDLE_PIXELS,
                    HANDLE_PIXELS,
                    WHITE,
                );
                draw_rectangle_lines(
                    at.x - HANDLE_PIXELS / 2.0,
                    at.y - HANDLE_PIXELS / 2.0,
                    HANDLE_PIXELS,
                    HANDLE_PIXELS,
                    1.0,
                    BLACK,
                );
            }
        }
        draw_text(size_label(&rect), r.x, r.y - 6.0, 17.0, WHITE);
    }
    if let Some(rect) = state.terrain.preview().filter(|_| editable) {
        let r = canvas.rect(rect);
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.3, 0.9, 0.5, 0.35));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, GREEN);
        draw_text(size_label(&rect), r.x, r.y - 6.0, 17.0, GREEN);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hollow_steps::room::RoomId;

    #[test]
    fn the_eraser_skips_arrivals_of_other_rooms_and_of_a_link() {
        let rect = Rect::new(0.0, 0.0, 32.0, 48.0);
        let inbound = Selection::Inbound {
            room: RoomId::Second,
            exit: 0,
        };
        // Both arrivals are drawn on top of the item, yet the item is what gets erased.
        let markers = [
            (Selection::Item(0), rect, GOLD),
            (inbound, rect, PURPLE),
            (Selection::LinkArrival(0), rect, PURPLE),
        ];
        assert_eq!(
            marker_at(&markers, vec2(10.0, 10.0)),
            Some(Selection::Item(0))
        );
        assert_eq!(marker_at(&markers[1..], vec2(10.0, 10.0)), None);
    }
}
