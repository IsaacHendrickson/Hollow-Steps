//! The dungeon generator's rules (`docs/dungeon-gen.md`): the `generator` section of
//! `variety.ron`. This is only the numbers and their checks; `dungeon_gen` reads them, and
//! `enabled` is what `decide_generation` looks at when a run starts (`game::generate_link`).
//!
//! The section is optional in a file (a `variety.ron` from before it still loads, with the
//! defaults below), and so is each field in it. Whole numbers are signed here so that a negative
//! one is refused by `validate` with the field named, not by the parser without it. Unknown
//! fields are refused.
use serde::Deserialize;

use super::flip::MAX_PERCENT;

/// The fewest and most rooms a run may have, start and boss included.
pub const MIN_ROOMS: i32 = 3;
pub const MAX_ROOMS: i32 = 40;
/// The fewest rooms on the main path from start to boss, both included.
pub const MIN_SPINE: i32 = 2;
/// The most keys (and so locks) a run may have: keys are `Key 1` to `Key 8`.
pub const MAX_KEYS: i32 = 8;
/// How many times the generator may start over before it gives up.
pub const MIN_ATTEMPTS: i32 = 1;
pub const MAX_ATTEMPTS: i32 = 10_000;

/// An inclusive range of whole numbers, `min` to `max`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CountRange {
    pub min: i32,
    pub max: i32,
}

impl CountRange {
    /// Both ends from `low` to `high`, and `min` not above `max`. `field` names the range in the
    /// error (`rooms`, `spine`, `keys`).
    fn check(&self, field: &str, low: i32, high: i32) -> Result<(), String> {
        for (end, value) in [("min", self.min), ("max", self.max)] {
            if !(low..=high).contains(&value) {
                return Err(format!(
                    "generator.{field}.{end} must be a whole number from {low} to {high}, not {value}"
                ));
            }
        }
        if self.min > self.max {
            return Err(format!(
                "generator.{field}.min is {}, above generator.{field}.max, which is {}",
                self.min, self.max
            ));
        }
        Ok(())
    }

    /// `(min, max)` as counts. Only meaningful once `validate` has passed: a negative end
    /// would read as 0.
    pub fn bounds(&self) -> (usize, usize) {
        (
            usize::try_from(self.min).unwrap_or(0),
            usize::try_from(self.max).unwrap_or(0),
        )
    }
}

/// What the `generator` section of `variety.ron` says.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GeneratorRules {
    /// Whether a run is built by the generator when F4's "Generate next run" says Auto. Off by
    /// default.
    pub enabled: bool,
    /// Rooms in a run, start and boss included: `MIN_ROOMS` to `MAX_ROOMS`.
    pub rooms: CountRange,
    /// Rooms on the main path from start to boss, both included: `MIN_SPINE` or more, and the
    /// most no higher than `rooms.max`.
    pub spine: CountRange,
    /// Keys, and so locked links, in a run: 0 to `MAX_KEYS`.
    pub keys: CountRange,
    /// Chance, in percent, that a piece is placed mirrored: a finite number from 0 to 100.
    pub mirror_percent: f32,
    /// How many tries the generator gets before it reports failure: `MIN_ATTEMPTS` to
    /// `MAX_ATTEMPTS`.
    pub attempts: i32,
}

impl Default for GeneratorRules {
    /// What a file that says nothing gets. `data/variety.ron` spells the same numbers out.
    fn default() -> Self {
        Self {
            enabled: false,
            rooms: CountRange { min: 6, max: 12 },
            spine: CountRange { min: 4, max: 7 },
            keys: CountRange { min: 0, max: 2 },
            mirror_percent: 50.0,
            attempts: 200,
        }
    }
}

impl GeneratorRules {
    /// Every number in range, each range in order, and the spine no longer than the most rooms.
    /// The error names the field (`generator.rooms.min`, `generator.attempts`, ...).
    pub fn validate(&self) -> Result<(), String> {
        self.rooms.check("rooms", MIN_ROOMS, MAX_ROOMS)?;
        self.spine.check("spine", MIN_SPINE, MAX_ROOMS)?;
        if self.spine.max > self.rooms.max {
            return Err(format!(
                "generator.spine.max is {}, above generator.rooms.max, which is {}",
                self.spine.max, self.rooms.max
            ));
        }
        self.keys.check("keys", 0, MAX_KEYS)?;
        let percent = self.mirror_percent;
        if !percent.is_finite() || !(0.0..=MAX_PERCENT).contains(&percent) {
            return Err(format!(
                "generator.mirror_percent must be a number from 0 to 100, not {percent}"
            ));
        }
        if !(MIN_ATTEMPTS..=MAX_ATTEMPTS).contains(&self.attempts) {
            return Err(format!(
                "generator.attempts must be a whole number from {MIN_ATTEMPTS} to {MAX_ATTEMPTS}, \
                 not {}",
                self.attempts
            ));
        }
        Ok(())
    }

    /// The tries the generator gets. Only meaningful once `validate` has passed.
    pub fn attempt_limit(&self) -> u32 {
        u32::try_from(self.attempts).unwrap_or(0)
    }
}

/// What F4's "Generate next run" row says: the same three answers as the flip rows.
pub type GenerateChoice = super::flip::FlipChoice;

/// Whether the next run is built by the generator (`docs/dungeon-gen.md`). `Never` plays the
/// fixed dungeon whatever the rules say. `Always` asks the generator even when the rules have it
/// off or the pool has no Start or Boss piece, so a pool that cannot build comes back with the
/// generator's own reason instead of a silent fixed dungeon. `Auto` generates when `enabled` is
/// set in `variety.ron` and the pool has both a Start and a Boss piece (`pool_has_ends`), and
/// plays the fixed dungeon otherwise.
pub fn decide_generation(
    choice: GenerateChoice,
    rules: &GeneratorRules,
    pool_has_ends: bool,
) -> bool {
    match choice {
        GenerateChoice::Never => false,
        GenerateChoice::Always => true,
        GenerateChoice::Auto => rules.enabled && pool_has_ends,
    }
}

#[cfg(test)]
mod tests;
