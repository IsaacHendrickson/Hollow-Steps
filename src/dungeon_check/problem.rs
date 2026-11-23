//! What the dungeon checker reports: one `Problem` per finding, with the words the editor shows.
//! Rooms are named by their display name (their own name, or `First`, `Room 3`).
use crate::ability::Ability;
use crate::exit_lock::{lock_label, FIRST_KEY, LAST_KEY};
use crate::piece::PieceRole;
use crate::room::RoomId;

#[derive(Clone, Debug, PartialEq)]
pub enum Problem {
    /// `run.ron` names a room that is not among the rooms.
    MissingRoom { field: &'static str, room: RoomId },
    /// The hub has no exit to the start room, so the check starts at the start room's spawn.
    HubDoesNotLeadToStart { hub: String, start: String },
    /// A room (not the hub, not the final boss room) that no walk from the start reaches.
    Unreachable { room: String },
    /// The final boss room cannot be reached.
    BossUnreachable { room: String },
    /// A locked exit in a reachable room whose key lies nowhere the player can get to.
    KeyUnreachable { room: String, exit: String, key: u8 },
    /// A locked exit whose key is only reachable by going through that same exit.
    KeyBehindItsOwnLock { room: String, exit: String, key: u8 },
    /// More exits are locked by a key than keys of that number are placed (a key is used up).
    TooManyLocks { key: u8, locks: usize, keys: usize },
    /// An exit marked as needing abilities that cannot be picked up before it, and that no other
    /// way into the room lets the player use.
    AbilityUnobtainable {
        room: String,
        exit: String,
        needs: Vec<Ability>,
    },
    /// An entrance the player can reach from which no exit can be taken, holding `holding`.
    Trap {
        room: String,
        entrance: String,
        holding: Vec<Ability>,
    },
    /// A gate whose entrance the room does not have (`docs/gates.md`); it changes no result.
    OrphanGate {
        room: String,
        from: String,
        exit: String,
    },
    /// Two rooms name the same map cell (`docs/dungeon-map.md`); the map moves the second.
    SameMapCell {
        first: String,
        second: String,
        cell: (i32, i32),
    },
    /// The pool of pieces has none with this role; it needs a Start one and a Boss one
    /// (`docs/pieces.md`).
    NoPieceWithRole { role: PieceRole },
    /// A piece with no exit: nothing can be built onward from it.
    PieceWithoutExit { room: String },
    /// A piece with a locked exit; keys come from the generator through pickup slots.
    PieceLockedExit { room: String, exit: String, key: u8 },
    /// A piece that holds a Key item; keys come from the generator through pickup slots.
    PieceHoldsKey { room: String, key: u8 },
    /// The room `run.ron` names as `field` is a piece of another role than that field needs.
    PieceRoleMismatch {
        room: String,
        field: &'static str,
        role: PieceRole,
        expected: PieceRole,
    },
    /// A gate in a piece names an arrival that cannot be carried to another neighbour: the piece
    /// has `exits` exits into `neighbour`, and it needs exactly one.
    PieceGateCannotTravel {
        room: String,
        gate: String,
        neighbour: String,
        exits: usize,
    },
    /// A piece with a gate that needs an ability, and no other piece has an Ability or Any slot.
    PieceAbilityWithoutSlot { room: String },
}

fn names(abilities: &[Ability]) -> String {
    match abilities {
        [] => "no abilities".to_owned(),
        _ => abilities
            .iter()
            .map(|ability| ability.name())
            .collect::<Vec<_>>()
            .join(" and "),
    }
}

/// "Key 3"; a number outside 1 to 8 (a layout that fails its own checks) is written as is.
fn key(id: u8) -> String {
    match (FIRST_KEY..=LAST_KEY).contains(&id) {
        true => lock_label(Some(id)).unwrap_or_default(),
        false => format!("Key {id}"),
    }
}

impl Problem {
    /// The sentence the editor shows.
    pub fn text(&self) -> String {
        match self {
            Self::MissingRoom { field, room } => {
                format!("run.ron: {field} {room:?} is not a room")
            }
            Self::HubDoesNotLeadToStart { hub, start } => format!(
                "The hub ({hub}) has no exit to the start room ({start}); \
                 checking from the start room's spawn"
            ),
            Self::Unreachable { room } => format!("{room} can never be reached from the start"),
            Self::BossUnreachable { room } => {
                format!("The final boss room, {room}, can never be reached")
            }
            Self::KeyUnreachable {
                room,
                exit,
                key: id,
            } => format!(
                "{room}: the {exit} is locked by {}, and no {} can be reached",
                key(*id),
                key(*id)
            ),
            Self::KeyBehindItsOwnLock {
                room,
                exit,
                key: id,
            } => format!(
                "{room}: the {exit} is locked by {}, and the only {} lies behind that same lock",
                key(*id),
                key(*id)
            ),
            Self::TooManyLocks {
                key: id,
                locks,
                keys,
            } => format!(
                "{} is used up by the exit it opens, but {locks} exits are locked by it and \
                 {keys} {} placed",
                key(*id),
                if *keys == 1 { "is" } else { "are" }
            ),
            Self::AbilityUnobtainable { room, exit, needs } => format!(
                "{room}: the {exit} needs {}, which cannot be picked up before it",
                names(needs)
            ),
            Self::Trap {
                room,
                entrance,
                holding,
            } => format!(
                "{room}: arriving at {entrance} with {}, no exit can be taken",
                names(holding)
            ),
            Self::OrphanGate { room, from, exit } => format!(
                "{room}: a gate from {from} to the {exit} names an entrance that no longer exists"
            ),
            Self::SameMapCell {
                first,
                second,
                cell: (x, y),
            } => format!(
                "{first} and {second} both have the map cell ({x}, {y}); the map moves {second}"
            ),
            Self::NoPieceWithRole { role } => format!(
                "No piece has the {} role; a pool needs at least one Start piece and one Boss piece",
                role.name()
            ),
            Self::PieceWithoutExit { room } => {
                format!("{room}: a piece needs at least one exit, and this one has none")
            }
            Self::PieceLockedExit {
                room,
                exit,
                key: id,
            } => format!(
                "{room}: a piece cannot have a locked exit, but its {exit} is locked by {}; \
                 keys come from the generator through pickup slots",
                key(*id)
            ),
            Self::PieceHoldsKey { room, key: id } => format!(
                "{room}: a piece cannot hold a Key item ({}); keys come from the generator \
                 through pickup slots, so add a Key or Any slot instead",
                key(*id)
            ),
            Self::PieceRoleMismatch {
                room,
                field,
                role,
                expected,
            } => format!(
                "{room}: run.ron's {field} is a {} piece, but it must be a {} piece",
                role.name(),
                expected.name()
            ),
            Self::PieceGateCannotTravel {
                room,
                gate,
                neighbour,
                exits,
            } => format!(
                "{room}: the gate {gate} cannot be carried to another neighbour: the piece has {} \
                 into {neighbour}, and needs exactly one, the port the player comes in through",
                match exits {
                    0 => "no exit".to_owned(),
                    count => format!("{count} exits"),
                }
            ),
            Self::PieceAbilityWithoutSlot { room } => format!(
                "{room}: a gate in this piece needs an ability, but no other piece has an \
                 Ability or Any pickup slot"
            ),
        }
    }
}
