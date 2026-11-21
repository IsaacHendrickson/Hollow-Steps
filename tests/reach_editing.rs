//! The reach numbers in the editor's session and in the game: the interval of the numbers as
//! edited, where the preview starts, and the line a save of `player.ron` leaves behind.
//! Editing First: Second's exit 0 leads back and arrives at (860, 436).
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::Selection;
use hollow_steps::game::GameState;
use hollow_steps::room::RoomId;
use hollow_steps::tuning::Field;
use macroquad::math::Vec2;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-reach-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn move_speed_of(game: &GameState) -> f32 {
    game.player_draft().saved().player.move_speed
}

// ---- the interval of the numbers as edited ----

#[test]
fn the_interval_follows_the_numbers_as_edited_and_the_key_notices_a_change() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    let (key, interval) = (session.reach_key(), session.reach_interval());
    assert_eq!(session.reach_key(), key, "nothing changed");
    assert!(interval.usable(), "{interval:?}");

    let faster = session.player.edited().player.move_speed * 1.5;
    session.player.set_field(Field::MoveSpeed, faster).unwrap();

    assert_ne!(
        session.reach_key(),
        key,
        "a number behind the reach changed"
    );
    let longer = session.reach_interval();
    assert!(longer.low() > interval.low(), "{longer:?} vs {interval:?}");
    assert!(
        longer.high() > interval.high(),
        "{longer:?} vs {interval:?}"
    );
}

#[test]
fn an_enemy_number_leaves_the_key_alone() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    let key = session.reach_key();
    let health = session.player.edited().enemy.default_health + 1;
    session
        .player
        .set_field(Field::EnemyDefaultHealth, f32::from(health))
        .unwrap();
    assert_ne!(
        session.player.edited(),
        session.player.saved(),
        "the number did change"
    );
    assert_eq!(session.reach_key(), key, "but it is not the player's");
}

// ---- where the preview starts ----

#[test]
fn the_preview_starts_at_the_spawn_or_the_selected_arrival() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    let spawn = Vec2::from(session.draft.layout.spawn);
    let arrival = Selection::Inbound {
        room: RoomId::Second,
        exit: 0,
    };

    assert_eq!(session.reach_origin(None), spawn);
    assert_eq!(session.reach_origin(Some(Selection::Exit(0))), spawn);
    assert_eq!(
        session.reach_origin(Some(arrival)),
        Vec2::new(860.0, 436.0),
        "the point the player arrives at"
    );
    let gone = Selection::Inbound {
        room: RoomId::Second,
        exit: 9,
    };
    assert_eq!(session.reach_origin(Some(gone)), spawn, "no such arrival");
}

// ---- the line a save leaves ----

#[test]
fn a_save_that_changes_the_numbers_leaves_the_new_reach_line_once() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    assert_eq!(game.take_reach_note(), None, "nothing saved yet");
    let faster = move_speed_of(&game) * 1.5;
    game.set_tuning_field(Field::MoveSpeed, faster).unwrap();

    game.save_tuning().unwrap();

    let note = game.take_reach_note().expect("the numbers changed");
    assert!(
        note.starts_with("jump ") && note.contains(" px, dash jump "),
        "{note}"
    );
    assert!(note.ends_with("; run Check dungeon"), "{note}");
    assert_eq!(game.take_reach_note(), None, "taken once");
}

#[test]
fn a_save_that_changes_no_player_number_leaves_no_line() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    let faster = move_speed_of(&game) * 1.5;
    game.set_tuning_field(Field::MoveSpeed, faster).unwrap();
    game.save_tuning().unwrap();
    game.take_reach_note();

    game.save_tuning().unwrap();
    assert_eq!(game.take_reach_note(), None, "the numbers are the same");

    let health = game.player_draft().saved().enemy.default_health + 1;
    game.set_tuning_field(Field::EnemyDefaultHealth, f32::from(health))
        .unwrap();
    game.save_tuning().unwrap();
    assert_eq!(
        game.take_reach_note(),
        None,
        "an enemy number is not the reach"
    );

    let health = game.player_draft().saved().player.max_health + 1;
    game.set_tuning_field(Field::MaxHealth, f32::from(health))
        .unwrap();
    game.save_tuning().unwrap();
    assert!(
        game.take_reach_note().is_some(),
        "any changed player number refreshes the line"
    );
}

#[test]
fn each_save_leaves_the_line_of_its_own_numbers() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    let speed = move_speed_of(&game);
    game.set_tuning_field(Field::MoveSpeed, speed * 1.25)
        .unwrap();
    game.save_tuning().unwrap();
    let saved_line = game.take_reach_note().unwrap();

    // A second save of other numbers leaves a different line.
    game.set_tuning_field(Field::MoveSpeed, speed * 1.75)
        .unwrap();
    game.save_tuning().unwrap();
    let newer = game.take_reach_note().unwrap();
    assert_ne!(newer, saved_line);
}
