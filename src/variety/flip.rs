//! The dungeon flip's rules and the choice of whether a run flips (`docs/variety.md`): the
//! `variety.ron` file, and `decide_flip`, a pure function of the player's F4 choice, the rules and
//! the run's seed.
//!
//! `data/variety.ron` is embedded; an `assets/variety.ron` replaces it whole (`assets::read_override`).
//! Every field is required, unknown fields are refused and the number is validated when the game
//! loads, like `difficulty.ron`. F5 does not re-read it, and the F4 row never writes it.
use serde::Deserialize;

use crate::assets::read_override;
use crate::seed::stream;

pub const FILE_NAME: &str = "variety.ron";
const DEFAULTS: &str = include_str!("../../data/variety.ron");
/// What the draw of the flip is for: the label of its stream (key 0, there is one draw a run).
const LABEL: &str = "flip";
/// The most a percent can be.
pub const MAX_PERCENT: f32 = 100.0;

/// What `variety.ron` says.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VarietyRules {
    /// Percent of runs that play the dungeon mirrored left to right: a finite number from 0
    /// (never) to 100 (always).
    pub flip_percent: f32,
}

impl Default for VarietyRules {
    fn default() -> Self {
        Self::from_ron(DEFAULTS).expect("valid variety defaults")
    }
}

impl VarietyRules {
    /// Parses and validates a file's text.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let rules: Self = ron::from_str(text).map_err(|error| format!("variety: {error}"))?;
        rules.validate()?;
        Ok(rules)
    }

    /// The `variety.ron` in `assets` if there is one (it replaces the defaults whole), otherwise
    /// the embedded `data/variety.ron`. A file that exists but cannot be read or is invalid is an
    /// error naming it, never a fallback.
    pub fn load(assets: &std::path::Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Self::from_ron(DEFAULTS).map_err(|error| format!("data/variety.ron: {error}")),
        }
    }

    /// `flip_percent` is a finite number from 0 to 100. The error names the field.
    pub fn validate(&self) -> Result<(), String> {
        let percent = self.flip_percent;
        if !percent.is_finite() || !(0.0..=MAX_PERCENT).contains(&percent) {
            return Err(format!(
                "flip_percent must be a number from 0 to 100, not {percent}"
            ));
        }
        Ok(())
    }
}

/// What F4's "Flip next run" row says. `Auto` leaves it to the rules and the seed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FlipChoice {
    /// The rules decide: a run flips when its seed's draw is under `flip_percent`.
    #[default]
    Auto,
    /// No run flips, whatever the rules say.
    Never,
    /// Every run flips, whatever the rules say.
    Always,
}

impl FlipChoice {
    pub const ALL: [FlipChoice; 3] = [Self::Auto, Self::Never, Self::Always];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto",
            Self::Never => "Never",
            Self::Always => "Always",
        }
    }
}

/// Whether the run with `seed` flips. `Never` and `Always` override the rules; `Auto` flips when
/// the first number of `stream(seed, "flip", 0)`, as a percent, is under `flip_percent`. So 0
/// never flips, 100 always does, and the same seed always gives the same answer.
pub fn decide_flip(choice: FlipChoice, rules: &VarietyRules, seed: u64) -> bool {
    match choice {
        FlipChoice::Never => false,
        FlipChoice::Always => true,
        FlipChoice::Auto => {
            // Exact in f64: the draw is below 1, so below 100 as a percent, and 0 is below nothing.
            let draw = f64::from(stream(seed, LABEL, 0).unit()) * 100.0;
            draw < f64::from(rules.flip_percent)
        }
    }
}

#[cfg(test)]
mod tests;
