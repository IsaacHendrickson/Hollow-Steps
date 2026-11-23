//! The dungeon generator (`docs/dungeon-gen.md`): builds a dungeon from a pool of pieces
//! (`docs/pieces.md`) and a seed, and proves it with the dungeon checker. Pure: no files, no game,
//! no window. The game calls it when a run starts (`game::generate_link`).
//!
//! `generate(pieces, hub, config, rules, seed)` sorts the pieces by id, refuses a pool or rules
//! that no seed could build from (`GenError`), then makes up to `rules.attempts` attempts. An
//! attempt mirrors each piece with `mirror_percent`, grows a tree (a spine from the Start piece
//! to the Boss piece, then side branches), pairs the exits, locks some spine links, puts keys
//! and the abilities the gates need in slots, and builds the rooms. It succeeds when
//! `dungeon_check::check` of the plan's catalog is empty; otherwise the next attempt draws again.
//! The same inputs always give the same plan, whatever order the pieces came in.
//!
//! - `pool`: the sorted pool and each piece as one attempt places it.
//! - `tree`: the rooms as they are added and the links between them.
//! - `grow`: the spine, the locks, rooms for what needs a slot, the side branches.
//! - `demand`: what the slots must hold, where each may go, and the matching that fills them.
//! - `build`: the rooms as they play, and the hub aimed at the start room.
//! - `attempt`: one attempt end to end, and where its random streams come from.
use crate::editor::RoomLayout;
use crate::piece::{pool_check, PieceRole};
use crate::room::RoomId;
use crate::run::RunConfig;
use crate::variety::GeneratorRules;

mod attempt;
mod build;
mod demand;
mod error;
mod grow;
mod plan;
mod pool;
mod tree;

pub use error::GenError;
pub use plan::{DungeonPlan, PlannedRoom};

use attempt::{Inputs, Limits};
use pool::{Pool, Port};

/// Builds a dungeon. `pieces` is every room whose layout has a piece mark, `hub` the hub room
/// (`config.hub_room`, with an exit to `config.start_room`), `rules` the `generator` section of
/// `variety.ron`. `rules.enabled` is the game's switch and is not looked at here.
pub fn generate(
    pieces: &[(RoomId, RoomLayout)],
    hub: &(RoomId, RoomLayout),
    config: &RunConfig,
    rules: &GeneratorRules,
    seed: u64,
) -> Result<DungeonPlan, GenError> {
    rules.validate().map_err(GenError::BadRules)?;
    let pool = Pool::new(pieces)?;
    let limits = check_pool(&pool, pieces, hub, config, rules)?;
    let inputs = Inputs {
        pool: &pool,
        limits: &limits,
        mirror_percent: rules.mirror_percent,
        hub,
        config,
        seed,
    };
    let attempts = rules.attempt_limit();
    let mut last = Vec::new();
    for number in 1..=attempts {
        match attempt::run(&inputs, number) {
            Ok(plan) => return Ok(plan),
            Err(problems) => last = problems,
        }
    }
    Err(GenError::Failed {
        attempts,
        problems: last,
    })
}

/// Everything that can be known without drawing: a pool and rules no seed could build from are
/// refused here with the reason. Returns the sizes an attempt works within.
fn check_pool(
    pool: &Pool,
    pieces: &[(RoomId, RoomLayout)],
    hub: &(RoomId, RoomLayout),
    config: &RunConfig,
    rules: &GeneratorRules,
) -> Result<Limits, GenError> {
    if let Some(piece) = pool.pieces.iter().find(|piece| piece.id == hub.0) {
        return Err(GenError::PieceIsTheHub { room: piece.id });
    }
    for role in [PieceRole::Start, PieceRole::Boss] {
        if pool.with_role(role).is_empty() {
            return Err(GenError::NoPieceWithRole { role });
        }
    }
    let problems = pool_check(pieces, config);
    if !problems.is_empty() {
        let texts = problems.iter().map(crate::dungeon_check::Problem::text);
        return Err(GenError::BadPool(texts.collect()));
    }
    if config.hub_room != Some(hub.0) {
        return Err(GenError::HubMismatch {
            configured: config.hub_room,
            hub: hub.0,
        });
    }
    if !hub
        .1
        .exits
        .iter()
        .any(|exit| exit.target_room == config.start_room)
    {
        return Err(GenError::HubDoesNotLeadToStart {
            hub: hub.0,
            start: config.start_room,
        });
    }

    let middles = pool.with_role(PieceRole::Middle).len();
    // One Start piece, one Boss piece and every Middle piece: the most rooms a plan can have.
    let capacity = 2 + middles;
    let (rooms_min, rooms_max) = rules.rooms.bounds();
    let (spine_min, spine_max) = rules.spine.bounds();
    let (keys_min, keys_max) = rules.keys.bounds();
    for (field, need) in [("rooms.min", rooms_min), ("spine.min", spine_min)] {
        if need > capacity {
            return Err(GenError::TooFewPieces {
                field,
                need,
                can: capacity,
                middles,
            });
        }
    }
    let key_slots: usize = pool
        .pieces
        .iter()
        .filter(|piece| piece.role != PieceRole::Boss)
        .map(|piece| {
            let slots = piece.layout.slots.iter();
            slots.filter(|slot| slot.kind.holds_key()).count()
        })
        .sum();
    if keys_min > key_slots {
        return Err(GenError::NotEnoughKeySlots {
            need: keys_min,
            have: key_slots,
        });
    }
    let spine_top = spine_max.min(capacity);
    if keys_min + 1 > spine_top {
        return Err(GenError::KeysDoNotFit {
            need: keys_min,
            spine_max: spine_top,
        });
    }
    if !some_ports_pair(pool, rules.mirror_percent) {
        return Err(GenError::NoPairablePorts);
    }
    Ok(Limits {
        spine: (spine_min.max(keys_min + 1), spine_top),
        rooms: (rooms_min, rooms_max.min(capacity)),
        keys: (keys_min, keys_max.min(key_slots)),
    })
}

/// Whether two different pieces have exits that can pair, in the sides an attempt may give them:
/// as authored with no mirroring, mirrored with 100 percent, either with anything between.
fn some_ports_pair(pool: &Pool, mirror_percent: f32) -> bool {
    let sides: &[bool] = if mirror_percent <= 0.0 {
        &[false]
    } else if mirror_percent >= 100.0 {
        &[true]
    } else {
        &[false, true]
    };
    let ports = |piece: usize, mirrored: bool| -> Vec<Port> {
        pool.pieces[piece]
            .layout
            .exits
            .iter()
            .map(|exit| {
                let port = Port::of(exit);
                if mirrored {
                    port.mirrored()
                } else {
                    port
                }
            })
            .collect()
    };
    let count = pool.pieces.len();
    (0..count).any(|first| {
        (first + 1..count).any(|second| {
            sides.iter().any(|a| {
                sides.iter().any(|b| {
                    let (left, right) = (ports(first, *a), ports(second, *b));
                    left.iter().any(|x| right.iter().any(|y| x.pairs_with(*y)))
                })
            })
        })
    })
}
