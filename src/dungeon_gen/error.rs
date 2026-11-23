//! Why the generator gives no dungeon (`docs/dungeon-gen.md` says how to read each one).
use std::fmt;

use crate::piece::PieceRole;
use crate::room::RoomId;

/// How many of the last attempt's problems the message lists before it says "and N more".
const SHOWN_PROBLEMS: usize = 6;

/// A reason `generate` returns no plan. The first group is the pool or the rules being unfit,
/// which no seed can fix and which is found before any attempt; `Failed` is every attempt
/// running out.
#[derive(Clone, Debug, PartialEq)]
pub enum GenError {
    /// The rules fail `GeneratorRules::validate` (the text says which field).
    BadRules(String),
    /// A room in the pool has no piece mark.
    NotAPiece { room: RoomId },
    /// Two rooms in the pool have the same id.
    DuplicatePiece { room: RoomId },
    /// A piece has the hub's id.
    PieceIsTheHub { room: RoomId },
    /// The pool has no piece of this role.
    NoPieceWithRole { role: PieceRole },
    /// The pool breaks one of the rules `piece::pool_check` states; each is its sentence.
    BadPool(Vec<String>),
    /// `run.ron`'s `hub_room` is not the hub that was passed in.
    HubMismatch {
        configured: Option<RoomId>,
        hub: RoomId,
    },
    /// The hub has no exit to the start room.
    HubDoesNotLeadToStart { hub: RoomId, start: RoomId },
    /// A rule asks for more rooms than the pool can fill: one Start piece, one Boss piece and
    /// every Middle piece is the most.
    TooFewPieces {
        field: &'static str,
        need: usize,
        can: usize,
        middles: usize,
    },
    /// `keys.min` is more than the Key and Any slots outside Boss pieces.
    NotEnoughKeySlots { need: usize, have: usize },
    /// `keys.min` locks do not fit on a spine of at most `spine_max` rooms (one lock per link).
    KeysDoNotFit { need: usize, spine_max: usize },
    /// No two pieces in the pool have exits that can be paired, mirrored or not.
    NoPairablePorts,
    /// Every attempt ended with a dungeon the checker refused, or with no dungeon at all.
    Failed {
        attempts: u32,
        /// What stopped the last attempt, in the words of the checker or the generator.
        problems: Vec<String>,
    },
}

fn role_name(role: PieceRole) -> &'static str {
    role.name()
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BadRules(text) => write!(f, "the generator rules are not valid: {text}"),
            Self::NotAPiece { room } => {
                write!(f, "{room:?} is in the pool but has no piece mark")
            }
            Self::DuplicatePiece { room } => {
                write!(f, "{room:?} is in the pool twice")
            }
            Self::PieceIsTheHub { room } => {
                write!(f, "{room:?} is a piece and also the hub")
            }
            Self::NoPieceWithRole { role } => {
                write!(f, "the pool has no {} piece", role_name(*role))
            }
            Self::BadPool(problems) => {
                write!(
                    f,
                    "the pool is not fit to build from: {}",
                    problems.join("; ")
                )
            }
            Self::HubMismatch { configured, hub } => match configured {
                Some(room) => write!(
                    f,
                    "run.ron's hub_room is {room:?}, but the hub given is {hub:?}"
                ),
                None => write!(f, "run.ron has no hub_room, but a hub ({hub:?}) was given"),
            },
            Self::HubDoesNotLeadToStart { hub, start } => {
                write!(f, "the hub {hub:?} has no exit to the start room {start:?}")
            }
            Self::TooFewPieces {
                field,
                need,
                can,
                middles,
            } => write!(
                f,
                "{field} is {need} but the pool can fill at most {can} rooms (one Start piece, \
                 one Boss piece and {middles} Middle piece{})",
                if *middles == 1 { "" } else { "s" }
            ),
            Self::NotEnoughKeySlots { need, have } => write!(
                f,
                "keys.min is {need} but only {have} Key slot{} (Key and Any slots count, Boss \
                 pieces' do not)",
                if *have == 1 { " exists" } else { "s exist" }
            ),
            Self::KeysDoNotFit { need, spine_max } => write!(
                f,
                "keys.min is {need} but a spine of at most {spine_max} rooms has only {} link{} \
                 to lock",
                spine_max.saturating_sub(1),
                if *spine_max == 2 { "" } else { "s" }
            ),
            Self::NoPairablePorts => write!(
                f,
                "no two pieces in the pool have exits that can be paired (Left with Right, Top \
                 with Bottom, a door with a door)"
            ),
            Self::Failed { attempts, problems } => {
                write!(
                    f,
                    "no valid dungeon in {attempts} attempt{}; the last attempt stopped at: ",
                    if *attempts == 1 { "" } else { "s" }
                )?;
                let shown = problems.iter().take(SHOWN_PROBLEMS);
                write!(f, "{}", shown.cloned().collect::<Vec<_>>().join("; "))?;
                if problems.len() > SHOWN_PROBLEMS {
                    write!(f, "; and {} more", problems.len() - SHOWN_PROBLEMS)?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for GenError {}
