//! The map's rules: where rooms are placed, what is reported, and the room file's `map_cell`.
//! The discovery rules through `GameState` are in `game::map_tests`.
use macroquad::math::{Rect, Vec2};

use super::place::{nearest_free, Wide};
use super::*;
use crate::editor::{RoomDraft, RoomLayout};
use crate::room::{test_rooms, Exit, RoomEnemies};

fn id(n: usize) -> RoomId {
    RoomId::Custom(n)
}

fn room(n: usize, cell: Option<Cell>, exits: &[(RoomEdge, usize)]) -> MapRoomInput {
    MapRoomInput {
        id: id(n),
        name: format!("R{n}"),
        cell,
        exits: exits
            .iter()
            .map(|&(edge, target)| MapExit {
                edge: Some(edge),
                target: id(target),
                lock: None,
            })
            .collect(),
    }
}

fn cell_of(map: &MapLayout, n: usize) -> Cell {
    map.room(id(n)).expect("room is on the map").cell
}

fn source_of(map: &MapLayout, n: usize) -> CellSource {
    map.room(id(n)).expect("room is on the map").source
}

fn link(map: &MapLayout, from: usize, to: usize) -> MapLink {
    *map.links
        .iter()
        .find(|link| link.from == id(from) && link.to == id(to))
        .expect("link exists")
}

// ---- where rooms go ---------------------------------------------------------------------------

#[test]
fn a_room_with_a_cell_keeps_it() {
    let map = MapLayout::build(&[
        room(0, Some((5, -3)), &[(RoomEdge::Right, 1)]),
        room(1, None, &[]),
    ]);
    assert_eq!(cell_of(&map, 0), (5, -3));
    assert_eq!(source_of(&map, 0), CellSource::Explicit);
    assert_eq!(cell_of(&map, 1), (6, -3), "the neighbor follows its exit");
}

#[test]
fn each_exit_side_puts_the_neighbor_next_to_it() {
    for (edge, expected) in [
        (RoomEdge::Right, (11, 10)),
        (RoomEdge::Left, (9, 10)),
        (RoomEdge::Bottom, (10, 11)),
        (RoomEdge::Top, (10, 9)),
    ] {
        let map = MapLayout::build(&[room(0, Some((10, 10)), &[(edge, 1)]), room(1, None, &[])]);
        assert_eq!(cell_of(&map, 1), expected, "{edge:?}");
        assert_eq!(source_of(&map, 1), CellSource::Walked, "{edge:?}");
    }
}

#[test]
fn a_chain_of_exits_is_followed_room_by_room() {
    let map = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1)]),
        room(1, None, &[(RoomEdge::Right, 2)]),
        room(2, None, &[(RoomEdge::Bottom, 3)]),
        room(3, None, &[]),
    ]);
    assert_eq!(cell_of(&map, 0), (0, 0), "the lowest id starts the walk");
    assert_eq!(cell_of(&map, 1), (1, 0));
    assert_eq!(cell_of(&map, 2), (2, 0));
    assert_eq!(cell_of(&map, 3), (2, 1));
}

#[test]
fn a_taken_cell_moves_the_room_to_the_nearest_free_one() {
    let map = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1), (RoomEdge::Right, 2)]),
        room(1, None, &[]),
        room(2, None, &[]),
    ]);
    assert_eq!(cell_of(&map, 1), (1, 0));
    assert_eq!(
        cell_of(&map, 2),
        (1, -1),
        "(1, 0) is taken, so the cell above it"
    );
}

#[test]
fn a_cell_named_by_a_room_is_reserved_before_any_walk() {
    // Room 1 would land on (1, 0), which room 2 names for itself.
    let map = MapLayout::build(&[
        room(0, Some((0, 0)), &[(RoomEdge::Right, 1)]),
        room(1, None, &[]),
        room(2, Some((1, 0)), &[]),
    ]);
    assert_eq!(cell_of(&map, 2), (1, 0));
    assert_eq!(cell_of(&map, 1), (1, -1));
}

#[test]
fn nearest_free_tries_equal_distances_in_one_fixed_order() {
    let mut taken: std::collections::HashSet<Wide> = [(0, 0)].into();
    for expected in [(0, -1), (-1, 0), (1, 0), (0, 1), (0, -2)] {
        let spot = nearest_free((0, 0), &taken);
        assert_eq!(spot, expected);
        taken.insert(spot);
    }
    assert_eq!(
        nearest_free((9, 9), &taken),
        (9, 9),
        "a free cell stays where it is"
    );
}

#[test]
fn a_door_says_nothing_about_direction() {
    let mut start = room(0, Some((0, 0)), &[]);
    start.exits = vec![
        MapExit {
            edge: None,
            target: id(1),
            lock: None,
        },
        MapExit {
            edge: None,
            target: id(2),
            lock: None,
        },
    ];
    let map = MapLayout::build(&[start, room(1, None, &[]), room(2, None, &[])]);
    assert_eq!(cell_of(&map, 1), (0, -1));
    assert_eq!(cell_of(&map, 2), (-1, 0));
}

#[test]
fn with_no_cell_anywhere_the_lowest_id_is_the_origin() {
    let first = MapRoomInput {
        id: RoomId::First,
        name: "First".into(),
        cell: None,
        exits: vec![MapExit {
            edge: Some(RoomEdge::Right),
            target: id(5),
            lock: None,
        }],
    };
    // Listed highest id first on purpose.
    let map = MapLayout::build(&[room(5, None, &[]), first]);
    assert_eq!(map.room(RoomId::First).unwrap().cell, (0, 0));
    assert_eq!(cell_of(&map, 5), (1, 0));
}

#[test]
fn rooms_no_walk_reaches_go_in_a_row_below_everything_in_id_order() {
    let map = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1)]),
        room(1, None, &[]),
        room(2, None, &[]),
        // An exit into the map is not a way in from it.
        room(3, None, &[(RoomEdge::Left, 0)]),
        room(4, None, &[]),
    ]);
    assert_eq!(cell_of(&map, 0), (0, 0));
    assert_eq!(cell_of(&map, 1), (1, 0));
    assert_eq!(
        [2, 3, 4].map(|n| (cell_of(&map, n), source_of(&map, n))),
        [
            ((0, 1), CellSource::Row),
            ((1, 1), CellSource::Row),
            ((2, 1), CellSource::Row)
        ]
    );
}

#[test]
fn the_row_starts_under_the_leftmost_room_and_clears_the_lowest_one() {
    let map = MapLayout::build(&[
        room(
            0,
            Some((5, 5)),
            &[(RoomEdge::Left, 1), (RoomEdge::Bottom, 2)],
        ),
        room(1, None, &[]),
        room(2, None, &[]),
        room(3, None, &[]),
    ]);
    assert_eq!(cell_of(&map, 1), (4, 5));
    assert_eq!(cell_of(&map, 2), (5, 6));
    assert_eq!(
        cell_of(&map, 3),
        (4, 7),
        "left edge is x = 4, below the lowest row (6)"
    );
}

// ---- conflicts --------------------------------------------------------------------------------

#[test]
fn two_rooms_with_one_cell_are_reported_and_the_second_is_moved() {
    let map = MapLayout::build(&[room(0, Some((2, 2)), &[]), room(1, Some((2, 2)), &[])]);
    assert_eq!(
        map.conflicts,
        [MapConflict {
            kept: id(0),
            moved: id(1),
            cell: (2, 2)
        }]
    );
    assert_eq!(
        (cell_of(&map, 0), source_of(&map, 0)),
        ((2, 2), CellSource::Explicit)
    );
    assert_eq!(
        (cell_of(&map, 1), source_of(&map, 1)),
        ((2, 1), CellSource::Conflict)
    );
    assert!(map.in_conflict(id(0)) && map.in_conflict(id(1)));
}

#[test]
fn a_moved_room_never_takes_a_cell_another_room_named() {
    let map = MapLayout::build(&[
        room(0, Some((2, 2)), &[]),
        room(1, Some((2, 2)), &[]),
        room(2, Some((2, 1)), &[]),
    ]);
    assert_eq!(
        cell_of(&map, 2),
        (2, 1),
        "named cells win over a moved room"
    );
    assert_eq!(cell_of(&map, 1), (1, 2));
    assert_eq!(map.conflicts.len(), 1);
}

#[test]
fn three_rooms_on_one_cell_make_two_conflicts_with_the_first() {
    let map = MapLayout::build(&[
        room(0, Some((0, 0)), &[]),
        room(1, Some((0, 0)), &[]),
        room(2, Some((0, 0)), &[]),
    ]);
    assert_eq!(map.conflicts.len(), 2);
    assert!(map.conflicts.iter().all(|conflict| conflict.kept == id(0)));
    assert_eq!(
        [cell_of(&map, 1), cell_of(&map, 2)],
        [(0, -1), (-1, 0)],
        "each moved room takes its own free cell"
    );
}

#[test]
fn a_moved_room_still_places_its_neighbors_from_where_it_ended_up() {
    let map = MapLayout::build(&[
        room(0, Some((2, 2)), &[]),
        room(1, Some((2, 2)), &[(RoomEdge::Right, 3)]),
        room(3, None, &[]),
    ]);
    assert_eq!(cell_of(&map, 1), (2, 1));
    assert_eq!(cell_of(&map, 3), (3, 1));
}

// ---- determinism ------------------------------------------------------------------------------

fn tangle() -> Vec<MapRoomInput> {
    vec![
        room(0, None, &[(RoomEdge::Right, 1), (RoomEdge::Bottom, 2)]),
        room(1, None, &[(RoomEdge::Left, 0), (RoomEdge::Right, 3)]),
        room(2, Some((1, 1)), &[(RoomEdge::Top, 0), (RoomEdge::Right, 4)]),
        room(3, None, &[(RoomEdge::Bottom, 4), (RoomEdge::Right, 5)]),
        room(4, None, &[(RoomEdge::Left, 2)]),
        room(5, None, &[]),
        room(6, Some((1, 1)), &[]),
        room(7, None, &[(RoomEdge::Top, 6)]),
    ]
}

#[test]
fn the_same_rooms_give_the_same_map_whatever_order_they_are_listed_in() {
    let rooms = tangle();
    let expected = MapLayout::build(&rooms);
    assert_eq!(MapLayout::build(&rooms), expected);
    let mut reversed = rooms.clone();
    reversed.reverse();
    assert_eq!(MapLayout::build(&reversed), expected);
    let mut rotated = rooms;
    rotated.rotate_left(3);
    assert_eq!(MapLayout::build(&rotated), expected);
}

#[test]
fn no_two_rooms_share_a_cell() {
    let map = MapLayout::build(&tangle());
    let mut cells: Vec<Cell> = map.rooms.iter().map(|room| room.cell).collect();
    cells.sort_unstable();
    cells.dedup();
    assert_eq!(cells.len(), map.rooms.len());
}

// ---- links ------------------------------------------------------------------------------------

#[test]
fn a_link_is_one_way_when_the_target_has_no_exit_back() {
    let map = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1), (RoomEdge::Bottom, 2)]),
        room(1, None, &[(RoomEdge::Left, 0)]),
        room(2, None, &[]),
    ]);
    assert!(!link(&map, 0, 1).one_way && !link(&map, 1, 0).one_way);
    assert!(link(&map, 0, 2).one_way);
    let two = link(&map, 0, 2);
    assert_eq!((two.from_cell, two.to_cell), ((0, 0), (0, 1)));
}

#[test]
fn a_link_carries_the_exits_key_number() {
    let mut start = room(0, None, &[(RoomEdge::Right, 1), (RoomEdge::Right, 2)]);
    start.exits[0].lock = Some(3);
    let map = MapLayout::build(&[start, room(1, None, &[]), room(2, None, &[])]);
    assert_eq!(link(&map, 0, 1).lock, Some(3));
    assert_eq!(link(&map, 0, 2).lock, None);
}

#[test]
fn an_entrance_is_an_exit_of_another_room() {
    let map = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1), (RoomEdge::Left, 0)]),
        room(1, None, &[]),
        room(2, None, &[]),
    ]);
    assert!(map.has_entrance(id(1)));
    assert!(!map.has_entrance(id(0)), "an exit into itself is no way in");
    assert!(!map.has_entrance(id(2)));
}

#[test]
fn odd_input_is_tolerated() {
    assert_eq!(MapLayout::build(&[]), MapLayout::default());
    // An exit to a room that is not listed draws nothing; a room listed twice counts once.
    let map = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 9)]),
        room(0, Some((4, 4)), &[]),
    ]);
    assert_eq!(map.rooms.len(), 1);
    assert!(map.links.is_empty());
}

// ---- the room file ----------------------------------------------------------------------------

fn layout() -> RoomLayout {
    let [room, _, _] = test_rooms();
    RoomLayout::from_room(&room)
}

fn written(layout: &RoomLayout) -> String {
    let document = RoomEnemies {
        enemies: Vec::new(),
        boss: None,
        layout: Some(layout.clone()),
    };
    ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default()).unwrap()
}

fn read(text: &str) -> RoomLayout {
    ron::from_str::<RoomEnemies>(text).unwrap().layout.unwrap()
}

#[test]
fn a_cell_survives_a_ron_round_trip() {
    let mut with = layout();
    with.map_cell = Some((3, -2));
    let text = written(&with);
    assert!(text.contains("map_cell"), "{text}");
    assert_eq!(read(&text), with);
}

#[test]
fn no_cell_leaves_the_file_as_it_was_and_reads_back_as_none() {
    let without = layout();
    assert_eq!(without.map_cell, None);
    let text = written(&without);
    assert!(!text.contains("map_cell"), "{text}");
    assert_eq!(read(&text), without);
}

#[test]
fn an_older_room_file_loads_without_a_cell() {
    let old = r#"(enemies: [], layout: Some((
        bounds: (0.0, 0.0, 960.0, 460.0),
        floor_y: 460.0,
        spawn: (180.0, 436.0),
        items: [],
        exits: [],
    )))"#;
    assert_eq!(read(old).map_cell, None);
}

#[test]
fn a_room_keeps_its_cell_through_the_layout() {
    let [mut room, mut other, _] = test_rooms();
    room.map_cell = Some((-7, 12));
    let layout = RoomLayout::from_room(&room);
    assert_eq!(layout.map_cell, Some((-7, 12)));
    layout.apply(&mut other).unwrap();
    assert_eq!(other.map_cell, Some((-7, 12)));
    assert_eq!(MapRoomInput::from_room(&other).cell, Some((-7, 12)));
}

#[test]
fn a_new_room_does_not_copy_its_templates_cell() {
    let mut template = layout();
    template.map_cell = Some((1, 1));
    assert_eq!(RoomDraft::new_room(&template).layout.map_cell, None);
}

#[test]
fn the_cell_must_be_from_minus_64_to_64_on_both_numbers() {
    for good in [(0, 0), (64, 64), (-64, -64), (-64, 64), (64, -64)] {
        assert_eq!(check_cell(good), Ok(()), "{good:?}");
        let mut layout = layout();
        layout.map_cell = Some(good);
        assert_eq!(layout.validate_geometry(), Ok(()), "{good:?}");
    }
    for bad in [
        (65, 0),
        (0, 65),
        (-65, 0),
        (0, -65),
        (i32::MAX, 0),
        (0, i32::MIN),
    ] {
        let error = check_cell(bad).unwrap_err();
        assert!(error.contains("map cell"), "{error}");
        let mut layout = layout();
        layout.map_cell = Some(bad);
        let error = layout.validate_geometry().unwrap_err();
        assert!(error.contains("map cell"), "{bad:?}: {error}");
    }
}

// ---- rooms as the map takes them --------------------------------------------------------------

#[test]
fn a_room_and_its_layout_give_the_same_map_input() {
    let [_, mut room, _] = test_rooms();
    room.name = Some("Cellar".into());
    room.exits[0].locked_by = Some(2);
    let from_room = MapRoomInput::from_room(&room);
    assert_eq!(from_room.name, "Cellar");
    assert_eq!(from_room.exits[0].lock, Some(2));
    assert_eq!(from_room.exits[0].edge, Some(room.exits[0].edge));
    assert_eq!(
        MapRoomInput::from_layout(room.id, &RoomLayout::from_room(&room)),
        from_room
    );
}

#[test]
fn an_unnamed_room_shows_its_default_name_and_a_door_has_no_side() {
    let [mut room, _, _] = test_rooms();
    room.exits[0] = Exit::doorway(
        Rect::new(100.0, 100.0, 48.0, 72.0),
        RoomId::Second,
        Vec2::ZERO,
    );
    let input = MapRoomInput::from_room(&room);
    assert_eq!(input.name, "First");
    assert_eq!(input.exits[0].edge, None);
    assert_eq!(input.exits[0].target, RoomId::Second);
}

// ---- discovery as a plain set -----------------------------------------------------------------

#[test]
fn discovery_records_once_and_clears() {
    let mut seen = MapDiscovery::default();
    assert!(seen.is_empty());
    assert!(seen.record(id(1)));
    assert!(!seen.record(id(1)), "a room is new only the first time");
    assert!(seen.record(RoomId::First));
    assert_eq!(
        (seen.len(), seen.contains(id(1)), seen.contains(id(2))),
        (2, true, false)
    );
    seen.clear();
    assert!(seen.is_empty() && !seen.contains(id(1)));
}
