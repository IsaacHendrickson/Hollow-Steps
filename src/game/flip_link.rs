//! How `GameState` owns the dungeon flip's choice (`crate::variety`, `docs/variety.md`): the rules
//! from `variety.ron`, what F4's "Flip next run" says, and whether the run in progress is flipped.
//!
//! The decision is made once, in `start_run`, from the run's seed and kept for the run's life
//! (through the result screen). The hub, a game with no hub and a test room are never flipped.
//! `variety.ron` is read at start-up and F5 does not re-read it, like `difficulty.ron`.
use std::path::Path;

use macroquad::math::Vec2;

use super::GameState;
use crate::room::Exit;
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

impl GameState {
    /// Where the player arrives after `exit` of the room being played, once the rooms have been
    /// rebuilt: the `spawn` of the rebuilt room's exit with the same edge, offset or door and
    /// target (the same exit by its `ExitRef`). `exit` is the copy from before the rebuild, whose
    /// `spawn` is not mirrored. Falls back to that copy's `spawn` if the room has no such exit.
    pub(super) fn rebuilt_arrival(&self, exit: &Exit) -> Vec2 {
        let same = |other: &&Exit| {
            other.target_room == exit.target_room
                && other.edge == exit.edge
                && other.offset == exit.offset
                && other.door == exit.door
        };
        let rebuilt = self.room().exits.iter().find(same);
        rebuilt.map_or(exit.spawn, |rebuilt| rebuilt.spawn)
    }

    /// A run can start with the player already outside the hub (the editor put him there), and
    /// the rooms it builds are mirrored, so his old spot is no spot of theirs: he stands at the
    /// room's spawn instead. An unflipped run leaves him where he is, as it always did.
    pub(super) fn land_in_flipped_room(&mut self) {
        if self.run.flip.current {
            let (spawn, bounds) = (self.room().spawn, self.world_bounds());
            self.player.place_at(spawn, bounds);
        }
    }

    /// F2 and Open room refuse while the run in progress is flipped. The editor reads the room
    /// files, so it would show the unflipped dungeon, and Save would put that over the flipped
    /// rooms being played. The hub, an unflipped run and a test room are not affected.
    pub(super) fn refuse_editor_when_flipped(&self) -> Result<(), String> {
        if self.run_flipped() {
            return Err(
                "this run is flipped (the dungeon is mirrored), and the editor works on the \
                 unflipped files: abandon the run from the pause menu, or set Flip next run to \
                 Never in F4 and start a new run, then open the editor"
                    .into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod play_tests;
