//! Flipping a whole dungeon: every room that `flipped` names is mirrored, and everything that
//! points into or out of a mirrored room is made to agree. A room mirrored alone would break the
//! dungeon (you would leave its left wall and arrive at the neighbour's left wall), so the
//! caller flips every room but the hub together; this function is correct for any set.
use std::collections::HashMap;

use super::{mirror_document, mirror_exit_ref, mirror_point};
use crate::editor::RoomLayout;
use crate::gates::Entrance;
use crate::room::{RoomEnemies, RoomId};

/// Mirrors every room of `documents` for which `flipped` is true, then makes the links agree:
///
/// - the `spawn` of every exit (in any room) that leads into a flipped room is mirrored in that
///   room's bounds, so the player still arrives on a free spot beside the matching exit;
/// - a gate's exit reference moves with its exit (a flipped room's gates, by `mirror_layout`);
/// - a gate's `Entrance::Arrival` follows the exit it names when that exit's room is flipped.
///   A reference carries no length, which a top or bottom exit needs, so the exit is looked up in
///   its room as it was before the flip.
///
/// A room whose document has no layout cannot be mirrored (it has no bounds) and is left as it
/// is, and so are the spawns that lead into it. Rooms are matched by id.
pub fn flip_world(documents: &mut [(RoomId, RoomEnemies)], flipped: impl Fn(RoomId) -> bool) {
    // The rooms as they were: after the flip a room no longer holds the exit a gate names.
    let before: HashMap<RoomId, RoomLayout> = documents
        .iter()
        .filter_map(|(id, document)| Some((*id, document.layout.clone()?)))
        .collect();
    for (id, document) in documents.iter_mut() {
        if flipped(*id) {
            *document = mirror_document(document);
        }
    }
    for (_, document) in documents.iter_mut() {
        let Some(layout) = &mut document.layout else {
            continue;
        };
        for exit in &mut layout.exits {
            let target = before.get(&exit.target_room);
            if let Some(target) = target.filter(|_| flipped(exit.target_room)) {
                exit.spawn = mirror_point(target.bounds, exit.spawn);
            }
        }
        for gate in &mut layout.gates {
            let Entrance::Arrival { room, exit } = &mut gate.from else {
                continue;
            };
            let source = before.get(room).filter(|_| flipped(*room));
            let moved = source.and_then(|s| mirror_exit_ref(exit, &s.exits, s.bounds));
            if let Some(moved) = moved {
                *exit = moved;
            }
        }
    }
}
