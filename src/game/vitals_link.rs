//! How `GameState` drives the player vitals (`crate::vitals`): the input the player acts on
//! while a heal runs, health handed back to the player by healing and sword regain, and the
//! tunable accessors the F4 window and F5 reload use.
//!
//! `Health` stays the authority for damage, invincibility and death. Healing and regain change
//! the vitals' own `f32` health, which is then copied into `Health` (rounded up, so a player
//! with any health left is never shown or treated as dead). Damage goes through `Health` first;
//! the vitals only hear about it afterwards.
use std::path::PathBuf;

use super::GameState;
use crate::enemy_attack::DamageHit;
use crate::health::MAX_HEALTH;
use crate::player::{Player, PlayerInput};
use crate::vitals::{self, HealInput, VitalsConfig, VitalsState};

/// Vitals at full health with `config`, as at the start of the game or after a respawn.
pub(super) fn fresh_vitals(config: VitalsConfig) -> VitalsState {
    VitalsState::new(config, f32::from(MAX_HEALTH))
}

/// Copies the vitals' health into the player's `Health` if it changed since `before`.
pub(super) fn mirror_health(player: &mut Player, vitals: &VitalsState, before: f32) {
    if vitals.health() != before {
        player.set_health_current(vitals.health().ceil() as u16);
    }
}

/// A sword hit on an enemy or the boss: regain part of the trailing bar's gap.
pub(super) fn regain(player: &mut Player, vitals: &mut VitalsState) {
    let before = vitals.health();
    vitals.regain();
    mirror_health(player, vitals, before);
}

/// The damage `Player::take_attack_damage` applied for `hit`.
pub(super) fn applied_damage(hit: DamageHit) -> f32 {
    f32::from(hit.damage.clamp(0, i32::from(u16::MAX)) as u16)
}

impl GameState {
    /// The player vitals: health with its trailing bar, mana and the heal in progress.
    pub fn vitals(&self) -> &VitalsState {
        &self.vitals
    }

    /// The vitals tunables in effect.
    pub fn vitals_config(&self) -> &VitalsConfig {
        self.vitals.config()
    }

    /// True when the vitals tunables in effect differ from `vitals.ron` (or the defaults).
    pub fn vitals_unsaved(&self) -> bool {
        *self.vitals.config() != self.saved_vitals
    }

    /// Puts new vitals tunables into effect at once. Health, mana and a heal in progress are
    /// kept. Invalid values are refused and nothing changes.
    pub fn set_vitals_config(&mut self, config: VitalsConfig) -> Result<(), String> {
        config.validate()?;
        self.vitals.set_config(config);
        Ok(())
    }

    /// Writes the vitals tunables in effect to `vitals.ron` in the assets folder and returns
    /// its path.
    pub fn save_vitals(&mut self) -> Result<PathBuf, String> {
        let path = vitals::save(&self.assets, self.vitals.config())?;
        self.saved_vitals = *self.vitals.config();
        Ok(path)
    }

    /// The input the player acts on this tick. While a heal runs the player walks at
    /// `heal_move_scale` of its speed and cannot jump, dash or attack; a swing or dash already
    /// under way finishes.
    pub(super) fn input_for_player(&self, input: PlayerInput) -> PlayerInput {
        if !self.vitals.heal_active() {
            return input;
        }
        let axis = if input.move_axis.is_finite() {
            input.move_axis.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        PlayerInput {
            move_axis: axis * self.vitals.move_scale(),
            jump_pressed: false,
            jump_held: false,
            attack_pressed: false,
            dash_pressed: false,
            ..input
        }
    }

    /// Advances the vitals by one tick, after the player has moved (the heal needs to know
    /// whether the player is on the ground).
    pub(super) fn update_vitals(&mut self, input: PlayerInput, delta_seconds: f32) {
        let before = self.vitals.health();
        self.vitals.update(
            delta_seconds,
            HealInput {
                held: input.heal_held,
                grounded: self.player.is_grounded(),
            },
        );
        mirror_health(&mut self.player, &self.vitals, before);
    }
}
