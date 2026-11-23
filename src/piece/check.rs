//! The rules for a pool of pieces (`docs/pieces.md`): what a generator needs from the rooms that
//! are marked as pieces before it can build a dungeon out of them. Pure, like the dungeon
//! checker, which calls `pool_check` after its own checks.
//!
//! The pool is every room whose layout carries a piece mark. With none, nothing is reported and
//! every other result of the checker stands. Otherwise: (1) there is a Start piece and a Boss
//! piece; (2) every piece has an exit; (3) no piece has a locked exit or a Key item, because keys
//! come from the generator through pickup slots; (4) the rooms `run.ron` names as `start_room` and
//! `final_boss_room`, when they are pieces, have the matching role; (5) every gate in a piece that
//! names an arrival can be carried to another neighbour, which needs the piece to have exactly
//! one exit into the room that arrival comes from (the port the player came in through); (6) a
//! piece with a gate that needs an ability has another piece with an Ability or Any slot to
//! find it in.
use super::{PieceInfo, PieceRole};
use crate::dungeon_check::Problem;
use crate::editor::RoomLayout;
use crate::gates::{Entrance, ExitRef, Gate};
use crate::item::Item;
use crate::room::RoomId;
use crate::room_name::room_title;
use crate::run::RunConfig;

#[cfg(test)]
mod tests;

/// A room of the pool: its id, its layout and its mark.
struct Piece<'a> {
    id: RoomId,
    layout: &'a RoomLayout,
    info: PieceInfo,
}

/// Checks the pool of pieces among `rooms` (every room, pieces or not) under the run rules
/// `config`. Empty when no room is a piece or nothing is wrong. Problems come in the order of
/// the rules above, and within a rule in the order of the rooms.
pub fn pool_check(rooms: &[(RoomId, RoomLayout)], config: &RunConfig) -> Vec<Problem> {
    let pieces: Vec<Piece> = rooms
        .iter()
        .filter_map(|(id, layout)| {
            layout.piece.map(|info| Piece {
                id: *id,
                layout,
                info,
            })
        })
        .collect();
    if pieces.is_empty() {
        return Vec::new();
    }
    let title = |id: RoomId| {
        let named = rooms.iter().find(|(room, _)| *room == id);
        room_title(id, named.and_then(|(_, layout)| layout.name.as_deref()))
    };
    let mut problems = Vec::new();
    for role in [PieceRole::Start, PieceRole::Boss] {
        if !pieces.iter().any(|piece| piece.info.role == role) {
            problems.push(Problem::NoPieceWithRole { role });
        }
    }
    for piece in &pieces {
        if piece.layout.exits.is_empty() {
            problems.push(Problem::PieceWithoutExit {
                room: title(piece.id),
            });
        }
    }
    for piece in &pieces {
        keys_belong_to_slots(piece, &title(piece.id), &mut problems);
    }
    for (field, id, expected) in [
        ("start_room", config.start_room, PieceRole::Start),
        ("final_boss_room", config.final_boss_room, PieceRole::Boss),
    ] {
        let named = pieces.iter().find(|piece| piece.id == id);
        if let Some(piece) = named.filter(|piece| piece.info.role != expected) {
            problems.push(Problem::PieceRoleMismatch {
                room: title(piece.id),
                field,
                role: piece.info.role,
                expected,
            });
        }
    }
    for piece in &pieces {
        for gate in &piece.layout.gates {
            if let Some(problem) = gate_that_cannot_travel(piece, gate, &title) {
                problems.push(problem);
            }
        }
    }
    for piece in &pieces {
        let needs_ability = piece.layout.gates.iter().any(|gate| !gate.needs.is_empty());
        let found_elsewhere = pieces.iter().any(|other| {
            other.id != piece.id
                && other
                    .layout
                    .slots
                    .iter()
                    .any(|slot| slot.kind.holds_ability())
        });
        if needs_ability && !found_elsewhere {
            problems.push(Problem::PieceAbilityWithoutSlot {
                room: title(piece.id),
            });
        }
    }
    problems
}

/// Rule 3: a lock and a Key item are each reported (once per key number in the room's items).
fn keys_belong_to_slots(piece: &Piece, room: &str, problems: &mut Vec<Problem>) {
    for exit in &piece.layout.exits {
        if let Some(key) = exit.locked_by {
            problems.push(Problem::PieceLockedExit {
                room: room.to_owned(),
                exit: ExitRef::of(exit).describe(),
                key,
            });
        }
    }
    for placed in &piece.layout.items {
        if let Item::Key(key) = placed.item {
            let problem = Problem::PieceHoldsKey {
                room: room.to_owned(),
                key,
            };
            if !problems.contains(&problem) {
                problems.push(problem);
            }
        }
    }
}

/// Rule 5: a gate from an arrival names the room that arrival comes from. It can be carried to
/// whichever neighbour is joined at the same port only if the piece has exactly one exit into
/// that room. A gate from the spawn belongs to the piece itself and always travels.
fn gate_that_cannot_travel(
    piece: &Piece,
    gate: &Gate,
    title: &dyn Fn(RoomId) -> String,
) -> Option<Problem> {
    let Entrance::Arrival {
        room: neighbour,
        exit: from,
    } = gate.from
    else {
        return None;
    };
    let exits = piece
        .layout
        .exits
        .iter()
        .filter(|exit| exit.target_room == neighbour)
        .count();
    if exits == 1 {
        return None;
    }
    let needs = gate
        .needs
        .iter()
        .map(|ability| ability.name())
        .collect::<Vec<_>>()
        .join(" and ");
    let neighbour = title(neighbour);
    Some(Problem::PieceGateCannotTravel {
        room: title(piece.id),
        gate: format!(
            "from the arrival from {neighbour} ({}) to the {}, needing {needs}",
            from.describe(),
            gate.exit.describe()
        ),
        neighbour,
        exits,
    })
}
