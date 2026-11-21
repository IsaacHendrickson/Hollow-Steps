//! The dungeon checker on small dungeons built in code. Rooms are the first fixture room with
//! their exits and items replaced; the checker never looks at geometry, so exits only need
//! different offsets to be different exits.
use super::*;
use crate::editor::{ExitPlacement, ItemPlacement};
use crate::gates::Gate;
use crate::room::{test_rooms, RoomEdge};

const HUB: RoomId = RoomId::Custom(0);
const FIRST: RoomId = RoomId::First;
const SECOND: RoomId = RoomId::Second;
const BOSS: RoomId = RoomId::Boss;
const SPARE: RoomId = RoomId::Custom(1);

fn exit(offset: f32, to: RoomId) -> ExitPlacement {
    ExitPlacement::edge_exit(RoomEdge::Right, offset, 96.0, to, (160.0, 376.0))
}

fn locked(offset: f32, to: RoomId, key: u8) -> ExitPlacement {
    ExitPlacement {
        locked_by: Some(key),
        ..exit(offset, to)
    }
}

fn room(id: RoomId, exits: Vec<ExitPlacement>, items: &[Item]) -> (RoomId, RoomLayout) {
    let mut layout = RoomLayout::from_room(&test_rooms()[0]);
    layout.exits = exits;
    layout.items = items
        .iter()
        .map(|item| ItemPlacement {
            item: *item,
            position: (100.0, 440.0),
        })
        .collect();
    (id, layout)
}

fn hub_config() -> RunConfig {
    RunConfig::from_ron("(hub_room: Custom(0), start_room: First, final_boss_room: Boss)").unwrap()
}

fn plain_config() -> RunConfig {
    RunConfig::from_ron("(start_room: First, final_boss_room: Boss)").unwrap()
}

/// The hub, with its one exit into First, then the rooms given.
fn dungeon(rooms: Vec<(RoomId, RoomLayout)>) -> Vec<(RoomId, RoomLayout)> {
    let mut all = vec![room(HUB, vec![exit(100.0, FIRST)], &[])];
    all.extend(rooms);
    all
}

fn gate_from(room: RoomId, offset: f32, to: &ExitPlacement) -> Gate {
    Gate {
        from: Entrance::Arrival {
            room,
            exit: ExitRef::Edge {
                edge: RoomEdge::Right,
                offset,
            },
        },
        exit: ExitRef::of(to),
        needs: vec![Ability::Dash],
    }
}

fn problems(rooms: &[(RoomId, RoomLayout)]) -> Vec<Problem> {
    check(rooms, &hub_config())
}

fn texts(problems: &[Problem]) -> Vec<String> {
    problems.iter().map(Problem::text).collect()
}

/// First (with a key and the Dash), Second (a door locked by that key to the boss, gated by the
/// Dash, and an exit that leads back into itself) and the boss.
fn valid() -> Vec<(RoomId, RoomLayout)> {
    let to_boss = locked(200.0, BOSS, 1);
    let mut second = room(
        SECOND,
        vec![
            exit(100.0, FIRST),
            to_boss.clone(),
            exit(300.0, SECOND), // an exit into its own room
        ],
        &[],
    );
    second.1.gates = vec![gate_from(FIRST, 100.0, &to_boss)];
    dungeon(vec![
        room(
            FIRST,
            vec![exit(100.0, SECOND), exit(300.0, HUB)],
            &[Item::Key(1), Item::Dash],
        ),
        second,
        room(BOSS, vec![exit(100.0, SECOND)], &[]),
    ])
}

// ---- a dungeon that works ----

#[test]
fn a_valid_dungeon_has_no_problems_with_a_hub_and_without_one() {
    assert_eq!(problems(&valid()), []);
    assert_eq!(check(&valid(), &plain_config()), []);
}

#[test]
fn the_hub_is_never_reported_though_nothing_leads_into_it() {
    let mut rooms = valid();
    rooms[1].1.exits.pop(); // First no longer has its way back to the hub
    assert_eq!(problems(&rooms), []);
}

#[test]
fn the_check_is_the_same_every_time() {
    let mut rooms = valid();
    rooms.push(room(SPARE, vec![], &[]));
    assert_eq!(problems(&rooms), problems(&rooms));
}

// ---- rooms that cannot be reached ----

#[test]
fn a_room_nothing_leads_to_is_reported_by_its_display_name() {
    let mut rooms = valid();
    rooms.push(room(SPARE, vec![exit(100.0, FIRST)], &[]));
    assert_eq!(
        problems(&rooms),
        [Problem::Unreachable {
            room: "Room 2".into()
        }]
    );
    rooms[4].1.name = Some("Cellar".into());
    assert_eq!(
        texts(&problems(&rooms)),
        ["Cellar can never be reached from the start"]
    );
}

#[test]
fn an_unreachable_final_boss_room_is_its_own_problem_and_not_listed_twice() {
    let mut rooms = valid();
    rooms[2].1.exits.retain(|exit| exit.target_room != BOSS);
    rooms[2].1.gates.clear();
    assert_eq!(
        problems(&rooms),
        [Problem::BossUnreachable {
            room: "Boss".into()
        }]
    );
}

#[test]
fn a_hub_with_no_exit_to_the_start_room_is_reported_and_the_check_goes_on() {
    let mut rooms = valid();
    rooms[0].1.exits = vec![exit(100.0, SECOND)];
    assert_eq!(
        problems(&rooms),
        [Problem::HubDoesNotLeadToStart {
            hub: "Room 1".into(),
            start: "First".into(),
        }]
    );
}

#[test]
fn a_run_rule_that_names_a_room_that_does_not_exist_is_reported_and_nothing_else_is() {
    let rooms = valid();
    let config =
        RunConfig::from_ron("(hub_room: Custom(0), start_room: Custom(7), final_boss_room: Boss)")
            .unwrap();
    assert_eq!(
        check(&rooms, &config),
        [Problem::MissingRoom {
            field: "start_room",
            room: RoomId::Custom(7)
        }]
    );
}

// ---- locks ----

#[test]
fn a_lock_whose_key_exists_nowhere_is_reported() {
    let mut rooms = valid();
    rooms[1]
        .1
        .items
        .retain(|placed| placed.item != Item::Key(1));
    let found = problems(&rooms);
    assert!(
        found.contains(&Problem::KeyUnreachable {
            room: "Second".into(),
            exit: "right edge exit at 200".into(),
            key: 1
        }),
        "{found:?}"
    );
}

#[test]
fn a_key_in_a_room_the_player_cannot_get_to_does_not_count() {
    let mut rooms = valid();
    rooms[1]
        .1
        .items
        .retain(|placed| placed.item != Item::Key(1));
    rooms.push(room(SPARE, vec![exit(100.0, FIRST)], &[Item::Key(1)]));
    let found = problems(&rooms);
    assert!(
        found
            .iter()
            .any(|p| matches!(p, Problem::KeyUnreachable { key: 1, .. })),
        "{found:?}"
    );
    assert!(found
        .iter()
        .any(|p| matches!(p, Problem::Unreachable { .. })));
}

#[test]
fn a_key_that_only_lies_behind_its_own_lock_is_told_apart() {
    let mut rooms = valid();
    rooms[1]
        .1
        .items
        .retain(|placed| placed.item != Item::Key(1));
    rooms[1].1.exits[0] = locked(100.0, SECOND, 1); // First's way into Second needs Key 1
    rooms[2].1.items.push(ItemPlacement {
        item: Item::Key(1),
        position: (100.0, 440.0),
    });
    let found = problems(&rooms);
    assert!(
        found.contains(&Problem::KeyBehindItsOwnLock {
            room: "First".into(),
            exit: "right edge exit at 100".into(),
            key: 1
        }),
        "{found:?}"
    );
    assert!(!found
        .iter()
        .any(|p| matches!(p, Problem::KeyUnreachable { .. })));
}

#[test]
fn a_lock_in_a_room_that_cannot_be_reached_is_left_to_that_room() {
    let mut rooms = valid();
    rooms.push(room(SPARE, vec![locked(100.0, FIRST, 5)], &[]));
    let found = problems(&rooms);
    assert!(found.contains(&Problem::Unreachable {
        room: "Room 2".into()
    }));
    assert!(!found
        .iter()
        .any(|p| matches!(p, Problem::KeyUnreachable { .. })));
}

#[test]
fn a_key_is_used_up_by_one_door() {
    let mut rooms = valid();
    // A second door that Key 1 locks, with only the one key placed.
    rooms[1].1.exits.push(locked(500.0, SECOND, 1));
    assert_eq!(
        problems(&rooms),
        [Problem::TooManyLocks {
            key: 1,
            locks: 2,
            keys: 1
        }]
    );
    assert_eq!(
        texts(&problems(&rooms)),
        ["Key 1 is used up by the exit it opens, but 2 exits are locked by it and 1 is placed"]
    );
    // One key per door is fine, and so is a different number for the other door.
    rooms[1].1.items.push(ItemPlacement {
        item: Item::Key(1),
        position: (200.0, 440.0),
    });
    assert_eq!(problems(&rooms), []);
}

// ---- abilities ----

#[test]
fn an_ability_pickup_behind_its_own_gate_is_reported() {
    // First's way into Second needs the Dash, which only lies in Second. The gate never opens.
    let to_second = exit(100.0, SECOND);
    let mut first = room(FIRST, vec![to_second.clone(), exit(300.0, BOSS)], &[]);
    first.1.gates = vec![Gate {
        from: Entrance::Arrival {
            room: HUB,
            exit: ExitRef::Edge {
                edge: RoomEdge::Right,
                offset: 100.0,
            },
        },
        exit: ExitRef::of(&to_second),
        needs: vec![Ability::Dash],
    }];
    let rooms = dungeon(vec![
        first,
        room(SECOND, vec![exit(100.0, FIRST)], &[Item::Dash]),
        // The boss room leaves the run: coming back from it would be another way into First,
        // and the gate is only on the way in from the hub.
        room(BOSS, vec![exit(100.0, HUB)], &[]),
    ]);
    assert_eq!(
        problems(&rooms),
        [
            Problem::Unreachable {
                room: "Second".into()
            },
            Problem::AbilityUnobtainable {
                room: "First".into(),
                exit: "right edge exit at 100".into(),
                needs: vec![Ability::Dash],
            },
        ]
    );
    assert_eq!(
        problems(&rooms)[1].text(),
        "First: the right edge exit at 100 needs Dash, which cannot be picked up before it"
    );
}

#[test]
fn a_gated_exit_that_another_way_in_lets_the_player_use_is_not_reported() {
    // Starting in First, the exit to the boss needs Dash from the spawn, but coming back from
    // Second it is free. No Dash lies anywhere.
    let to_boss = exit(300.0, BOSS);
    let mut first = room(FIRST, vec![exit(100.0, SECOND), to_boss.clone()], &[]);
    first.1.gates = vec![Gate {
        from: Entrance::Spawn,
        exit: ExitRef::of(&to_boss),
        needs: vec![Ability::Dash],
    }];
    let rooms = vec![
        first,
        room(SECOND, vec![exit(100.0, FIRST)], &[]),
        room(BOSS, vec![exit(100.0, FIRST)], &[]),
    ];
    assert_eq!(check(&rooms, &plain_config()), []);
}

#[test]
fn a_gate_blocks_one_direction_only() {
    // Second is a pit: going in from First is free, climbing back out needs the Dash.
    let way_out = exit(100.0, FIRST);
    let mut second = room(SECOND, vec![way_out.clone()], &[]);
    second.1.gates = vec![gate_from(FIRST, 100.0, &way_out)];
    let mut rooms = dungeon(vec![
        room(FIRST, vec![exit(100.0, SECOND), exit(300.0, BOSS)], &[]),
        second,
        room(BOSS, vec![exit(100.0, FIRST)], &[]),
    ]);

    // No Dash anywhere: getting in works (Second is reachable) and getting out does not.
    let found = problems(&rooms);
    assert!(
        !found
            .iter()
            .any(|p| matches!(p, Problem::Unreachable { .. })),
        "{found:?}"
    );
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found.contains(&Problem::AbilityUnobtainable {
        room: "Second".into(),
        exit: "right edge exit at 100".into(),
        needs: vec![Ability::Dash],
    }));
    assert!(found.contains(&Problem::Trap {
        room: "Second".into(),
        entrance: "the arrival from First (right edge exit at 100)".into(),
        holding: vec![],
    }));

    // The Dash lying in the pit itself is picked up before the climb.
    rooms[2].1.items.push(ItemPlacement {
        item: Item::Dash,
        position: (100.0, 440.0),
    });
    assert_eq!(problems(&rooms), []);
}

// ---- traps ----

/// First, with a free way to the boss, a free way into Second and one into a room that holds the
/// Dash; Second's only exit needs the Dash from the arrival from First.
fn pit_before_the_dash(gated: bool) -> Vec<(RoomId, RoomLayout)> {
    let way_out = exit(100.0, FIRST);
    let mut second = room(SECOND, vec![way_out.clone()], &[]);
    if gated {
        second.1.gates = vec![gate_from(FIRST, 100.0, &way_out)];
    }
    dungeon(vec![
        room(
            FIRST,
            vec![exit(100.0, SECOND), exit(300.0, SPARE), exit(500.0, BOSS)],
            &[],
        ),
        second,
        room(BOSS, vec![exit(100.0, FIRST)], &[]),
        room(SPARE, vec![exit(100.0, FIRST)], &[Item::Dash]),
    ])
}

#[test]
fn an_entrance_reached_before_the_ability_that_gets_out_is_a_trap() {
    // The Dash can be had (in Spare), so the gate opens for a player who found it first, but
    // arriving in Second without it leaves no way out.
    assert_eq!(
        problems(&pit_before_the_dash(true)),
        [Problem::Trap {
            room: "Second".into(),
            entrance: "the arrival from First (right edge exit at 100)".into(),
            holding: vec![],
        }]
    );
    assert_eq!(
        problems(&pit_before_the_dash(true))[0].text(),
        "Second: arriving at the arrival from First (right edge exit at 100) with no abilities, \
         no exit can be taken"
    );
}

#[test]
fn a_dead_end_with_its_way_back_open_is_not_a_trap() {
    assert_eq!(problems(&pit_before_the_dash(false)), []);
}

#[test]
fn a_room_with_no_exit_is_a_trap_unless_it_is_the_final_boss_room() {
    let mut rooms = pit_before_the_dash(false);
    rooms[2].1.exits.clear(); // Second: no way out at all
    rooms[3].1.exits.clear(); // the boss room: the run ends when the boss falls
    let found = problems(&rooms);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        matches!(&found[0], Problem::Trap { room, .. } if room == "Second"),
        "{found:?}"
    );
}

#[test]
fn an_exit_into_the_hub_is_a_way_out() {
    let mut rooms = pit_before_the_dash(false);
    rooms[2].1.exits = vec![exit(100.0, HUB)]; // Second: the only exit leaves the run
    assert_eq!(problems(&rooms), []);
}

// ---- gates that name no entrance ----

#[test]
fn a_gate_naming_an_entrance_the_room_does_not_have_is_reported_and_changes_nothing() {
    let mut rooms = valid();
    let way_out = rooms[2].1.exits[0].clone();
    rooms[2].1.gates.push(gate_from(FIRST, 9.0, &way_out));
    assert_eq!(
        problems(&rooms),
        [Problem::OrphanGate {
            room: "Second".into(),
            from: "the arrival from First (right edge exit at 9)".into(),
            exit: "right edge exit at 100".into(),
        }]
    );
}

// ---- the sentences ----

#[test]
fn every_problem_reads_as_a_sentence_that_names_what_to_look_at() {
    let cases = [
        (
            Problem::MissingRoom {
                field: "hub_room",
                room: RoomId::Custom(3),
            },
            "run.ron: hub_room Custom(3) is not a room",
        ),
        (
            Problem::HubDoesNotLeadToStart {
                hub: "Hub".into(),
                start: "First".into(),
            },
            "The hub (Hub) has no exit to the start room (First); checking from the start room's spawn",
        ),
        (
            Problem::Unreachable { room: "Cellar".into() },
            "Cellar can never be reached from the start",
        ),
        (
            Problem::BossUnreachable { room: "Boss".into() },
            "The final boss room, Boss, can never be reached",
        ),
        (
            Problem::KeyUnreachable {
                room: "Cellar".into(),
                exit: "left edge exit at 40".into(),
                key: 3,
            },
            "Cellar: the left edge exit at 40 is locked by Key 3, and no Key 3 can be reached",
        ),
        (
            Problem::KeyBehindItsOwnLock {
                room: "Cellar".into(),
                exit: "door at (600, 300)".into(),
                key: 2,
            },
            "Cellar: the door at (600, 300) is locked by Key 2, and the only Key 2 lies behind that same lock",
        ),
        (
            Problem::TooManyLocks {
                key: 5,
                locks: 3,
                keys: 2,
            },
            "Key 5 is used up by the exit it opens, but 3 exits are locked by it and 2 are placed",
        ),
    ];
    for (problem, text) in cases {
        assert_eq!(problem.text(), text);
    }
}
