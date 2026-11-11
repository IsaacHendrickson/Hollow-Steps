//! The room editor's window state and canvas drawing, moved out of `main.rs` (which keeps the
//! frame loop). `EditorUi` is everything the editor remembers between frames that is not part
//! of the edited room; `draw_editor` runs one editor frame.

use crate::editor_panel::{BossPropertyUi, PropertyUi};
use crate::{body_draw, editor_panel, terrain_canvas};
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_guard::{perform, GuardStep, GuardedAction, UnsavedGuard};
use hollow_steps::editor_history::{shortcut, tracked, History};
use hollow_steps::editor_tools::{tool_shortcut, EditorTool, TerrainTool};
use hollow_steps::editor_view::{
    grid_lines, wrap_words, CanvasRegions, CanvasView, Grid, PanDrag, CANVAS_MARGIN,
};
use hollow_steps::game::GameState;
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::search::SearchList;
use macroquad::prelude::*;

#[derive(Default)]
pub(crate) struct EditorUi {
    pub(crate) definition: SearchList,
    pub(crate) room: SearchList,
    pub(crate) target: SearchList,
    pub(crate) new_entity: String,
    pub(crate) edge: usize,
    /// What a click or drag on the canvas does.
    pub(crate) tool: EditorTool,
    /// The selected solid and the solid drag in progress.
    pub(crate) terrain: TerrainTool,
    pub(crate) canvas_focused: bool,
    pub(crate) selected: Option<Selection>,
    pub(crate) dragging: Option<(Selection, Vec2)>,
    pub(crate) destination: Option<usize>,
    pub(crate) properties: bool,
    pub(crate) property: PropertyUi,
    pub(crate) boss_property: Option<BossPropertyUi>,
    /// Canvas transform after the user zooms or pans; `None` fits the room to the window.
    pub(crate) view: Option<CanvasView>,
    /// The destination view `view` belongs to; switching rooms refits.
    pub(crate) view_destination: Option<usize>,
    pub(crate) pan: PanDrag,
    pub(crate) grid: Grid,
    /// Asks before closing or replacing a session with unsaved changes.
    pub(crate) guard: UnsavedGuard,
    /// A room switch asked for by the panel this frame (New room); `draw_editor` runs it
    /// through `guard`.
    pub(crate) requested: Option<GuardedAction>,
    /// Undo and redo for the draft; reset with the rest of this state when a room is opened
    /// or created.
    pub(crate) history: History,
}

impl EditorUi {
    /// State for a newly opened or created room, or a reopened editor: everything resets
    /// (selection, view, undo history, dialogs) except the grid and snap settings, which
    /// belong to the editor rather than to a room.
    pub(crate) fn for_new_session(&self) -> Self {
        Self {
            grid: self.grid,
            ..Self::default()
        }
    }

    fn clear_hidden_property_error(&mut self, enemy_count: usize) {
        if !self.properties
            || !matches!(self.selected, Some(Selection::Enemy(i)) if i < enemy_count)
        {
            self.property.error = None;
        }
    }
}

fn editor_markers(
    editor: &EditorSession,
    destination: Option<usize>,
    selected: Option<Selection>,
) -> Vec<(Selection, Rect, Color)> {
    let body =
        |p: Vec2, size: Vec2| Rect::new(p.x - size.x / 2.0, p.y - size.y / 2.0, size.x, size.y);
    if let Some(i) = destination {
        return editor
            .draft
            .layout
            .exits
            .get(i)
            .map(|exit| {
                vec![(
                    Selection::Destination(i),
                    body(exit.spawn.into(), vec2(32.0, 48.0)),
                    PURPLE,
                )]
            })
            .unwrap_or_default();
    }
    let mut markers = Vec::new();
    for (i, enemy) in editor.draft.enemies.iter().enumerate() {
        if let Ok(size) = editor.definitions.size(&enemy.definition) {
            markers.push((
                Selection::Enemy(i),
                body(
                    vec2(
                        enemy.patrol_left,
                        editor.draft.layout.floor_y - size.y / 2.0,
                    ),
                    size,
                ),
                RED,
            ));
        }
    }
    if let Some(boss) = &editor.draft.boss {
        if let Some(definition) = editor
            .content
            .boss_path(&boss.definition)
            .and_then(|p| editor.content.bosses.get(&p))
        {
            markers.push((
                Selection::Boss,
                body(boss.position.into(), definition.size.into()),
                PURPLE,
            ));
        }
    }
    for (i, item) in editor.draft.layout.items.iter().enumerate() {
        markers.push((
            Selection::Item(i),
            body(item.position.into(), ITEM_SIZE),
            GOLD,
        ));
    }
    for (i, exit) in editor.draft.layout.exits.iter().enumerate() {
        markers.push((
            Selection::Exit(i),
            exit.exit().zone(editor.draft.layout.rect()),
            GREEN,
        ));
    }
    markers.push((
        Selection::Spawn,
        body(editor.draft.layout.spawn.into(), vec2(32.0, 48.0)),
        SKYBLUE,
    ));
    if let Some(Selection::Enemy(i) | Selection::PatrolLeft(i) | Selection::PatrolRight(i)) =
        selected
    {
        if let Some(enemy) = editor.draft.enemies.get(i) {
            if let Ok(size) = editor.definitions.size(&enemy.definition) {
                let y = editor.draft.layout.floor_y - size.y - 14.0;
                for (selection, x, handle_y) in [
                    (Selection::PatrolLeft(i), enemy.patrol_left, y),
                    (Selection::PatrolRight(i), enemy.patrol_right, y - 24.0),
                ] {
                    markers.push((selection, body(vec2(x, handle_y), vec2(14.0, 20.0)), ORANGE));
                }
            }
        }
    }
    markers
}

pub(crate) fn draw_editor(
    game: &mut GameState,
    state: &mut EditorUi,
    body_debug: bool,
    body_debug_overlay: &body_draw::BodyDebugOverlay,
) {
    let background = Color::from_rgba(16, 18, 31, 255);
    clear_background(background);
    let player_position = game.player().position();
    let editor = game.editor_mut().expect("editor open");
    if let Ok(definitions) = editor.content.placement_definitions() {
        editor.definitions = definitions;
    }
    let mut panel = None;
    let mut dialog = None;
    egui_macroquad::ui(|ctx| {
        // The integration divides native wheel events by 90. Normalize once to 28 points per
        // wheel step; egui then routes/consumes the delta for the hovered scroll area.
        let wheel = mouse_wheel();
        let delta =
            egui_macroquad::egui::vec2(wheel.0.clamp(-3.0, 3.0), wheel.1.clamp(-3.0, 3.0)) * 28.0;
        ctx.input_mut(|input| {
            input.raw_scroll_delta = delta;
            input.smooth_scroll_delta = delta;
        });
        panel = Some(editor_panel::draw_panel(ctx, editor, state));
        dialog = editor_panel::draw_unsaved_dialog(ctx, &state.guard);
    });
    let panel = panel.expect("egui panel");
    // While the dialog is open nothing behind it may take input.
    let pointer_blocked = panel.blocks_pointer || state.guard.is_open();
    let keys_blocked = panel.keyboard || state.guard.is_open();
    let mut save = panel.save;
    let test = panel.test && !state.guard.is_open();
    let open = panel.open;

    let layout = state
        .destination
        .and_then(|i| editor.draft.layout.exits.get(i))
        .and_then(|exit| {
            editor
                .catalog()
                .into_iter()
                .find(|(id, _)| *id == exit.target_room)
                .map(|(_, layout)| layout)
        })
        .unwrap_or_else(|| editor.draft.layout.clone());
    // The status message is wrapped first: its line count sets the footer height, which
    // sets the canvas viewport. The wrap width depends only on the window and panel width.
    let window = vec2(screen_width(), screen_height());
    let wrap_width = CanvasRegions::new(window, panel.left, 0).footer_wrap_width();
    let footer_lines = wrap_words(&editor.message, wrap_width, |text| {
        measure_text(text, None, 17, 1.0).width
    });
    let regions = CanvasRegions::new(window, panel.left, footer_lines.len());
    let mouse = Vec2::from(mouse_position());
    let over_viewport = regions.viewport.contains(mouse) && !pointer_blocked;
    if state.view_destination != state.destination {
        state.view = None;
        state.view_destination = state.destination;
    }
    if is_key_pressed(KeyCode::Home) && !keys_blocked {
        state.view = None;
    }
    let mut canvas = state
        .view
        .unwrap_or_else(|| CanvasView::fit(layout.rect(), regions.fit_area()));
    // Over the canvas the wheel zooms about the cursor. Over the panel it only scrolls the
    // panel: egui gets the wheel delta above, and only the hovered scroll area uses it.
    let pan_held = is_mouse_button_down(MouseButton::Middle)
        || (is_key_down(KeyCode::Space)
            && !keys_blocked
            && is_mouse_button_down(MouseButton::Left));
    let pan_delta = state
        .pan
        .update(pan_held && (state.pan.is_panning() || over_viewport), mouse);
    let wheel = if over_viewport { mouse_wheel().1 } else { 0.0 };
    let mut moved = false;
    if wheel != 0.0 {
        canvas.zoom_at(mouse, wheel);
        moved = true;
    }
    if pan_delta != Vec2::ZERO {
        canvas.pan_by(pan_delta);
        moved = true;
    }
    if moved {
        canvas.keep_visible(layout.rect(), regions.viewport);
        state.view = Some(canvas);
    }
    // While a pan is in progress the left button belongs to it, not to the markers.
    let panning = state.pan.is_panning();
    let area = canvas.rect(layout.rect());
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, LIGHTGRAY);
    // Grid lines start at the room's corner so they line up with its edges, and snapping
    // uses the same origin.
    let origin = layout.rect().point();
    if let Some(step) = state.grid.line_step(canvas.scale) {
        let top_left = canvas.world(regions.viewport.point());
        let seen = Rect::new(
            top_left.x,
            top_left.y,
            regions.viewport.w / canvas.scale,
            regions.viewport.h / canvas.scale,
        );
        if let Some(visible) = layout.rect().intersect(seen) {
            let from = canvas.screen(visible.point());
            let to = canvas.screen(vec2(visible.right(), visible.bottom()));
            let color = Color::new(1.0, 1.0, 1.0, 0.12);
            for x in grid_lines(visible.x, visible.right(), origin.x, step) {
                let x = canvas.screen(vec2(x, 0.0)).x;
                draw_line(x, from.y, x, to.y, 1.0, color);
            }
            for y in grid_lines(visible.y, visible.bottom(), origin.y, step) {
                let y = canvas.screen(vec2(0.0, y)).y;
                draw_line(from.x, y, to.x, y, 1.0, color);
            }
        }
    }
    if state.destination.is_some() {
        for exit in &layout.exits {
            let r = canvas.rect(exit.exit().zone(layout.rect()));
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, GREEN);
        }
    }
    let floor = canvas.screen(vec2(layout.bounds.0, layout.floor_y));
    draw_line(floor.x, floor.y, area.right(), floor.y, 3.0, GRAY);
    let markers = editor_markers(editor, state.destination, state.selected);
    let world = canvas.world(mouse);
    let over_canvas = area.contains(mouse) && over_viewport;
    state.terrain.sync(&editor.draft);
    let reach = terrain_canvas::handle_reach(&canvas);
    if is_mouse_button_pressed(MouseButton::Left) {
        state.canvas_focused = over_canvas;
    }
    if is_mouse_button_pressed(MouseButton::Left) && over_canvas && !panning {
        if state.tool == EditorTool::Place && state.destination.is_none() {
            if let Some((_, placement)) = state
                .definition
                .selected
                .and_then(|i| editor.content.placement_catalog().get(i).cloned())
            {
                let at = state.grid.snap(world, origin);
                match tracked(&mut state.history, editor, |e| e.place(placement, at)) {
                    Ok(selection) => {
                        state.selected = Some(selection);
                        editor.dirty = true;
                        editor.message.clear();
                    }
                    Err(error) => editor.message = error,
                }
            }
        } else if state.tool == EditorTool::Terrain && state.destination.is_none() {
            terrain_canvas::press_draw(state, editor, world);
        } else if state.tool == EditorTool::Erase && state.destination.is_none() {
            terrain_canvas::erase(state, editor, &markers, world);
        } else {
            let hit = markers.iter().rev().find(|(_, rect, _)| {
                let r = canvas.rect(*rect);
                Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0).contains(mouse)
            });
            // A solid (or a handle of the selected one) can take the click before the markers.
            if terrain_canvas::press_select(state, editor, hit.is_some(), world, reach) {
                // The solid is selected and its move or resize has begun.
            } else {
                state.selected = hit.map(|(selection, _, _)| match *selection {
                    Selection::PatrolLeft(i) | Selection::PatrolRight(i) => Selection::Enemy(i),
                    other => other,
                });
                state.dragging =
                    hit.map(|(selection, rect, _)| (*selection, world - rect.center()));
                if state.dragging.is_some() {
                    // The whole drag, press to release, becomes one undo step.
                    state.history.begin_gesture(&editor.draft);
                }
                if state.selected == Some(Selection::Boss) {
                    if let Some(path) = editor
                        .draft
                        .boss
                        .as_ref()
                        .and_then(|b| editor.content.boss_path(&b.definition))
                    {
                        if !state.boss_property.as_ref().is_some_and(|p| p.path == path) {
                            state.boss_property = Some(BossPropertyUi::new(editor, path));
                        }
                        state.properties = true;
                        state.property.error = None;
                    }
                } else if state.properties {
                    state.boss_property = None;
                    if let Some(Selection::Enemy(index)) = state.selected {
                        let path = editor
                            .content
                            .entity_path(&editor.draft.enemies[index].definition);
                        state.property = PropertyUi::for_entity(editor, path);
                    }
                }
            }
        }
    }
    if is_mouse_button_down(MouseButton::Left) && !panning {
        if state.terrain.has_gesture() {
            terrain_canvas::drag(state, editor, world);
        } else if let (true, Some((selection, offset))) = (over_canvas, state.dragging) {
            let target = state.grid.snap(world - offset, origin);
            match editor.move_selection(selection, target) {
                Ok(()) => {
                    editor.dirty = true;
                    editor.message.clear();
                }
                Err(error) => editor.message = error,
            }
        }
    }
    // Also covers a release that was missed (focus lost mid-drag), so a drag cannot stay
    // open and block undo.
    if !is_mouse_button_down(MouseButton::Left) {
        state.dragging = None;
        terrain_canvas::release(state, editor);
        state.history.end_gesture(&editor.draft);
    }
    if is_mouse_button_pressed(MouseButton::Right) && over_canvas {
        terrain_canvas::erase(state, editor, &markers, world);
    } else if is_key_pressed(KeyCode::Delete)
        && state.canvas_focused
        && !keys_blocked
        && !pointer_blocked
    {
        terrain_canvas::delete_selected(state, editor);
    }
    if state.canvas_focused && !keys_blocked {
        let down = |left, right| is_key_down(left) || is_key_down(right);
        if let Some(key) = shortcut(
            down(KeyCode::LeftControl, KeyCode::RightControl),
            down(KeyCode::LeftShift, KeyCode::RightShift),
            is_key_pressed(KeyCode::Z),
            is_key_pressed(KeyCode::Y),
        ) {
            editor_panel::apply_history(editor, state, key);
        }
        // Tool keys are for the room tools; the properties panel hides the tool buttons.
        let modified = down(KeyCode::LeftControl, KeyCode::RightControl)
            || down(KeyCode::LeftAlt, KeyCode::RightAlt);
        if let Some(tool) = tool_shortcut(
            modified || state.properties,
            is_key_pressed(KeyCode::V),
            is_key_pressed(KeyCode::P),
            is_key_pressed(KeyCode::T),
            is_key_pressed(KeyCode::E),
        ) {
            editor_panel::choose_tool(editor, state, tool);
        }
    }
    let erase_target =
        terrain_canvas::erase_target(state, editor, &markers, world).filter(|_| over_canvas);
    terrain_canvas::draw(state, &layout, &canvas, erase_target);
    for (selection, rect, color) in editor_markers(editor, state.destination, state.selected) {
        let r = canvas.rect(rect);
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            Color::new(color.r, color.g, color.b, 0.45),
        );
        draw_rectangle_lines(
            r.x,
            r.y,
            r.w,
            r.h,
            if state.selected == Some(selection) {
                3.0
            } else {
                1.0
            },
            if state.selected == Some(selection) {
                WHITE
            } else {
                color
            },
        );
    }
    if body_debug && state.destination.is_none() {
        // Preview authored placements and unsaved definitions without touching runtime bodies.
        let enemies = editor
            .draft
            .enemies
            .iter()
            .filter_map(|placement| {
                editor
                    .definitions
                    .spawn(
                        &placement.definition,
                        placement.patrol_left,
                        placement.patrol_right,
                        editor.draft.layout.world_bounds(),
                    )
                    .ok()
            })
            .collect::<Vec<_>>();
        let boss_preview = editor.draft.boss.as_ref().and_then(|placement| {
            editor
                .content
                .bosses
                .values()
                .find(|definition| definition.name == placement.definition)
                .and_then(|definition| {
                    BossSpecials::from_definition(
                        std::sync::Arc::new(definition.clone()),
                        placement.position.into(),
                    )
                    .ok()
                })
        });
        body_draw::draw_body_debug_overlay(
            body_debug_overlay,
            &enemies,
            boss_preview.as_ref(),
            player_position,
            |position| canvas.screen(position),
            canvas.scale,
        );
    }
    // Zoomed content can reach past the viewport, so the header and footer are drawn over
    // it on an opaque strip. The egui panel covers the right side.
    for r in [regions.header, regions.footer] {
        draw_rectangle(r.x, r.y, r.w, r.h, background);
    }
    draw_text(
        format!(
            "{:?} - {}",
            editor.id,
            if state.destination.is_some() {
                "destination room view"
            } else {
                "authored room"
            }
        ),
        CANVAS_MARGIN,
        30.0,
        24.0,
        WHITE,
    );
    draw_text(
        "F2: close | V/P/T/E: tools | Drag: move | Right click / Del: delete",
        CANVAS_MARGIN,
        52.0,
        17.0,
        LIGHTGRAY,
    );
    draw_text(
        "Wheel: zoom | Middle/Space-drag: pan | Home: reset | Ctrl+Z / Ctrl+Y: undo / redo",
        CANVAS_MARGIN,
        70.0,
        17.0,
        LIGHTGRAY,
    );
    // Status text sits in the footer so long filesystem/validation errors stay readable.
    for (i, line) in footer_lines.iter().enumerate() {
        draw_text(
            line,
            CANVAS_MARGIN,
            regions.footer_baseline(i),
            17.0,
            ORANGE,
        );
    }
    let pending_error = if let Some(p) = &mut state.boss_property {
        p.sync(editor);
        p.error.clone()
    } else if state.properties {
        state.property.error.clone()
    } else {
        None
    };
    state.clear_hidden_property_error(editor.draft.enemies.len());
    if save && pending_error.is_some() {
        editor.message = format!(
            "Save failed: {}",
            pending_error.as_deref().unwrap_or_default()
        );
        save = false;
    }
    egui_macroquad::draw();
    if test {
        // The main loop notices the switch and resets its clocks and camera. A refusal
        // is left in the editor's message.
        if game.start_test_room().is_ok() {
            return;
        }
    }
    if save {
        state.dragging = None;
        if let Err(error) = game.save_editor() {
            game.editor_mut().expect("open").message = format!("Save failed: {error}");
        }
    }
    // Closing, opening a room and New room all go through the unsaved-changes guard.
    let dirty = game.editor().is_some_and(|session| session.dirty);
    let mut step = GuardStep::Wait;
    if let Some(action) = state.requested.take().or(open.map(GuardedAction::OpenRoom)) {
        step = state.guard.request(action, dirty);
    }
    if let Some(choice) = dialog {
        step = state.guard.choose(choice);
    }
    if step == GuardStep::Save {
        // Same rule as the panel's Save: unapplied invalid text blocks it.
        let result = match &pending_error {
            Some(error) => Err(format!("Save failed: {error}")),
            None => game
                .save_editor()
                .map_err(|error| format!("Save failed: {error}")),
        };
        if let Err(error) = &result {
            game.editor_mut().expect("open").message = error.clone();
        }
        step = state.guard.save_finished(result);
    }
    if let GuardStep::Perform(action) = step {
        let result = perform(game, &action);
        if action != GuardedAction::Close {
            *state = state.for_new_session();
        }
        if let (Err(error), Some(editor)) = (result, game.editor_mut()) {
            editor.message = error;
        }
    }
}

#[cfg(test)]
mod tests;
