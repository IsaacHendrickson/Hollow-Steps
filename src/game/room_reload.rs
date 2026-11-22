//! Re-reading the assets folder while the game runs (`GameState::reload_rooms`, F5). The rooms
//! come back with the layer of the world in play over them, so a rebuilt room matches the others;
//! `difficulty.ron` is not re-read.
use super::{world_load, GameState};
use crate::charms::CharmCatalog;
use crate::feel::FeelConfig;
use crate::tuning::TuningConfig;
use crate::upgrades::UpgradeCatalog;
use crate::vitals::VitalsConfig;

impl GameState {
    /// Re-read every room and its enemy definitions before applying any changes.
    /// Enemy gameplay restarts; matching body points, player, pickups, and unchanged bosses stay intact.
    /// Changed boss definitions restart from their authored placement.
    ///
    /// The player and enemy tunables in `player.ron`, the hit-stop and shake values in
    /// `feel.ron`, the vitals values in `vitals.ron`, the charms in `charms.ron` and the upgrades
    /// in `upgrades.ron` are re-read too; the player keeps its state (health, mana and a heal in progress included) and the
    /// charms it wears stay on, layered on the re-read numbers. Nothing changes if any file fails
    /// to load.
    pub fn reload_rooms(&mut self) -> Result<(), String> {
        if self.test_run.is_some() {
            return Err("leave the test room (F2) before reloading".into());
        }
        let tuning = TuningConfig::load(&self.assets)?;
        let feel = FeelConfig::load(&self.assets)?;
        let vitals = VitalsConfig::load(&self.assets)?;
        let charms = CharmCatalog::load(&self.assets, vitals.max_mana)?;
        let upgrades = UpgradeCatalog::load(&self.assets)?;
        let (rooms, terrain_styles) =
            world_load::load_world(&self.assets, &charms, &self.tiers.layer)?;
        for mut reloaded in rooms {
            if let Some(current) = self.rooms.iter_mut().find(|room| room.id == reloaded.id) {
                // Prefer exact placements before matching moved placements by name/occurrence.
                let mut available = current.enemies.iter().map(Some).collect::<Vec<_>>();
                let mut matched = vec![false; reloaded.enemies.len()];
                for (index, enemy) in reloaded.enemies.iter_mut().enumerate() {
                    if let Some(previous) = available.iter_mut().find(|old| {
                        old.is_some_and(|old| old.placement_key() == enemy.placement_key())
                    }) {
                        enemy.preserve_body_motion_from(previous.take().unwrap());
                        matched[index] = true;
                    }
                }
                for (index, enemy) in reloaded.enemies.iter_mut().enumerate() {
                    if !matched[index] {
                        if let Some(previous) = available
                            .iter_mut()
                            .find(|old| old.is_some_and(|old| old.name() == enemy.name()))
                        {
                            enemy.preserve_body_motion_from(previous.take().unwrap());
                        }
                    }
                }
                current.enemies = std::mem::take(&mut reloaded.enemies);
                // The layers and the terrain look are only a picture, so a reload can swap them.
                current.background = std::mem::take(&mut reloaded.background);
                current.terrain_style = std::mem::take(&mut reloaded.terrain_style);
                let same_boss = match (&current.boss_special, &reloaded.boss_special) {
                    (Some(current), Some(reloaded)) => current.same_definition(reloaded),
                    (None, None) => true,
                    _ => false,
                };
                if !same_boss {
                    current.boss_special = reloaded.boss_special.take();
                }
            } else {
                self.rooms.push(reloaded);
            }
        }
        self.set_tuning(tuning)?;
        self.terrain_styles = terrain_styles;
        self.player_draft.mark_saved();
        self.feel.set_config(feel);
        self.player_draft.reload_vitals(vitals);
        // Charms you hold that the file no longer lists are dropped; the rest stay on.
        self.charms = charms;
        self.upgrades = upgrades;
        self.charm_state.retain_known(&self.charms);
        // An open Charms or Upgrades panel with nothing unsaved shows the re-read list.
        self.refresh_catalog_drafts();
        self.prune_owned_charm_placements();
        self.apply_stats();
        Ok(())
    }
}
