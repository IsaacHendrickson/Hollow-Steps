use hollow_steps::boss_special::BossSpecials;
use hollow_steps::camera::{Camera, CameraConfig};
use hollow_steps::coin::COIN_RADIUS;
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_guard::{perform, GuardStep, GuardedAction, UnsavedGuard};
use hollow_steps::editor_history::{shortcut, tracked, History};
use hollow_steps::editor_tools::{tool_shortcut, EditorTool, TerrainTool};
use hollow_steps::editor_view::{
    grid_lines, wrap_words, CanvasRegions, CanvasView, Grid, PanDrag, CANVAS_MARGIN,
};
use hollow_steps::game::{GameState, VIEW_HEIGHT, VIEW_WIDTH};
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomEdge;
use hollow_steps::search::SearchList;
use hollow_steps::timestep::{FixedStep, InputLatch};
use macroquad::prelude::*;
mod body_draw;
mod editor_panel;
mod hud;
mod startup;
mod terrain_canvas;
mod tuning_panel;
use editor_panel::{BossPropertyUi, PropertyUi};
use startup::{present, screen_camera};

fn window_conf() -> Conf {
    Conf {
        window_title: "Hollow Steps".to_owned(),
        window_width: VIEW_WIDTH as i32,
        window_height: VIEW_HEIGHT as i32,
        window_resizable: true,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    // The world and HUD are drawn at the virtual resolution into this target, which is
    // then scaled to the window with letterboxing.
    let view = vec2(VIEW_WIDTH, VIEW_HEIGHT);
    let screen = render_target(VIEW_WIDTH as u32, VIEW_HEIGHT as u32);
    screen.texture.set_filter(FilterMode::Linear);
    let mut game = startup::load_game(&screen, view).await;
    let mut reload_result = None;
    let mut reload_scroll = 0;
    let mut editor_ui = EditorUi::default();
    let mut body_debug = false;
    let mut tuning_panel = tuning_panel::TuningPanel::default();
    // True while a text box in the tuning window has focus, so typing does not reach the game.
    let mut ui_wants_keyboard = false;
    let body_debug_overlay = body_draw::BodyDebugOverlay::new();
    let mut fixed_step = FixedStep::default();
    let mut input_latch = InputLatch::default();
    let camera_config = CameraConfig::default();
    let mut camera = new_camera(&game, camera_config, view);
    let mut camera_room = game.room().id;
    // Whether a room from the editor was being played last frame.
    let mut testing = false;

    loop {
        let frame_seconds = get_frame_time();
        if is_key_pressed(KeyCode::F3) {
            body_debug = !body_debug;
        }
        // F6 plays the room being edited (the Test room button does the same); a draft
        // that is not a valid room stays in the editor with the reason in its message.
        if is_key_pressed(KeyCode::F6) && game.editor().is_some() && !editor_ui.guard.is_open() {
            let _ = game.start_test_room();
        }
        if is_key_pressed(KeyCode::F2) && game.testing_room() {
            // Back to the same editor session. Its UI state (undo history, selection, view)
            // stays as it was, unlike opening the editor fresh.
            let _ = game.end_test_room();
        } else if is_key_pressed(KeyCode::F2) {
            // Opening the editor never asks anything; closing it with unsaved changes
            // opens the Save / Discard / Cancel dialog instead.
            let toggle = match game.editor() {
                Some(session) => {
                    editor_ui.guard.request(GuardedAction::Close, session.dirty)
                        == GuardStep::Perform(GuardedAction::Close)
                }
                None => true,
            };
            if toggle {
                if let Err(error) = game.toggle_editor() {
                    reload_result = Some(Err(error));
                }
                editor_ui = editor_ui.for_new_session();
                fixed_step.reset();
                input_latch.clear();
            }
        }
        if game.testing_room() != testing {
            // A test room started or ended: the world changed under the camera and any
            // input or time left over from the other mode must not carry across.
            testing = game.testing_room();
            fixed_step.reset();
            input_latch.clear();
            camera = new_camera(&game, camera_config, view);
            camera_room = game.room().id;
        }
        if game.editor().is_some() {
            ui_wants_keyboard = false;
            draw_editor(&mut game, &mut editor_ui, body_debug, &body_debug_overlay);
            if game.editor().is_none() && !game.testing_room() {
                // The dialog (Save or Discard) closed the editor.
                editor_ui = editor_ui.for_new_session();
                fixed_step.reset();
                input_latch.clear();
            }
            next_frame().await;
            continue;
        }
        if is_key_pressed(KeyCode::F4) {
            tuning_panel.toggle();
        }
        if is_key_pressed(KeyCode::F5) {
            reload_result = Some(game.reload_rooms());
            reload_scroll = 0;
        }
        if !ui_wants_keyboard
            && (is_key_pressed(KeyCode::I)
                || (game.inventory_open() && is_key_pressed(KeyCode::Escape)))
        {
            game.toggle_inventory();
        }
        // Edge input is latched until a tick consumes it; only the first tick of a frame
        // sees it, while held input applies to every tick.
        input_latch.record(if ui_wants_keyboard {
            PlayerInput::default()
        } else {
            read_player_input()
        });
        for _ in 0..fixed_step.advance(frame_seconds) {
            game.update(input_latch.take_tick(), fixed_step.step_seconds());
            if game.room().id == camera_room {
                let player = game.player();
                camera.update(
                    player.position(),
                    player.facing(),
                    game.room().bounds,
                    view,
                    fixed_step.step_seconds(),
                );
            } else {
                camera_room = game.room().id;
                camera = new_camera(&game, camera_config, view);
            }
        }
        // Nothing reacts to gameplay events here yet; drop them so the queue stays empty.
        drop(game.take_events());

        // Screen shake moves only the picture, not the camera that follows the player.
        let world_view = camera.view_rect_shaken(view, game.shake_offset());
        set_camera(&screen_camera(world_view, Some(screen.clone())));
        draw_scene(&game);
        set_camera(&screen_camera(
            Rect::new(0.0, 0.0, view.x, view.y),
            Some(screen.clone()),
        ));
        hud::draw_hud(&game, view);
        if game.testing_room() {
            hud::draw_test_banner(view);
        }
        if game.inventory_open() {
            hud::draw_inventory(&game, view);
        }
        if let Some(result) = &reload_result {
            hud::draw_reload_result(
                result,
                "Room reload failed - F5 to retry",
                &mut reload_scroll,
                view,
            );
        }
        let letterbox = present(&screen, view);
        // Drain the integration's event subscription while the editor is closed, so reopening
        // cannot replay earlier gameplay clicks or keys into its controls.
        egui_macroquad::ui(|ctx| {
            tuning_panel::scroll_from_wheel(ctx, mouse_wheel());
            tuning_panel.draw(ctx, &mut game);
            ui_wants_keyboard = ctx.wants_keyboard_input();
        });
        egui_macroquad::draw();
        if body_debug {
            // The overlay draws in window space, so project world points through the
            // camera and letterbox.
            let top_left = world_view.point();
            body_draw::draw_body_debug_overlay(
                &body_debug_overlay,
                &game.room().enemies,
                game.room().boss_special.as_ref(),
                game.player().position(),
                |position| letterbox.offset + (position - top_left) * letterbox.scale,
                letterbox.scale,
            );
        }
        next_frame().await;
    }
}

fn new_camera(game: &GameState, config: CameraConfig, view: Vec2) -> Camera {
    let player = game.player();
    Camera::new(
        config,
        player.position(),
        player.facing(),
        game.room().bounds,
        view,
    )
}

fn read_player_input() -> PlayerInput {
    let mut move_axis = 0.0;
    if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
        move_axis += 1.0;
    }
    if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
        move_axis -= 1.0;
    }

    PlayerInput {
        move_axis,
        jump_pressed: is_key_pressed(KeyCode::Space) || is_key_pressed(KeyCode::Z),
        pickup_pressed: is_key_pressed(KeyCode::Up),
        jump_held: is_key_down(KeyCode::Space) || is_key_down(KeyCode::Z),
        attack_pressed: is_key_pressed(KeyCode::X),
        dash_pressed: is_key_pressed(KeyCode::LeftShift) || is_key_pressed(KeyCode::RightShift),
        up_held: is_key_down(KeyCode::Up),
        down_held: is_key_down(KeyCode::Down),
    }
}

fn draw_scene(game: &GameState) {
    let room = game.room();
    let bounds = game.world_bounds();
    let player = game.player();
    let player_position = player.position();
    let player_size = player.size();

    clear_background(Color::from_rgba(16, 18, 31, 255));
    draw_rectangle(
        bounds.left,
        bounds.floor_y,
        bounds.right - bounds.left,
        room.bounds.bottom() - bounds.floor_y,
        Color::from_rgba(34, 37, 58, 255),
    );
    draw_line(
        bounds.left,
        bounds.floor_y,
        bounds.right,
        bounds.floor_y,
        4.0,
        Color::from_rgba(111, 122, 164, 255),
    );
    for solid in room.terrain.solids() {
        draw_rectangle(
            solid.x,
            solid.y,
            solid.w,
            solid.h,
            Color::from_rgba(34, 37, 58, 255),
        );
        draw_rectangle_lines(
            solid.x,
            solid.y,
            solid.w,
            solid.h,
            3.0,
            Color::from_rgba(111, 122, 164, 255),
        );
    }
    draw_rectangle_lines(
        room.bounds.x,
        room.bounds.y,
        room.bounds.w,
        room.bounds.h,
        2.0,
        Color::from_rgba(62, 70, 103, 255),
    );
    for exit in &room.exits {
        let zone = exit.zone(room.bounds);
        draw_rectangle(
            zone.x,
            zone.y,
            zone.w,
            zone.h,
            if room.exits_locked() {
                Color::from_rgba(220, 65, 70, 190)
            } else {
                Color::from_rgba(82, 172, 164, 100)
            },
        );
        if room.exits_locked() {
            draw_rectangle_lines(zone.x, zone.y, zone.w, zone.h, 3.0, RED);
        }
    }
    for item in game.items() {
        let nearby = item.can_pick_up(player);
        let top_left = item.position - ITEM_SIZE / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            ITEM_SIZE.x,
            ITEM_SIZE.y,
            if nearby { YELLOW } else { GOLD },
        );
        if nearby {
            draw_rectangle_lines(
                top_left.x - 3.0,
                top_left.y - 3.0,
                ITEM_SIZE.x + 6.0,
                ITEM_SIZE.y + 6.0,
                2.0,
                WHITE,
            );
            draw_text(
                format!("Up: pick up {}", item.item.name()),
                top_left.x - 80.0,
                top_left.y - 16.0,
                20.0,
                WHITE,
            );
        }
    }
    for coin in game.coins() {
        draw_circle(coin.position.x, coin.position.y, COIN_RADIUS, GOLD);
        draw_circle_lines(
            coin.position.x,
            coin.position.y,
            COIN_RADIUS - 2.0,
            1.0,
            YELLOW,
        );
    }
    for enemy in &game.room().enemies {
        for blast in enemy
            .behavior()
            .blasts()
            .iter()
            .chain(enemy.behavior().rings())
        {
            let color = if blast.exploded { RED } else { ORANGE };
            draw_circle_lines(blast.position.x, blast.position.y, blast.radius, 2.0, color);
        }
        let top_left = enemy.position() - enemy.size() / 2.0;
        body_draw::draw_enemy_body(enemy);
        body_draw::draw_enemy_attacks(enemy);
        draw_rectangle(top_left.x, top_left.y - 8.0, enemy.size().x, 4.0, DARKGRAY);
        draw_rectangle(
            top_left.x,
            top_left.y - 8.0,
            enemy.size().x * f32::from(enemy.health()) / f32::from(enemy.max_health()),
            4.0,
            RED,
        );
    }
    if let Some(boss) = &room.boss_special {
        draw_boss_special(boss, bounds.floor_y);
    }
    if let Some(hitbox) = player.strike_hitbox() {
        draw_rectangle(
            hitbox.x,
            hitbox.y,
            hitbox.w,
            hitbox.h,
            Color::from_rgba(255, 231, 160, 150),
        );
        draw_rectangle_lines(hitbox.x, hitbox.y, hitbox.w, hitbox.h, 2.0, GOLD);
    }
    draw_rectangle(
        player_position.x - player_size.x / 2.0,
        player_position.y - player_size.y / 2.0,
        player_size.x,
        player_size.y,
        if player.health().flash_on() {
            WHITE
        } else {
            Color::from_rgba(110, 160, 230, 255)
        },
    );
}

fn draw_boss_special(boss: &BossSpecials, floor_y: f32) {
    if !boss.is_defeated() {
        body_draw::draw_boss_attacks(boss);
        if let Some((target_x, radius)) = boss.landing_preview() {
            draw_rectangle(target_x - radius, floor_y - 12.0, radius * 2.0, 12.0, RED);
        }
        for ring in boss.rings() {
            let color = if ring.exploded() { ORANGE } else { RED };
            draw_circle_lines(
                ring.position().x,
                ring.position().y,
                ring.radius(),
                3.0,
                color,
            );
        }
        let top_left = boss.position() - boss.size() / 2.0;
        draw_rectangle(
            top_left.x,
            top_left.y,
            boss.size().x,
            boss.size().y,
            if boss.is_telegraphing() {
                ORANGE
            } else {
                PURPLE
            },
        );
    } else if boss.is_death_effect_active() {
        draw_circle(boss.position().x, boss.position().y, boss.size().x, WHITE);
    }
}

#[derive(Default)]
struct EditorUi {
    definition: SearchList,
    room: SearchList,
    target: SearchList,
    new_entity: String,
    edge: usize,
    /// What a click or drag on the canvas does.
    tool: EditorTool,
    /// The selected solid and the solid drag in progress.
    terrain: TerrainTool,
    canvas_focused: bool,
    selected: Option<Selection>,
    dragging: Option<(Selection, Vec2)>,
    destination: Option<usize>,
    properties: bool,
    property: PropertyUi,
    boss_property: Option<BossPropertyUi>,
    /// Canvas transform after the user zooms or pans; `None` fits the room to the window.
    view: Option<CanvasView>,
    /// The destination view `view` belongs to; switching rooms refits.
    view_destination: Option<usize>,
    pan: PanDrag,
    grid: Grid,
    /// Asks before closing or replacing a session with unsaved changes.
    guard: UnsavedGuard,
    /// A room switch asked for by the panel this frame (New room); `draw_editor` runs it
    /// through `guard`.
    requested: Option<GuardedAction>,
    /// Undo and redo for the draft; reset with the rest of this state when a room is opened
    /// or created.
    history: History,
}

impl EditorUi {
    /// State for a newly opened or created room, or a reopened editor: everything resets
    /// (selection, view, undo history, dialogs) except the grid and snap settings, which
    /// belong to the editor rather than to a room.
    fn for_new_session(&self) -> Self {
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

fn draw_editor(
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
mod editor_ui_tests {
    use super::*;

    #[test]
    fn a_new_session_forgets_history_and_selection_but_keeps_grid_settings() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        let mut game = GameState::try_new(&assets).unwrap();
        game.toggle_editor().unwrap();
        let mut state = EditorUi::default();
        state.grid.enabled = true;
        state.grid.size = 16.0;
        state.selected = Some(Selection::Item(0));
        state.tool = EditorTool::Terrain;
        state.terrain.select(Some(0));
        state.view = Some(CanvasView {
            offset: Vec2::ZERO,
            scale: 2.0,
        });
        let editor = game.editor_mut().unwrap();
        let before = editor.draft.clone();
        tracked(&mut state.history, editor, |e| {
            e.draft.delete(Selection::Item(0))
        })
        .unwrap();
        assert!(state.history.can_undo());

        let next = state.for_new_session();
        assert!(!next.history.can_undo() && !next.history.can_redo());
        assert_eq!(next.selected, None);
        assert_eq!(
            (next.tool, next.terrain.selected()),
            (EditorTool::Select, None)
        );
        assert!(next.view.is_none());
        assert!(next.grid.enabled);
        assert_eq!(next.grid.size, 16.0);
        // The old state is untouched, so the draft it recorded can still be restored.
        assert!(state.history.can_undo());
        assert_ne!(game.editor().unwrap().draft, before);
    }

    #[test]
    fn authored_boss_has_a_canvas_marker() {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        let mut game = GameState::try_new(&assets).unwrap();
        game.edit_room(hollow_steps::room::RoomId::Boss).unwrap();
        let editor = game.editor().unwrap();
        let placement = editor.draft.boss.as_ref().unwrap();
        assert!(editor_markers(editor, None, None)
            .iter()
            .any(|(selection, rect, _)| {
                *selection == Selection::Boss
                    && rect.center() == Vec2::from(placement.position)
                    && rect.size() == vec2(48.0, 48.0)
            }));
    }

    #[test]
    fn property_error_only_survives_a_visible_selected_enemy() {
        for (properties, selected, retained) in [
            (true, Some(Selection::Enemy(0)), true),
            (false, Some(Selection::Enemy(0)), false),
            (true, None, false),
            (true, Some(Selection::Item(0)), false),
            (true, Some(Selection::Exit(0)), false),
            (true, Some(Selection::Enemy(1)), false),
        ] {
            let mut state = EditorUi {
                properties,
                selected,
                property: PropertyUi {
                    error: Some("invalid text".into()),
                    ..Default::default()
                },
                ..Default::default()
            };
            state.clear_hidden_property_error(1);
            assert_eq!(state.property.error.is_some(), retained);
        }
    }
}
