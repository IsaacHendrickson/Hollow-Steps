//! Door exits end to end: author a door in each of two rooms, link them in the editor, Save,
//! restart from the files and walk through both doors with Up.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::RoomLayout;
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::{RoomEnemies, RoomId};
use macroquad::math::Vec2;

const DT: f32 = 1.0 / 128.0;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-link-{}-{}",
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

    fn document(&self, file: &str) -> RoomEnemies {
        ron::de::from_bytes(&std::fs::read(self.0.join("rooms").join(file)).unwrap()).unwrap()
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn up() -> PlayerInput {
    PlayerInput {
        pickup_pressed: true,
        ..PlayerInput::default()
    }
}

/// A door each in Second and First, linked to each other and saved; returns their placements
/// as saved: (First's door, Second's door).
fn linked_doors(
    temp: &TempAssets,
) -> (
    hollow_steps::editor::ExitPlacement,
    hollow_steps::editor::ExitPlacement,
) {
    let mut game = temp.game();
    // Second gets its door first, so there is one to connect to.
    game.edit_room(RoomId::Second).unwrap();
    let second_door = game
        .editor_mut()
        .unwrap()
        .draft
        .add_door(RoomId::First, Vec2::new(180.0, 436.0))
        .unwrap();
    game.save_editor().unwrap();
    let hollow_steps::editor::Selection::Exit(second_index) = second_door else {
        panic!("add_door selects the door");
    };

    game.edit_room(RoomId::First).unwrap();
    let session = game.editor_mut().unwrap();
    let first_index = session.begin_link_new_door().unwrap();
    session.link_choose(RoomId::Second).unwrap();
    session.link_connect(second_index).unwrap();
    game.save_editor().unwrap();

    let first = temp.document("first.ron").layout.unwrap();
    let second = temp.document("second.ron").layout.unwrap();
    (
        first.exits[first_index].clone(),
        second.exits[second_index].clone(),
    )
}

#[test]
fn linking_two_doors_makes_each_arrive_in_front_of_the_other() {
    let temp = TempAssets::new();
    let (first_door, second_door) = linked_doors(&temp);
    assert!(first_door.is_door() && second_door.is_door());
    // First's door stands at First's spawn, Second's at Second's spawn.
    assert_eq!(first_door.door, Some((156.0, 388.0, 48.0, 72.0)));
    assert_eq!(second_door.door, Some((136.0, 328.0, 48.0, 72.0)));
    assert_eq!(first_door.target_room, RoomId::Second);
    assert_eq!(second_door.target_room, RoomId::First);
    // Each arrival is standing on the bottom edge of the other room's door.
    assert_eq!(first_door.spawn, (160.0, 376.0));
    assert_eq!(second_door.spawn, (180.0, 436.0));
}

#[test]
fn the_saved_files_hold_the_door_rectangle_and_keep_edge_exits_as_they_were() {
    let temp = TempAssets::new();
    linked_doors(&temp);
    let text = std::fs::read_to_string(temp.0.join("rooms/first.ron")).unwrap();
    assert_eq!(text.matches("door:").count(), 1, "{text}");
    assert!(text.contains("door: (156.0, 388.0, 48.0, 72.0)"), "{text}");
    assert_eq!(
        text.matches("edge:").count(),
        1,
        "the edge exit keeps its edge: {text}"
    );
    let layout: RoomLayout = temp.document("first.ron").layout.unwrap();
    assert_eq!(layout.exits.iter().filter(|e| e.is_door()).count(), 1);
}

#[test]
fn after_a_restart_up_in_front_of_each_door_walks_through_to_the_other() {
    let temp = TempAssets::new();
    linked_doors(&temp);
    let mut game = temp.game();

    // The player starts at First's spawn, which is where its door stands.
    assert_eq!(game.room().id, RoomId::First);
    game.update(PlayerInput::default(), DT);
    assert!(
        game.door_in_reach().is_some(),
        "the prompt shows at the door"
    );

    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.player().position(), Vec2::new(160.0, 376.0));
    game.update(PlayerInput::default(), DT);
    assert!(
        game.door_in_reach().is_some(),
        "and again at the door it came out of"
    );

    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.player().position(), Vec2::new(180.0, 436.0));
}

#[test]
fn a_door_link_cannot_start_while_another_link_is_open() {
    let temp = TempAssets::new();
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    session.begin_link_new_door().unwrap();
    let doors = session.draft.layout.exits.len();
    let error = session.begin_link_new_door().unwrap_err();
    assert!(error.contains("finish or cancel"), "{error}");
    assert_eq!(
        session.draft.layout.exits.len(),
        doors,
        "no second door was added"
    );
}
