//! The room editor's window state and canvas drawing, moved out of `main.rs` (which keeps the
//! frame loop). `EditorUi` is everything the editor remembers between frames that is not part
//! of the edited room; `draw_editor` runs one editor frame.

use crate::editor_panel::{BossPropertyUi, LookUi, PropertyUi};
use crate::player_panel::Status;
use crate::{body_draw, editor_panel, editor_visuals, terrain_canvas, terrain_draw};
use hollow_steps::editor::Selection;
use hollow_steps::editor_guard::{perform, save_all, GuardStep, GuardedAction, UnsavedGuard};
use hollow_steps::editor_history::{shortcut, tracked, History};
use hollow_steps::editor_tools::{tool_shortcut, EditorTool, TerrainTool};
use hollow_steps::editor_view::{
    grid_lines, wrap_words, CanvasRegions, CanvasView, Grid, PanDrag, CANVAS_MARGIN,
};
use hollow_steps::game::GameState;
use hollow_steps::room::RoomId;
use hollow_steps::room_name::NamePrompt;
use hollow_steps::search::SearchList;
use macroquad::prelude::*;

pub(crate) use markers::{editor_markers, selected_by_press};

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
    /// The "New room" window while it is open: it asks for the name before the unsaved-changes
    /// guard is consulted.
    pub(crate) name_prompt: Option<NamePrompt>,
    /// Undo and redo for the draft; reset with the rest of this state when a room is opened
    /// or created.
    pub(crate) history: History,
    /// The Room look section's drag state and the canvas preview's cached draw list.
    pub(crate) look: LookUi,
    /// The Player panel: whether it is open, its Save request and what it last said.
    pub(crate) player: crate::player_panel::PlayerUi,
    /// Room A's window state while B is open in the exit-link flow.
    pub(crate) link_stash: Option<Box<linking::LinkStash>>,
    /// "New exit here" was pressed: the next click in the room places an exit.
    pub(crate) link_placing: bool,
    /// Undo history of the rooms the exit-link flow parked with unsaved changes.
    pub(crate) parked_histories: Vec<(RoomId, History)>,
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
    let mut naming = None;
    let mut link_choice = None;
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
        let unsaved = editor.unsaved_rooms();
        dialog = editor_panel::draw_unsaved_dialog_titled(
            ctx,
            &state.guard,
            &|id| editor.title_of(id),
            &unsaved,
        );
        link_choice = editor_panel::draw_link_dialog(ctx, editor);
        if let Some(prompt) = &mut state.name_prompt {
            naming = editor_panel::draw_name_dialog(ctx, editor, prompt);
        }
    });
    let panel = panel.expect("egui panel");
    // While a window of the exit-link flow is open nothing behind it may take input.
    let modal =
        state.guard.is_open() || state.name_prompt.is_some() || linking::link_dialog_open(editor);
    if let Some(choice) = link_choice {
        state.apply_link_choice(editor, choice);
    }
    // With B open only exits can be selected, placed and dragged: no other tool, no delete,
    // no undo, so Cancel never has to undo anything.
    let linking = linking::link_connecting(editor);
    if linking {
        state.tool = EditorTool::Select;
        state.properties = false;
    }
    let pointer_blocked = panel.blocks_pointer || modal;
    let keys_blocked = panel.keyboard || modal;
    let mut save = panel.save;
    let test = panel.test && !modal;
    let open = panel.open;

    let (layout_room, layout) = state
        .destination
        .and_then(|i| editor.draft.layout.exits.get(i))
        .and_then(|exit| {
            editor
                .catalog()
                .into_iter()
                .find(|(id, _)| *id == exit.target_room)
        })
        .unwrap_or_else(|| (editor.id, editor.draft.layout.clone()));
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
    // The part of the room in the viewport, in world coordinates.
    let top_left = canvas.world(regions.viewport.point());
    let seen = Rect::new(
        top_left.x,
        top_left.y,
        regions.viewport.w / canvas.scale,
        regions.viewport.h / canvas.scale,
    );
    // The room as the game draws it: the same draw list, through the canvas zoom and pan, kept
    // until the look or the terrain changes. The outline and grid go over it.
    let look_shapes = state
        .look
        .preview
        .shapes(layout_room, &layout, &editor.styles);
    terrain_draw::draw_shapes_mapped(look_shapes.shapes, seen, canvas.scale, |p| canvas.screen(p));
    let styled = !look_shapes.shapes.is_empty();
    draw_rectangle_lines(area.x, area.y, area.w, area.h, 2.0, LIGHTGRAY);
    // Grid lines start at the room's corner so they line up with its edges, and snapping
    // uses the same origin.
    let origin = layout.rect().point();
    if let Some(step) = state.grid.line_step(canvas.scale) {
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
        if linking && state.link_placing {
            let at = state.grid.snap(world, origin);
            state.place_link_exit(editor, at);
        } else if state.tool == EditorTool::Place && state.destination.is_none() {
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
            let hit = markers.iter().rev().find(|(selection, rect, _)| {
                let r = canvas.rect(*rect);
                (!linking || linking::live_while_linking(*selection))
                    && Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0).contains(mouse)
            });
            // A solid (or a handle of the selected one) can take the click before the markers.
            if !linking && terrain_canvas::press_select(state, editor, hit.is_some(), world, reach)
            {
                // The solid is selected and its move or resize has begun.
            } else {
                state.selected = hit.map(|(selection, _, _)| selected_by_press(*selection));
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
            match editor.drag_selection(selection, target) {
                Ok(()) => editor.message.clear(),
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
    if is_mouse_button_pressed(MouseButton::Right) && over_canvas && !linking {
        terrain_canvas::erase(state, editor, &markers, world);
    } else if is_key_pressed(KeyCode::Delete)
        && state.canvas_focused
        && !keys_blocked
        && !pointer_blocked
        && !linking
    {
        terrain_canvas::delete_selected(state, editor);
    }
    if state.canvas_focused && !keys_blocked && !linking {
        let down = |left, right| is_key_down(left) || is_key_down(right);
        if let Some(key) = shortcut(
            down(KeyCode::LeftControl, KeyCode::RightControl),
            down(KeyCode::LeftShift, KeyCode::RightShift),
            is_key_pressed(KeyCode::Z),
            is_key_pressed(KeyCode::Y),
        ) {
            if !state.player.apply_history(editor, key) {
                editor_panel::apply_history(editor, state, key);
            }
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
    terrain_canvas::draw(state, &layout, &canvas, erase_target, styled);
    // Recomputed after this frame's edits. Enemies and the boss are drawn at their visual size
    // under their hitbox outline.
    let drawn = editor_markers(editor, state.destination, state.selected);
    let previews = editor_visuals::visual_previews(editor, &drawn);
    editor_visuals::draw_visual_previews(&previews, state.selected, &canvas);
    editor_visuals::draw_markers(&drawn, state.selected, &canvas);
    if let Some(settings) = state.player.canvas_preview() {
        editor_visuals::draw_player_preview(editor, settings, &canvas);
    }
    if linking {
        linking::draw_exit_highlights(editor, &canvas);
    }
    arrival_labels::draw(&drawn, editor, &canvas);
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
                .boss_definitions()
                .ok()?
                .spawn(&placement.definition, placement.position.into())
                .ok()
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
            "{} - {}",
            editor.room_title(),
            if let Some(banner) = editor.link_banner() {
                banner
            } else if state.destination.is_some() {
                "destination room view".to_owned()
            } else {
                "authored room".to_owned()
            }
        ),
        CANVAS_MARGIN,
        30.0,
        24.0,
        WHITE,
    );
    draw_text(
        if linking {
            "F2: cancel link | Click an exit to select it | Drag: move the selected exit"
        } else {
            "F2: close | V/P/T/E: tools | Drag: move | Right click / Del: delete"
        },
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
    // Refused text in the room name field blocks Save the same way.
    let pending_error = pending_error.or_else(|| editor.name_error());
    state.clear_hidden_property_error(editor.draft.enemies.len());
    if save && pending_error.is_some() {
        editor.message = format!(
            "Save failed: {}",
            pending_error.as_deref().unwrap_or_default()
        );
        save = false;
    }
    egui_macroquad::draw();
    // The game takes the Player panel's edits into effect; its Save goes through the game.
    game.sync_player_from_editor();
    if std::mem::take(&mut state.player.save_requested) {
        state.player.status = match game.save_player() {
            Ok(paths) => Status::Saved(
                paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
            ),
            Err(error) => Status::Error(error),
        };
    }
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
    state.finish_naming(naming);
    // Closing, opening a room and New room all go through the unsaved-changes guard.
    let mut step = GuardStep::Wait;
    if let Some(action) = state.requested.take().or(open.map(GuardedAction::OpenRoom)) {
        step = state.guard.request_for(action, game.editor());
    }
    if let Some(choice) = dialog {
        step = state.guard.choose(choice);
    }
    if step == GuardStep::Save {
        // Same rule as the panel's Save: unapplied invalid text blocks it.
        let result = match &pending_error {
            Some(error) => Err(format!("Save failed: {error}")),
            None => save_all(game).map_err(|error| format!("Save failed: {error}")),
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

mod arrival_labels;
#[cfg(test)]
mod arrival_tests;
mod linking;
#[cfg(test)]
mod linking_tests;
mod markers;
mod naming;
#[cfg(test)]
mod naming_tests;
#[cfg(test)]
mod tests;
