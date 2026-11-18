//! The short messages the settings screen shows after a rebind.

use macroquad::input::KeyCode;

use super::{key_name, RebindError, Swap};

/// What `key` is called on screen.
pub fn key_label(key: Option<KeyCode>) -> &'static str {
    key.and_then(key_name).unwrap_or("-")
}

/// "Left Shift: swapped with Dash (alternate)": `key` was in use there, so that slot got this
/// slot's old key (or is empty now). Kept short: the message line is one line wide.
pub fn swap_note(key: KeyCode, swap: Swap) -> String {
    let name = key_label(Some(key));
    let (action, slot) = (swap.action.label(), swap.slot.label());
    match swap.now {
        Some(_) => format!("{name}: swapped with {action} ({slot})"),
        None => format!("{name}: taken from {action} ({slot})"),
    }
}

/// Why a key press was refused. `key` is the key that was pressed, when there was one.
pub fn error_note(error: RebindError, key: Option<KeyCode>) -> String {
    match error {
        RebindError::NotBindable => "That key cannot be used".to_owned(),
        RebindError::PrimaryOf(action) => format!(
            "{} is {}'s primary key; change it first",
            key_label(key),
            action.label()
        ),
        RebindError::PrimaryRequired => {
            "A primary key can't be cleared; press a key instead".to_owned()
        }
    }
}
