//! How `GameState` owns generated runs (`crate::dungeon_gen`, `docs/dungeon-gen.md`): what F4's
//! "Generate next run" says, the plan of the run in progress, and the step of `start_run` that
//! makes it.
//!
//! The plan is made once, when a run starts, from the pieces as the room files hold them (never
//! from the rooms in play), the hub's layout, `run.ron`, the `generator` rules and the run's
//! seed, and kept for the run's life, through its result screen: every rebuild in the run (F5)
//! builds the same world from it (`world_load::generated`). The hub phase, a game with no hub, a
//! test room and the world after the result screen's Continue never have one, so they play the
//! authored rooms. A run whose plan cannot be made does not start.
use super::world_load::generated::read_sources;
use super::GameState;
use crate::dungeon_gen::{generate, DungeonPlan};
use crate::variety::{decide_generation, GenerateChoice};

/// F4's choice for the next run and the plan of the run in progress.
#[derive(Debug, Default)]
pub(super) struct GenerateState {
    pub(super) choice: GenerateChoice,
    /// The plan of the run in progress, or whose result screen is showing.
    pub(super) plan: Option<DungeonPlan>,
}

impl GameState {
    /// What F4's "Generate next run" says. It is not a file and not saved.
    pub fn generate_choice(&self) -> GenerateChoice {
        self.run.generate.choice
    }

    /// Sets what the next runs do (F4's "Generate next run"). A run already going keeps its own
    /// dungeon.
    pub fn set_generate_choice(&mut self, choice: GenerateChoice) {
        self.run.generate.choice = choice;
    }

    /// True while a run built by the generator is in progress, or its result screen is showing.
    /// False in the hub, in a game with no hub and in a test room.
    pub fn run_generated(&self) -> bool {
        self.live_run() && self.run.generate.plan.is_some()
    }

    /// The plan of the live run (`run_generated`): which pieces it is made of, where each key and
    /// ability lies.
    pub fn run_plan(&self) -> Option<&DungeonPlan> {
        self.run_generated()
            .then_some(self.run.generate.plan.as_ref())
            .flatten()
    }

    /// The plan for the run that starts with `seed`, or `None` when the fixed dungeon is played.
    /// F4's choice and `variety.ron` decide (`decide_generation`); nothing is read from the room
    /// files unless one of them may say yes. A plan the generator cannot make comes back as its
    /// reason, and the run does not start.
    pub(super) fn plan_run(&self, seed: u64) -> Result<Option<DungeonPlan>, String> {
        let Some(hub) = self.run.config.hub_room else {
            return Ok(None);
        };
        let rules = self.variety_rules().generator;
        let choice = self.run.generate.choice;
        if !decide_generation(choice, &rules, true) {
            return Ok(None);
        }
        let live_hub = self.rooms.iter().find(|room| room.id == hub);
        let sources = read_sources(&self.assets, hub, live_hub)?;
        if !decide_generation(choice, &rules, sources.has_ends()) {
            return Ok(None);
        }
        generate(
            &sources.pieces,
            &sources.hub,
            &self.run.config,
            &rules,
            seed,
        )
        .map(Some)
        .map_err(|error| error.to_string())
    }
}
