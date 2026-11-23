//! The pool the generator builds from: the pieces sorted by id, and each piece as one attempt
//! places it (mirrored or not, with its gates read into plain indices).
use super::error::GenError;
use crate::ability::Ability;
use crate::editor::{ExitPlacement, RoomLayout};
use crate::gates::{Entrance, ExitRef, Gate};
use crate::mirror::mirror_layout;
use crate::piece::{PieceRole, SlotKind};
use crate::room::{RoomEdge, RoomId};

/// A room of the pool: its id, its mark and its layout as authored.
pub(super) struct Piece {
    pub id: RoomId,
    pub role: PieceRole,
    pub weight: u8,
    pub layout: RoomLayout,
}

/// Every piece, sorted by room id, so the result never depends on the order they were given in.
pub(super) struct Pool {
    pub pieces: Vec<Piece>,
}

impl Pool {
    /// Sorts the rooms by id. A room with no piece mark, or an id used twice, is refused.
    pub fn new(rooms: &[(RoomId, RoomLayout)]) -> Result<Self, GenError> {
        let mut pieces = Vec::with_capacity(rooms.len());
        for (id, layout) in rooms {
            let info = layout.piece.ok_or(GenError::NotAPiece { room: *id })?;
            pieces.push(Piece {
                id: *id,
                role: info.role,
                weight: info.weight,
                layout: layout.clone(),
            });
        }
        pieces.sort_by_key(|piece| piece.id.index());
        if let Some(pair) = pieces.windows(2).find(|pair| pair[0].id == pair[1].id) {
            return Err(GenError::DuplicatePiece { room: pair[0].id });
        }
        Ok(Self { pieces })
    }

    /// The indices of the pieces with `role`, in id order.
    pub fn with_role(&self, role: PieceRole) -> Vec<usize> {
        (0..self.pieces.len())
            .filter(|index| self.pieces[*index].role == role)
            .collect()
    }
}

/// One side of a link: a door, or an edge. Two ports pair when both are doors, or their edges are
/// opposite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Port {
    door: bool,
    edge: RoomEdge,
}

impl Port {
    pub fn of(exit: &ExitPlacement) -> Self {
        Self {
            door: exit.is_door(),
            edge: exit.edge,
        }
    }

    /// The port this one becomes in a mirror: Left and Right swap, a door stays a door.
    pub fn mirrored(self) -> Self {
        let edge = match self.edge {
            RoomEdge::Left => RoomEdge::Right,
            RoomEdge::Right => RoomEdge::Left,
            other => other,
        };
        Self { edge, ..self }
    }

    pub fn pairs_with(self, other: Self) -> bool {
        if self.door || other.door {
            return self.door && other.door;
        }
        matches!(
            (self.edge, other.edge),
            (RoomEdge::Left, RoomEdge::Right)
                | (RoomEdge::Right, RoomEdge::Left)
                | (RoomEdge::Top, RoomEdge::Bottom)
                | (RoomEdge::Bottom, RoomEdge::Top)
        )
    }
}

/// Where a gate's entrance is, in terms of the piece's own exits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Entry {
    /// The room's own spawn: the crossing belongs to the piece and travels with it.
    Spawn,
    /// The arrival through this exit (its index in the piece): the port that led to the room the
    /// gate was authored against.
    Port(usize),
    /// An arrival whose port is not one exit: the gate cannot be carried and is dropped.
    Lost,
}

/// A gate read into indices: where it comes from, which exit it is on and what it needs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct GateInfo {
    pub from: Entry,
    /// The exit the gate is on; `None` for a gate naming an exit the piece does not have.
    pub exit: Option<usize>,
    /// The abilities the crossing needs, one bit each (`ability_bit`).
    pub needs: u8,
}

/// The bit of an ability in a mask.
pub(super) fn ability_bit(ability: Ability) -> u8 {
    Ability::ALL
        .iter()
        .position(|known| *known == ability)
        .map_or(0, |place| 1 << place)
}

/// A piece as one attempt places it. `gates` has one entry per gate of `layout`.
pub(super) struct Oriented {
    pub mirrored: bool,
    pub layout: RoomLayout,
    pub gates: Vec<GateInfo>,
}

impl Oriented {
    pub fn new(piece: &Piece, mirrored: bool) -> Self {
        let layout = if mirrored {
            mirror_layout(&piece.layout)
        } else {
            piece.layout.clone()
        };
        let gates = layout
            .gates
            .iter()
            .map(|gate| read_gate(&layout, gate))
            .collect();
        Self {
            mirrored,
            layout,
            gates,
        }
    }

    /// Exits that gates cross into: the port of a gate that goes on to a different exit. A piece
    /// joined at one of these is crossed the way its author meant, outward from the start.
    pub fn gate_ports(&self) -> Vec<usize> {
        self.gates
            .iter()
            .filter_map(|gate| match (gate.from, gate.exit) {
                (Entry::Port(port), Some(exit)) if port != exit => Some(port),
                _ => None,
            })
            .collect()
    }

    pub fn has_slot_for(&self, key: bool) -> bool {
        self.layout
            .slots
            .iter()
            .any(|slot| slot_fits(slot.kind, key))
    }
}

/// Whether a slot of this kind can hold a key (`key`) or an ability.
pub(super) fn slot_fits(kind: SlotKind, key: bool) -> bool {
    if key {
        kind.holds_key()
    } else {
        kind.holds_ability()
    }
}

fn read_gate(layout: &RoomLayout, gate: &Gate) -> GateInfo {
    let exit = layout
        .exits
        .iter()
        .position(|placement| ExitRef::of(placement) == gate.exit);
    let from = match gate.from {
        Entrance::Spawn => Entry::Spawn,
        Entrance::Arrival { room, .. } => {
            let ports: Vec<usize> = layout
                .exits
                .iter()
                .enumerate()
                .filter(|(_, exit)| exit.target_room == room)
                .map(|(index, _)| index)
                .collect();
            match ports.as_slice() {
                [port] => Entry::Port(*port),
                _ => Entry::Lost,
            }
        }
    };
    let needs = gate
        .needs
        .iter()
        .fold(0, |mask, ability| mask | ability_bit(*ability));
    GateInfo { from, exit, needs }
}
