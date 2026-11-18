//! How `GameState` owns the charms (`crate::charms`): the one function that puts the player
//! and vitals numbers into effect, and equipping from the inventory.
//!
//! The base numbers live in the `PlayerDraft` (what `player.ron`, `vitals.ron`, F4 and the
//! editor's Player panel read and write). The player and the vitals run on those numbers with
//! the equipped charms layered on top (`charms::effective`), and every place that changes either
//! side (a new game, F5, F4, the editor, a discard, a vitals reload, equipping) ends in
//! `apply_stats`, so the two can never disagree. Owned and equipped charms belong to the run:
//! a respawn or a room change does not touch them.
use super::GameState;
use crate::charms::{effective, CharmCatalog, CharmState, Toggle};

impl GameState {
    /// Every charm there is.
    pub fn charm_catalog(&self) -> &CharmCatalog {
        &self.charms
    }

    /// Which charms are owned and which are equipped.
    pub fn charm_state(&self) -> &CharmState {
        &self.charm_state
    }

    /// Maximum mana without charms: `max_mana` as `vitals.ron` and the Player panel have it.
    /// A charm's cost is held against this, not against the lowered maximum.
    pub fn base_max_mana(&self) -> f32 {
        self.player_draft.edited_vitals().max_mana
    }

    /// Maximum health without charms: `max_health` as `player.ron` and the Player panel have it.
    pub fn base_max_health(&self) -> f32 {
        f32::from(self.player_draft.edited().player.max_health)
    }

    /// Equips the charm `id`, or takes it off if it is on. Equipping is refused (and nothing
    /// changes) when the equipped charms plus it would cost more than the base maximum mana.
    /// The new numbers are in effect at once; the player keeps its position and timers, and
    /// nothing is refilled.
    pub fn toggle_charm(&mut self, id: &str) -> Toggle {
        let base_max_mana = self.base_max_mana();
        let outcome = self.charm_state.toggle(&self.charms, id, base_max_mana);
        if matches!(outcome, Toggle::Equipped | Toggle::Unequipped) {
            self.apply_stats();
        }
        outcome
    }

    /// Puts the base numbers plus the equipped charms into effect: the player's movement, health
    /// and sword, the vitals' mana, and every enemy's knockback. The one place that does it.
    ///
    /// The player keeps its position, velocity and timers. A raised maximum health adds the same
    /// health, a lowered one cuts the excess (`Health::set_limits`,
    /// `VitalsState::set_max_health`); mana is cut to a lower maximum and never refilled.
    pub(super) fn apply_stats(&mut self) {
        let equipped = self.charm_state.equipped_charms(&self.charms);
        let (tuning, vitals) = effective(
            self.player_draft.edited(),
            self.player_draft.edited_vitals(),
            &equipped,
        );
        self.player.set_config(tuning.player);
        self.vitals.set_config(vitals);
        self.vitals
            .set_max_health(f32::from(tuning.player.max_health));
        for enemy in self.rooms.iter_mut().flat_map(|room| &mut room.enemies) {
            enemy.set_tuning(tuning.enemy);
        }
    }
}
