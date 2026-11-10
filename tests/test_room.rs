mod support;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::RoomLayout;
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomId;
use macroquad::math::{Rect, Vec2};

const PLATFORM: Rect = Rect {
    x: 600.0,
    y: 380.0,
    w: 120.0,
    h: 20.0,
};

fn editing() -> GameState {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn run(game: &mut GameState, seconds: f32, input: PlayerInput) {
    for _ in 0..(seconds * 120.0) as u32 {
        game.update(input, 1.0 / 120.0);
    }
}

fn idle() -> PlayerInput {
    PlayerInput::default()
}

#[test]
fn the_test_room_plays_the_unsaved_draft_from_its_spawn() {
    let mut game = editing();
    {
        let draft = &mut game.editor_mut().unwrap().draft;
        draft.add_terrain(PLATFORM).unwrap();
        draft.layout.spawn = (660.0, 300.0);
    }
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert!(
        game.editor().is_none(),
        "the editor is parked while playing"
    );
    assert_eq!(game.room().terrain.solids(), [PLATFORM]);
    assert_eq!(game.player().position(), Vec2::new(660.0, 300.0));

    // The game runs: the player falls from the spawn and lands on the platform.
    run(&mut game, 2.0, idle());
    let player = game.player();
    assert_eq!(player.position().y + player.size().y / 2.0, PLATFORM.y);
    assert!(player.is_grounded());
}

#[test]
fn returning_restores_the_same_session_with_the_draft_intact() {
    let mut game = editing();
    let saved = RoomLayout::from_room(game.room());
    let before = {
        let session = game.editor_mut().unwrap();
        session.draft.add_terrain(PLATFORM).unwrap();
        session.dirty = true;
        session.message = "hello".into();
        session.draft.clone()
    };
    game.start_test_room().unwrap();
    run(&mut game, 1.0, idle());
    game.end_test_room().unwrap();

    assert!(!game.testing_room());
    let session = game.editor().unwrap();
    assert_eq!(session.id, RoomId::First);
    assert_eq!(session.draft, before);
    assert!(session.dirty, "still unsaved");
    assert_eq!(session.message, "hello");
    // The saved room is back, without the unsaved platform.
    assert!(game.room().terrain.is_empty());
    assert_eq!(RoomLayout::from_room(game.room()), saved);
}

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-test-room-{}-{}",
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
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// Every file under `root` with its contents, keyed by path relative to `root`.
fn files(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(root, &path, out);
            } else {
                let relative = path.strip_prefix(root).unwrap().to_owned();
                out.insert(relative, std::fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

#[test]
fn testing_a_room_writes_nothing_to_disk() {
    let temp = TempAssets::new();
    let before = files(&temp.0);
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    {
        let session = game.editor_mut().unwrap();
        session.draft.add_terrain(PLATFORM).unwrap();
        session.dirty = true;
    }
    game.start_test_room().unwrap();
    run(&mut game, 1.0, idle());
    assert_eq!(files(&temp.0), before, "nothing is written while testing");
    game.end_test_room().unwrap();
    assert_eq!(files(&temp.0), before, "nothing is written on return");

    // The unsaved platform is still only in the draft; a fresh game never sees it.
    assert_eq!(game.editor().unwrap().draft.layout.terrain.len(), 1);
    let fresh = GameState::try_new(&temp.0).unwrap();
    assert!(fresh.room().terrain.is_empty());
}

#[test]
fn playing_does_not_change_the_player_or_the_saved_room() {
    let mut game = editing();
    let player_before = game.player().clone();
    let items_before = game.items().len();
    let coins_before = game.coin_count();
    assert!(items_before > 0, "the fixture room has items");
    // Stand on the first item so one pickup press takes it.
    let item = game.editor().unwrap().draft.layout.items[0].position;
    game.editor_mut().unwrap().draft.layout.spawn = (item.0, 436.0);

    game.start_test_room().unwrap();
    run(&mut game, 0.5, idle());
    let pickup = PlayerInput {
        pickup_pressed: true,
        ..idle()
    };
    game.update(pickup, 1.0 / 120.0);
    assert_eq!(
        game.player().collected_items().len(),
        player_before.collected_items().len() + 1
    );
    assert_eq!(game.items().len(), items_before - 1);

    game.end_test_room().unwrap();
    assert_eq!(game.player(), &player_before);
    assert_eq!(
        game.items().len(),
        items_before,
        "the saved room keeps its item"
    );
    assert_eq!(game.coin_count(), coins_before);
}

#[test]
fn an_invalid_draft_shows_the_validation_error_and_does_not_start() {
    let mut game = editing();
    let player_before = game.player().clone();
    {
        // Straight into the draft, past the editor's own checks: a solid over the spawn.
        let draft = &mut game.editor_mut().unwrap().draft;
        let spawn = draft.layout.spawn;
        draft
            .layout
            .terrain
            .push((spawn.0 - 40.0, spawn.1 - 20.0, 80.0, 40.0));
    }
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert!(!game.testing_room());
    let session = game.editor().expect("the editor stays open");
    assert!(session.message.contains(&error), "{}", session.message);
    assert!(
        session.message.contains("Cannot test"),
        "{}",
        session.message
    );
    assert_eq!(game.player(), &player_before);
    assert!(game.room().terrain.is_empty());
}

#[test]
fn a_test_needs_the_editor_and_cannot_be_started_twice() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    assert!(game.start_test_room().unwrap_err().contains("editor"));
    assert!(game.end_test_room().is_err());
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    assert!(game.start_test_room().unwrap_err().contains("already"));
    assert!(game.testing_room());
}

#[test]
fn exits_do_nothing_in_a_test_room() {
    let mut game = editing();
    game.start_test_room().unwrap();
    assert!(
        !game.room().exits.is_empty(),
        "the room has an exit on the right"
    );
    let walk_right = PlayerInput {
        move_axis: 1.0,
        ..idle()
    };
    // Walk until the player stands inside the exit zone (x from 928 on the right edge).
    // Outside a test this is where the walk ends in the next room (see tests/rooms.rs).
    for _ in 0..600 {
        game.update(walk_right, 1.0 / 60.0);
        if game.player().position().x > 930.0 {
            assert_eq!(game.room().id, RoomId::First, "the exit did not fire");
            game.update(walk_right, 1.0 / 60.0);
            assert_eq!(game.room().id, RoomId::First);
            return;
        }
    }
    panic!("the player never reached the exit zone");
}

#[test]
fn f2_returns_and_reload_or_opening_a_room_wait_until_then() {
    let mut game = editing();
    game.start_test_room().unwrap();
    assert!(game.reload_rooms().unwrap_err().contains("test room"));
    assert!(game
        .edit_room(RoomId::Second)
        .unwrap_err()
        .contains("test room"));
    assert!(game.save_editor().is_err());
    assert!(game.testing_room());

    // F2 calls toggle_editor, which ends the test instead of opening a second session.
    game.toggle_editor().unwrap();
    assert!(!game.testing_room());
    assert!(game.editor().is_some());
    game.reload_rooms().unwrap();
}

#[test]
fn an_unsaved_new_room_can_be_tested_and_leaves_no_trace() {
    let mut game = editing();
    game.editor_mut().unwrap().new_room();
    let id = game.editor().unwrap().id;
    assert_ne!(id, RoomId::First);

    game.start_test_room().unwrap();
    assert_eq!(game.room().id, id);
    run(&mut game, 0.5, idle());
    game.end_test_room().unwrap();

    assert_eq!(game.room().id, RoomId::First, "back where the game was");
    assert!(
        game.edit_room(id).is_err(),
        "the unsaved room was never added"
    );
    let session = game.editor().unwrap();
    assert_eq!(session.id, id);
    assert!(session.new && session.dirty);
}

#[test]
fn a_boss_room_is_tested_with_its_boss_and_keeps_the_saved_boss() {
    let mut game = editing();
    game.edit_room(RoomId::Boss).unwrap();
    assert!(game.room().boss_special.is_some());
    game.start_test_room().unwrap();
    assert!(
        game.room().boss_special.is_some(),
        "the draft's boss is placed"
    );
    run(&mut game, 0.5, idle());
    game.end_test_room().unwrap();
    assert!(game.room().boss_special.is_some());
    assert_eq!(game.editor().unwrap().id, RoomId::Boss);
}
