//! Rooms a run's seed may mirror (`docs/variety.md`): which of the rooms marked `flippable` are
//! played mirrored left to right in a run. Pure rules: a `RoomFlips` is a choice and a seed, and
//! `flips(room)` is a function of them and the room's id alone.
//!
//! Each room has its own draw, `stream(seed, "flip-room", key of the room)`, so the same seed
//! always mirrors the same rooms, a room's draw does not depend on how many rooms there are, and
//! the dungeon flip's own draw (`flip`) and the enemy picks (`enemies`) never share numbers with
//! it. The draw is a chance in percent (`room_flip_percent` in `variety.ron`, 50 by default, so
//! each flippable room is mirrored in half of the runs).
//!
//! Only the game's world loading asks (`mirror::FlippedDocuments`), for a run. The editor, Save,
//! the hub and a test room never do, so outside a run every room is as authored.
use super::flip::{FlipChoice, VarietyRules};
use crate::room::RoomId;
use crate::seed::{fnv1a, stream};

/// What the draw of a room is for: the label of its stream.
const LABEL: &str = "flip-room";

/// A `skip_serializing_if` for a flag that is written only when it is set.
pub(crate) fn is_false(flag: &bool) -> bool {
    !*flag
}

/// Which flippable rooms a run mirrors.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RoomFlips {
    /// None (F4's Never, a test room, the hub, a game with no hub).
    #[default]
    Off,
    /// Every flippable room (F4's Always).
    All,
    /// Each flippable room whose own draw for `seed` is under `percent`.
    Chance { seed: u64, percent: f32 },
}

impl RoomFlips {
    /// Whether a room that is marked flippable is mirrored. A room that is not marked is never
    /// asked: the caller checks the mark first.
    pub fn flips(self, room: RoomId) -> bool {
        match self {
            Self::Off => false,
            Self::All => true,
            Self::Chance { seed, percent } => {
                let key = fnv1a(format!("room:{}", room.index()).as_bytes());
                // Exact in f64: the draw is below 1, so below 100 as a percent, and 0 is below
                // nothing.
                let draw = f64::from(stream(seed, LABEL, key).unit()) * 100.0;
                draw < f64::from(percent)
            }
        }
    }
}

/// What a run with `seed` does about flippable rooms. `Never` and `Always` override the rules;
/// `Auto` leaves each room to its draw against `room_flip_percent`.
pub fn decide_room_flips(choice: FlipChoice, rules: &VarietyRules, seed: u64) -> RoomFlips {
    match choice {
        FlipChoice::Never => RoomFlips::Off,
        FlipChoice::Always => RoomFlips::All,
        FlipChoice::Auto => RoomFlips::Chance {
            seed,
            percent: rules.room_flip_percent,
        },
    }
}
