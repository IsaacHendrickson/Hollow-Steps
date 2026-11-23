//! The pool rules on small pools built in code. Rooms are the first fixture room with their
//! exits, items, gates, mark and slots replaced; the rules never look at geometry.
use super::*;
use crate::ability::Ability;
use crate::editor::{ExitPlacement, ItemPlacement};
use crate::piece::{PickupSlot, SlotKind};
use crate::room::{test_rooms, RoomEdge};

const START: RoomId = RoomId::Custom(10);
const MIDDLE: RoomId = RoomId::Custom(11);
const BOSS: RoomId = RoomId::Custom(12);

fn exit(offset: f32, to: RoomId) -> ExitPlacement {
    ExitPlacement::edge_exit(RoomEdge::Right, offset, 96.0, to, (160.0, 376.0))
}

fn piece(
    id: RoomId,
    role: PieceRole,
    exits: Vec<ExitPlacement>,
    slots: &[SlotKind],
) -> (RoomId, RoomLayout) {
    let mut layout = RoomLayout::from_room(&test_rooms()[0]);
    layout.exits = exits;
    layout.items.clear();
    layout.gates.clear();
    layout.piece = Some(PieceInfo::new(role));
    layout.slots = slots
        .iter()
        .enumerate()
        .map(|(place, kind)| PickupSlot::new((100.0 + 40.0 * place as f32, 440.0), *kind))
        .collect();
    (id, layout)
}

/// A gate on the way in from `neighbour`'s exit at `offset` to `to`, needing the Dash.
fn gate_from(neighbour: RoomId, offset: f32, to: &ExitPlacement) -> Gate {
    Gate {
        from: Entrance::Arrival {
            room: neighbour,
            exit: ExitRef::Edge {
                edge: RoomEdge::Right,
                offset,
            },
        },
        exit: ExitRef::of(to),
        needs: vec![Ability::Dash],
    }
}

fn config() -> RunConfig {
    RunConfig::from_ron("(start_room: First, final_boss_room: Boss)").unwrap()
}

/// Start (with an Ability slot), Middle (gated: from the way in from Start, the exit to Boss
/// needs the Dash) and Boss: every rule holds.
fn clean() -> Vec<(RoomId, RoomLayout)> {
    let to_boss = exit(200.0, BOSS);
    let mut middle = piece(
        MIDDLE,
        PieceRole::Middle,
        vec![exit(100.0, START), to_boss.clone()],
        &[SlotKind::Key],
    );
    middle.1.gates = vec![gate_from(START, 100.0, &to_boss)];
    vec![
        piece(
            START,
            PieceRole::Start,
            vec![exit(100.0, MIDDLE)],
            &[SlotKind::Ability, SlotKind::Any],
        ),
        middle,
        piece(BOSS, PieceRole::Boss, vec![exit(100.0, MIDDLE)], &[]),
    ]
}

fn texts(problems: &[Problem]) -> Vec<String> {
    problems.iter().map(Problem::text).collect()
}

fn named(rooms: &mut [(RoomId, RoomLayout)], id: RoomId, name: &str) {
    let found = rooms.iter_mut().find(|(room, _)| *room == id).unwrap();
    found.1.name = Some(name.to_owned());
}

fn layout_of(rooms: &mut [(RoomId, RoomLayout)], id: RoomId) -> &mut RoomLayout {
    &mut rooms.iter_mut().find(|(room, _)| *room == id).unwrap().1
}

// ---- no pool, a clean pool ----

#[test]
fn no_pieces_reports_nothing_even_for_rooms_that_would_break_every_rule() {
    let mut rooms = clean();
    for (_, layout) in &mut rooms {
        layout.piece = None;
        layout.exits[0].locked_by = Some(1);
        layout.items.push(ItemPlacement {
            item: Item::Key(1),
            position: (100.0, 440.0),
        });
    }
    layout_of(&mut rooms, BOSS).exits.clear();
    assert_eq!(pool_check(&rooms, &config()), []);
    assert_eq!(pool_check(&[], &config()), []);
}

#[test]
fn a_clean_pool_reports_nothing() {
    assert_eq!(pool_check(&clean(), &config()), []);
}

#[test]
fn slots_of_a_room_that_is_not_a_piece_do_nothing() {
    let mut rooms = clean();
    let mut stray = piece(
        RoomId::Custom(20),
        PieceRole::Middle,
        vec![],
        &[SlotKind::Key],
    );
    stray.1.piece = None;
    rooms.push(stray);
    assert_eq!(pool_check(&rooms, &config()), []);
}

// ---- rule 1: a Start piece and a Boss piece ----

#[test]
fn a_pool_without_a_start_piece_says_so() {
    let mut rooms = clean();
    layout_of(&mut rooms, START).piece = Some(PieceInfo::new(PieceRole::Middle));
    let found = pool_check(&rooms, &config());
    assert_eq!(
        found,
        [Problem::NoPieceWithRole {
            role: PieceRole::Start
        }]
    );
    assert!(found[0].text().contains("Start"));
}

#[test]
fn a_pool_without_a_boss_piece_says_so() {
    let mut rooms = clean();
    layout_of(&mut rooms, BOSS).piece = Some(PieceInfo::new(PieceRole::Middle));
    assert_eq!(
        pool_check(&rooms, &config()),
        [Problem::NoPieceWithRole {
            role: PieceRole::Boss
        }]
    );
}

#[test]
fn a_pool_of_middle_pieces_only_needs_both_a_start_and_a_boss() {
    let mut rooms = clean();
    for (_, layout) in &mut rooms {
        layout.piece = Some(PieceInfo::new(PieceRole::Middle));
    }
    assert_eq!(
        pool_check(&rooms, &config()),
        [
            Problem::NoPieceWithRole {
                role: PieceRole::Start
            },
            Problem::NoPieceWithRole {
                role: PieceRole::Boss
            },
        ]
    );
}

#[test]
fn several_start_and_boss_pieces_are_fine() {
    let mut rooms = clean();
    rooms.push(piece(
        RoomId::Custom(13),
        PieceRole::Start,
        vec![exit(100.0, MIDDLE)],
        &[],
    ));
    rooms.push(piece(
        RoomId::Custom(14),
        PieceRole::Boss,
        vec![exit(100.0, MIDDLE)],
        &[],
    ));
    assert_eq!(pool_check(&rooms, &config()), []);
}

// ---- rule 2: every piece has an exit ----

#[test]
fn a_piece_with_no_exit_is_reported_by_name() {
    let mut rooms = clean();
    named(&mut rooms, BOSS, "Throne");
    layout_of(&mut rooms, BOSS).exits.clear();
    let found = pool_check(&rooms, &config());
    assert_eq!(
        found,
        [Problem::PieceWithoutExit {
            room: "Throne".to_owned()
        }]
    );
    assert!(found[0].text().starts_with("Throne:"));
}

#[test]
fn a_room_that_is_not_a_piece_may_have_no_exit() {
    let mut rooms = clean();
    let mut dead_end = piece(RoomId::Custom(20), PieceRole::Middle, vec![], &[]);
    dead_end.1.piece = None;
    rooms.push(dead_end);
    assert_eq!(pool_check(&rooms, &config()), []);
}

// ---- rule 3: no lock, no key ----

#[test]
fn a_locked_exit_in_a_piece_is_reported_with_the_exit_and_the_reason() {
    let mut rooms = clean();
    named(&mut rooms, MIDDLE, "Gallery");
    layout_of(&mut rooms, MIDDLE).exits[0].locked_by = Some(3);
    let found = pool_check(&rooms, &config());
    assert_eq!(
        found,
        [Problem::PieceLockedExit {
            room: "Gallery".to_owned(),
            exit: "right edge exit at 100".to_owned(),
            key: 3,
        }]
    );
    let text = found[0].text();
    assert!(text.starts_with("Gallery:"), "{text}");
    assert!(text.contains("Key 3"), "{text}");
    assert!(text.contains("pickup slots"), "{text}");
}

#[test]
fn a_key_item_in_a_piece_is_reported_with_the_reason() {
    let mut rooms = clean();
    named(&mut rooms, MIDDLE, "Gallery");
    layout_of(&mut rooms, MIDDLE).items.push(ItemPlacement {
        item: Item::Key(2),
        position: (100.0, 440.0),
    });
    let found = pool_check(&rooms, &config());
    assert_eq!(
        found,
        [Problem::PieceHoldsKey {
            room: "Gallery".to_owned(),
            key: 2
        }]
    );
    assert!(found[0].text().contains("generator"));
}

#[test]
fn two_keys_of_one_number_in_a_piece_are_one_problem_and_two_numbers_are_two() {
    let mut rooms = clean();
    let items = &mut layout_of(&mut rooms, MIDDLE).items;
    for key in [2, 2, 5] {
        items.push(ItemPlacement {
            item: Item::Key(key),
            position: (100.0, 440.0),
        });
    }
    let keys: Vec<u8> = pool_check(&rooms, &config())
        .iter()
        .filter_map(|problem| match problem {
            Problem::PieceHoldsKey { key, .. } => Some(*key),
            _ => None,
        })
        .collect();
    assert_eq!(keys, [2, 5]);
}

#[test]
fn other_items_in_a_piece_are_fine() {
    let mut rooms = clean();
    layout_of(&mut rooms, MIDDLE).items.push(ItemPlacement {
        item: Item::Dash,
        position: (100.0, 440.0),
    });
    assert_eq!(pool_check(&rooms, &config()), []);
}

// ---- rule 4: the rooms run.ron names ----

#[test]
fn the_start_room_must_be_a_start_piece_when_it_is_a_piece() {
    let rooms = clean();
    let config = RunConfig::from_ron("(start_room: Custom(11), final_boss_room: Boss)").unwrap();
    let found = pool_check(&rooms, &config);
    assert_eq!(
        found,
        [Problem::PieceRoleMismatch {
            room: room_title(MIDDLE, None),
            field: "start_room",
            role: PieceRole::Middle,
            expected: PieceRole::Start,
        }]
    );
    let text = found[0].text();
    assert!(text.contains("start_room") && text.contains("Middle") && text.contains("Start"));
}

#[test]
fn the_final_boss_room_must_be_a_boss_piece_when_it_is_a_piece() {
    let rooms = clean();
    let config = RunConfig::from_ron("(start_room: First, final_boss_room: Custom(10))").unwrap();
    assert_eq!(
        pool_check(&rooms, &config),
        [Problem::PieceRoleMismatch {
            room: room_title(START, None),
            field: "final_boss_room",
            role: PieceRole::Start,
            expected: PieceRole::Boss,
        }]
    );
}

#[test]
fn a_matching_role_or_a_named_room_that_is_not_a_piece_is_fine() {
    let rooms = clean();
    let config = RunConfig::from_ron("(start_room: Custom(10), final_boss_room: Custom(12))");
    assert_eq!(pool_check(&rooms, &config.unwrap()), []);
    // First and Boss are not in the pool at all: nothing to compare.
    assert_eq!(pool_check(&rooms, &RunConfig::default()), []);
}

// ---- rule 5: a gate that names an arrival can travel ----

fn gate_problem(rooms: &[(RoomId, RoomLayout)]) -> Vec<Problem> {
    pool_check(rooms, &config())
        .into_iter()
        .filter(|problem| matches!(problem, Problem::PieceGateCannotTravel { .. }))
        .collect()
}

#[test]
fn a_gate_from_the_arrival_through_the_one_port_into_that_room_travels() {
    assert_eq!(gate_problem(&clean()), []);
}

#[test]
fn a_gate_from_the_spawn_always_travels() {
    let mut rooms = clean();
    let to_boss = exit(200.0, BOSS);
    layout_of(&mut rooms, MIDDLE).gates = vec![Gate {
        from: Entrance::Spawn,
        exit: ExitRef::of(&to_boss),
        needs: vec![Ability::Dash],
    }];
    assert_eq!(gate_problem(&rooms), []);
}

#[test]
fn a_gate_from_a_room_the_piece_has_no_exit_into_is_reported_with_the_gate_described() {
    let mut rooms = clean();
    named(&mut rooms, MIDDLE, "Gallery");
    let to_boss = exit(200.0, BOSS);
    layout_of(&mut rooms, MIDDLE).gates = vec![gate_from(RoomId::Custom(30), 40.0, &to_boss)];
    let found = gate_problem(&rooms);
    assert_eq!(found.len(), 1);
    let Problem::PieceGateCannotTravel {
        room,
        gate,
        neighbour,
        exits,
    } = &found[0]
    else {
        unreachable!()
    };
    assert_eq!((room.as_str(), *exits), ("Gallery", 0));
    assert_eq!(neighbour, &room_title(RoomId::Custom(30), None));
    assert!(gate.contains("right edge exit at 40"), "{gate}");
    assert!(gate.contains("right edge exit at 200"), "{gate}");
    assert!(gate.contains("Dash"), "{gate}");
    let text = found[0].text();
    assert!(
        text.starts_with("Gallery:") && text.contains("no exit into"),
        "{text}"
    );
}

#[test]
fn a_gate_from_a_room_the_piece_has_two_exits_into_is_reported() {
    let mut rooms = clean();
    layout_of(&mut rooms, MIDDLE).exits.push(exit(300.0, START));
    let found = gate_problem(&rooms);
    assert_eq!(found.len(), 1);
    assert!(matches!(
        &found[0],
        Problem::PieceGateCannotTravel { exits: 2, .. }
    ));
    assert!(found[0].text().contains("2 exits into"));
}

#[test]
fn each_gate_that_cannot_travel_is_its_own_problem() {
    let mut rooms = clean();
    let to_boss = exit(200.0, BOSS);
    let to_start = exit(100.0, START);
    layout_of(&mut rooms, MIDDLE).gates = vec![
        gate_from(RoomId::Custom(30), 40.0, &to_boss),
        gate_from(RoomId::Custom(30), 40.0, &to_start),
        gate_from(START, 100.0, &to_boss),
    ];
    assert_eq!(gate_problem(&rooms).len(), 2);
}

#[test]
fn a_gate_in_a_room_that_is_not_a_piece_is_not_looked_at() {
    let mut rooms = clean();
    let to_boss = exit(200.0, BOSS);
    let layout = layout_of(&mut rooms, MIDDLE);
    layout.gates = vec![gate_from(RoomId::Custom(30), 40.0, &to_boss)];
    layout.piece = None;
    layout_of(&mut rooms, START).piece = Some(PieceInfo::new(PieceRole::Middle));
    layout_of(&mut rooms, BOSS).piece = Some(PieceInfo::new(PieceRole::Boss));
    let found = pool_check(&rooms, &config());
    assert!(found
        .iter()
        .all(|problem| !matches!(problem, Problem::PieceGateCannotTravel { .. })));
}

// ---- rule 6: an ability a gate needs can be found elsewhere ----

fn without_slots(rooms: &mut [(RoomId, RoomLayout)], id: RoomId) {
    layout_of(rooms, id).slots.clear();
}

#[test]
fn a_gate_that_needs_an_ability_wants_an_ability_slot_in_another_piece() {
    let mut rooms = clean();
    named(&mut rooms, MIDDLE, "Gallery");
    without_slots(&mut rooms, START);
    let found = pool_check(&rooms, &config());
    assert_eq!(
        found,
        [Problem::PieceAbilityWithoutSlot {
            room: "Gallery".to_owned()
        }]
    );
    assert!(found[0].text().starts_with("Gallery:"));
}

#[test]
fn a_key_slot_does_not_hold_an_ability_but_an_any_slot_does() {
    let mut rooms = clean();
    layout_of(&mut rooms, START).slots = vec![PickupSlot::new((100.0, 440.0), SlotKind::Key)];
    assert_eq!(pool_check(&rooms, &config()).len(), 1);
    layout_of(&mut rooms, START).slots[0].kind = SlotKind::Any;
    assert_eq!(pool_check(&rooms, &config()), []);
}

#[test]
fn the_pieces_own_ability_slot_does_not_count() {
    let mut rooms = clean();
    without_slots(&mut rooms, START);
    layout_of(&mut rooms, MIDDLE)
        .slots
        .push(PickupSlot::new((200.0, 440.0), SlotKind::Ability));
    assert_eq!(pool_check(&rooms, &config()).len(), 1);
}

#[test]
fn a_slot_in_a_room_that_is_not_a_piece_does_not_count() {
    let mut rooms = clean();
    without_slots(&mut rooms, START);
    let mut other = piece(
        RoomId::Custom(20),
        PieceRole::Middle,
        vec![],
        &[SlotKind::Ability],
    );
    other.1.piece = None;
    rooms.push(other);
    assert_eq!(pool_check(&rooms, &config()).len(), 1);
}

#[test]
fn a_piece_without_gates_needs_no_ability_slot() {
    let mut rooms = clean();
    layout_of(&mut rooms, MIDDLE).gates.clear();
    without_slots(&mut rooms, START);
    assert_eq!(pool_check(&rooms, &config()), []);
}

// ---- order ----

#[test]
fn problems_come_in_the_order_of_the_rules() {
    let mut rooms = clean();
    layout_of(&mut rooms, BOSS).piece = Some(PieceInfo::new(PieceRole::Middle)); // rule 1
    layout_of(&mut rooms, START).exits.clear(); // rule 2
    layout_of(&mut rooms, MIDDLE).exits[0].locked_by = Some(1); // rule 3
    without_slots(&mut rooms, START); // rule 6
    let kinds: Vec<&str> = pool_check(&rooms, &config())
        .iter()
        .map(|problem| match problem {
            Problem::NoPieceWithRole { .. } => "role",
            Problem::PieceWithoutExit { .. } => "exit",
            Problem::PieceLockedExit { .. } => "lock",
            Problem::PieceAbilityWithoutSlot { .. } => "ability",
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["role", "exit", "lock", "ability"]);
}

#[test]
fn every_problem_names_its_room() {
    let mut rooms = clean();
    named(&mut rooms, START, "Atrium");
    layout_of(&mut rooms, START).exits.clear();
    assert!(texts(&pool_check(&rooms, &config()))[0].contains("Atrium"));
}
