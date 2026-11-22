//! The Map action: its shipped key, that a keybinds file written before it existed still loads,
//! and that it can be rebound like every other action.
use macroquad::input::KeyCode;

use super::*;

#[test]
fn the_map_is_on_m_by_default_with_no_alternate() {
    let binds = Keybinds::default();
    assert_eq!(binds.key(Action::Map, Slot::Primary), Some(KeyCode::M));
    assert_eq!(binds.key(Action::Map, Slot::Alternate), None);
    assert_eq!(binds.check(), Ok(()));
    assert!(binds.triggered(Action::Map, |key| key == KeyCode::M));
    assert_eq!(Action::from_name("map"), Some(Action::Map));
    assert_eq!(Action::Map.label(), "Map");
}

#[test]
fn a_file_from_before_the_map_existed_still_loads_with_the_default_key() {
    // The nine actions the game saved before there was a Map, heal moved to H.
    let older = r#"(
        bindings: {
            "move_left": ["A", "Left"],
            "move_right": ["D", "Right"],
            "jump": ["Space", "Z"],
            "attack": ["X"],
            "dash": ["Left Shift", "Right Shift"],
            "heal": ["H"],
            "up": ["Up"],
            "down": ["Down"],
            "inventory": ["I"],
        },
    )"#;
    let binds = Keybinds::from_ron(older).unwrap();
    assert_eq!(binds.key(Action::Heal, Slot::Primary), Some(KeyCode::H));
    assert_eq!(binds.key(Action::Map, Slot::Primary), Some(KeyCode::M));
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn a_small_file_without_the_map_keeps_the_default_key() {
    let binds = Keybinds::from_ron(r#"(bindings: {"jump": ["q"]})"#).unwrap();
    assert_eq!(binds.key(Action::Map, Slot::Primary), Some(KeyCode::M));
}

#[test]
fn the_map_key_is_written_and_read_back() {
    let mut binds = Keybinds::default();
    binds
        .rebind(Action::Map, Slot::Primary, KeyCode::N)
        .unwrap();
    binds
        .rebind(Action::Map, Slot::Alternate, KeyCode::Tab)
        .unwrap();
    let text = binds.to_ron().unwrap();
    assert!(text.contains("\"map\""), "{text}");
    assert_eq!(Keybinds::from_ron(&text), Ok(binds));
}

#[test]
fn a_rebound_map_answers_to_the_new_key_and_leaves_m_free() {
    let mut binds = Keybinds::default();
    binds
        .rebind(Action::Map, Slot::Primary, KeyCode::N)
        .unwrap();
    assert!(binds.triggered(Action::Map, |key| key == KeyCode::N));
    assert!(!binds.triggered(Action::Map, |key| key == KeyCode::M));
    // M is free for another action now.
    binds
        .rebind(Action::Heal, Slot::Primary, KeyCode::M)
        .unwrap();
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn taking_the_key_of_another_action_for_the_map_trades_keys() {
    let mut binds = Keybinds::default();
    let rebound = binds
        .rebind(Action::Map, Slot::Primary, KeyCode::I)
        .unwrap();
    assert_eq!(
        rebound.swapped.map(|swap| swap.action),
        Some(Action::Inventory)
    );
    assert_eq!(
        binds.key(Action::Inventory, Slot::Primary),
        Some(KeyCode::M)
    );
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn a_file_that_gives_m_to_another_action_and_says_nothing_of_the_map_is_refused_by_name() {
    // The rule for every action a file leaves out: it keeps its shipped key, and no key may be
    // used twice. The message names both actions so the file can be fixed.
    let error = Keybinds::from_ron(r#"(bindings: {"heal": ["M"]})"#).unwrap_err();
    assert!(error.contains("Heal") && error.contains("Map"), "{error}");
}
