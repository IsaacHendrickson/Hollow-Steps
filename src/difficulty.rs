//! Difficulty tiers, as pure rules: what each tier above 0 adds to enemy and boss health, to the
//! damage they deal and to the coins a run earns (`data/difficulty.ron`).
//!
//! No window and no `GameState` here. The tier is the number of dungeons cleared, kept in the
//! `Profile`; `Difficulty::scale(tier)` turns it into one multiplier for each of the five numbers
//! (1.0 at tier 0). `game::tier_link` layers them on when a world is built and `game::run_link`
//! applies the coin one when a run ends. A tier is never written back to a room or definition
//! file. `docs/run-and-profile.md` has the design.
//!
//! `data/difficulty.ron` is embedded; an `assets/difficulty.ron` replaces it whole
//! (`assets::read_override`). Every field is required, unknown fields are refused and every
//! number is validated when the game loads.

use serde::{Deserialize, Serialize};

use crate::assets::read_override;

pub const FILE_NAME: &str = "difficulty.ron";
const DEFAULTS: &str = include_str!("../data/difficulty.ron");

/// The most percent one tier may add to a number.
pub const MAX_PERCENT: f32 = 100.0;
/// A profile tier above `max_effective_tier` counts as it; its own range.
pub const MIN_EFFECTIVE_TIER: u32 = 1;
pub const MAX_EFFECTIVE_TIER: u32 = 99;

/// What one tier adds (`data/difficulty.ron`). Each percent is the share ADDED per tier above 0,
/// so tier 3 at 10 percent is 1.3 times the base.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Difficulty {
    pub enemy_health_percent: f32,
    pub enemy_damage_percent: f32,
    pub boss_health_percent: f32,
    pub boss_damage_percent: f32,
    pub coin_percent: f32,
    /// A profile tier above this counts as this one.
    pub max_effective_tier: u32,
}

/// The multipliers one tier gives, 1.0 meaning "as authored". Never below 1.0.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TierScale {
    pub enemy_health: f64,
    pub enemy_damage: f64,
    pub boss_health: f64,
    pub boss_damage: f64,
    pub coin: f64,
}

impl TierScale {
    /// Tier 0: every number as authored.
    pub const NEUTRAL: Self = Self {
        enemy_health: 1.0,
        enemy_damage: 1.0,
        boss_health: 1.0,
        boss_damage: 1.0,
        coin: 1.0,
    };

    /// True when nothing is changed (every multiplier is exactly 1.0).
    pub fn is_neutral(&self) -> bool {
        *self == Self::NEUTRAL
    }

    /// An enemy's health at this tier: rounded, never below 1 (a health of 0 stays 0, there is
    /// nothing to scale), and never above what an enemy can hold.
    pub fn enemy_health(&self, base: u16) -> u16 {
        if base == 0 {
            return 0;
        }
        scaled(u32::from(base), self.enemy_health).clamp(1, u32::from(u16::MAX)) as u16
    }

    /// A boss's health at this tier: rounded, never below 1 (0 stays 0).
    pub fn boss_health(&self, base: u32) -> u32 {
        scaled(base, self.boss_health)
    }

    /// Damage an enemy deals at this tier: rounded; 0 stays 0 and anything above 0 stays above 0.
    pub fn enemy_damage(&self, damage: u32) -> u32 {
        scaled(damage, self.enemy_damage)
    }

    /// Damage a boss deals at this tier (same rule as `enemy_damage`).
    pub fn boss_damage(&self, damage: u32) -> u32 {
        scaled(damage, self.boss_damage)
    }

    /// A signed hit's damage; zero and negative damage pass through unchanged.
    pub fn enemy_hit_damage(&self, damage: i32) -> i32 {
        signed(damage, |amount| self.enemy_damage(amount))
    }

    /// A signed boss hit's damage; zero and negative damage pass through unchanged.
    pub fn boss_hit_damage(&self, damage: i32) -> i32 {
        signed(damage, |amount| self.boss_damage(amount))
    }

    /// The coins a run earned, with the tier's bonus: rounded, saturating.
    pub fn coins(&self, earned: u32) -> u32 {
        scaled(earned, self.coin)
    }
}

/// `base` times `multiplier`, rounded to the nearest whole number (halves up). 0 stays 0, a
/// result is never below 1 for a positive `base`, and it saturates at `u32::MAX`. A multiplier of
/// exactly 1.0 returns `base` untouched.
fn scaled(base: u32, multiplier: f64) -> u32 {
    if base == 0 || multiplier == 1.0 {
        return base;
    }
    (f64::from(base) * multiplier)
        .round()
        .clamp(1.0, f64::from(u32::MAX)) as u32
}

fn signed(damage: i32, scale: impl FnOnce(u32) -> u32) -> i32 {
    match u32::try_from(damage) {
        Ok(amount) => i32::try_from(scale(amount)).unwrap_or(i32::MAX),
        Err(_) => damage,
    }
}

impl Default for Difficulty {
    fn default() -> Self {
        Self::from_ron(DEFAULTS).expect("valid difficulty defaults")
    }
}

impl Difficulty {
    /// Parses and validates a file's text.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let difficulty: Self =
            ron::from_str(text).map_err(|error| format!("difficulty: {error}"))?;
        difficulty.validate()?;
        Ok(difficulty)
    }

    /// The `difficulty.ron` in `assets` if there is one (it replaces the defaults whole),
    /// otherwise the embedded `data/difficulty.ron`. A file that exists but cannot be read or is
    /// invalid is an error naming it, never a fallback.
    pub fn load(assets: &std::path::Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => {
                Self::from_ron(DEFAULTS).map_err(|error| format!("data/difficulty.ron: {error}"))
            }
        }
    }

    /// The file text, in the pretty format the editors use.
    pub fn to_ron(&self) -> Result<String, String> {
        crate::definition_editor::serialize(self)
    }

    /// Every percent is a finite number from 0 to 100 and the cap is from 1 to 99. The error
    /// names the first field that is wrong.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("enemy_health_percent", self.enemy_health_percent),
            ("enemy_damage_percent", self.enemy_damage_percent),
            ("boss_health_percent", self.boss_health_percent),
            ("boss_damage_percent", self.boss_damage_percent),
            ("coin_percent", self.coin_percent),
        ] {
            if !value.is_finite() || !(0.0..=MAX_PERCENT).contains(&value) {
                return Err(format!(
                    "{name} must be a number from 0 to 100, not {value}"
                ));
            }
        }
        if !(MIN_EFFECTIVE_TIER..=MAX_EFFECTIVE_TIER).contains(&self.max_effective_tier) {
            return Err(format!(
                "max_effective_tier must be from {MIN_EFFECTIVE_TIER} to {MAX_EFFECTIVE_TIER}, not {}",
                self.max_effective_tier
            ));
        }
        Ok(())
    }

    /// The tier that counts: `tier`, but no more than `max_effective_tier`.
    pub fn effective_tier(&self, tier: u32) -> u32 {
        tier.min(self.max_effective_tier)
    }

    /// The multipliers for a profile `tier`: 1.0 at tier 0, then growing by each percent per
    /// tier, in a straight line up to `max_effective_tier`.
    pub fn scale(&self, tier: u32) -> TierScale {
        let tiers = f64::from(self.effective_tier(tier));
        let grow = |percent: f32| 1.0 + f64::from(percent) * tiers / 100.0;
        TierScale {
            enemy_health: grow(self.enemy_health_percent),
            enemy_damage: grow(self.enemy_damage_percent),
            boss_health: grow(self.boss_health_percent),
            boss_damage: grow(self.boss_damage_percent),
            coin: grow(self.coin_percent),
        }
    }
}

#[cfg(test)]
mod tests;
