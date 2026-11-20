//! Macroquad adapter for the hub's shop: this frame's keys and mouse in, `GameState` calls out.
//! The rules (what a purchase does, where the rows are) live in `hollow_steps::upgrades` and
//! `hollow_steps::shop_menu`.

use hollow_steps::camera::Letterbox;
use hollow_steps::game::GameState;
use hollow_steps::shop_menu::upgrade_at;
use macroquad::prelude::*;

/// Feeds this frame's input to the shop while it is open: Up/Down (or W/S) move the cursor,
/// Enter buys the selected upgrade, the mouse selects a row by moving onto it and buys by clicking
/// it. Escape is not handled here (`GameState::press_escape` closes the shop). `keyboard` is
/// false while a text box of another window has the keys, `pointer` while another window has the
/// mouse.
///
/// The Up press that opened the shop was the pickup key of an earlier frame, so it is never also
/// a cursor step.
pub fn handle(game: &mut GameState, view: Vec2, keyboard: bool, pointer: bool) {
    if !game.shop_open() {
        return;
    }
    if keyboard {
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            game.shop_step(-1, view);
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            game.shop_step(1, view);
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            game.shop_press();
        }
    }
    if !pointer {
        return;
    }
    // The same fit the frame is drawn with, so the mouse lands on what is on screen.
    let letterbox = Letterbox::fit(vec2(screen_width(), screen_height()), view);
    let mouse = letterbox.window_to_virtual(Vec2::from(mouse_position()));
    let Some(index) = upgrade_at(&game.shop_layout(view), mouse) else {
        return;
    };
    // Only a moving mouse selects, so a mouse left resting on a row does not undo the
    // keyboard's choice.
    if mouse_delta_position() != Vec2::ZERO {
        game.shop_hover(index);
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        game.shop_click(index);
    }
}
