//! How `GameState` owns the charms (`crate::charms`): the one function that puts the player
//! and vitals numbers into effect, and equipping from the inventory.
//!
//! The base numbers live in the `PlayerDraft` (what `player.ron`, `vitals.ron`, F4 and the
//! editor's Player panel read and write). The player and the vitals run on those numbers with
//! the equipped charms layered on top (`charms::effective`), and every place that changes either
//! side (a new game, F5, F4, the editor, a discard, a vitals reload, equipping) ends in
//! `apply_stats`, so the two can never disagree. Owned and equipped charms belong to the run:
//! a respawn or a room change does not touch them.
use macroquad::math::Vec2;

use super::GameState;
use crate::charm_menu::{self, CharmMenu, InventoryLayout};
use crate::charms::{effective_with, Charm, CharmCatalog, CharmState, Effect, Toggle};
use crate::hud_bars::BarValues;
use crate::upgrades::permanent_effects;

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

    /// Maximum health without charms: `max_health` as `player.ron` and the Player panel have it,
    /// plus the health the permanent upgrades give. The bars measure a charm's change against
    /// this, so the divider and the locked end show only what charms do.
    pub fn base_max_health(&self) -> f32 {
        let (tuning, _) = effective_with(
            self.player_draft.edited(),
            self.player_draft.edited_vitals(),
            &[],
            &self.permanent_effects(),
        );
        f32::from(tuning.player.max_health)
    }

    /// The effect of every upgrade level bought. Only a game with a hub has them: with none the
    /// profile is never used, so the game plays as it always did.
    fn permanent_effects(&self) -> Vec<Effect> {
        if self.run_config().has_hub() {
            permanent_effects(self.profile(), &self.upgrades)
        } else {
            Vec::new()
        }
    }

    /// What the health and mana bars show: the vitals, and the base maximums the bars measure a
    /// charm's change against (more capacity after the base part, or a locked end).
    pub fn bar_values(&self) -> BarValues {
        BarValues::from(&self.vitals).with_base(self.base_max_health(), self.base_max_mana())
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

    /// The cursor and the last refusal of the inventory's Charms section.
    pub fn charm_menu(&self) -> &CharmMenu {
        &self.charm_menu
    }

    /// The charms the player owns, in the order the inventory lists them.
    pub fn owned_charms(&self) -> Vec<&Charm> {
        charm_menu::owned_charms(&self.charms, &self.charm_state)
    }

    /// The selected charm of the inventory list, if a charm is owned.
    pub fn selected_charm(&self) -> Option<&Charm> {
        let owned = self.owned_charms();
        owned.get(self.charm_menu.selected(owned.len())?).copied()
    }

    /// The maximum mana now and after toggling the selected charm (equal when the toggle would
    /// be refused); none when no charm is owned.
    pub fn charm_mana_preview(&self) -> Option<(f32, f32)> {
        let id = &self.selected_charm()?.id;
        let base = self.base_max_mana();
        let now = self.charm_state.max_mana(&self.charms, base);
        let after = charm_menu::max_mana_after(&self.charms, &self.charm_state, id, base);
        Some((now, after))
    }

    /// Where everything in the open inventory goes on a `view`-sized screen.
    pub fn inventory_layout(&self, view: Vec2) -> InventoryLayout {
        let items = self.player.collected_items().len();
        let count = self.owned_charms().len();
        let visible = charm_menu::visible_rows(view, items, count);
        let first = self.charm_menu.first(count, visible);
        charm_menu::layout(view, items, count, first)
    }

    /// Up or down in the inventory's charm list (negative is up), wrapping around. Does nothing
    /// while the inventory is closed.
    pub fn charm_step(&mut self, steps: i32, view: Vec2) {
        if !self.inventory_open {
            return;
        }
        let items = self.player.collected_items().len();
        let count = self.owned_charms().len();
        let visible = charm_menu::visible_rows(view, items, count);
        self.charm_menu.step(count, steps, visible);
    }

    /// Selects the charm at `index` of the list (the mouse resting on its row). Does nothing
    /// while the inventory is closed.
    pub fn charm_hover(&mut self, index: usize) {
        if self.inventory_open {
            let count = self.owned_charms().len();
            self.charm_menu.select(index, count);
        }
    }

    /// Equips the selected charm, or takes it off. A refusal changes nothing and leaves its
    /// message in `charm_menu`. None while the inventory is closed or when no charm is owned.
    pub fn charm_press(&mut self) -> Option<Toggle> {
        if !self.inventory_open {
            return None;
        }
        let base_max_mana = self.base_max_mana();
        let outcome = self
            .charm_menu
            .toggle(&self.charms, &mut self.charm_state, base_max_mana)?;
        if matches!(outcome, Toggle::Equipped | Toggle::Unequipped) {
            self.apply_stats();
        }
        Some(outcome)
    }

    /// A click on the charm at `index`: selects it, then equips or takes it off.
    pub fn charm_click(&mut self, index: usize) -> Option<Toggle> {
        self.charm_hover(index);
        let count = self.owned_charms().len();
        if index >= count {
            return None;
        }
        self.charm_press()
    }

    /// Puts the base numbers plus the permanent upgrades and the equipped charms into effect: the
    /// player's movement, health and sword, the vitals' mana, and every enemy's knockback. The one
    /// place that does it.
    ///
    /// The player keeps its position, velocity and timers. A raised maximum health adds the same
    /// health, a lowered one cuts the excess (`Health::set_limits`,
    /// `VitalsState::set_max_health`); mana is cut to a lower maximum and never refilled.
    pub(super) fn apply_stats(&mut self) {
        let equipped = self.charm_state.equipped_charms(&self.charms);
        let (tuning, vitals) = effective_with(
            self.player_draft.edited(),
            self.player_draft.edited_vitals(),
            &equipped,
            &self.permanent_effects(),
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
