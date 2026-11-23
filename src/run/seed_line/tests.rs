use super::*;
use crate::difficulty::TierScale;
use crate::run::{is_tier_line, result_lines, RunOutcome};
use crate::seed::parse_seed;

fn result(outcome: RunOutcome) -> RunResult {
    RunResult {
        outcome,
        coins_earned: 9,
        coins_banked: 4,
        tier_before: 1,
        tier_after: if outcome == RunOutcome::Cleared { 2 } else { 1 },
    }
}

fn profile() -> Profile {
    Profile {
        coins: 25,
        tier: 2,
        ..Profile::default()
    }
}

#[test]
fn the_line_is_seed_and_sixteen_hex_digits() {
    assert_eq!(seed_line(0xAB), "Seed: 00000000000000ab");
    assert_eq!(seed_line(0), "Seed: 0000000000000000");
    assert_eq!(seed_line(u64::MAX), "Seed: ffffffffffffffff");
}

#[test]
fn the_digits_on_the_line_read_back_as_the_seed() {
    for seed in [0, 1, 0xDEAD_BEEF, u64::MAX, 0x0123_4567_89AB_CDEF] {
        let line = seed_line(seed);
        let digits = line.strip_prefix("Seed: ").unwrap();
        assert_eq!(parse_seed(digits), Ok(seed));
    }
}

#[test]
fn the_seed_line_is_the_last_line_of_every_outcome() {
    for outcome in [RunOutcome::Cleared, RunOutcome::Died, RunOutcome::Abandoned] {
        let plain = result_lines(&result(outcome), &profile());
        let lines = result_lines_with_seed(&result(outcome), &profile(), None, Some(0xAB));
        assert_eq!(lines.len(), plain.len() + 1, "{outcome:?}");
        assert_eq!(lines.last().unwrap(), "Seed: 00000000000000ab");
        assert_eq!(
            lines[..plain.len()],
            plain[..],
            "{outcome:?}: the rest is unchanged"
        );
    }
}

#[test]
fn the_seed_comes_after_the_tier_line_and_the_bonus_line() {
    let bonus = CoinBonus::new(
        2,
        &TierScale {
            coin: 1.3,
            ..TierScale::NEUTRAL
        },
    );
    let lines = result_lines_with_seed(
        &result(RunOutcome::Cleared),
        &profile(),
        bonus.as_ref(),
        Some(7),
    );
    assert_eq!(
        lines,
        [
            "Coins earned: 9",
            "Tier 2 bonus: +30%",
            "Coins banked: 4",
            "Total banked: 25",
            "Difficulty tier 1 -> 2",
            "Seed: 0000000000000007"
        ]
    );
    // The drawing gilds the tier line, and the seed line after it must not take its place.
    assert!(is_tier_line(&lines[4]));
    assert!(!is_tier_line(lines.last().unwrap()));
    assert!(
        !is_tier_line(&lines[1]),
        "the bonus line is not the tier line"
    );
}

#[test]
fn a_run_with_no_seed_adds_no_line() {
    let lines = result_lines_with_seed(&result(RunOutcome::Died), &profile(), None, None);
    assert_eq!(lines, result_lines(&result(RunOutcome::Died), &profile()));
    assert!(lines.iter().all(|line| !line.contains("Seed")));
}
