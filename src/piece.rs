//! Dungeon pieces (`docs/pieces.md`): a room marked as one piece of a dungeon that a generator
//! can build, and the pickup slots where that generator may put a key or an ability. This is the
//! data and the checks on one room; `check` holds the rules for a whole pool of pieces. Nothing
//! here changes how a room plays: the game never reads a mark or a slot.
use std::fmt;

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::editor::RoomLayout;
use crate::item::ITEM_SIZE;
use crate::terrain::Terrain;

mod check;
pub use check::pool_check;

/// The lightest and heaviest a piece can be: how often a generator reaches for it.
pub const MIN_WEIGHT: u8 = 1;
pub const MAX_WEIGHT: u8 = 10;
/// The most pickup slots one room holds.
pub const MAX_SLOTS: usize = 8;

/// Where in a dungeon a piece can go.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PieceRole {
    /// Where the run begins.
    Start,
    /// Anywhere between the start and the boss.
    Middle,
    /// The last room.
    Boss,
}

impl PieceRole {
    pub const ALL: [Self; 3] = [Self::Start, Self::Middle, Self::Boss];

    pub fn name(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Middle => "Middle",
            Self::Boss => "Boss",
        }
    }
}

/// The mark that makes a room a piece. Absent in a room file means the room is not one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PieceInfo {
    pub role: PieceRole,
    /// How likely a generator is to pick this piece, `MIN_WEIGHT` to `MAX_WEIGHT`.
    #[serde(default = "default_weight")]
    pub weight: u8,
}

fn default_weight() -> u8 {
    MIN_WEIGHT
}

impl PieceInfo {
    /// A piece of this role at the default weight.
    pub fn new(role: PieceRole) -> Self {
        Self {
            role,
            weight: default_weight(),
        }
    }
}

/// What a generator may put in a slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SlotKind {
    Key,
    Ability,
    /// Either a key or an ability.
    Any,
}

impl SlotKind {
    pub const ALL: [Self; 3] = [Self::Key, Self::Ability, Self::Any];

    pub fn name(self) -> &'static str {
        match self {
            Self::Key => "Key",
            Self::Ability => "Ability",
            Self::Any => "Any",
        }
    }

    /// The letter drawn in the slot's marker on the editor canvas.
    pub fn letter(self) -> &'static str {
        match self {
            Self::Key => "K",
            Self::Ability => "A",
            Self::Any => "?",
        }
    }

    pub fn holds_key(self) -> bool {
        matches!(self, Self::Key | Self::Any)
    }

    pub fn holds_ability(self) -> bool {
        matches!(self, Self::Ability | Self::Any)
    }
}

/// A spot where the generator may put a key or an ability. It is not an item: the game and the
/// Test room never show it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PickupSlot {
    /// Center of the spot, where an item's center would be.
    pub position: (f32, f32),
    pub kind: SlotKind,
}

impl PickupSlot {
    pub fn new(position: (f32, f32), kind: SlotKind) -> Self {
        Self { position, kind }
    }
}

/// Why a room's piece mark or slots are refused. `slot` and the numbers in a message count
/// from 1, the way the editor lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PieceError {
    WeightOutOfRange { weight: u8 },
    TooManySlots { count: usize },
    SlotNotFinite { slot: usize },
    SlotOutsideRoom { slot: usize },
    SlotInsideSolid { slot: usize },
    SlotsTooClose { first: usize, second: usize },
}

impl fmt::Display for PieceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WeightOutOfRange { weight } => write!(
                f,
                "piece weight must be from {MIN_WEIGHT} to {MAX_WEIGHT}, not {weight}"
            ),
            Self::TooManySlots { count } => write!(
                f,
                "a room holds at most {MAX_SLOTS} pickup slots, not {count}"
            ),
            Self::SlotNotFinite { slot } => {
                write!(f, "pickup slot {slot} has a position that is not a number")
            }
            Self::SlotOutsideRoom { slot } => {
                write!(
                    f,
                    "pickup slot {slot} must be inside the room above the floor"
                )
            }
            Self::SlotInsideSolid { slot } => {
                write!(f, "pickup slot {slot} must not be inside terrain")
            }
            Self::SlotsTooClose { first, second } => write!(
                f,
                "pickup slots {first} and {second} are closer than an item ({} by {} px)",
                ITEM_SIZE.x, ITEM_SIZE.y
            ),
        }
    }
}

impl RoomLayout {
    /// Whether the room is marked as a dungeon piece.
    pub fn is_piece(&self) -> bool {
        self.piece.is_some()
    }

    /// The weight is in range, there are at most `MAX_SLOTS` slots, each slot is a finite point
    /// inside the room above the floor and outside every solid (the check an item gets), and no
    /// two slots are closer than an item. Slots count even when the room is not marked: an
    /// unmarked room keeps its slots.
    pub fn check_piece(&self, terrain: &Terrain) -> Result<(), PieceError> {
        if let Some(info) = &self.piece {
            if !(MIN_WEIGHT..=MAX_WEIGHT).contains(&info.weight) {
                return Err(PieceError::WeightOutOfRange {
                    weight: info.weight,
                });
            }
        }
        if self.slots.len() > MAX_SLOTS {
            return Err(PieceError::TooManySlots {
                count: self.slots.len(),
            });
        }
        for (index, slot) in self.slots.iter().enumerate() {
            let number = index + 1;
            if !Vec2::from(slot.position).is_finite() {
                return Err(PieceError::SlotNotFinite { slot: number });
            }
            if !self.contains_body(slot.position, ITEM_SIZE) {
                return Err(PieceError::SlotOutsideRoom { slot: number });
            }
            if terrain.overlaps_body(slot.position.into(), ITEM_SIZE) {
                return Err(PieceError::SlotInsideSolid { slot: number });
            }
        }
        for (second, later) in self.slots.iter().enumerate() {
            for (first, earlier) in self.slots[..second].iter().enumerate() {
                let apart = (Vec2::from(later.position) - Vec2::from(earlier.position)).abs();
                if apart.x < ITEM_SIZE.x && apart.y < ITEM_SIZE.y {
                    return Err(PieceError::SlotsTooClose {
                        first: first + 1,
                        second: second + 1,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
