use hollow_steps::assets;
use hollow_steps::camera::{Camera, CameraConfig};
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_guard::{GuardStep, GuardedAction};
use hollow_steps::game::{GameState, VIEW_HEIGHT, VIEW_WIDTH};
use hollow_steps::player::PlayerInput;
use hollow_steps::player_body::PlayerBody;
use hollow_steps::search::SearchList;
use hollow_steps::terrain_style::TerrainLook;
use hollow_steps::timestep::{FixedStep, InputLatch};
use macroquad::prelude::*;
mod body_draw;
mod editor_panel;
mod editor_ui;
mod editor_visuals;
mod effects;
mod hud;
mod postfx_pass;
mod scene;
mod startup;
mod terrain_canvas;
mod terrain_draw;
mod tuning_panel;
use editor_ui::{draw_editor, EditorUi};
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
    // Invalid effect settings are shown like a failed reload.
    let (mut effects, effects_error) = effects::Effects::load(&assets::assets_root());
    let mut reload_result = effects_error.map(Err);
    // The health and mana bars' settings; an invalid file is shown the same way.
    let (mut hud, hud_error) = hud::Hud::load(&assets::assets_root(), effects.particle_config());
    if let Some(error) = hud_error {
        reload_result = Some(Err(error));
    }
    // Vignette and glow over the world picture; a settings or shader error is shown the same way.
    let (mut post_pass, post_error) = postfx_pass::PostPass::load(&assets::assets_root(), view);
    if let Some(error) = post_error {
        reload_result = Some(Err(error));
    }
    // The solids' draw list, rebuilt only when the room, its solids or its palette change.
    let mut terrain_look = TerrainLook::default();
    // The player's body; a load error is shown like a failed reload and keeps the plain box.
    let mut player_body = match PlayerBody::load(&assets::assets_root()) {
        Ok(body) => body,
        Err(error) => {
            reload_result = Some(Err(error));
            None
        }
    };
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
            effects.clear();
            hud.clear();
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
            reload_result = Some(
                game.reload_rooms()
                    .and_then(|()| effects.reload())
                    .and_then(|()| hud.reload(&assets::assets_root(), effects.particle_config()))
                    .and_then(|()| post_pass.reload(&assets::assets_root()))
                    .and_then(|()| {
                        player_body = PlayerBody::load(&assets::assets_root())?;
                        Ok(())
                    }),
            );
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
            if !game.inventory_open() {
                effects.update(fixed_step.step_seconds());
                hud.update(fixed_step.step_seconds());
                if let Some(body) = &mut player_body {
                    let player = game.player();
                    body.update(
                        player.is_grounded(),
                        player.velocity(),
                        player.strike_hitbox().is_some(),
                        fixed_step.step_seconds(),
                    );
                }
            }
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
        let events = game.take_located_events();
        effects.handle(&events);
        hud.handle(&events, view, game.vitals().max_health());

        // Screen shake moves only the picture, not the camera that follows the player.
        let world_view = camera.view_rect_shaken(view, game.shake_offset());
        set_camera(&screen_camera(world_view, Some(screen.clone())));
        scene::draw_scene(
            &game,
            player_body.as_ref(),
            &mut terrain_look,
            camera.view_rect(view),
        );
        effects.draw();
        // The HUD is drawn after the post-process pass so the vignette never dims it.
        let frame = post_pass.process(&screen, view);
        set_camera(&screen_camera(
            Rect::new(0.0, 0.0, view.x, view.y),
            Some(frame.clone()),
        ));
        hud::draw_hud(&game, &hud, view);
        // The health bar's damage particles, in screen space on top of the HUD.
        hud.draw_particles();
        if game.testing_room() {
            hud::draw_test_banner(view);
        }
        if game.inventory_open() {
            hud::draw_inventory(&game, view);
        }
        if let Some(result) = &reload_result {
            hud::draw_reload_result(
                result,
                "Reload failed - F5 to retry",
                &mut reload_scroll,
                view,
            );
        }
        let letterbox = present(&frame, view);
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
        heal_held: is_key_down(KeyCode::C),
    }
}
