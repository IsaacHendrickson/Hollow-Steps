//! Macroquad adapter for the pause menu: this frame's keys and mouse in, `GameState` calls out.
//! The rules (what each button does, where the buttons are) live in `hollow_steps::pause_menu`.

use hollow_steps::camera::Letterbox;
use hollow_steps::game::GameState;
use hollow_steps::pause_menu::{item_at, PauseOutcome};
use macroquad::prelude::*;

/// Feeds this frame's input to the pause menu when it is open: Up/Down (or W/S) move the
/// selection, Enter presses it, the mouse selects a button by resting on it and presses it by
/// clicking. Escape is not handled here (`GameState::press_escape`). `keyboard` is false while
/// a text box of another window has the keys. Returns `Exit` when Exit was pressed.
pub fn handle(game: &mut GameState, view: Vec2, keyboard: bool) -> PauseOutcome {
    let Some(screen) = game.pause_menu().screen() else {
        return PauseOutcome::Continue;
    };
    if keyboard {
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            game.pause_step(-1);
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            game.pause_step(1);
        }
        if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::KpEnter) {
            return game.pause_activate_selected();
        }
    }
    // The same fit the frame is drawn with, so the mouse lands on what is on screen.
    let letterbox = Letterbox::fit(vec2(screen_width(), screen_height()), view);
    let mouse = letterbox.window_to_virtual(Vec2::from(mouse_position()));
    let hovered = item_at(screen, view, mouse);
    if let Some(item) = hovered {
        // Only a moving mouse selects, so a mouse left resting on a button does not undo
        // the keyboard's choice.
        if mouse_delta_position() != Vec2::ZERO {
            game.pause_hover(item);
        }
        if is_mouse_button_pressed(MouseButton::Left) {
            return game.pause_activate(item);
        }
    }
    PauseOutcome::Continue
}
