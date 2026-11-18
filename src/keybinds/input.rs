//! From the bindings and this frame's key states to what the player does.

use macroquad::input::KeyCode;

use super::notes::key_label;
use super::{Action, Keybinds, Slot};
use crate::player::PlayerInput;

/// The player's intent for one frame. `down` says whether a key is held and `pressed` whether it
/// went down this frame (the front end passes macroquad's `is_key_down` and `is_key_pressed`).
/// Held keys feed `move_axis` and the `*_held` flags; pressed keys feed the edge flags.
pub fn player_input(
    binds: &Keybinds,
    down: impl Fn(KeyCode) -> bool,
    pressed: impl Fn(KeyCode) -> bool,
) -> PlayerInput {
    let held = |action| binds.triggered(action, &down);
    let hit = |action| binds.triggered(action, &pressed);
    let mut move_axis = 0.0;
    if held(Action::MoveRight) {
        move_axis += 1.0;
    }
    if held(Action::MoveLeft) {
        move_axis -= 1.0;
    }
    PlayerInput {
        move_axis,
        jump_pressed: hit(Action::Jump),
        jump_held: held(Action::Jump),
        attack_pressed: hit(Action::Attack),
        pickup_pressed: hit(Action::Up),
        dash_pressed: hit(Action::Dash),
        up_held: held(Action::Up),
        down_held: held(Action::Down),
        heal_held: held(Action::Heal),
    }
}

/// The controls reminder at the bottom of the HUD, from each action's primary key.
pub fn hint_line(binds: &Keybinds) -> String {
    let key = |action| key_label(binds.key(action, Slot::Primary));
    format!(
        "{}/{} move   {} jump   {} attack   {} pickup   {} dash   {} inventory   F5 reload",
        key(Action::MoveLeft),
        key(Action::MoveRight),
        key(Action::Jump),
        key(Action::Attack),
        key(Action::Up),
        key(Action::Dash),
        key(Action::Inventory),
    )
}
