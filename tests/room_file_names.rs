//! `load_extra_rooms` refuses a `room_*.ron` file whose number is not a plain number without
//! leading zeros. The rule stays strict; the error must say which file and what to do about it.
mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{load_extra_rooms, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::room::{load_rooms, RoomEnemies, RoomId};

const HINT: &str =
    "room files are named room_<number>.ron with no leading zeros; rename or move it";

/// A temp copy of the fixture assets, removed on drop.
struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-room-file-names-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["bosses", "rooms", "entities", "behaviors"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
    fn write(&self, file: &str, source: &str) {
        std::fs::write(self.0.join("rooms").join(file), source).unwrap();
    }
    fn load(&self) -> Result<Vec<hollow_steps::room::Room>, String> {
        let enemies = EnemyDefinitions::load(&self.0).unwrap();
        load_extra_rooms(&self.0, &enemies)
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// The source of a valid custom room: the fixture boss room's layout, no exits, no enemies.
fn room_source(assets: &Path) -> String {
    let [_, _, room] = load_rooms(assets).unwrap();
    let mut document: RoomEnemies = ron::from_str("(enemies: [])").unwrap();
    let mut layout = RoomLayout::from_room(&room);
    layout.exits.clear();
    document.layout = Some(layout);
    ron::to_string(&document).unwrap()
}

#[test]
fn a_room_file_with_a_non_numeric_name_names_the_file_and_the_fix() {
    let assets = Assets::new();
    assets.write("room_old.ron", &room_source(&assets.0));
    let error = assets.load().unwrap_err();
    assert!(
        error.contains("room_old.ron is not a valid room file name"),
        "{error}"
    );
    assert!(error.contains(HINT), "{error}");
}

#[test]
fn a_room_file_with_a_leading_zero_names_the_file_and_the_fix() {
    let assets = Assets::new();
    assets.write("room_01.ron", &room_source(&assets.0));
    let error = assets.load().unwrap_err();
    assert!(
        error.contains("room_01.ron is not a valid room file name"),
        "{error}"
    );
    assert!(error.contains(HINT), "{error}");
}

#[test]
fn an_empty_or_signed_number_is_refused_the_same_way() {
    for file in ["room_.ron", "room_+3.ron"] {
        let assets = Assets::new();
        assets.write(file, &room_source(&assets.0));
        let error = assets.load().unwrap_err();
        assert!(
            error.contains(&format!("{file} is not a valid room file name")),
            "{error}"
        );
        assert!(error.contains(HINT), "{error}");
    }
}

#[test]
fn a_plain_numbered_room_file_still_loads_and_other_files_are_ignored() {
    let assets = Assets::new();
    assets.write("room_3.ron", &room_source(&assets.0));
    assets.write("notes.ron", "this is not a room");
    let rooms = assets.load().unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].id, RoomId::Custom(3));
}
