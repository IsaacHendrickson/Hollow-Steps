//! The keybinds through `GameState`: the Settings screen rebinds, saves, resets, and a new run
//! starts from what was saved.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use macroquad::input::KeyCode;

use super::*;
use crate::keybinds::{self, Action, Keybinds, Slot};
use crate::pause_menu::{PauseItem, PauseOutcome, PauseScreen};

fn test_game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

/// A private empty config folder, removed afterwards.
struct Config(PathBuf);

impl Config {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-keybind-link-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self) -> PathBuf {
        self.0.join(keybinds::FILE_NAME)
    }
}

impl Drop for Config {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A game on Settings with `path` as its keybinds file.
fn settings_game(path: &Path) -> GameState {
    let mut game = test_game();
    game.use_keybinds_file(path.to_owned()).unwrap();
    game.press_escape();
    game.pause_activate(PauseItem::Settings);
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Settings));
    game
}

/// Presses `slot` of `action`'s row and then `key`, as the front end does.
fn rebind(game: &mut GameState, action: Action, slot: Slot, key: KeyCode) {
    game.pause_activate_slot(action, slot);
    assert_eq!(game.pause_menu().listening(), Some((action, slot)));
    game.pause_bind_key(key);
}

#[test]
fn a_key_pressed_while_waiting_is_bound_and_saved() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    assert!(!config.file().exists());
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::Q);
    assert_eq!(
        game.keybinds().key(Action::Jump, Slot::Primary),
        Some(KeyCode::Q)
    );
    assert_eq!(game.pause_menu().listening(), None);
    assert_eq!(game.pause_menu().note(), None);
    let saved = keybinds::load(&config.file()).unwrap();
    assert_eq!(&saved, game.keybinds());
}

#[test]
fn keybinds_saved_in_one_run_are_in_effect_in_the_next() {
    let config = Config::new();
    {
        let mut game = settings_game(&config.file());
        rebind(&mut game, Action::Attack, Slot::Primary, KeyCode::E);
        rebind(&mut game, Action::Dash, Slot::Alternate, KeyCode::R);
        game.pause_activate_slot(Action::Heal, Slot::Primary);
        game.pause_bind_key(KeyCode::V);
    }
    let mut next_run = test_game();
    assert!(next_run.keybinds().is_default());
    assert_eq!(next_run.use_keybinds_file(config.file()), Ok(()));
    let binds = next_run.keybinds();
    assert_eq!(binds.key(Action::Attack, Slot::Primary), Some(KeyCode::E));
    assert_eq!(binds.key(Action::Dash, Slot::Alternate), Some(KeyCode::R));
    assert_eq!(binds.key(Action::Heal, Slot::Primary), Some(KeyCode::V));
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::Space));
}

#[test]
fn a_key_in_use_trades_places_and_the_message_line_says_so() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::X);
    let binds = game.keybinds();
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::X));
    assert_eq!(
        binds.key(Action::Attack, Slot::Primary),
        Some(KeyCode::Space)
    );
    assert_eq!(
        game.pause_menu().note(),
        Some("X: swapped with Attack (primary)")
    );
    assert_eq!(&keybinds::load(&config.file()).unwrap(), game.keybinds());
}

#[test]
fn a_refused_key_keeps_waiting_and_changes_nothing() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    game.pause_activate_slot(Action::Attack, Slot::Alternate);
    game.pause_bind_key(KeyCode::C); // Heal's primary key
    assert!(game.keybinds().is_default());
    assert_eq!(
        game.pause_menu().listening(),
        Some((Action::Attack, Slot::Alternate))
    );
    assert!(game.pause_menu().note().unwrap().contains("Heal"));
    assert!(!config.file().exists());
    // Another key then works, and the old message goes.
    game.pause_bind_key(KeyCode::Q);
    assert_eq!(
        game.keybinds().key(Action::Attack, Slot::Alternate),
        Some(KeyCode::Q)
    );
    assert_eq!(game.pause_menu().listening(), None);
    assert_eq!(game.pause_menu().note(), None);
    // A key that cannot be bound is refused the same way.
    game.pause_activate_slot(Action::Jump, Slot::Primary);
    game.pause_bind_key(KeyCode::Escape);
    assert_eq!(
        game.keybinds().key(Action::Jump, Slot::Primary),
        Some(KeyCode::Space)
    );
    assert_eq!(
        game.pause_menu().listening(),
        Some((Action::Jump, Slot::Primary))
    );
}

#[test]
fn delete_empties_an_alternate_key_and_never_a_primary_key() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    game.pause_activate_slot(Action::Jump, Slot::Alternate);
    game.pause_clear_key();
    assert_eq!(game.keybinds().key(Action::Jump, Slot::Alternate), None);
    assert_eq!(game.pause_menu().listening(), None);
    assert_eq!(&keybinds::load(&config.file()).unwrap(), game.keybinds());

    let before = game.keybinds().clone();
    game.pause_activate_slot(Action::Jump, Slot::Primary);
    game.pause_clear_key();
    assert_eq!(game.keybinds(), &before);
    assert_eq!(
        game.pause_menu().listening(),
        Some((Action::Jump, Slot::Primary))
    );
    assert!(game.pause_menu().note().unwrap().contains("primary"));
}

#[test]
fn keys_pressed_while_nothing_waits_change_nothing() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    game.pause_bind_key(KeyCode::Q);
    game.pause_clear_key();
    assert!(game.keybinds().is_default());
    assert!(!config.file().exists());
}

#[test]
fn reset_keybinds_restores_the_defaults_and_removes_the_saved_file() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::Q);
    rebind(&mut game, Action::Dash, Slot::Primary, KeyCode::X);
    assert!(config.file().exists());
    game.pause_hover(PauseItem::ResetKeybinds);
    assert_eq!(game.pause_activate_selected(), PauseOutcome::Continue);
    assert!(game.keybinds().is_default());
    assert!(!config.file().exists());
    assert_eq!(
        game.pause_menu().note(),
        Some("Keybinds reset to the defaults")
    );
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Settings));
    // The next run starts from the defaults.
    let mut next_run = test_game();
    next_run.use_keybinds_file(config.file()).unwrap();
    assert!(next_run.keybinds().is_default());
}

#[test]
fn reset_by_mouse_click_does_the_same_and_resetting_defaults_is_harmless() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    assert_eq!(
        game.pause_activate(PauseItem::ResetKeybinds),
        PauseOutcome::Continue
    );
    assert!(game.keybinds().is_default());
    rebind(&mut game, Action::Heal, Slot::Primary, KeyCode::V);
    assert_eq!(
        game.pause_activate(PauseItem::ResetKeybinds),
        PauseOutcome::Continue
    );
    assert!(game.keybinds().is_default());
    assert!(!config.file().exists());
}

#[test]
fn only_exit_is_left_for_the_front_end() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    for item in PauseScreen::Settings.items() {
        assert_ne!(game.pause_activate(*item), PauseOutcome::Exit);
        game.pause_menu.stop_listening();
    }
    game.pause_activate(PauseItem::Back);
    game.pause_hover(PauseItem::Exit);
    assert_eq!(game.pause_activate_selected(), PauseOutcome::Exit);
}

#[test]
fn escape_while_waiting_cancels_the_wait_without_changing_a_key() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    game.pause_activate_slot(Action::Jump, Slot::Primary);
    game.press_escape();
    assert_eq!(game.pause_menu().listening(), None);
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Settings));
    assert!(game.keybinds().is_default());
    assert!(!config.file().exists());
}

#[test]
fn a_bad_saved_file_gives_the_defaults_and_an_error_and_is_left_until_a_key_changes() {
    let config = Config::new();
    let bad = r#"(bindings: {"jump": ["Banana"]})"#;
    std::fs::write(config.file(), bad).unwrap();
    let mut game = test_game();
    let error = game.use_keybinds_file(config.file()).unwrap_err();
    assert!(
        error.contains("Banana") && error.contains(keybinds::FILE_NAME),
        "{error}"
    );
    assert!(game.keybinds().is_default());
    assert_eq!(std::fs::read_to_string(config.file()).unwrap(), bad);
    // Changing a key is the player's choice to replace the file.
    game.press_escape();
    game.pause_activate(PauseItem::Settings);
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::Q);
    assert_eq!(keybinds::load(&config.file()).as_ref(), Ok(game.keybinds()));
}

#[test]
fn a_save_that_fails_keeps_the_change_for_this_run_and_says_so() {
    let config = Config::new();
    std::fs::write(config.0.join("blocker"), "a file, not a folder").unwrap();
    let mut game = test_game();
    // The file cannot even be read from there, which the start-up error says.
    let error = game
        .use_keybinds_file(config.0.join("blocker").join(keybinds::FILE_NAME))
        .unwrap_err();
    assert!(error.contains("blocker"), "{error}");
    assert!(game.keybinds().is_default());
    game.press_escape();
    game.pause_activate(PauseItem::Settings);
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::Q);
    assert_eq!(
        game.keybinds().key(Action::Jump, Slot::Primary),
        Some(KeyCode::Q)
    );
    assert!(game
        .pause_menu()
        .note()
        .unwrap()
        .starts_with("Keybinds not saved"));
}

#[test]
fn a_game_with_no_keybinds_file_never_writes_one() {
    let mut game = test_game();
    game.press_escape();
    game.pause_activate(PauseItem::Settings);
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::Q);
    assert_eq!(
        game.keybinds().key(Action::Jump, Slot::Primary),
        Some(KeyCode::Q)
    );
    assert_eq!(game.pause_menu().note(), None);
    assert_eq!(game.keybinds_path, None);
    game.pause_activate(PauseItem::ResetKeybinds);
    assert_eq!(game.keybinds(), &Keybinds::default());
}

#[test]
fn rebinding_does_not_move_the_world_and_the_menu_still_stops_it() {
    let config = Config::new();
    let mut game = settings_game(&config.file());
    let player = game.player.clone();
    rebind(&mut game, Action::Jump, Slot::Primary, KeyCode::Q);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: true,
            ..PlayerInput::default()
        },
        1.0 / 120.0,
    );
    assert_eq!(game.player.position(), player.position());
    assert!(game.is_paused());
}
