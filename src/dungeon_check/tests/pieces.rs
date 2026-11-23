//! The dungeon checker with rooms marked as pieces (`docs/pieces.md`): a piece is not a room that
//! "can never be reached", a reachable piece is walked like any room, and with no piece the
//! pool rules add nothing.
use super::*;
use crate::piece::{PickupSlot, PieceInfo, PieceRole, SlotKind};

const POOL_START: RoomId = RoomId::Custom(10);
const POOL_BOSS: RoomId = RoomId::Custom(12);

fn marked(
    (id, mut layout): (RoomId, RoomLayout),
    role: PieceRole,
    slots: &[SlotKind],
) -> (RoomId, RoomLayout) {
    layout.piece = Some(PieceInfo::new(role));
    layout.slots = slots
        .iter()
        .enumerate()
        .map(|(place, kind)| PickupSlot::new((100.0 + 40.0 * place as f32, 440.0), *kind))
        .collect();
    (id, layout)
}

/// Three rooms that nothing in the fixed dungeon leads to, joined to each other: a pool.
fn pool(mark: bool) -> Vec<(RoomId, RoomLayout)> {
    let rooms = vec![
        room(POOL_START, vec![exit(100.0, SPARE)], &[]),
        room(
            SPARE,
            vec![exit(100.0, POOL_START), exit(300.0, POOL_BOSS)],
            &[],
        ),
        room(POOL_BOSS, vec![exit(100.0, SPARE)], &[]),
    ];
    let roles = [PieceRole::Start, PieceRole::Middle, PieceRole::Boss];
    match mark {
        true => rooms
            .into_iter()
            .zip(roles)
            .map(|(room, role)| marked(room, role, &[]))
            .collect(),
        false => rooms,
    }
}

fn with_pool(mark: bool) -> Vec<(RoomId, RoomLayout)> {
    let mut rooms = valid();
    rooms.extend(pool(mark));
    rooms
}

fn is_pool_problem(problem: &Problem) -> bool {
    matches!(
        problem,
        Problem::NoPieceWithRole { .. }
            | Problem::PieceWithoutExit { .. }
            | Problem::PieceLockedExit { .. }
            | Problem::PieceHoldsKey { .. }
            | Problem::PieceRoleMismatch { .. }
            | Problem::PieceGateCannotTravel { .. }
            | Problem::PieceAbilityWithoutSlot { .. }
    )
}

#[test]
fn rooms_nothing_leads_to_are_unreachable_unless_they_are_pieces() {
    let unmarked = problems(&with_pool(false));
    let rooms: Vec<&str> = unmarked
        .iter()
        .filter_map(|problem| match problem {
            Problem::Unreachable { room } => Some(room.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(rooms.len(), 3, "{:?}", texts(&unmarked));
    assert_eq!(problems(&with_pool(true)), []);
}

#[test]
fn only_the_piece_is_left_out_of_the_unreachable_report() {
    let mut rooms = with_pool(true);
    rooms.push(room(RoomId::Custom(40), vec![exit(100.0, FIRST)], &[]));
    let found = problems(&rooms);
    assert_eq!(found.len(), 1, "{:?}", texts(&found));
    assert!(matches!(&found[0], Problem::Unreachable { .. }));
}

#[test]
fn a_dungeon_with_no_piece_gets_no_pool_problem() {
    assert!(problems(&valid()).iter().all(|p| !is_pool_problem(p)));
    // A dungeon that breaks the ordinary rules still adds none of the pool's.
    let broken = dungeon(vec![
        room(FIRST, vec![locked(100.0, SECOND, 1), exit(300.0, HUB)], &[]),
        room(SECOND, vec![exit(100.0, FIRST)], &[Item::Key(2)]),
        room(BOSS, vec![], &[]),
    ]);
    let found = problems(&broken);
    assert!(!found.is_empty());
    assert!(found.iter().all(|problem| !is_pool_problem(problem)));
}

#[test]
fn the_pool_rules_come_last() {
    let mut rooms = valid();
    rooms.push(room(SPARE, vec![], &[]));
    rooms.push(room(RoomId::Custom(41), vec![], &[]));
    let mut ordinary = problems(&rooms);
    assert!(!ordinary.is_empty());
    layout_of(&mut rooms, SPARE).piece = Some(PieceInfo::new(PieceRole::Middle));
    let marked = problems(&rooms);
    // SPARE is no longer reported as unreachable; everything else stands, and the pool's follow.
    ordinary
        .retain(|problem| !matches!(problem, Problem::Unreachable { room } if room == "Room 2"));
    let (before, after) = marked.split_at(ordinary.len());
    assert_eq!(before, ordinary.as_slice());
    assert!(!after.is_empty() && after.iter().all(is_pool_problem));
}

fn layout_of(rooms: &mut [(RoomId, RoomLayout)], id: RoomId) -> &mut RoomLayout {
    &mut rooms.iter_mut().find(|(room, _)| *room == id).unwrap().1
}

/// `valid()` with a piece the player can walk into from First that holds the Dash: the gate in
/// Second is opened by the piece, and the pool around it is complete.
fn fixed_dungeon_through_a_piece(dash: bool) -> Vec<(RoomId, RoomLayout)> {
    let to_boss = locked(200.0, BOSS, 1);
    let mut second = room(SECOND, vec![exit(100.0, FIRST), to_boss.clone()], &[]);
    second.1.gates = vec![gate_from(FIRST, 100.0, &to_boss)];
    let dash: &[Item] = if dash { &[Item::Dash] } else { &[] };
    dungeon(vec![
        room(
            FIRST,
            vec![exit(100.0, SECOND), exit(300.0, SPARE), exit(500.0, HUB)],
            &[Item::Key(1)],
        ),
        second,
        room(BOSS, vec![exit(100.0, SECOND)], &[]),
        marked(
            room(
                SPARE,
                vec![
                    exit(100.0, FIRST),
                    exit(300.0, POOL_START),
                    exit(500.0, POOL_BOSS),
                ],
                dash,
            ),
            PieceRole::Middle,
            &[],
        ),
        marked(
            room(POOL_START, vec![exit(100.0, SPARE)], &[]),
            PieceRole::Start,
            &[],
        ),
        marked(
            room(POOL_BOSS, vec![exit(100.0, SPARE)], &[]),
            PieceRole::Boss,
            &[],
        ),
    ])
}

#[test]
fn a_piece_the_player_can_reach_is_walked_and_its_pickups_are_collected() {
    assert_eq!(problems(&fixed_dungeon_through_a_piece(true)), []);
    // Take the Dash out of the piece and the walk finds the gate in Second can never open:
    // the piece was walked, not skipped.
    let found = problems(&fixed_dungeon_through_a_piece(false));
    assert!(
        found
            .iter()
            .any(|problem| matches!(problem, Problem::AbilityUnobtainable { .. })),
        "{:?}",
        texts(&found)
    );
}

#[test]
fn a_reachable_piece_with_a_lock_nobody_can_open_is_reported_twice_over() {
    let mut rooms = fixed_dungeon_through_a_piece(true);
    layout_of(&mut rooms, SPARE).exits[1].locked_by = Some(2);
    let found = problems(&rooms);
    assert!(found
        .iter()
        .any(|p| matches!(p, Problem::KeyUnreachable { key: 2, .. })));
    assert!(found
        .iter()
        .any(|p| matches!(p, Problem::PieceLockedExit { key: 2, .. })));
}

#[test]
fn a_trap_in_a_reachable_piece_is_still_found() {
    let mut rooms = fixed_dungeon_through_a_piece(true);
    // The piece's only way out needs an ability held nowhere before it: stuck on arrival.
    let layout = layout_of(&mut rooms, SPARE);
    layout.exits.truncate(1);
    let way_out = layout.exits[0].clone();
    layout.gates = vec![Gate {
        from: Entrance::Arrival {
            room: FIRST,
            exit: ExitRef::Edge {
                edge: RoomEdge::Right,
                offset: 300.0,
            },
        },
        exit: ExitRef::of(&way_out),
        needs: vec![Ability::DoubleJump],
    }];
    let found = problems(&rooms);
    assert!(
        found
            .iter()
            .any(|problem| matches!(problem, Problem::Trap { .. })),
        "{:?}",
        texts(&found)
    );
}
