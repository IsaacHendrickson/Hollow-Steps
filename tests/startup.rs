mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomId;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-startup-{}-{}",
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

    /// Adds `rooms/room_<id>.ron` by saving a new room through the editor, then renaming it.
    fn add_custom_room(&self, id: usize) {
        let mut game = GameState::try_new(&self.0).unwrap();
        game.toggle_editor().unwrap();
        game.editor_mut().unwrap().new_room();
        game.save_editor().unwrap();
        std::fs::rename(
            self.0.join("rooms").join("room_0.ron"),
            self.0.join("rooms").join(format!("room_{id}.ron")),
        )
        .unwrap();
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn a_missing_assets_folder_is_a_load_error_that_names_it_and_never_panics() {
    let name = format!("hollow-startup-missing-{}", std::process::id());
    let missing = std::env::temp_dir().join(&name);
    let error = GameState::try_new(&missing).unwrap_err();
    assert!(error.contains(&name), "{error}");
}

#[test]
fn a_broken_room_file_is_a_load_error_that_names_the_file_and_loading_recovers_when_fixed() {
    let temp = TempAssets::new();
    let room = temp.0.join("rooms").join("first.ron");
    let good = std::fs::read_to_string(&room).unwrap();
    std::fs::write(&room, "( this is not a room").unwrap();
    let error = GameState::try_new(&temp.0).unwrap_err();
    assert!(error.contains("first.ron"), "{error}");
    std::fs::write(&room, good).unwrap();
    assert_eq!(
        GameState::try_new(&temp.0).unwrap().room().id,
        RoomId::First
    );
}

#[test]
fn saving_a_new_room_makes_it_current_even_though_it_sorts_before_existing_rooms() {
    let temp = TempAssets::new();
    temp.add_custom_room(2);
    let mut game = GameState::try_new(&temp.0).unwrap();
    assert_eq!(game.room().id, RoomId::First);
    game.toggle_editor().unwrap();
    game.editor_mut().unwrap().new_room();
    game.save_editor().unwrap();
    // room_0 now sorts ahead of room_2 in the rebuilt room list.
    assert_eq!(game.room().id, RoomId::Custom(0));
    game.toggle_editor().unwrap();
    game.edit_room(RoomId::Custom(2)).unwrap();
    assert_eq!(game.room().id, RoomId::Custom(2));
    game.edit_room(RoomId::Custom(0)).unwrap();
    assert_eq!(game.room().id, RoomId::Custom(0));
    game.edit_room(RoomId::Second).unwrap();
    assert_eq!(game.room().id, RoomId::Second);
    assert!(game.edit_room(RoomId::Custom(99)).is_err());
    assert_eq!(game.room().id, RoomId::Second);
}

#[test]
fn exits_find_their_target_room_by_id_not_by_position() {
    let temp = TempAssets::new();
    temp.add_custom_room(7);
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    editor.draft.layout.exits[0].target_room = RoomId::Custom(7);
    editor.draft.layout.exits[0].spawn = (160.0, 436.0);
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    assert_eq!(game.room().id, RoomId::First);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Custom(7));
}
