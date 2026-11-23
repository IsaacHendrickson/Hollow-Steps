//! One attempt: orient the pieces, grow the tree, build the rooms and ask the checker. An attempt
//! is a pure function of the seed and its number: every random choice comes from
//! `stream(seed, "dungeon-gen", key)` with the attempt number mixed into the key, so attempt 2
//! is a fresh draw and nothing else about the inputs decides it.
use super::build::build;
use super::grow::grow;
use super::plan::DungeonPlan;
use super::pool::{Oriented, Pool};
use crate::dungeon_check::{check, Problem};
use crate::editor::RoomLayout;
use crate::particles::Rng;
use crate::room::RoomId;
use crate::run::RunConfig;
use crate::seed::{fnv1a, stream};

/// What every random choice of the generator is for: the label of its streams.
const LABEL: &str = "dungeon-gen";

/// The sizes the rules and the pool allow, worked out once. Counts of rooms.
pub(super) struct Limits {
    /// Rooms on the spine, start and boss included.
    pub spine: (usize, usize),
    /// Rooms in the dungeon; the top is the most the pool can fill.
    pub rooms: (usize, usize),
    /// Locks, and so keys; the top is capped by the Key and Any slots there are.
    pub keys: (usize, usize),
}

/// Everything an attempt reads.
pub(super) struct Inputs<'a> {
    pub pool: &'a Pool,
    pub limits: &'a Limits,
    pub mirror_percent: f32,
    pub hub: &'a (RoomId, RoomLayout),
    pub config: &'a RunConfig,
    pub seed: u64,
}

/// An attempt in progress: the pieces as this attempt places them, and where its draws come from.
pub(super) struct Ctx<'a> {
    pub pool: &'a Pool,
    pub oriented: Vec<Oriented>,
    pub limits: &'a Limits,
    pub seed: u64,
    pub attempt: u32,
}

fn stream_of(seed: u64, attempt: u32, what: &str, number: u64) -> Rng {
    let key = fnv1a(format!("{attempt}:{what}:{number}").as_bytes());
    stream(seed, LABEL, key)
}

impl Ctx<'_> {
    /// The stream for `what` (and `number`, which one of them) in this attempt.
    pub fn rng(&self, what: &str, number: u64) -> Rng {
        stream_of(self.seed, self.attempt, what, number)
    }
}

/// Attempt `number` (from 1): the plan, or what stopped it in words.
pub(super) fn run(inputs: &Inputs, number: u32) -> Result<DungeonPlan, Vec<String>> {
    let oriented = inputs
        .pool
        .pieces
        .iter()
        .map(|piece| {
            // One stream per piece, so a piece's side does not depend on the others.
            let mut rng = stream_of(inputs.seed, number, "mirror", piece.id.index() as u64);
            let draw = f64::from(rng.unit()) * 100.0;
            Oriented::new(piece, draw < f64::from(inputs.mirror_percent))
        })
        .collect();
    let ctx = Ctx {
        pool: inputs.pool,
        oriented,
        limits: inputs.limits,
        seed: inputs.seed,
        attempt: number,
    };
    let grown = grow(&ctx)?;
    let (rooms, hub) = build(&ctx, &grown, inputs.hub, inputs.config)?;
    let plan = DungeonPlan {
        seed: inputs.seed,
        rooms,
        hub_room: inputs.hub.0,
        hub,
        attempts: number,
    };
    let problems = check(&plan.catalog(), inputs.config);
    if problems.is_empty() {
        Ok(plan)
    } else {
        Err(problems.iter().map(Problem::text).collect())
    }
}
