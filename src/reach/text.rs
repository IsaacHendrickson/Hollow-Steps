//! The reach line the Player panel and F4 show after a save that changed the numbers behind it.
use super::Interval;

/// `jump 183 px, dash jump 296 px; run Check dungeon`: the longest gap a player without Dash can
/// cross (every buff at its maximum) and the longest with it, and what to do about the gates
/// marked against the old numbers. An interval that is empty says why instead
/// (`Interval::error`): no gap can need Dash with these numbers.
pub fn line(interval: &Interval) -> String {
    interval.error().unwrap_or_else(|| {
        format!(
            "jump {:.0} px, dash jump {:.0} px; run Check dungeon",
            interval.low(),
            interval.high()
        )
    })
}

#[cfg(test)]
mod tests;
