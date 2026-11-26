//! The room preview on rooms loaded from the fixtures (`tests/fixtures/rooms`): what a loaded
//! room adds to a layout (enemy patrol ranges, the boss, coins), and how a room is named.
mod support;

use std::path::Path;
use std::sync::OnceLock;

use hollow_steps::player::PlayerConfig;
use hollow_steps::room::RoomId;
use hollow_steps::room_preview::{self, load, Ruler};

fn ruler() -> Ruler {
    static RULER: OnceLock<Ruler> = OnceLock::new();
    *RULER.get_or_init(|| {
        let base = PlayerConfig::default();
        Ruler::from_configs(&base, &base)
    })
}

fn preview(id: RoomId) -> String {
    let loaded = load::load_one(&support::fixture_assets(), id).expect("fixture room loads");
    assert_eq!(loaded.id, id);
    room_preview::render(&loaded.scene(), &ruler(), 8.0)
}

/// The rows of the picture: the lines between the header and the table.
fn picture(text: &str) -> Vec<&str> {
    let start = text.lines().position(|line| line.is_empty()).unwrap() + 1;
    let end = text
        .lines()
        .position(|line| line.starts_with("Placed things"))
        .unwrap()
        - 1;
    text.lines().skip(start).take(end - start - 1).collect()
}

#[test]
fn a_loaded_room_shows_the_patrol_range_of_its_enemy() {
    let text = preview(RoomId::First);
    let row = text
        .lines()
        .find(|line| line.contains("enemy"))
        .expect("the fixture's chase enemy has a row");
    assert!(
        row.contains("chase") && row.contains("patrol x 560 to 800 (240 px)"),
        "{row}"
    );
    let picture = picture(&text);
    let line = picture
        .iter()
        .find(|line| line.contains('<'))
        .expect("a patrol line");
    assert!(line.contains('>') && line.contains('E'), "{line}");
    // 240 px at 8 px per character is 30 cells between the ends.
    let (left, right) = (line.find('<').unwrap(), line.find('>').unwrap());
    assert_eq!(right - left, 30, "{line}");
}

#[test]
fn a_loaded_room_shows_its_boss_and_the_built_in_coins() {
    let text = preview(RoomId::Boss);
    assert!(
        text.lines()
            .any(|line| line.contains("boss") && line.contains("Ash Warden")),
        "{text}"
    );
    assert!(picture(&text).iter().any(|line| line.contains('B')));
    // The first room's coins are part of the built-in room, not of its file.
    assert!(preview(RoomId::First)
        .lines()
        .any(|line| line.contains(" coin ")));
}

#[test]
fn the_picture_of_a_loaded_room_is_as_wide_as_its_bounds() {
    let text = preview(RoomId::Second);
    let columns = room_preview::columns(800.0, 8.0);
    for line in picture(&text) {
        assert_eq!(line.chars().count(), 6 + columns, "{line:?}");
    }
    assert!(
        text.contains("Room second: 800 x 360 px at (80, 40)"),
        "{text}"
    );
}

#[test]
fn rooms_are_named_by_their_file_names() {
    let assets = support::fixture_assets();
    let id = |text: &str| load::parse_id(&assets, text);
    assert_eq!(id("first"), Ok(RoomId::First));
    assert_eq!(id("SECOND"), Ok(RoomId::Second));
    assert_eq!(id("boss"), Ok(RoomId::Boss));
    assert_eq!(id("3"), Ok(RoomId::Custom(3)));
    assert_eq!(id("room_12"), Ok(RoomId::Custom(12)));
    for bad in ["", "room_", "007", "room_x", "third", "-1"] {
        assert!(id(bad).unwrap_err().contains("is not a room"), "{bad:?}");
    }
    assert_eq!(room_preview::room_label(RoomId::Custom(4)), "room_4");
    assert_eq!(room_preview::room_label(RoomId::Boss), "boss");
}

#[test]
fn a_room_file_names_its_room_and_its_assets_folder() {
    let assets = support::fixture_assets();
    let file = |name: &str| assets.join("rooms").join(name);
    assert_eq!(
        load::id_of_path(&file("first.ron")),
        Ok((RoomId::First, assets.clone()))
    );
    assert_eq!(
        load::id_of_path(&file("room_5.ron")),
        Ok((RoomId::Custom(5), assets.clone()))
    );
    for bad in ["notes.ron", "room_05.ron", "first.txt"] {
        assert!(load::id_of_path(&file(bad)).is_err(), "{bad}");
    }
    assert!(
        load::id_of_path(Path::new("first.ron")).is_err(),
        "no folder to take assets from"
    );
}

#[test]
fn a_missing_room_file_is_refused_with_the_path() {
    let error = load::load_one(&support::fixture_assets(), RoomId::Custom(99)).unwrap_err();
    assert!(error.contains("room_99.ron"), "{error}");
}
