//! Macroquad adapter for the inventory's Charms section: this frame's keys and mouse in,
//! `GameState` calls out. The rules (what may be equipped, where the rows are) live in
//! `hollow_steps::charms` and `hollow_steps::charm_menu`.

use hollow_steps::camera::Letterbox;
use hollow_steps::charm_menu::charm_at;
use hollow_steps::game::GameState;
use macroquad::prelude::*;

/// Feeds this frame's input to the charm list while the inventory is open: Up/Down (or W/S)
/// move the cursor, Enter equips or takes off the selected charm, the mouse selects a row by
/// moving onto it and equips or takes off by clicking it. Escape and the inventory key are not
/// handled here (`main.rs`). `keyboard` is false while a text box of another window has the keys.
pub fn handle(game: &mut GameState, view: Vec2, keyboard: bool) {
    if !game.inventory_open() {
        return;
    }
    if keyboard {
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            game.charm_step(-1, view);
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            game.charm_step(1, view);
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            game.charm_press();
        }
    }
    // The same fit the frame is drawn with, so the mouse lands on what is on screen.
    let letterbox = Letterbox::fit(vec2(screen_width(), screen_height()), view);
    let mouse = letterbox.window_to_virtual(Vec2::from(mouse_position()));
    let Some(index) = charm_at(&game.inventory_layout(view), mouse) else {
        return;
    };
    // Only a moving mouse selects, so a mouse left resting on a row does not undo the
    // keyboard's choice.
    if mouse_delta_position() != Vec2::ZERO {
        game.charm_hover(index);
    }
    if is_mouse_button_pressed(MouseButton::Left) {
        game.charm_click(index);
    }
}
