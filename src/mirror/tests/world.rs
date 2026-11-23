//! `flip_world` on the fixture dungeon: links, arrivals and gates after the flip.
use crate::editor::RoomLayout;
use crate::exit_arrival::{arrival_error, arrival_next_to};
use crate::gates::{entrances_of, Entrance, ExitRef};
use crate::mirror::fixture::{self, BOSS, CLOSET, HUB, SECOND, START, TOWER};
use crate::mirror::{flip_world, mirror_layout, mirror_point};
use crate::room::{RoomEdge, RoomEnemies, RoomId};
use macroquad::math::Vec2;

fn flipped_with(good: bool, flipped: impl Fn(RoomId) -> bool) -> Vec<(RoomId, RoomEnemies)> {
    let mut documents = fixture::dungeon(good);
    flip_world(&mut documents, flipped);
    documents
}

fn all_but_hub(id: RoomId) -> bool {
    id != HUB
}

fn layout_of(documents: &[(RoomId, RoomEnemies)], id: RoomId) -> RoomLayout {
    let (_, document) = documents.iter().find(|(room, _)| *room == id).unwrap();
    document.layout.clone().unwrap()
}

#[test]
fn nothing_flipped_changes_nothing() {
    assert_eq!(flipped_with(true, |_| false), fixture::dungeon(true));
}

#[test]
fn every_flipped_room_is_mirrored_and_the_hub_is_not() {
    let (before, after) = (fixture::dungeon(true), flipped_with(true, all_but_hub));
    for id in [START, SECOND, BOSS, TOWER, CLOSET] {
        let (old, new) = (layout_of(&before, id), layout_of(&after, id));
        let mut expected = mirror_layout(&old);
        // Only the arrivals differ from a plain mirror: they land in other rooms.
        for (exit, moved) in expected.exits.iter_mut().zip(&new.exits) {
            exit.spawn = moved.spawn;
        }
        for (gate, moved) in expected.gates.iter_mut().zip(&new.gates) {
            gate.from = moved.from;
        }
        assert_eq!(new, expected, "{id:?}");
    }
    let (old, new) = (layout_of(&before, HUB), layout_of(&after, HUB));
    assert_eq!(new.spawn, old.spawn);
    assert_eq!(
        new.exits[0].edge, old.exits[0].edge,
        "the hub's door keeps its side"
    );
    assert_eq!(new.exits[0].offset, old.exits[0].offset);
}

#[test]
fn an_arrival_into_a_flipped_room_is_mirrored_in_that_rooms_bounds() {
    let (before, after) = (fixture::dungeon(true), flipped_with(true, all_but_hub));
    for (id, document) in &before {
        let (old_exits, new_exits) = (
            &layout_of(&before, *id).exits,
            &layout_of(&after, *id).exits,
        );
        assert_eq!(
            document.layout.as_ref().unwrap().exits.len(),
            new_exits.len()
        );
        for (old, new) in old_exits.iter().zip(new_exits) {
            let target = layout_of(&before, old.target_room);
            let expected = if all_but_hub(old.target_room) {
                mirror_point(target.bounds, old.spawn)
            } else {
                old.spawn
            };
            assert_eq!(new.spawn, expected, "{id:?} into {:?}", old.target_room);
        }
    }
}

#[test]
fn an_exit_into_its_own_room_is_mirrored_once() {
    let mut documents = fixture::dungeon(true);
    let layout = documents[2].1.layout.as_mut().unwrap();
    let mut itself = layout.exits[0].clone();
    itself.target_room = SECOND;
    itself.spawn = (180.0, 376.0);
    layout.exits.push(itself);
    let bounds = layout.bounds;
    flip_world(&mut documents, |id| id == SECOND);
    let moved = layout_of(&documents, SECOND);
    assert_eq!(moved.exits[3].spawn, mirror_point(bounds, (180.0, 376.0)));
}

/// Every set of rooms that could be flipped (the hub never is: 32 of them).
fn every_set() -> Vec<[bool; 5]> {
    (0..32u32)
        .map(|bits| std::array::from_fn(|room| bits & (1 << room) != 0))
        .collect()
}

fn flipped_in(set: [bool; 5]) -> impl Fn(RoomId) -> bool {
    move |id| {
        [START, SECOND, BOSS, TOWER, CLOSET]
            .iter()
            .position(|r| *r == id)
            .is_some_and(|i| set[i])
    }
}

#[test]
fn every_arrival_is_a_valid_spot_whichever_rooms_are_flipped() {
    for set in every_set() {
        let documents = flipped_with(true, flipped_in(set));
        for (id, _) in &documents {
            for exit in layout_of(&documents, *id).exits {
                let target = layout_of(&documents, exit.target_room);
                let error = arrival_error(&target, Vec2::from(exit.spawn));
                assert_eq!(error, None, "{set:?}: {id:?} into {:?}", exit.target_room);
            }
        }
    }
}

#[test]
fn every_arrival_is_still_the_spot_beside_the_exit_it_comes_from() {
    // The fixture puts each arrival where `arrival_next_to` says; after the flip it must be the
    // same spot in the mirrored room, not just any free one.
    for set in every_set() {
        let documents = flipped_with(true, flipped_in(set));
        for (id, _) in &documents {
            for exit in layout_of(&documents, *id).exits {
                let target = layout_of(&documents, exit.target_room);
                let back = target
                    .exits
                    .iter()
                    .position(|e| e.target_room == *id)
                    .unwrap();
                let beside = arrival_next_to(&target, back).unwrap();
                let (dx, dy) = (beside.x - exit.spawn.0, beside.y - exit.spawn.1);
                assert!(
                    dx.abs() < 1e-3 && dy.abs() < 1e-3,
                    "{set:?}: {id:?} into {:?}",
                    exit.target_room
                );
            }
        }
    }
}

#[test]
fn every_flipped_room_passes_the_geometry_check() {
    for set in every_set() {
        let documents = flipped_with(true, flipped_in(set));
        for (id, _) in &documents {
            let layout = layout_of(&documents, *id);
            layout
                .validate_geometry()
                .unwrap_or_else(|e| panic!("{set:?} {id:?}: {e}"));
        }
    }
}

#[test]
fn gates_and_entrances_still_agree_with_the_exits_whichever_rooms_are_flipped() {
    for set in every_set() {
        let documents = flipped_with(true, flipped_in(set));
        let catalog = fixture::catalog(&documents);
        for (id, layout) in &catalog {
            layout.validate_gates().unwrap();
            let entrances: Vec<Entrance> = entrances_of(*id, &catalog)
                .into_iter()
                .map(|(entrance, _)| entrance)
                .collect();
            for gate in &layout.gates {
                assert!(
                    entrances.contains(&gate.from),
                    "{set:?} {id:?}: {:?} is not an entrance",
                    gate.from
                );
            }
        }
    }
}

#[test]
fn a_gate_on_a_flipped_top_exit_looks_up_the_exit_length() {
    // TOWER's gate is on its bottom exit (offset 300, length 96, in a room 640 wide), and comes
    // from SECOND's top exit (offset 300, length 96, in a room 800 wide).
    let documents = flipped_with(true, all_but_hub);
    let gate = &layout_of(&documents, TOWER).gates[0];
    assert_eq!(
        gate.exit,
        ExitRef::Edge {
            edge: RoomEdge::Bottom,
            offset: 244.0
        }
    );
    assert_eq!(
        gate.from,
        Entrance::Arrival {
            room: SECOND,
            exit: ExitRef::Edge {
                edge: RoomEdge::Top,
                offset: 404.0
            }
        }
    );
}

#[test]
fn an_arrival_gate_follows_a_flipped_edge_exit_and_a_door() {
    let documents = flipped_with(true, all_but_hub);
    let gate = &layout_of(&documents, SECOND).gates[0];
    // START's right exit is now its left one at the same height; SECOND's locked exit, too.
    assert_eq!(
        gate.from,
        Entrance::Arrival {
            room: START,
            exit: ExitRef::Edge {
                edge: RoomEdge::Left,
                offset: 364.0
            }
        }
    );
    assert_eq!(
        gate.exit,
        ExitRef::Edge {
            edge: RoomEdge::Left,
            offset: 264.0
        }
    );
    // A door is named by its rectangle, which flips with the room.
    let door = layout_of(&documents, START).exits[2].door.unwrap();
    assert_eq!(
        ExitRef::of(&layout_of(&documents, START).exits[2]),
        ExitRef::Door {
            x: door.0,
            y: door.1,
            width: door.2,
            height: door.3
        }
    );
    assert_eq!(door, (212.0, 388.0, 48.0, 72.0));
}

#[test]
fn an_entrance_from_a_room_that_is_not_flipped_stays_as_it_was() {
    let documents = flipped_with(true, |id| id == SECOND);
    let gate = &layout_of(&documents, SECOND).gates[0];
    assert_eq!(
        gate.from,
        layout_of(&fixture::dungeon(true), SECOND).gates[0].from
    );
    assert_eq!(
        gate.exit,
        ExitRef::Edge {
            edge: RoomEdge::Left,
            offset: 264.0
        }
    );
    // And a gate in a room that is not flipped follows an exit that is: START flipped only.
    let documents = flipped_with(true, |id| id == START);
    let gate = &layout_of(&documents, SECOND).gates[0];
    assert_eq!(
        gate.from,
        Entrance::Arrival {
            room: START,
            exit: ExitRef::Edge {
                edge: RoomEdge::Left,
                offset: 364.0
            }
        }
    );
    assert_eq!(
        gate.exit,
        ExitRef::Edge {
            edge: RoomEdge::Right,
            offset: 264.0
        }
    );
}

#[test]
fn rooms_with_no_layout_are_left_alone() {
    let mut documents = fixture::dungeon(true);
    documents[3].1.layout = None; // BOSS
    let before = documents.clone();
    flip_world(&mut documents, all_but_hub);
    assert_eq!(documents[3], before[3]);
    // An arrival into it cannot be mirrored (its bounds are unknown) and stays.
    let into_boss = layout_of(&documents, SECOND)
        .exits
        .iter()
        .find(|e| e.target_room == BOSS)
        .unwrap()
        .spawn;
    let old = layout_of(&before, SECOND)
        .exits
        .iter()
        .find(|e| e.target_room == BOSS)
        .unwrap()
        .spawn;
    assert_eq!(into_boss, old);
}

#[test]
fn flipping_twice_gives_the_dungeon_back() {
    let mut documents = fixture::dungeon(true);
    flip_world(&mut documents, all_but_hub);
    flip_world(&mut documents, all_but_hub);
    assert_eq!(documents, fixture::dungeon(true));
}

#[test]
fn the_fixture_dungeon_is_valid_as_it_stands() {
    let catalog = fixture::catalog(&fixture::dungeon(true));
    for (id, layout) in &catalog {
        layout
            .validate_geometry()
            .unwrap_or_else(|e| panic!("{id:?}: {e}"));
        for exit in &layout.exits {
            let target = &catalog
                .iter()
                .find(|(t, _)| *t == exit.target_room)
                .unwrap()
                .1;
            assert_eq!(
                arrival_error(target, Vec2::from(exit.spawn)),
                None,
                "{id:?}"
            );
        }
    }
}
