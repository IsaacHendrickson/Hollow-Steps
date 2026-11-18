//! The keys that can be bound, with the one name each has on screen and in the keybinds file.
//!
//! Keys that are not listed cannot be bound, on purpose: Escape (the pause menu), Enter and
//! numpad Enter (the menu's confirm), Backspace and Delete (clear a slot in the settings
//! screen), F1 to F25 (editor, debug and reload keys), the Super keys (the operating system)
//! and the keys with no stable meaning across keyboards (Menu, Pause, Print Screen, Scroll
//! Lock, Num Lock, the world keys).

use macroquad::input::KeyCode;

/// Every bindable key and its name. Names are unique ignoring case.
pub const BINDABLE: &[(KeyCode, &str)] = &[
    (KeyCode::A, "A"),
    (KeyCode::B, "B"),
    (KeyCode::C, "C"),
    (KeyCode::D, "D"),
    (KeyCode::E, "E"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::I, "I"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::M, "M"),
    (KeyCode::N, "N"),
    (KeyCode::O, "O"),
    (KeyCode::P, "P"),
    (KeyCode::Q, "Q"),
    (KeyCode::R, "R"),
    (KeyCode::S, "S"),
    (KeyCode::T, "T"),
    (KeyCode::U, "U"),
    (KeyCode::V, "V"),
    (KeyCode::W, "W"),
    (KeyCode::X, "X"),
    (KeyCode::Y, "Y"),
    (KeyCode::Z, "Z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::Left, "Left"),
    (KeyCode::Right, "Right"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"),
    (KeyCode::Space, "Space"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::CapsLock, "Caps Lock"),
    (KeyCode::LeftShift, "Left Shift"),
    (KeyCode::RightShift, "Right Shift"),
    (KeyCode::LeftControl, "Left Ctrl"),
    (KeyCode::RightControl, "Right Ctrl"),
    (KeyCode::LeftAlt, "Left Alt"),
    (KeyCode::RightAlt, "Right Alt"),
    (KeyCode::Apostrophe, "Apostrophe"),
    (KeyCode::Comma, "Comma"),
    (KeyCode::Minus, "Minus"),
    (KeyCode::Period, "Period"),
    (KeyCode::Slash, "Slash"),
    (KeyCode::Semicolon, "Semicolon"),
    (KeyCode::Equal, "Equals"),
    (KeyCode::LeftBracket, "Left Bracket"),
    (KeyCode::Backslash, "Backslash"),
    (KeyCode::RightBracket, "Right Bracket"),
    (KeyCode::GraveAccent, "Grave"),
    (KeyCode::Insert, "Insert"),
    (KeyCode::Home, "Home"),
    (KeyCode::End, "End"),
    (KeyCode::PageUp, "Page Up"),
    (KeyCode::PageDown, "Page Down"),
    (KeyCode::Kp0, "Num 0"),
    (KeyCode::Kp1, "Num 1"),
    (KeyCode::Kp2, "Num 2"),
    (KeyCode::Kp3, "Num 3"),
    (KeyCode::Kp4, "Num 4"),
    (KeyCode::Kp5, "Num 5"),
    (KeyCode::Kp6, "Num 6"),
    (KeyCode::Kp7, "Num 7"),
    (KeyCode::Kp8, "Num 8"),
    (KeyCode::Kp9, "Num 9"),
    (KeyCode::KpDecimal, "Num Decimal"),
    (KeyCode::KpDivide, "Num Divide"),
    (KeyCode::KpMultiply, "Num Multiply"),
    (KeyCode::KpSubtract, "Num Minus"),
    (KeyCode::KpAdd, "Num Plus"),
];

/// The name of a bindable key; `None` for a key that cannot be bound.
pub fn key_name(key: KeyCode) -> Option<&'static str> {
    BINDABLE
        .iter()
        .find(|(candidate, _)| *candidate == key)
        .map(|(_, name)| *name)
}

/// The key called `name`, ignoring case and surrounding spaces; `None` if no bindable key has
/// that name.
pub fn key_from_name(name: &str) -> Option<KeyCode> {
    let name = name.trim();
    BINDABLE
        .iter()
        .find(|(_, candidate)| candidate.eq_ignore_ascii_case(name))
        .map(|(key, _)| *key)
}

pub fn is_bindable(key: KeyCode) -> bool {
    key_name(key).is_some()
}

/// The first bindable key (in table order) that `pressed` reports, so two keys pressed in the
/// same frame always give the same answer.
pub fn first_pressed(pressed: impl Fn(KeyCode) -> bool) -> Option<KeyCode> {
    BINDABLE
        .iter()
        .map(|(key, _)| *key)
        .find(|&key| pressed(key))
}
