//! How `GameState` owns the difficulty layer (`crate::difficulty`, `docs/run-and-profile.md`):
//! the rules from `difficulty.ron`, the tier the rooms were built at and the multipliers that
//! come with it, and the coin bonus of the run that just ended.
//!
//! The rule is that a tier is layered on when rooms are built and never written back. Every
//! place that builds rooms for play (`world_load`, a run's start, the F5 reload, the editor's
//! Save, a test room) puts a `Layer` over them; the editor session and the room files never
//! see one, which is also why Save keeps a placement the tier left out. A run holds one layer from its start to its end: the profile's tier is read when the
//! run starts (`start_run`), so changing it in F4 takes effect at the next run. In the hub and in
//! a game with no hub the layer is `Layer::BASE`, which changes nothing.
use super::GameState;
use crate::difficulty::{Difficulty, TierScale};
use crate::room::Room;
use crate::run::{CoinBonus, RunPhase};
use crate::variety;

/// The tier a set of rooms was built at, and what it multiplies.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Layer {
    /// The profile's tier when the layer was made: what the player knows as "Tier N".
    pub(super) profile_tier: u32,
    /// The tier that counts: `profile_tier`, but no more than `max_effective_tier`.
    pub(super) tier: u32,
    pub(super) scale: TierScale,
    /// The run's seed, which decides the optional enemy placements (`crate::variety`). `None`
    /// leaves every placement in: the hub, a test room, and a game with no hub.
    pub(super) seed: Option<u64>,
    /// The run plays the dungeon mirrored (`world_load` flips every room but the hub). Kept with
    /// the layer so every rebuild during the run, F5 included, makes the same world.
    pub(super) flip: bool,
}

impl Layer {
    /// Tier 0: nothing is changed. The hub, and a game with no hub.
    pub(super) const BASE: Self = Self {
        profile_tier: 0,
        tier: 0,
        scale: TierScale::NEUTRAL,
        seed: None,
        flip: false,
    };

    /// The same layer for a run with `seed`: the optional placements are decided by it.
    pub(super) fn with_seed(self, seed: u64) -> Self {
        Self {
            seed: Some(seed),
            ..self
        }
    }

    /// The same layer for a run that plays the dungeon mirrored, or not.
    pub(super) fn with_flip(self, flip: bool) -> Self {
        Self { flip, ..self }
    }

    /// Layers the tier over every room of a freshly built world.
    pub(super) fn apply(&self, rooms: &mut [Room]) {
        for room in rooms {
            self.apply_room(room);
        }
    }

    /// Layers the tier over one freshly built room: placements whose `min_tier` is above the
    /// tier are left out (a boss left out locks no exit), then, for a run with a seed, the
    /// optional placements the seed decides against (`variety::thin`; a boss is never touched),
    /// and the rest have their health scaled.
    pub(super) fn apply_room(&self, room: &mut Room) {
        if self.seed.is_some() {
            // Numbered over every placement, before the tier leaves any out.
            variety::mark_identities(&mut room.enemies);
        }
        room.enemies.retain(|enemy| enemy.min_tier() <= self.tier);
        if let Some(seed) = self.seed {
            variety::thin(&mut room.enemies, room.id, seed);
        }
        if room
            .boss_special
            .as_ref()
            .is_some_and(|boss| boss.min_tier() > self.tier)
        {
            room.boss_special = None;
        }
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
    /// The rules the next run starts with: `difficulty.ron` as loaded at start-up, then what the
    /// F4 window set. F5 does not re-read the file.
    rules: Difficulty,
    /// The rules as the file has them (loaded, or last saved by F4).
    saved: Difficulty,
    pub(super) layer: Layer,
    /// What the run that just ended was paid for its tier; `None` when it was paid nothing.
    pub(super) bonus: Option<CoinBonus>,
}

impl TierState {
    pub(super) fn new(rules: Difficulty) -> Self {
        Self {
            rules,
            saved: rules,
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
            seed: None,
            flip: false,
        }
    }
}

impl GameState {
    /// What a tier adds (`difficulty.ron`).
    pub fn difficulty(&self) -> &Difficulty {
        &self.tiers.rules
    }

    /// Sets the rules the next run starts with (F4's Difficulty section). Rules that fail
    /// `Difficulty::validate` are refused and nothing changes. The run in progress keeps the
    /// layer it started with, so the numbers it plays at do not move.
    pub fn set_difficulty(&mut self, rules: Difficulty) -> Result<(), String> {
        rules.validate()?;
        self.tiers.rules = rules;
        Ok(())
    }

    /// Whether the rules differ from `difficulty.ron` as loaded or last saved.
    pub fn difficulty_unsaved(&self) -> bool {
        self.tiers.rules != self.tiers.saved
    }

    /// Writes the rules to `assets/difficulty.ron` through a staged file and returns its path.
    /// On a failure the old file stays and the rules stay unsaved.
    pub fn save_difficulty(&mut self) -> Result<std::path::PathBuf, String> {
        let text = self.tiers.rules.to_file_text()?;
        let path = self.assets.join(crate::difficulty::FILE_NAME);
        crate::tuning::write_staged(&path, &text)?;
        self.tiers.saved = self.tiers.rules;
        Ok(path)
    }

    /// The tier the HUD shows: the profile's in the hub, the run's during a run (and in a test
    /// room, the profile's); nothing with no hub or on the result screen.
    pub fn hud_tier(&self) -> Option<u32> {
        match self.run_phase()? {
            RunPhase::Hub => Some(self.run.profile.tier),
            RunPhase::Running => Some(self.tiers.layer.profile_tier),
            RunPhase::Result(_) => None,
        }
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
