//! How `GameState` owns the difficulty layer (`crate::difficulty`, `docs/run-and-profile.md`):
//! the rules from `difficulty.ron`, the tier the rooms were built at and the multipliers that
//! come with it, and the coin bonus of the run that just ended.
//!
//! The rule is that a tier is layered on when rooms are built and never written back. Every
//! place that builds rooms for play (`world_load`, a run's start, the F5 reload, the editor's
//! Save, a test room) puts a `Layer` over them; the editor session and the room files never
//! see one. A run holds one layer from its start to its end: the profile's tier is read when the
//! run starts (`start_run`), so changing it in F4 takes effect at the next run. In the hub and in
//! a game with no hub the layer is `Layer::BASE`, which changes nothing.
use super::GameState;
use crate::difficulty::{Difficulty, TierScale};
use crate::room::Room;
use crate::run::CoinBonus;

/// The tier a set of rooms was built at, and what it multiplies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Layer {
    /// The profile's tier when the layer was made: what the player knows as "Tier N".
    pub(super) profile_tier: u32,
    /// The tier that counts: `profile_tier`, but no more than `max_effective_tier`.
    pub(super) tier: u32,
    pub(super) scale: TierScale,
}

impl Layer {
    /// Tier 0: nothing is changed. The hub, and a game with no hub.
    pub(super) const BASE: Self = Self {
        profile_tier: 0,
        tier: 0,
        scale: TierScale::NEUTRAL,
    };

    /// Layers the tier over every room of a freshly built world.
    pub(super) fn apply(&self, rooms: &mut [Room]) {
        for room in rooms {
            self.apply_room(room);
        }
    }

    /// Layers the tier over one freshly built room: enemy and boss health scaled.
    pub(super) fn apply_room(&self, room: &mut Room) {
        for enemy in &mut room.enemies {
            enemy.scale_health(&self.scale);
        }
        if let Some(boss) = &mut room.boss_special {
            boss.scale_health(&self.scale);
        }
    }
}

/// The rules, the layer the rooms in play were built with, and the last run's coin bonus.
#[derive(Debug)]
pub(super) struct TierState {
    /// `difficulty.ron` as loaded at start-up; F5 does not re-read it.
    rules: Difficulty,
    pub(super) layer: Layer,
    /// What the run that just ended was paid for its tier; `None` when it was paid nothing.
    pub(super) bonus: Option<CoinBonus>,
}

impl TierState {
    pub(super) fn new(rules: Difficulty) -> Self {
        Self {
            rules,
            layer: Layer::BASE,
            bonus: None,
        }
    }

    /// The layer for a profile at `tier`.
    pub(super) fn layer_for(&self, tier: u32) -> Layer {
        Layer {
            profile_tier: tier,
            tier: self.rules.effective_tier(tier),
            scale: self.rules.scale(tier),
        }
    }
}

impl GameState {
    /// What a tier adds (`difficulty.ron`).
    pub fn difficulty(&self) -> &Difficulty {
        &self.tiers.rules
    }

    /// The multipliers the rooms in play were built with: neutral in the hub and in a game with
    /// no hub, the run's tier during a run, the profile's tier in a test room.
    pub fn tier_scale(&self) -> TierScale {
        self.tiers.layer.scale
    }

    /// The tier that counts in the rooms in play (the profile's tier, capped).
    pub fn world_tier(&self) -> u32 {
        self.tiers.layer.tier
    }

    /// The layer for the profile as it is now.
    pub(super) fn profile_layer(&self) -> Layer {
        self.tiers.layer_for(self.run.profile.tier)
    }

    /// The coin bonus of the run whose result is showing, if it was paid one.
    pub fn result_bonus(&self) -> Option<&CoinBonus> {
        self.run_result().and(self.tiers.bonus.as_ref())
    }
}
