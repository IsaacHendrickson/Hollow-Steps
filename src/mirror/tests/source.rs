//! `FlippedDocuments` and the loaders that take their documents from it: what a flipped world is
//! built from, and that the plain loaders (the editor's) are exactly what they were.
use std::path::{Path, PathBuf};

use crate::editor::RoomLayout;
use crate::editor_files::{load_extra_rooms, load_extra_rooms_from, read_document, room_filename};
use crate::enemy::EnemyDefinitions;
use crate::mirror::fixture::{self, BOSS, CLOSET, HUB, SECOND, START, TOWER};
use crate::mirror::{flip_world, FlippedDocuments};
use crate::room::{load_rooms, load_rooms_from, test_rooms, Room, RoomEnemies, RoomId};

/// A private assets folder holding `documents` as room files, plus everything else the loaders
/// need from `tests/fixtures`.
struct Assets(PathBuf);

impl Assets {
    fn new(documents: &[(RoomId, RoomEnemies)]) -> Self {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("hollow-steps-source-{}-{n}", std::process::id()));
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        copy_dir(&fixtures, &path);
        let assets = Self(path);
        for (id, document) in documents {
            assets.write(*id, &ron::to_string(document).unwrap());
        }
        assets
    }

    fn write(&self, id: RoomId, text: &str) {
        std::fs::write(self.rooms().join(room_filename(id)), text).unwrap();
    }

    fn rooms(&self) -> PathBuf {
        self.0.join("rooms")
    }

    fn path(&self, id: RoomId) -> PathBuf {
        self.rooms().join(room_filename(id))
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn layouts_of(rooms: &[Room]) -> Vec<(RoomId, RoomLayout)> {
    rooms
        .iter()
        .map(|r| (r.id, RoomLayout::from_room(r)))
        .collect()
}

/// Every room of the folder the way the plain loaders build it.
fn plain_rooms(assets: &Path) -> Vec<Room> {
    let mut rooms = load_rooms(assets).unwrap().into_iter().collect::<Vec<_>>();
    rooms.extend(load_extra_rooms(assets, &EnemyDefinitions::load(assets).unwrap()).unwrap());
    rooms
}

/// Every room built from `set`'s documents.
fn flipped_rooms(assets: &Path, set: &FlippedDocuments) -> Result<Vec<Room>, String> {
    let read = |path: &Path| set.document(path);
    let mut rooms = load_rooms_from(assets, &read)?
        .into_iter()
        .collect::<Vec<_>>();
    let definitions = EnemyDefinitions::load(assets)?;
    rooms.extend(load_extra_rooms_from(assets, &definitions, &read)?);
    Ok(rooms)
}

#[test]
fn the_plain_loaders_are_the_flipped_loaders_given_the_files() {
    // `load_rooms` and `load_extra_rooms` are the `_from` versions reading the files.
    let assets = Assets::new(&fixture::dungeon(true));
    let (first, second) = (
        load_rooms(&assets.0).unwrap(),
        load_rooms_from(&assets.0, &read_document).unwrap(),
    );
    assert_eq!(layouts_of(&first), layouts_of(&second));
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let (first, second) = (
        load_extra_rooms(&assets.0, &definitions).unwrap(),
        load_extra_rooms_from(&assets.0, &definitions, &read_document).unwrap(),
    );
    assert_eq!(layouts_of(&first), layouts_of(&second));
    // And the fixture dungeon comes back out of them as it went in.
    let mut built = layouts_of(&plain_rooms(&assets.0));
    built.sort_by_key(|(id, _)| id.index());
    let mut expected = fixture::layouts(true);
    expected.sort_by_key(|(id, _)| id.index());
    assert_eq!(built, expected);
}

#[test]
fn every_room_but_the_hub_is_flipped_and_the_rooms_are_what_flip_world_says() {
    let assets = Assets::new(&fixture::dungeon(true));
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    let mut expected = fixture::dungeon(true);
    flip_world(&mut expected, |id| id != HUB);
    let mut built = layouts_of(&flipped_rooms(&assets.0, &set).unwrap());
    built.sort_by_key(|(id, _)| id.index());
    let mut expected: Vec<_> = expected
        .into_iter()
        .map(|(id, document)| (id, document.layout.unwrap()))
        .collect();
    expected.sort_by_key(|(id, _)| id.index());
    assert_eq!(built, expected);
    assert_eq!(expected.len(), 6, "all six rooms: {TOWER:?} {CLOSET:?}");
}

#[test]
fn a_game_with_no_hub_flips_every_room() {
    let assets = Assets::new(&fixture::dungeon(true));
    let set = FlippedDocuments::read(&assets.0, None);
    let mut expected = fixture::dungeon(true);
    flip_world(&mut expected, |_| true);
    for (id, document) in expected {
        let got = set.document(&assets.path(id)).unwrap();
        assert_eq!(got, document, "{id:?}");
    }
}

#[test]
fn rooms_without_a_layout_are_left_as_they_are() {
    // The fixtures' own three rooms have no layout, so there is nothing to mirror.
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let set = FlippedDocuments::read(&assets, None);
    let flipped = flipped_rooms(&assets, &set).unwrap();
    let plain = plain_rooms(&assets);
    assert_eq!(layouts_of(&flipped), layouts_of(&plain));
    assert_eq!(flipped.len(), test_rooms().len());
}

#[test]
fn the_flip_never_writes_a_file() {
    let assets = Assets::new(&fixture::dungeon(true));
    let before: Vec<_> = [HUB, START, SECOND, BOSS]
        .iter()
        .map(|id| std::fs::read_to_string(assets.path(*id)).unwrap())
        .collect();
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    flipped_rooms(&assets.0, &set).unwrap();
    let after: Vec<_> = [HUB, START, SECOND, BOSS]
        .iter()
        .map(|id| std::fs::read_to_string(assets.path(*id)).unwrap())
        .collect();
    assert_eq!(before, after);
}

#[test]
fn a_file_that_cannot_be_read_fails_the_way_the_plain_loaders_fail() {
    let assets = Assets::new(&fixture::dungeon(true));
    assets.write(SECOND, "this is not a room");
    let plain = load_rooms(&assets.0).unwrap_err();
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    let flipped = flipped_rooms(&assets.0, &set).unwrap_err();
    assert_eq!(flipped, plain);
    assert!(flipped.contains("second.ron"), "{flipped}");
    // The same for a custom room.
    let assets = Assets::new(&fixture::dungeon(true));
    assets.write(TOWER, "(enemies: [");
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let plain = load_extra_rooms(&assets.0, &definitions).unwrap_err();
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    let flipped = load_extra_rooms_from(&assets.0, &definitions, &|p: &Path| set.document(p));
    assert_eq!(flipped.unwrap_err(), plain);
    assert!(plain.contains("room_1.ron"), "{plain}");
}

#[test]
fn a_missing_fixed_room_file_is_the_plain_loaders_error_too() {
    let assets = Assets::new(&fixture::dungeon(true));
    std::fs::remove_file(assets.path(BOSS)).unwrap();
    let plain = load_rooms(&assets.0).unwrap_err();
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    let flipped = flipped_rooms(&assets.0, &set).unwrap_err();
    assert_eq!(flipped, plain);
}

#[test]
fn files_with_other_names_are_the_loaders_to_refuse_and_are_not_read_here() {
    let assets = Assets::new(&fixture::dungeon(true));
    let odd = ron::to_string(&fixture::dungeon(true)[5].1).unwrap();
    for name in ["room_01.ron", "room_x.ron", "notes.ron", "room_.ron"] {
        std::fs::write(assets.rooms().join(name), &odd).unwrap();
    }
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    // The loader still insists on its naming rule, flipped or not.
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let plain = load_extra_rooms(&assets.0, &definitions).unwrap_err();
    let flipped = load_extra_rooms_from(&assets.0, &definitions, &|p: &Path| set.document(p));
    assert_eq!(flipped.unwrap_err(), plain);
    // None of them was flipped by this pass: asking for one reads the file as it is.
    for name in ["room_01.ron", "notes.ron"] {
        let path = assets.rooms().join(name);
        assert_eq!(set.document(&path).unwrap(), read_document(&path).unwrap());
    }
}

#[test]
fn a_path_the_set_does_not_hold_is_read_as_it_is() {
    let assets = Assets::new(&fixture::dungeon(true));
    let set = FlippedDocuments::read(&assets.0, Some(HUB));
    let missing = assets.rooms().join("room_99.ron");
    assert_eq!(set.document(&missing), read_document(&missing));
    assert!(set.document(&missing).is_err());
    let hub = set.document(&assets.path(HUB)).unwrap();
    let mut file = read_document(&assets.path(HUB)).unwrap();
    // Only the hub exit's arrival into the flipped start room differs from the file.
    let mut flipped = fixture::dungeon(true);
    flip_world(&mut flipped, |id| id != HUB);
    let arrival = flipped[0].1.layout.as_ref().unwrap().exits[0].spawn;
    file.layout.as_mut().unwrap().exits[0].spawn = arrival;
    assert_eq!(hub, file);
}
