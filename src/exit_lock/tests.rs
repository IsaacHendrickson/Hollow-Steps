//! Key ids, labels, the opened-exits memory and how a lock is stored in a room file. The room
//! is the first fixture room: 960 x 460, one edge exit on the right.
use macroquad::math::{Rect, Vec2};

use super::*;
use crate::editor::{ExitPlacement, ItemPlacement, RoomLayout};
use crate::room::{test_rooms, Exit, RoomEdge, RoomId};

fn layout() -> RoomLayout {
    RoomLayout::from_room(&test_rooms()[0])
}

fn edge_exit(offset: f32, locked_by: Option<u8>) -> Exit {
    Exit {
        edge: RoomEdge::Right,
        offset,
        length: 96.0,
        target_room: RoomId::Second,
        spawn: Vec2::new(160.0, 376.0),
        door: None,
        locked_by,
    }
}

fn door(x: f32, locked_by: Option<u8>) -> Exit {
    Exit {
        locked_by,
        ..Exit::doorway(
            Rect::new(x, 388.0, 48.0, 72.0),
            RoomId::Second,
            Vec2::new(160.0, 376.0),
        )
    }
}

fn written(layout: &RoomLayout) -> RoomLayout {
    let text = ron::ser::to_string_pretty(layout, ron::ser::PrettyConfig::default()).unwrap();
    ron::from_str(&text).unwrap()
}

// ---- key ids, labels, the item ----

#[test]
fn key_ids_one_to_eight_exist_and_nothing_else_does() {
    for id in FIRST_KEY..=LAST_KEY {
        assert!(check_key_id(id).is_ok(), "{id}");
        assert!(check_lock(Some(id)).is_ok(), "{id}");
    }
    for id in [0, 9, 10, 255] {
        let error = check_key_id(id).unwrap_err();
        assert!(
            error.contains(&id.to_string()) && error.contains("Key 1"),
            "{error}"
        );
        assert!(check_lock(Some(id)).is_err(), "{id}");
    }
    assert!(check_lock(None).is_ok());
}

#[test]
fn keys_are_items_named_key_n_and_listed_for_placement() {
    assert_eq!(Item::Key(3).name(), "Key 3");
    for id in FIRST_KEY..=LAST_KEY {
        assert!(
            Item::ALL.contains(&Item::Key(id)),
            "Key {id} is in Item::ALL"
        );
        assert_eq!(Item::Key(id).name(), format!("Key {id}"));
        assert_eq!(Item::Key(id).grants(), None, "a key is not an ability");
        assert!(Item::Key(id).check().is_ok());
    }
    // Amber Shard, Dash and Double Jump, then the keys.
    assert_eq!(Item::ALL.len(), 3 + usize::from(LAST_KEY));
    assert!(Item::Key(0).check().is_err() && Item::Key(9).check().is_err());
    assert!(Item::Dash.check().is_ok() && Item::AmberShard.check().is_ok());
    assert_eq!(
        Item::Key(0).name(),
        "Key",
        "a bad id still has a name to show in an error"
    );
}

#[test]
fn a_key_is_written_as_key_n_in_a_room_file() {
    assert_eq!(ron::to_string(&Item::Key(3)).unwrap(), "Key(3)");
    assert_eq!(ron::from_str::<Item>("Key(3)").unwrap(), Item::Key(3));
    assert_eq!(ron::from_str::<Item>("Dash").unwrap(), Item::Dash);
}

#[test]
fn labels_for_a_locked_exit_a_notice_and_a_key_marker() {
    assert_eq!(lock_label(Some(3)).as_deref(), Some("Key 3"));
    assert_eq!(lock_label(None), None);
    assert_eq!(lock_message(3), "Locked: needs Key 3");
    assert_eq!(key_marker_label(Item::Key(3)).as_deref(), Some("3"));
    assert_eq!(key_marker_label(Item::Key(8)).as_deref(), Some("8"));
    assert_eq!(key_marker_label(Item::Dash), None);
    assert_eq!(key_marker_label(Item::AmberShard), None);
}

// ---- opened exits ----

#[test]
fn a_rebuilt_room_gets_its_opened_exit_open_again_and_the_rest_stay_locked() {
    let mut opened = OpenedExits::default();
    opened.record(RoomId::First, &edge_exit(100.0, Some(3)));
    let mut exits = [edge_exit(100.0, Some(3)), edge_exit(250.0, Some(3))];

    opened.apply_to_rebuilt(RoomId::First, &mut exits);

    assert_eq!(exits[0].locked_by, None);
    assert_eq!(
        exits[1].locked_by,
        Some(3),
        "a second door with the same key id"
    );
    assert_eq!(opened.len(), 1, "still remembered for the next rebuild");
    let mut again = [edge_exit(100.0, Some(3))];
    opened.apply_to_rebuilt(RoomId::First, &mut again);
    assert_eq!(again[0].locked_by, None);
}

#[test]
fn an_exit_that_was_not_locked_is_not_remembered() {
    let mut opened = OpenedExits::default();
    opened.record(RoomId::First, &edge_exit(100.0, None));
    assert!(opened.is_empty());
}

#[test]
fn recording_the_same_exit_twice_keeps_one_entry() {
    let mut opened = OpenedExits::default();
    opened.record(RoomId::First, &edge_exit(100.0, Some(3)));
    opened.record(RoomId::First, &edge_exit(100.0, Some(3)));
    assert_eq!(opened.len(), 1);
}

#[test]
fn only_the_rebuilt_room_is_touched() {
    let mut opened = OpenedExits::default();
    opened.record(RoomId::First, &edge_exit(100.0, Some(3)));
    opened.record(RoomId::Second, &edge_exit(100.0, Some(3)));

    let mut second = [edge_exit(100.0, Some(3))];
    opened.apply_to_rebuilt(RoomId::Second, &mut second);

    assert_eq!(second[0].locked_by, None);
    assert_eq!(opened.len(), 2, "the first room's entry is left alone");
    let mut first = [edge_exit(100.0, Some(3))];
    opened.apply_to_rebuilt(RoomId::First, &mut first);
    assert_eq!(first[0].locked_by, None);
}

#[test]
fn an_entry_that_matches_no_exit_is_forgotten() {
    for (name, rebuilt) in [
        ("moved", edge_exit(130.0, Some(3))),
        ("lock taken off", edge_exit(100.0, None)),
        ("another key", edge_exit(100.0, Some(4))),
    ] {
        let mut opened = OpenedExits::default();
        opened.record(RoomId::First, &edge_exit(100.0, Some(3)));
        let mut exits = [rebuilt];
        let before = exits[0];

        opened.apply_to_rebuilt(RoomId::First, &mut exits);

        assert_eq!(
            exits[0], before,
            "{name}: what stands there is new, so it is left as filed"
        );
        assert!(opened.is_empty(), "{name}: the entry is forgotten");
    }
    let mut opened = OpenedExits::default();
    opened.record(RoomId::First, &edge_exit(100.0, Some(3)));
    opened.apply_to_rebuilt(RoomId::First, &mut []);
    assert!(opened.is_empty(), "a deleted exit is forgotten too");
}

#[test]
fn doors_are_told_apart_by_their_rectangle() {
    let mut opened = OpenedExits::default();
    opened.record(RoomId::First, &door(380.0, Some(1)));
    let mut exits = [door(100.0, Some(1)), door(380.0, Some(1))];

    opened.apply_to_rebuilt(RoomId::First, &mut exits);

    assert_eq!(exits[0].locked_by, Some(1));
    assert_eq!(exits[1].locked_by, None);
}

// ---- the room file ----

#[test]
fn a_lock_is_written_after_the_spawn_and_an_unlocked_exit_is_written_as_before() {
    let mut exit =
        ExitPlacement::edge_exit(RoomEdge::Right, 364.0, 96.0, RoomId::Second, (160.0, 376.0));
    assert_eq!(exit.locked_by, None);
    exit.locked_by = Some(3);
    assert_eq!(
        ron::to_string(&exit).unwrap(),
        "(edge:Right,offset:364.0,length:96.0,target_room:Second,spawn:(160.0,376.0),locked_by:3)"
    );
    let mut door = ExitPlacement {
        door: Some((380.0, 388.0, 48.0, 72.0)),
        ..exit.clone()
    };
    door.locked_by = Some(8);
    assert_eq!(
        ron::to_string(&door).unwrap(),
        "(door:(380.0,388.0,48.0,72.0),target_room:Second,spawn:(160.0,376.0),locked_by:8)"
    );
}

#[test]
fn rooms_with_and_without_a_lock_survive_a_round_trip() {
    let plain = layout();
    assert_eq!(written(&plain), plain);

    let mut locked = layout();
    locked.exits[0].locked_by = Some(3);
    locked.exits.push(ExitPlacement {
        door: Some((380.0, 388.0, 48.0, 72.0)),
        locked_by: Some(5),
        ..ExitPlacement::edge_exit(RoomEdge::Bottom, 0.0, 0.0, RoomId::Second, (160.0, 376.0))
    });
    locked.items.push(ItemPlacement {
        item: Item::Key(3),
        position: (200.0, 450.0),
    });
    let back = written(&locked);
    assert_eq!(back, locked);
    assert_eq!(back.exits[0].locked_by, Some(3));
    assert_eq!(back.exits[1].locked_by, Some(5));
}

#[test]
fn an_exit_from_an_older_room_file_loads_unlocked() {
    let edge: ExitPlacement = ron::from_str(
        "(edge: Left, offset: 12.0, length: 96.0, target_room: First, spawn: (3.0, 4.0))",
    )
    .unwrap();
    let door: ExitPlacement =
        ron::from_str("(door: (1.0, 2.0, 30.0, 40.0), target_room: First, spawn: (3.0, 4.0))")
            .unwrap();
    assert_eq!((edge.locked_by, door.locked_by), (None, None));
}

#[test]
fn a_lock_in_a_file_is_read_with_either_kind_of_exit() {
    let edge: ExitPlacement = ron::from_str(
        "(edge: Left, offset: 12.0, length: 96.0, target_room: First, spawn: (3.0, 4.0), locked_by: 2)",
    )
    .unwrap();
    let door: ExitPlacement = ron::from_str(
        "(door: (1.0, 2.0, 30.0, 40.0), target_room: First, spawn: (3.0, 4.0), locked_by: 7)",
    )
    .unwrap();
    assert_eq!((edge.locked_by, door.locked_by), (Some(2), Some(7)));
    assert_eq!(edge.exit().locked_by, Some(2));
    assert_eq!(door.exit().locked_by, Some(7));
}

#[test]
fn from_room_keeps_the_lock_of_a_live_exit() {
    let mut room = test_rooms().into_iter().next().unwrap();
    room.exits[0].locked_by = Some(6);
    assert_eq!(RoomLayout::from_room(&room).exits[0].locked_by, Some(6));
    room.exits[0].locked_by = None;
    assert_eq!(RoomLayout::from_room(&room).exits[0].locked_by, None);
}

#[test]
fn a_room_is_refused_with_a_key_id_of_zero_or_nine_and_accepts_one_to_eight() {
    for id in [0, 9] {
        let mut lock = layout();
        lock.exits[0].locked_by = Some(id);
        let error = lock.validate_geometry().unwrap_err();
        assert!(
            error.contains("exit lock") && error.contains(&id.to_string()),
            "{error}"
        );

        let mut item = layout();
        item.items.push(ItemPlacement {
            item: Item::Key(id),
            position: (200.0, 450.0),
        });
        assert!(item.validate_geometry().is_err(), "key item {id}");
    }
    for id in FIRST_KEY..=LAST_KEY {
        let mut ok = layout();
        ok.exits[0].locked_by = Some(id);
        ok.items.push(ItemPlacement {
            item: Item::Key(id),
            position: (200.0, 450.0),
        });
        ok.validate_geometry()
            .unwrap_or_else(|error| panic!("key {id}: {error}"));
    }
}

// ---- where the label goes ----

#[test]
fn the_label_sits_inside_the_room_next_to_an_edge_band_and_in_the_middle_of_a_door() {
    let text = Vec2::new(40.0, 20.0);
    let side = Rect::new(100.0, 200.0, 32.0, 96.0); // centered on (116, 248)
    let across = Rect::new(300.0, 50.0, 96.0, 32.0); // centered on (348, 66)

    let at = |edge, zone| label_rect(edge, false, zone, text);
    assert_eq!(
        at(RoomEdge::Left, side),
        Rect::new(136.0, 238.0, 40.0, 20.0)
    );
    assert_eq!(
        at(RoomEdge::Right, side),
        Rect::new(56.0, 238.0, 40.0, 20.0)
    );
    assert_eq!(
        at(RoomEdge::Top, across),
        Rect::new(328.0, 86.0, 40.0, 20.0)
    );
    assert_eq!(
        at(RoomEdge::Bottom, across),
        Rect::new(328.0, 26.0, 40.0, 20.0)
    );

    let door = Rect::new(380.0, 388.0, 48.0, 72.0); // centered on (404, 424)
    for edge in [RoomEdge::Left, RoomEdge::Bottom] {
        assert_eq!(
            label_rect(edge, true, door, text),
            Rect::new(384.0, 414.0, 40.0, 20.0),
            "a door ignores the edge"
        );
    }
}

#[test]
fn a_label_never_overlaps_its_band() {
    let text = Vec2::new(40.0, 20.0);
    let zone = Rect::new(500.0, 300.0, 32.0, 96.0);
    for edge in [
        RoomEdge::Left,
        RoomEdge::Right,
        RoomEdge::Top,
        RoomEdge::Bottom,
    ] {
        let label = label_rect(edge, false, zone, text);
        assert!(!label.overlaps(&zone), "{edge:?}");
    }
}
