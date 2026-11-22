//! The coin bonus of a difficulty tier, as the result screen tells it: "Tier 3 bonus: +30%".
//! Pure text; `game::run_link` keeps the bonus of the run that just ended and `hud_run` draws it.
use super::screen::result_lines;
use super::RunResult;
use crate::difficulty::TierScale;
use crate::profile::Profile;

/// The extra coins a finished run was paid for the tier it was played at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoinBonus {
    /// The profile tier the run was played at (what the player knows as "Tier N").
    pub tier: u32,
    /// The bonus in percent, above zero (for example 30.0 for x1.3).
    pub percent: f64,
}

impl CoinBonus {
    /// The bonus a tier's `scale` pays, or `None` while it pays nothing (tier 0, or a coin
    /// percent of 0). Rounded to two decimals so float noise never shows.
    pub fn new(tier: u32, scale: &TierScale) -> Option<Self> {
        let percent = ((scale.coin - 1.0) * 10_000.0).round() / 100.0;
        (percent > 0.0).then_some(Self { tier, percent })
    }

    /// "Tier 3 bonus: +30%"; a fraction shows as it is ("+37.5%").
    pub fn line(&self) -> String {
        format!("Tier {} bonus: +{}%", self.tier, self.percent)
    }
}

/// The result screen's lines (`result_lines`) with the tier's bonus line right under the coins
/// earned, when there is a bonus.
pub fn result_lines_with_bonus(
    result: &RunResult,
    profile: &Profile,
    bonus: Option<&CoinBonus>,
) -> Vec<String> {
    let mut lines = result_lines(result, profile);
    if let Some(bonus) = bonus {
        lines.insert(1, bonus.line());
    }
    lines
}

#[cfg(test)]
mod tests;
