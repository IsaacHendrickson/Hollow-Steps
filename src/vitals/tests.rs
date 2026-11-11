use super::*;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

const DT: f32 = 0.01;
const MAX_HEALTH: f32 = 50.0;

fn config() -> VitalsConfig {
    VitalsConfig::default()
}

fn with(field: VitalsField, value: f32) -> VitalsConfig {
    config().with_field(field, value).unwrap()
}

fn state() -> VitalsState {
    VitalsState::new(config(), MAX_HEALTH)
}

/// A state with `mana` mana to start with, then `damage` taken.
fn hurt_with_mana(mana: f32, damage: f32) -> VitalsState {
    let mut vitals = VitalsState::new(with(VitalsField::ManaStart, mana), MAX_HEALTH);
    vitals.damage(damage);
    vitals
}

const HOLD: HealInput = HealInput {
    held: true,
    grounded: true,
};
const OPEN: HealInput = HealInput {
    held: false,
    grounded: true,
};
const FALLING_HOLD: HealInput = HealInput {
    held: true,
    grounded: false,
};

fn run(vitals: &mut VitalsState, seconds: f32, input: HealInput) {
    for _ in 0..(seconds / DT).round() as usize {
        vitals.update(DT, input);
    }
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.05,
        "{actual} is not close to {expected}"
    );
}

/// Holds the button until the heal is channeling (a little past the start delay).
fn channeling(mana: f32, damage: f32) -> VitalsState {
    let mut vitals = hurt_with_mana(mana, damage);
    run(&mut vitals, 0.30, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Channeling);
    vitals
}

/// Channeling, then the button let go: waiting out the stop delay.
fn stopping(mana: f32, damage: f32) -> VitalsState {
    let mut vitals = channeling(mana, damage);
    run(&mut vitals, 0.05, HOLD);
    vitals.update(DT, OPEN);
    assert_eq!(vitals.heal_phase(), HealPhase::Stopping);
    vitals
}

// ---- the starting values ----------------------------------------------------------------

#[test]
fn a_new_state_starts_full_with_the_starting_mana_and_no_heal() {
    let vitals = state();
    assert_eq!(vitals.health(), 50.0);
    assert_eq!(vitals.max_health(), 50.0);
    assert_eq!(vitals.trailing_health(), 50.0);
    assert_eq!(vitals.trail_hold_left(), 0.0);
    assert_eq!(vitals.mana(), 0.0);
    assert_eq!(vitals.max_mana(), 100.0);
    assert_eq!(vitals.pending_heal(), 0.0);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert!(!vitals.heal_active() && !vitals.heal_locked());
    assert_eq!(vitals.move_scale(), 1.0);
    let rich = VitalsState::new(with(VitalsField::ManaStart, 40.0), MAX_HEALTH);
    assert_eq!(rich.mana(), 40.0);
}

#[test]
fn the_shipped_defaults_are_the_agreed_starting_values() {
    assert_eq!(
        config(),
        VitalsConfig {
            max_mana: 100.0,
            mana_start: 0.0,
            mana_per_kill: 10.0,
            regain_fraction: 0.5,
            trail_hold: 0.5,
            trail_drain: 40.0,
            heal_start_delay: 0.25,
            heal_stop_delay: 0.2,
            heal_mana_per_second: 30.0,
            health_per_mana: 1.0,
            heal_move_scale: 0.5,
        }
    );
}

// ---- the trailing bar -------------------------------------------------------------------

#[test]
fn damage_drops_health_at_once_and_the_trailing_bar_stays() {
    let mut vitals = state();
    vitals.damage(20.0);
    assert_eq!(vitals.health(), 30.0);
    assert_eq!(vitals.trailing_health(), 50.0);
    assert_eq!(vitals.trail_hold_left(), 0.5);
}

#[test]
fn the_trailing_bar_holds_then_drains_and_never_goes_below_health() {
    let mut vitals = state();
    vitals.damage(20.0);
    run(&mut vitals, 0.45, OPEN);
    close(vitals.trailing_health(), 50.0);
    // 0.1 s past the hold at 40 per second.
    run(&mut vitals, 0.15, OPEN);
    close(vitals.trailing_health(), 46.0);
    for _ in 0..200 {
        vitals.update(DT, OPEN);
        assert!(vitals.trailing_health() >= vitals.health());
    }
    assert_eq!(vitals.trailing_health(), vitals.health());
    assert_eq!(vitals.health(), 30.0);
}

#[test]
fn a_huge_step_drains_the_trail_to_exactly_health() {
    let mut vitals = state();
    vitals.damage(20.0);
    vitals.update(1.0e9, OPEN);
    assert_eq!(vitals.trailing_health(), 30.0);
    assert_eq!(vitals.health(), 30.0);
}

#[test]
fn new_damage_restarts_the_hold_and_keeps_the_trailing_bar() {
    let mut vitals = state();
    vitals.damage(10.0);
    run(&mut vitals, 0.4, OPEN);
    vitals.damage(5.0);
    assert_eq!(vitals.trail_hold_left(), 0.5);
    run(&mut vitals, 0.4, OPEN);
    close(vitals.trailing_health(), 50.0);
    assert_eq!(vitals.health(), 35.0);
}

#[test]
fn damage_stops_at_zero_and_nonsense_amounts_do_nothing() {
    let mut vitals = state();
    vitals.damage(500.0);
    assert_eq!(vitals.health(), 0.0);
    assert_eq!(vitals.trailing_health(), 50.0);
    let before = vitals.clone();
    for amount in [0.0, -5.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        vitals.damage(amount);
        assert_eq!(vitals, before, "{amount}");
    }
}

// ---- regain -----------------------------------------------------------------------------

#[test]
fn regain_takes_back_a_fraction_of_the_gap_and_the_trail_does_not_move() {
    let mut vitals = state();
    vitals.damage(20.0);
    vitals.regain();
    assert_eq!(vitals.health(), 40.0);
    assert_eq!(vitals.trailing_health(), 50.0);
    vitals.regain();
    assert_eq!(vitals.health(), 45.0);
    assert_eq!(vitals.trailing_health(), 50.0);
}

#[test]
fn regain_uses_the_configured_fraction() {
    let mut vitals = VitalsState::new(with(VitalsField::RegainFraction, 1.0), MAX_HEALTH);
    vitals.damage(20.0);
    vitals.regain();
    assert_eq!(vitals.health(), 50.0);
    let mut none = VitalsState::new(with(VitalsField::RegainFraction, 0.0), MAX_HEALTH);
    none.damage(20.0);
    none.regain();
    assert_eq!(none.health(), 30.0);
}

#[test]
fn regain_gives_nothing_without_a_trail() {
    let mut vitals = state();
    vitals.regain();
    assert_eq!(vitals.health(), 50.0);
    vitals.damage(20.0);
    run(&mut vitals, 2.0, OPEN);
    assert_eq!(vitals.trailing_health(), vitals.health());
    vitals.regain();
    assert_eq!(vitals.health(), 30.0);
}

#[test]
fn regain_never_goes_past_max_health() {
    let mut vitals = VitalsState::new(with(VitalsField::RegainFraction, 1.0), MAX_HEALTH);
    vitals.damage(1.0);
    for _ in 0..5 {
        vitals.regain();
        assert!(vitals.health() <= vitals.max_health());
    }
}

// ---- mana -------------------------------------------------------------------------------

#[test]
fn a_kill_gives_mana_per_kill_up_to_the_cap() {
    let mut vitals = state();
    vitals.gain_kill_mana();
    assert_eq!(vitals.mana(), 10.0);
    for _ in 0..30 {
        vitals.gain_kill_mana();
        assert!(vitals.mana() <= vitals.max_mana());
    }
    assert_eq!(vitals.mana(), 100.0);
}

#[test]
fn a_smaller_mana_cap_cuts_mana_and_a_bigger_one_keeps_it() {
    let mut vitals = VitalsState::new(with(VitalsField::ManaStart, 80.0), MAX_HEALTH);
    vitals.set_config(with(VitalsField::MaxMana, 50.0));
    assert_eq!((vitals.mana(), vitals.max_mana()), (50.0, 50.0));
    vitals.set_config(config());
    assert_eq!((vitals.mana(), vitals.max_mana()), (50.0, 100.0));
}

// ---- healing ----------------------------------------------------------------------------

#[test]
fn holding_on_the_ground_waits_the_start_delay_before_any_mana_is_spent() {
    let mut vitals = hurt_with_mana(100.0, 30.0);
    run(&mut vitals, 0.20, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Starting);
    assert!(vitals.heal_active());
    assert_eq!((vitals.mana(), vitals.pending_heal()), (100.0, 0.0));
    run(&mut vitals, 0.10, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Channeling);
    assert!(vitals.mana() < 100.0);
}

#[test]
fn mana_converts_at_the_configured_rate_into_pending_health() {
    // 45 damage leaves 45 of room, so a second of converting is never capped.
    let mut vitals = channeling(100.0, 45.0);
    let (mana, pending) = (vitals.mana(), vitals.pending_heal());
    run(&mut vitals, 1.0, HOLD);
    close(mana - vitals.mana(), 30.0);
    close(vitals.pending_heal() - pending, 30.0);
    assert_eq!(vitals.health(), 5.0, "health waits for the release");
    // Spent mana and pending health always balance at health_per_mana.
    close(100.0 - vitals.mana(), vitals.pending_heal());
}

#[test]
fn health_per_mana_scales_what_each_mana_buys() {
    let config = with(VitalsField::HealthPerMana, 2.0);
    let config = config.with_field(VitalsField::ManaStart, 100.0).unwrap();
    let mut vitals = VitalsState::new(config, 100.0);
    vitals.damage(90.0);
    run(&mut vitals, 0.30, HOLD);
    let (mana, pending) = (vitals.mana(), vitals.pending_heal());
    run(&mut vitals, 1.0, HOLD);
    close(mana - vitals.mana(), 30.0);
    close(vitals.pending_heal() - pending, 60.0);
}

#[test]
fn converting_stops_at_full_health_and_keeps_the_rest_of_the_mana() {
    let mut vitals = channeling(100.0, 10.0);
    run(&mut vitals, 3.0, HOLD);
    close(vitals.pending_heal(), 10.0);
    close(vitals.mana(), 90.0);
    assert_eq!(vitals.heal_phase(), HealPhase::Channeling);
    run(&mut vitals, 0.1, OPEN);
    run(&mut vitals, 0.3, OPEN);
    assert_eq!(vitals.health(), 50.0);
    assert!(vitals.health() <= vitals.max_health());
}

#[test]
fn converting_stops_when_mana_runs_out() {
    let mut vitals = channeling(12.0, 40.0);
    run(&mut vitals, 2.0, HOLD);
    assert_eq!(vitals.mana(), 0.0);
    close(vitals.pending_heal(), 12.0);
    run(&mut vitals, 0.5, OPEN);
    close(vitals.health(), 22.0);
}

#[test]
fn what_is_pending_counts_against_the_room_left_for_health() {
    // Regained health shrinks the room; the pending health still never lifts health past max.
    let mut vitals = channeling(100.0, 30.0);
    run(&mut vitals, 0.5, HOLD);
    vitals.regain();
    run(&mut vitals, 3.0, HOLD);
    run(&mut vitals, 0.5, OPEN);
    assert!(vitals.health() <= vitals.max_health());
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
}

#[test]
fn releasing_waits_the_stop_delay_then_adds_the_pending_health() {
    let mut vitals = channeling(100.0, 30.0);
    run(&mut vitals, 0.5, HOLD);
    let pending = vitals.pending_heal();
    assert!(pending > 10.0);
    run(&mut vitals, 0.10, OPEN);
    assert_eq!(vitals.heal_phase(), HealPhase::Stopping);
    assert_eq!(vitals.health(), 20.0, "still waiting");
    assert_eq!(vitals.pending_heal(), pending, "no more converting");
    run(&mut vitals, 0.15, OPEN);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert_eq!(vitals.pending_heal(), 0.0);
    close(vitals.health(), 20.0 + pending);
    assert!(!vitals.heal_active());
}

#[test]
fn a_zero_stop_delay_adds_the_pending_health_on_the_release_tick() {
    let config = with(VitalsField::HealStopDelay, 0.0)
        .with_field(VitalsField::ManaStart, 100.0)
        .unwrap();
    let mut vitals = VitalsState::new(config, MAX_HEALTH);
    vitals.damage(30.0);
    run(&mut vitals, 0.5, HOLD);
    let pending = vitals.pending_heal();
    vitals.update(DT, OPEN);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    close(vitals.health(), 20.0 + pending);
}

#[test]
fn the_trailing_bar_never_ends_below_health_after_a_heal() {
    let mut vitals = channeling(100.0, 30.0);
    run(&mut vitals, 2.0, HOLD);
    run(&mut vitals, 0.5, OPEN);
    assert!(vitals.trailing_health() >= vitals.health());
}

#[test]
fn letting_go_during_the_start_delay_costs_nothing() {
    let mut vitals = hurt_with_mana(100.0, 30.0);
    run(&mut vitals, 0.10, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Starting);
    vitals.update(DT, OPEN);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert_eq!((vitals.mana(), vitals.pending_heal()), (100.0, 0.0));
    assert_eq!(vitals.health(), 20.0);
    assert!(!vitals.heal_locked(), "no new press is needed");
    run(&mut vitals, 0.10, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Starting);
}

#[test]
fn leaving_the_ground_while_channeling_applies_the_pending_health_at_once() {
    let mut vitals = channeling(100.0, 30.0);
    run(&mut vitals, 0.5, HOLD);
    let pending = vitals.pending_heal();
    assert!(pending > 10.0);
    vitals.update(DT, FALLING_HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert_eq!(vitals.pending_heal(), 0.0);
    close(vitals.health(), 20.0 + pending);
    assert!(!vitals.heal_active());
}

#[test]
fn leaving_the_ground_while_stopping_applies_the_pending_health_without_the_delay() {
    let mut vitals = stopping(100.0, 30.0);
    let pending = vitals.pending_heal();
    vitals.update(
        DT,
        HealInput {
            held: false,
            grounded: false,
        },
    );
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    close(vitals.health(), 20.0 + pending);
}

#[test]
fn leaving_the_ground_during_the_start_delay_ends_the_heal_for_free() {
    let mut vitals = hurt_with_mana(100.0, 30.0);
    run(&mut vitals, 0.10, HOLD);
    vitals.update(DT, FALLING_HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert_eq!(
        (vitals.mana(), vitals.pending_heal(), vitals.health()),
        (100.0, 0.0, 20.0)
    );
}

#[test]
fn nothing_starts_in_the_air() {
    let mut vitals = hurt_with_mana(100.0, 30.0);
    run(&mut vitals, 1.0, FALLING_HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert_eq!(vitals.mana(), 100.0);
}

// ---- damage cancels a heal ----------------------------------------------------------------

fn assert_cancelled(vitals: &VitalsState, mana: f32, health_after: f32) {
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert!(!vitals.heal_active());
    assert_eq!(vitals.pending_heal(), 0.0, "pending health is lost");
    assert_eq!(vitals.mana(), mana, "mana is not refunded");
    assert_eq!(vitals.health(), health_after);
    assert!(vitals.heal_locked());
    assert_eq!(vitals.move_scale(), 1.0);
}

#[test]
fn damage_during_the_start_delay_cancels_the_heal() {
    let mut vitals = hurt_with_mana(100.0, 20.0);
    run(&mut vitals, 0.10, HOLD);
    vitals.damage(5.0);
    assert_cancelled(&vitals, 100.0, 25.0);
}

#[test]
fn damage_while_channeling_loses_the_pending_health_and_keeps_the_spent_mana() {
    let mut vitals = channeling(100.0, 20.0);
    run(&mut vitals, 0.5, HOLD);
    let (mana, pending) = (vitals.mana(), vitals.pending_heal());
    assert!(mana < 100.0 && pending > 10.0);
    vitals.damage(5.0);
    assert_cancelled(&vitals, mana, 25.0);
}

#[test]
fn damage_while_stopping_loses_the_pending_health() {
    let mut vitals = stopping(100.0, 20.0);
    let mana = vitals.mana();
    assert!(vitals.pending_heal() > 0.0);
    vitals.damage(5.0);
    assert_cancelled(&vitals, mana, 25.0);
    run(&mut vitals, 1.0, OPEN);
    assert_eq!(vitals.health(), 25.0, "nothing is added later");
}

#[test]
fn a_cancelled_heal_needs_the_button_let_go_and_pressed_again() {
    let mut vitals = channeling(100.0, 20.0);
    vitals.damage(5.0);
    run(&mut vitals, 2.0, HOLD);
    assert_eq!(
        vitals.heal_phase(),
        HealPhase::Idle,
        "still held: no new heal"
    );
    assert!(vitals.heal_locked());
    vitals.update(DT, OPEN);
    assert!(!vitals.heal_locked());
    run(&mut vitals, 0.05, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Starting);
}

#[test]
fn a_release_before_the_hurt_frees_the_button_on_the_next_tick() {
    let mut vitals = stopping(100.0, 20.0);
    vitals.damage(5.0);
    vitals.update(DT, OPEN);
    assert!(!vitals.heal_locked());
    vitals.update(DT, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Starting);
}

#[test]
fn damage_with_no_heal_running_does_not_lock_the_button() {
    let mut vitals = state();
    vitals.damage(5.0);
    assert!(!vitals.heal_locked());
}

// ---- movement scale -----------------------------------------------------------------------

#[test]
fn the_move_scale_applies_through_every_part_of_a_heal() {
    let mut vitals = hurt_with_mana(100.0, 20.0);
    assert_eq!(vitals.move_scale(), 1.0);
    run(&mut vitals, 0.10, HOLD);
    assert_eq!(vitals.move_scale(), 0.5);
    run(&mut vitals, 0.30, HOLD);
    assert_eq!(vitals.move_scale(), 0.5);
    vitals.update(DT, OPEN);
    assert_eq!(vitals.move_scale(), 0.5);
    run(&mut vitals, 0.30, OPEN);
    assert_eq!(vitals.move_scale(), 1.0);
}

// ---- respawn ------------------------------------------------------------------------------

#[test]
fn respawn_resets_everything_to_the_starting_values() {
    let mut vitals = VitalsState::new(with(VitalsField::ManaStart, 25.0), MAX_HEALTH);
    vitals.damage(30.0);
    vitals.gain_kill_mana();
    run(&mut vitals, 0.6, HOLD);
    assert!(vitals.heal_active() && vitals.pending_heal() > 0.0);
    vitals.respawn();
    assert_eq!(vitals.health(), MAX_HEALTH);
    assert_eq!(vitals.trailing_health(), MAX_HEALTH);
    assert_eq!(vitals.trail_hold_left(), 0.0);
    assert_eq!(vitals.mana(), 25.0);
    assert_eq!(vitals.max_mana(), 100.0);
    assert_eq!(vitals.pending_heal(), 0.0);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert_eq!(vitals.config(), &with(VitalsField::ManaStart, 25.0));
}

#[test]
fn a_button_still_held_through_a_respawn_does_not_start_a_heal() {
    let mut vitals = channeling(100.0, 20.0);
    vitals.respawn();
    assert!(vitals.heal_locked());
    run(&mut vitals, 1.0, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    vitals.update(DT, OPEN);
    run(&mut vitals, 0.05, HOLD);
    assert_eq!(vitals.heal_phase(), HealPhase::Starting);
}

// ---- odd time -----------------------------------------------------------------------------

#[test]
fn zero_negative_and_non_finite_steps_let_no_time_pass() {
    let mut vitals = channeling(100.0, 30.0);
    run(&mut vitals, 0.3, HOLD);
    vitals.damage(1.0);
    run(&mut vitals, 0.3, OPEN);
    let mut channel = channeling(100.0, 30.0);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let before = channel.clone();
        channel.update(dt, HOLD);
        assert_eq!(channel, before, "{dt}");
        let before = vitals.clone();
        vitals.update(dt, OPEN);
        assert_eq!(vitals, before, "{dt}");
    }
}

#[test]
fn a_very_large_step_keeps_every_number_finite_and_in_range() {
    let mut vitals = channeling(100.0, 30.0);
    vitals.update(f32::MAX, HOLD);
    for value in [
        vitals.health(),
        vitals.trailing_health(),
        vitals.mana(),
        vitals.pending_heal(),
        vitals.trail_hold_left(),
    ] {
        assert!(value.is_finite());
    }
    assert!(vitals.mana() >= 0.0 && vitals.pending_heal() >= 0.0);
    assert!(vitals.health() + vitals.pending_heal() <= vitals.max_health() + 1e-3);
    vitals.update(f32::MAX, OPEN);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    assert!(vitals.health() <= vitals.max_health());
}

// ---- the data file ------------------------------------------------------------------------

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-vitals-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const DEFAULTS: &str = include_str!("../../data/vitals.ron");

#[test]
fn the_defaults_parse_and_validate() {
    assert!(VitalsConfig::from_ron(DEFAULTS).is_ok());
    assert!(config().validate().is_ok());
}

#[test]
fn parsing_is_strict() {
    let unknown = DEFAULTS.replace("max_mana: 100.0,", "max_mana: 100.0, wind: 3.0,");
    assert!(VitalsConfig::from_ron(&unknown)
        .unwrap_err()
        .contains("wind"));
    let missing = DEFAULTS.replace("trail_drain: 40.0,", "");
    assert!(VitalsConfig::from_ron(&missing).is_err());
    assert!(VitalsConfig::from_ron("").is_err());
    assert!(VitalsConfig::from_ron("(max_mana: 1.0)").is_err());
}

#[test]
fn out_of_range_and_non_finite_values_are_refused_naming_the_field() {
    for (from, to, key) in [
        (
            "regain_fraction: 0.5",
            "regain_fraction: 1.5",
            "vitals.regain_fraction",
        ),
        (
            "regain_fraction: 0.5",
            "regain_fraction: -0.1",
            "vitals.regain_fraction",
        ),
        (
            "trail_drain: 40.0",
            "trail_drain: 0.0",
            "vitals.trail_drain",
        ),
        ("trail_hold: 0.5", "trail_hold: NaN", "vitals.trail_hold"),
        (
            "heal_move_scale: 0.5",
            "heal_move_scale: 2.0",
            "vitals.heal_move_scale",
        ),
        (
            "health_per_mana: 1.0",
            "health_per_mana: 0.0",
            "vitals.health_per_mana",
        ),
        (
            "heal_mana_per_second: 30.0",
            "heal_mana_per_second: inf",
            "vitals.heal_mana_per_second",
        ),
        (
            "heal_start_delay: 0.25",
            "heal_start_delay: -1.0",
            "vitals.heal_start_delay",
        ),
        ("max_mana: 100.0", "max_mana: 0.0", "vitals.max_mana"),
    ] {
        let error = VitalsConfig::from_ron(&DEFAULTS.replace(from, to)).unwrap_err();
        assert!(error.contains(key), "{to}: {error}");
    }
}

#[test]
fn starting_mana_may_not_exceed_the_mana_cap() {
    let text = DEFAULTS.replace("mana_start: 0.0", "mana_start: 150.0");
    let error = VitalsConfig::from_ron(&text).unwrap_err();
    assert!(error.contains("mana_start"), "{error}");
}

#[test]
fn every_field_accepts_its_own_limits_and_with_field_refuses_outside_them() {
    let wide = with(VitalsField::MaxMana, 10_000.0);
    for field in VitalsField::ALL {
        let spec = field.spec();
        assert!(config().with_field(field, spec.min).is_ok(), "{}", spec.key);
        assert!(wide.with_field(field, spec.max).is_ok(), "{}", spec.key);
        assert!(
            config().with_field(field, spec.max * 2.0 + 1.0).is_err(),
            "{}",
            spec.key
        );
        assert!(
            config().with_field(field, f32::NAN).is_err(),
            "{}",
            spec.key
        );
        assert_eq!(field.get(&with_exact(field, spec.min)), spec.min);
    }
    assert!(config().with_field(VitalsField::ManaStart, 101.0).is_err());
}

fn with_exact(field: VitalsField, value: f32) -> VitalsConfig {
    config().with_field(field, value).unwrap()
}

#[test]
fn with_field_changes_one_number_and_leaves_the_rest() {
    let next = config().with_field(VitalsField::TrailDrain, 25.0).unwrap();
    assert_eq!(next.trail_drain, 25.0);
    assert_eq!(
        VitalsConfig {
            trail_drain: 40.0,
            ..next
        },
        config()
    );
}

#[test]
fn load_uses_the_embedded_defaults_without_a_file_and_the_file_whole_when_there_is_one() {
    let dir = TempDir::new();
    assert_eq!(VitalsConfig::load(&dir.0).unwrap(), config());
    let text = DEFAULTS.replace("mana_per_kill: 10.0", "mana_per_kill: 25.0");
    std::fs::write(dir.0.join(FILE_NAME), text).unwrap();
    assert_eq!(VitalsConfig::load(&dir.0).unwrap().mana_per_kill, 25.0);
}

#[test]
fn a_bad_override_file_is_an_error_naming_it_never_a_fallback() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(max_mana: 1.0)").unwrap();
    let error = VitalsConfig::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
    // A folder where the file should be cannot be read: also an error, not a silent default.
    let blocked = TempDir::new();
    std::fs::create_dir(blocked.0.join(FILE_NAME)).unwrap();
    assert!(VitalsConfig::load(&blocked.0)
        .unwrap_err()
        .contains(FILE_NAME));
}

#[test]
fn save_writes_a_file_that_loads_back_and_leaves_no_temporary_file() {
    let dir = TempDir::new();
    let changed = with(VitalsField::ManaPerKill, 25.0);
    let path = save(&dir.0, &changed).unwrap();
    assert_eq!(path, dir.0.join(FILE_NAME));
    assert_eq!(VitalsConfig::load(&dir.0).unwrap(), changed);
    assert!(!dir.0.join("vitals.ron.tmp").exists());
    // Saving again replaces the file.
    save(&dir.0, &config()).unwrap();
    assert_eq!(VitalsConfig::load(&dir.0).unwrap(), config());
}

#[test]
fn save_refuses_invalid_values_and_writes_nothing() {
    let dir = TempDir::new();
    let bad = VitalsConfig {
        regain_fraction: 2.0,
        ..config()
    };
    let error = save(&dir.0, &bad).unwrap_err();
    assert!(error.contains("vitals.regain_fraction"), "{error}");
    assert!(!dir.0.join(FILE_NAME).exists());
}

#[test]
fn a_failed_save_keeps_the_previous_file() {
    let dir = TempDir::new();
    save(&dir.0, &config()).unwrap();
    let before = std::fs::read_to_string(dir.0.join(FILE_NAME)).unwrap();
    // A folder where the temporary file goes makes the staged write fail.
    std::fs::create_dir(dir.0.join("vitals.ron.tmp")).unwrap();
    let error = save(&dir.0, &with(VitalsField::TrailDrain, 10.0)).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
    assert_eq!(
        std::fs::read_to_string(dir.0.join(FILE_NAME)).unwrap(),
        before
    );
}
