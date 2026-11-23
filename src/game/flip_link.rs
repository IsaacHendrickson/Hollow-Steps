//! How `GameState` owns the dungeon flip's choice (`crate::variety`, `docs/variety.md`): the rules
//! from `variety.ron`, what F4's "Flip next run" says, and whether the run in progress is flipped.
//!
//! The decision is made once, in `start_run`, from the run's seed and kept for the run's life
//! (through the result screen). The hub, a game with no hub and a test room are never flipped.
//! `variety.ron` is read at start-up and F5 does not re-read it, like `difficulty.ron`.
use std::path::Path;

use super::GameState;
use crate::variety::{decide_flip, FlipChoice, VarietyRules};

/// The rules, the F4 choice for the next run and the decision of the run in progress.
#[derive(Debug, Default)]
pub(super) struct FlipState {
    rules: VarietyRules,
    choice: FlipChoice,
    /// The run in progress (or whose result screen is showing) was started flipped.
    pub(super) current: bool,
}

impl FlipState {
    /// The state for the `variety.ron` in `assets`, or the embedded default.
    pub(super) fn load(assets: &Path) -> Result<Self, String> {
        Ok(Self {
            rules: VarietyRules::load(assets)?,
            ..Self::default()
        })
    }

    /// Whether the run with `seed` flips, by the rules and the choice.
    pub(super) fn decide(&self, seed: u64) -> bool {
        decide_flip(self.choice, &self.rules, seed)
    }
}

impl GameState {
    /// What `variety.ron` says (the embedded default when there is no file).
    pub fn variety_rules(&self) -> &VarietyRules {
        &self.run.flip.rules
    }

    /// What F4's "Flip next run" says. It is not a file and not saved.
    pub fn flip_choice(&self) -> FlipChoice {
        self.run.flip.choice
    }

    /// Sets what the next runs do (F4's "Flip next run"). A run already going keeps its own
    /// decision.
    pub fn set_flip_choice(&mut self, choice: FlipChoice) {
        self.run.flip.choice = choice;
    }

    /// True while a run that started flipped is in progress, or its result screen is showing.
    /// False in the hub, in a game with no hub and in a test room.
    pub fn run_flipped(&self) -> bool {
        let live = self.run_in_progress() || (self.test_run.is_none() && self.showing_result());
        live && self.run.flip.current
    }
}
