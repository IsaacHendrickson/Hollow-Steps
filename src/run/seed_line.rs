//! The result screen's seed line: "Seed: 00000000000000ab", the last line of the screen, so a
//! run's enemies can be found again by typing it into F4's fixed seed (`docs/variety.md`).
//! Pure text; `hud_run` draws it.
use super::bonus::result_lines_with_bonus;
use super::{CoinBonus, RunResult};
use crate::profile::Profile;
use crate::seed::format_seed;

/// The line that tells the player the run's seed: the 16 hex digits `parse_seed` reads back.
pub fn seed_line(seed: u64) -> String {
    format!("Seed: {}", format_seed(seed))
}

/// The result screen's lines (`result_lines_with_bonus`) with the seed line last, when the run
/// has a seed.
pub fn result_lines_with_seed(
    result: &RunResult,
    profile: &Profile,
    bonus: Option<&CoinBonus>,
    seed: Option<u64>,
) -> Vec<String> {
    let mut lines = result_lines_with_bonus(result, profile, bonus);
    lines.extend(seed.map(seed_line));
    lines
}

#[cfg(test)]
mod tests;
