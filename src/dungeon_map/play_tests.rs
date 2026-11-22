//! What the map screen in play draws: seen rooms, the rooms next to them, nothing else.
use super::*;
use crate::room::RoomEdge;

fn id(n: usize) -> RoomId {
    RoomId::Custom(n)
}

/// 0 - 1 - 2 - 3 in a row, every exit with one back, and room 1's way on to room 2 locked by
/// Key 4, and room 2 has a one-way drop to room 4.
fn dungeon() -> MapLayout {
    let exit = |edge, target: usize, lock| MapExit {
        edge: Some(edge),
        target: id(target),
        lock,
    };
    let room = |n: usize, exits: Vec<MapExit>| MapRoomInput {
        id: id(n),
        name: format!("Room {n}"),
        cell: None,
        exits,
    };
    MapLayout::build(&[
        room(0, vec![exit(RoomEdge::Right, 1, None)]),
        room(
            1,
            vec![
                exit(RoomEdge::Left, 0, None),
                exit(RoomEdge::Right, 2, Some(4)),
            ],
        ),
        room(
            2,
            vec![
                exit(RoomEdge::Left, 1, None),
                exit(RoomEdge::Right, 3, None),
                exit(RoomEdge::Bottom, 4, None),
            ],
        ),
        room(3, vec![exit(RoomEdge::Left, 2, None)]),
        room(4, vec![]),
    ])
}

fn seen(ids: &[usize]) -> MapDiscovery {
    let mut seen = MapDiscovery::default();
    for n in ids {
        seen.record(id(*n));
    }
    seen
}

fn drawn(map: &PlayMap) -> Vec<usize> {
    map.rooms
        .iter()
        .map(|room| match room.id {
            RoomId::Custom(n) => n,
            other => panic!("unexpected {other:?}"),
        })
        .collect()
}

#[test]
fn nothing_seen_draws_nothing() {
    let map = PlayMap::new(&dungeon(), &MapDiscovery::default(), id(0));
    assert_eq!(map, PlayMap::default());
    assert!(map.cells().is_empty());
}

#[test]
fn the_first_room_shows_filled_with_its_name_and_the_next_one_as_an_empty_box() {
    let map = PlayMap::new(&dungeon(), &seen(&[0]), id(0));
    assert_eq!(drawn(&map), [0, 1], "rooms 2, 3 and 4 are not drawn at all");
    let first = &map.rooms[0];
    assert!(first.visited && first.current);
    assert_eq!(first.name.as_deref(), Some("Room 0"));
    let next = &map.rooms[1];
    assert!(!next.visited && !next.current);
    assert_eq!(
        next.name, None,
        "an empty box gives its name away to no one"
    );
}

#[test]
fn walking_on_fills_the_room_and_reveals_the_next_boxes() {
    let map = PlayMap::new(&dungeon(), &seen(&[0, 1]), id(1));
    assert_eq!(drawn(&map), [0, 1, 2]);
    let visited: Vec<bool> = map.rooms.iter().map(|room| room.visited).collect();
    assert_eq!(visited, [true, true, false]);
    let current: Vec<bool> = map.rooms.iter().map(|room| room.current).collect();
    assert_eq!(
        current,
        [false, true, false],
        "only the room the player is in is marked"
    );
}

#[test]
fn a_room_seen_earlier_stays_filled_after_the_player_walks_back() {
    let map = PlayMap::new(&dungeon(), &seen(&[0, 1, 2]), id(0));
    assert_eq!(drawn(&map), [0, 1, 2, 3, 4]);
    assert!(map.rooms[2].visited && !map.rooms[2].current);
    assert!(map.rooms[0].current);
}

#[test]
fn the_links_are_the_exits_of_seen_rooms_only() {
    let map = PlayMap::new(&dungeon(), &seen(&[0, 1]), id(1));
    let pairs: Vec<(RoomId, RoomId)> = map.links.iter().map(|l| (l.from, l.to)).collect();
    assert_eq!(pairs, [(id(0), id(1)), (id(1), id(0)), (id(1), id(2))]);
    assert!(
        !pairs.iter().any(|(from, _)| *from == id(2)),
        "room 2 is not seen, so its exits are not known"
    );
}

#[test]
fn the_key_number_of_a_locked_exit_shows_once_its_room_is_seen() {
    let locked = |map: &PlayMap| {
        map.links
            .iter()
            .find(|link| link.from == id(1) && link.to == id(2))
            .map(|link| link.lock)
    };
    assert_eq!(
        locked(&PlayMap::new(&dungeon(), &seen(&[0]), id(0))),
        None,
        "room 1 unseen"
    );
    assert_eq!(
        locked(&PlayMap::new(&dungeon(), &seen(&[0, 1]), id(1))),
        Some(Some(4))
    );
}

#[test]
fn a_one_way_exit_is_marked_on_its_link() {
    let map = PlayMap::new(&dungeon(), &seen(&[0, 1, 2]), id(2));
    let drop = map.links.iter().find(|link| link.to == id(4)).unwrap();
    assert!(drop.one_way);
    let back = map
        .links
        .iter()
        .find(|link| link.to == id(1) && link.from == id(2))
        .unwrap();
    assert!(!back.one_way);
}

#[test]
fn the_cells_are_those_of_the_rooms_drawn() {
    let layout = dungeon();
    let map = PlayMap::new(&layout, &seen(&[0, 1]), id(1));
    let expected: Vec<Cell> = [0, 1, 2].map(|n| layout.room(id(n)).unwrap().cell).to_vec();
    assert_eq!(map.cells(), expected);
}

#[test]
fn a_seen_room_that_is_not_on_the_map_is_ignored() {
    let map = PlayMap::new(&dungeon(), &seen(&[0, 99]), id(0));
    assert_eq!(drawn(&map), [0, 1]);
}
