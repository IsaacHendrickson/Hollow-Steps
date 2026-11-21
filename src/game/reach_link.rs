//! The reach line after a save of the player numbers (`crate::reach`, `docs/reach.md`): when a
//! save changes the numbers a jump is measured from, the game keeps one line saying what the
//! player can now cross, for the Player panel and F4 to show next to "Saved to ...".
use super::GameState;
use crate::player::PlayerConfig;
use crate::reach::{line, Interval};
use crate::upgrades::max_totals;

impl GameState {
    /// The player numbers on disk now.
    pub(super) fn saved_player_numbers(&self) -> PlayerConfig {
        self.player_draft.saved().player
    }

    /// After a save of `player.ron`: when the numbers differ from `before`, the reach line for
    /// the saved numbers with the charms and upgrades the game has; otherwise no line.
    pub(super) fn note_reach(&mut self, before: PlayerConfig) {
        self.reach_note = (self.saved_player_numbers() != before).then(|| {
            let totals = max_totals(&self.upgrades);
            let saved = self.player_draft.saved();
            let vitals = self.player_draft.saved_vitals();
            line(&Interval::new(saved, vitals, &self.charms, &totals))
        });
    }

    /// The reach line the last save left, once: `Some` right after a save that changed the
    /// numbers behind it, for the window that saved to show.
    pub fn take_reach_note(&mut self) -> Option<String> {
        self.reach_note.take()
    }
}
