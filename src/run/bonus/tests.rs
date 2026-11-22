use super::*;
use crate::run::RunOutcome;

fn result(outcome: RunOutcome) -> RunResult {
    RunResult {
        outcome,
        coins_earned: 13,
        coins_banked: 13,
        tier_before: 3,
        tier_after: if outcome == RunOutcome::Cleared { 4 } else { 3 },
    }
}

fn scale_with_coin(coin: f64) -> TierScale {
    TierScale {
        coin,
        ..TierScale::NEUTRAL
    }
}

#[test]
fn a_tier_that_pays_a_bonus_says_so_in_whole_percent() {
    let bonus = CoinBonus::new(3, &scale_with_coin(1.0 + 0.10 * 3.0)).unwrap();
    assert_eq!(bonus.percent, 30.0);
    assert_eq!(bonus.line(), "Tier 3 bonus: +30%");
}

#[test]
fn a_fractional_bonus_shows_its_fraction_and_float_noise_never_shows() {
    let bonus = CoinBonus::new(3, &scale_with_coin(1.375)).unwrap();
    assert_eq!(bonus.line(), "Tier 3 bonus: +37.5%");
    let noisy = CoinBonus::new(7, &scale_with_coin(1.0 + 0.07 * 7.0)).unwrap();
    assert_eq!(noisy.line(), "Tier 7 bonus: +49%");
}

#[test]
fn no_bonus_is_no_line() {
    assert_eq!(CoinBonus::new(0, &TierScale::NEUTRAL), None);
    assert_eq!(CoinBonus::new(5, &scale_with_coin(1.0)), None);
    // A change too small to show counts as no bonus.
    assert_eq!(CoinBonus::new(5, &scale_with_coin(1.000_000_1)), None);
}

#[test]
fn the_bonus_line_sits_right_under_the_coins_earned() {
    let profile = Profile::default();
    let bonus = CoinBonus::new(3, &scale_with_coin(1.3)).unwrap();
    let lines = result_lines_with_bonus(&result(RunOutcome::Died), &profile, Some(&bonus));
    assert_eq!(lines[0], "Coins earned: 13");
    assert_eq!(lines[1], "Tier 3 bonus: +30%");
    assert!(lines[2].starts_with("Coins banked"));
    assert_eq!(lines.len(), 4);
}

#[test]
fn without_a_bonus_the_lines_are_exactly_the_old_ones() {
    let profile = Profile::default();
    for outcome in [RunOutcome::Cleared, RunOutcome::Died, RunOutcome::Abandoned] {
        let result = result(outcome);
        assert_eq!(
            result_lines_with_bonus(&result, &profile, None),
            result_lines(&result, &profile)
        );
    }
}

#[test]
fn after_a_clear_the_tier_line_stays_last() {
    let profile = Profile::default();
    let bonus = CoinBonus::new(3, &scale_with_coin(1.3)).unwrap();
    let lines = result_lines_with_bonus(&result(RunOutcome::Cleared), &profile, Some(&bonus));
    assert_eq!(lines.last().unwrap(), "Difficulty tier 3 -> 4");
    assert_eq!(lines.len(), 5);
}
