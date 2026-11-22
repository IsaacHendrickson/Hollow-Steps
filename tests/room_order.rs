//! Rooms load and rebuild in room id order, not in file-name text order: with eleven or more
//! custom rooms `room_10.ron` sorts before `room_2.ron` as text, but room 2 comes first.
mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::definition_editor::DefinitionDraft;
use hollow_steps::editor::{load_extra_rooms, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::room::{load_rooms, RoomEnemies, RoomId};

/// A temp copy of the fixture assets, removed on drop.
struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-room-order-{}-{}",
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
    /// Writes `room_<id>.ron` for each id, in the order given.
    fn write_rooms(&self, ids: &[usize]) {
        let source = room_source(&self.0);
        for id in ids {
            std::fs::write(self.0.join("rooms").join(format!("room_{id}.ron")), &source).unwrap();
        }
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

fn custom(ids: &[usize]) -> Vec<RoomId> {
    ids.iter().map(|id| RoomId::Custom(*id)).collect()
}

#[test]
fn extra_rooms_load_in_id_order_not_file_name_order() {
    let assets = Assets::new();
    assets.write_rooms(&[0, 2, 10, 11]);
    let enemies = EnemyDefinitions::load(&assets.0).unwrap();
    let rooms = load_extra_rooms(&assets.0, &enemies).unwrap();
    let ids: Vec<RoomId> = rooms.iter().map(|room| room.id).collect();
    assert_eq!(ids, custom(&[0, 2, 10, 11]));
}

#[test]
fn the_load_order_does_not_depend_on_the_order_the_files_were_written() {
    let assets = Assets::new();
    assets.write_rooms(&[11, 3, 10, 2, 0]);
    let enemies = EnemyDefinitions::load(&assets.0).unwrap();
    let rooms = load_extra_rooms(&assets.0, &enemies).unwrap();
    let ids: Vec<RoomId> = rooms.iter().map(|room| room.id).collect();
    assert_eq!(ids, custom(&[0, 2, 3, 10, 11]));
}

#[test]
fn a_single_digit_gap_and_no_extra_rooms_keep_working() {
    let assets = Assets::new();
    let enemies = EnemyDefinitions::load(&assets.0).unwrap();
    assert!(load_extra_rooms(&assets.0, &enemies).unwrap().is_empty());
    assets.write_rooms(&[9, 1]);
    let rooms = load_extra_rooms(&assets.0, &enemies).unwrap();
    let ids: Vec<RoomId> = rooms.iter().map(|room| room.id).collect();
    assert_eq!(ids, custom(&[1, 9]));
}

#[test]
fn rebuilding_the_rooms_for_a_save_gives_the_same_order_as_loading_them() {
    let assets = Assets::new();
    assets.write_rooms(&[0, 2, 10, 11]);
    let content = DefinitionDraft::load(&assets.0).unwrap();
    let (_, rebuilt) = content.prepare(&content.plan(&[]).unwrap()).unwrap();
    let ids: Vec<RoomId> = rebuilt.iter().map(|room| room.id).collect();
    let mut expected = vec![RoomId::First, RoomId::Second, RoomId::Boss];
    expected.extend(custom(&[0, 2, 10, 11]));
    assert_eq!(ids, expected);
}
