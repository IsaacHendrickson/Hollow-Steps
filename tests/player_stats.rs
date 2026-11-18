//! The player's health, sword and mana numbers: where they come from (`player.ron`, `vitals.ron`),
//! their ranges, and the rules they drive, apart from the game loop.
use hollow_steps::combat::{
    Attack, MeleeConfig, StrikeDirection, ATTACK_COOLDOWN, SWING_SECONDS, SWORD_DAMAGE,
};
use hollow_steps::enemy::Enemy;
use hollow_steps::health::{Health, INVINCIBILITY_SECONDS, MAX_HEALTH};
use hollow_steps::player::{Player, PlayerConfig, WorldBounds};
use hollow_steps::player_editor::PlayerDraft;
use hollow_steps::tuning::{EnemyTuning, Field, TuningConfig};
use hollow_steps::vitals::{VitalsConfig, VitalsField};
use macroquad::math::Vec2;

const NEW_FIELDS: [Field; 5] = [
    Field::MaxHealth,
    Field::InvincibilitySeconds,
    Field::StrikeDamage,
    Field::StrikeSwingSeconds,
    Field::StrikeCooldown,
];

// ---- the file ---------------------------------------------------------------------------------

#[test]
fn the_shipped_file_says_what_the_constants_used_to() {
    let config = TuningConfig::default();
    assert_eq!(config.player.max_health, MAX_HEALTH);
    assert_eq!(config.player.invincibility_seconds, INVINCIBILITY_SECONDS);
    assert_eq!(config.player.melee.damage, SWORD_DAMAGE);
    assert_eq!(config.player.melee.swing_seconds, SWING_SECONDS);
    assert_eq!(config.player.melee.cooldown, ATTACK_COOLDOWN);
    assert_eq!(config.player.melee, MeleeConfig::default());
    assert_eq!(
        Health::default(),
        Health::new(MAX_HEALTH, INVINCIBILITY_SECONDS)
    );
}

/// A `player.ron` written before health, immunity, damage, swing time and cooldown were in it.
const OLD_FILE: &str = r#"(
    player: (
        move_speed: 220.0, jump_speed: 480.0, gravity: 1400.0, max_fall_speed: 900.0,
        held_jump_gravity_multiplier: 0.5, coyote_seconds: 0.1, jump_buffer_seconds: 0.12,
        width: 32.0, height: 48.0, dash_speed: 800.0, dash_duration: 0.2, dash_cooldown: 0.5,
    ),
    strike: (
        forward_width: 38.0, forward_height: 32.0, up_width: 40.0, up_height: 38.0,
        down_width: 40.0, down_height: 38.0, pogo_speed: 450.0,
    ),
    enemy: (default_health: 30, knockback_speed: 260.0, knockback_seconds: 0.16),
)"#;

#[test]
fn an_older_file_without_the_new_numbers_loads_with_todays_values() {
    let old = TuningConfig::parse(OLD_FILE, "old player.ron").unwrap();
    assert_eq!(old, TuningConfig::default());
}

#[test]
fn the_new_numbers_have_ranges_and_refuse_values_outside_them() {
    let config = TuningConfig::default();
    for (field, bad) in [
        (Field::MaxHealth, 0.0),
        (Field::MaxHealth, 100_000.0),
        (Field::InvincibilitySeconds, -0.1),
        (Field::InvincibilitySeconds, 60.0),
        (Field::StrikeDamage, 0.0),
        (Field::StrikeDamage, 100_000.0),
        (Field::StrikeSwingSeconds, 0.0),
        (Field::StrikeCooldown, -1.0),
        (Field::StrikeCooldown, f32::NAN),
        (Field::MaxHealth, f32::INFINITY),
    ] {
        let error = config.with_field(field, bad).unwrap_err();
        assert!(error.contains(field.spec().key), "{field:?} {bad}: {error}");
    }
    // The edges of each range are allowed.
    for field in NEW_FIELDS {
        let spec = field.spec();
        assert!(config.with_field(field, spec.min).is_ok(), "{}", spec.key);
        assert!(config.with_field(field, spec.max).is_ok(), "{}", spec.key);
    }
}

#[test]
fn health_and_damage_are_whole_numbers() {
    let config = TuningConfig::default();
    for field in [Field::MaxHealth, Field::StrikeDamage] {
        assert!(field.spec().integer, "{field:?}");
    }
    let edited = config.with_field(Field::MaxHealth, 80.4).unwrap();
    assert_eq!(edited.player.max_health, 80);
    let edited = config.with_field(Field::StrikeDamage, 24.6).unwrap();
    assert_eq!(edited.player.melee.damage, 25);
}

#[test]
fn the_new_numbers_survive_save_and_load() {
    let edited = TuningConfig::default()
        .with_field(Field::MaxHealth, 80.0)
        .and_then(|c| c.with_field(Field::InvincibilitySeconds, 0.5))
        .and_then(|c| c.with_field(Field::StrikeDamage, 25.0))
        .and_then(|c| c.with_field(Field::StrikeSwingSeconds, 0.2))
        .and_then(|c| c.with_field(Field::StrikeCooldown, 0.6))
        .unwrap();
    let text = edited.to_ron().unwrap();
    for key in ["max_health: 80", "damage: 25", "cooldown: 0.6"] {
        assert!(text.contains(key), "{key}\n{text}");
    }
    assert_eq!(TuningConfig::parse(&text, "round trip").unwrap(), edited);
}

#[test]
fn dash_length_is_speed_times_duration_and_follows_both() {
    let config = TuningConfig::default().player;
    assert_eq!(config.dash_distance(), 160.0);
    let faster = TuningConfig::default()
        .with_field(Field::DashSpeed, 1_000.0)
        .and_then(|c| c.with_field(Field::DashDuration, 0.3))
        .unwrap();
    assert!((faster.player.dash_distance() - 300.0).abs() < 1e-3);
}

// ---- health -----------------------------------------------------------------------------------

#[test]
fn a_player_starts_and_respawns_at_its_max_health() {
    let mut health = Health::new(80, 1.0);
    assert_eq!((health.current(), health.max()), (80, 80));
    assert!(health.take_damage_amount(30));
    assert_eq!(health.current(), 50);
    health.respawn();
    assert_eq!(health.current(), 80);
    assert!(health.is_invincible(), "a respawn gives the immunity time");
}

#[test]
fn healing_never_goes_above_the_configured_max() {
    let mut health = Health::new(80, 0.0);
    health.set_current(500);
    assert_eq!(health.current(), 80);
}

#[test]
fn raising_the_max_adds_the_same_health_and_lowering_it_cuts_the_excess() {
    let mut health = Health::new(50, 0.0);
    health.set_limits(80, 0.0);
    assert_eq!(health.current(), 80, "full stays full");
    assert!(health.take_damage_amount(30));
    health.set_limits(100, 0.0);
    assert_eq!((health.current(), health.max()), (70, 100));
    health.set_limits(60, 0.0);
    assert_eq!((health.current(), health.max()), (60, 60));
    health.set_limits(60, 0.0);
    assert_eq!(health.current(), 60, "the same limits again change nothing");
}

#[test]
fn raising_the_max_never_brings_back_a_player_with_no_health() {
    let mut health = Health::new(10, 0.0);
    assert!(health.take_damage_amount(10));
    assert_eq!(health.current(), 0);
    health.set_limits(100, 0.0);
    assert_eq!(health.current(), 0);
}

#[test]
fn the_largest_max_does_not_overflow() {
    let mut health = Health::new(u16::MAX - 1, 0.0);
    health.set_limits(u16::MAX, 0.0);
    assert_eq!(health.current(), u16::MAX);
}

#[test]
fn the_immunity_time_is_the_one_given() {
    let mut health = Health::new(50, 0.25);
    assert!(health.take_damage_amount(10));
    health.update(0.2);
    assert!(health.is_invincible());
    assert!(!health.take_damage_amount(10), "no second hit while immune");
    health.update(0.1);
    assert!(!health.is_invincible());
    assert!(health.take_damage_amount(10));

    let mut none = Health::new(50, 0.0);
    assert!(none.take_damage_amount(10));
    assert!(!none.is_invincible());
    assert!(none.take_damage_amount(10));
}

#[test]
fn a_player_built_from_a_config_uses_its_health_numbers() {
    let config = PlayerConfig {
        max_health: 80,
        invincibility_seconds: 0.5,
        ..PlayerConfig::default()
    };
    let mut player = Player::new(Vec2::ZERO, config);
    assert_eq!(player.health().current(), 80);
    assert!(player.take_damage_amount(10));
    player.set_config(PlayerConfig {
        max_health: 120,
        ..config
    });
    assert_eq!(player.health().max(), 120);
    assert_eq!(player.health().current(), 110, "raised by the 40 added");
}

// ---- the sword --------------------------------------------------------------------------------

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

/// A code-made enemy with `health` health.
fn enemy(health: u16) -> Enemy {
    let tuning = EnemyTuning {
        default_health: health,
        ..TuningConfig::default().enemy
    };
    Enemy::new(200.0, 300.0, 100.0, 0.0, BOUNDS).spawned_with(tuning)
}

#[test]
fn a_hit_takes_the_damage_it_is_given_and_the_plain_hit_the_shipped_one() {
    let mut target = enemy(100);
    assert!(target.hit_for(1, 1.0, 25));
    assert_eq!(target.health(), 75);
    assert!(target.hit(2, 1.0));
    assert_eq!(target.health(), 75 - SWORD_DAMAGE);
}

#[test]
fn one_swing_hits_once_and_a_hit_cannot_take_more_than_is_left() {
    let mut target = enemy(100);
    assert!(target.hit_for(7, 1.0, 25));
    assert!(!target.hit_for(7, 1.0, 25), "the same swing");
    assert_eq!(target.health(), 75);
    assert!(target.hit_for(8, 1.0, u16::MAX));
    assert_eq!(target.health(), 0);
    assert!(!target.hit_for(9, 1.0, 25), "a dead enemy cannot be hit");
}

fn timed(swing_seconds: f32, cooldown: f32) -> MeleeConfig {
    MeleeConfig {
        damage: 10,
        swing_seconds,
        cooldown,
    }
}

#[test]
fn a_swing_lasts_its_swing_time_and_waits_out_its_cooldown() {
    let melee = timed(0.3, 0.5);
    let mut attack = Attack::default();
    let size = Vec2::new(32.0, 48.0);
    attack.update_timed(true, 1.0, StrikeDirection::Forward, 0.0, &melee);
    assert_eq!(attack.id(), 1);
    attack.update_timed(false, 1.0, StrikeDirection::Forward, 0.25, &melee);
    assert!(attack.hitbox(Vec2::ZERO, size).is_some());
    attack.update_timed(false, 1.0, StrikeDirection::Forward, 0.1, &melee);
    assert!(attack.hitbox(Vec2::ZERO, size).is_none());
    // 0.35 s in: the 0.5 s cooldown is not over, so a press does nothing.
    attack.update_timed(true, 1.0, StrikeDirection::Forward, 0.0, &melee);
    assert_eq!(attack.id(), 1);
    attack.update_timed(true, 1.0, StrikeDirection::Forward, 0.2, &melee);
    assert_eq!(attack.id(), 2);
}

#[test]
fn a_cooldown_shorter_than_the_swing_lets_a_new_swing_cut_in() {
    let melee = timed(0.5, 0.1);
    let mut attack = Attack::default();
    attack.update_timed(true, 1.0, StrikeDirection::Forward, 0.0, &melee);
    attack.update_timed(true, 1.0, StrikeDirection::Forward, 0.2, &melee);
    assert_eq!(attack.id(), 2);
}

#[test]
fn the_older_update_calls_use_the_shipped_timing() {
    let mut attack = Attack::default();
    attack.update(true, 1.0, 0.0);
    assert_eq!(attack.id(), 1);
    attack.update(true, 1.0, ATTACK_COOLDOWN - 0.01);
    assert_eq!(attack.id(), 1);
    attack.update(true, 1.0, 0.02);
    assert_eq!(attack.id(), 2);
}

// ---- the draft's vitals -----------------------------------------------------------------------

#[test]
fn the_vitals_are_clean_until_one_differs_and_the_stats_stay_clean_meanwhile() {
    let mut draft = PlayerDraft::default();
    assert!(!draft.is_dirty() && !draft.is_vitals_dirty());
    draft
        .set_vitals_field(VitalsField::ManaPerKill, 25.0)
        .unwrap();
    assert!(draft.is_dirty() && draft.is_vitals_dirty());
    assert!(!draft.is_stats_dirty());
    assert!(draft.is_vitals_field_dirty(VitalsField::ManaPerKill));
    assert!(!draft.is_vitals_field_dirty(VitalsField::MaxMana));
    assert_eq!(draft.edited_vitals().mana_per_kill, 25.0);
    assert_eq!(draft.saved_vitals().mana_per_kill, 10.0);

    draft.reset_vitals_field(VitalsField::ManaPerKill).unwrap();
    assert!(!draft.is_dirty());

    draft
        .set_vitals_field(VitalsField::HealMoveScale, 0.8)
        .unwrap();
    draft.set_field(Field::MoveSpeed, 250.0).unwrap();
    draft.reset_all();
    assert!(!draft.is_dirty(), "reset all covers the vitals too");
}

#[test]
fn a_vitals_value_outside_its_range_or_across_fields_is_refused() {
    let mut draft = PlayerDraft::default();
    for (field, bad) in [
        (VitalsField::ManaPerKill, -1.0),
        (VitalsField::ManaPerKill, 1.0e9),
        (VitalsField::MaxMana, 0.0),
        (VitalsField::RegainFraction, f32::NAN),
    ] {
        let error = draft.set_vitals_field(field, bad).unwrap_err();
        assert!(error.contains(field.spec().key), "{field:?}: {error}");
    }
    assert!(!draft.is_dirty(), "a refused value changes nothing");

    // The starting mana may not be above the cap.
    draft
        .set_vitals_field(VitalsField::ManaStart, 60.0)
        .unwrap();
    let error = draft
        .set_vitals_field(VitalsField::MaxMana, 40.0)
        .unwrap_err();
    assert!(
        error.contains("mana_start") && error.contains("max_mana"),
        "{error}"
    );
    assert_eq!(draft.edited_vitals().max_mana, 100.0);
}

#[test]
fn a_reset_that_would_break_the_cap_rule_is_refused_and_leaves_the_value() {
    let mut draft = PlayerDraft::new(TuningConfig::default()).with_vitals(
        VitalsConfig::default()
            .with_field(VitalsField::ManaStart, 80.0)
            .unwrap(),
    );
    draft
        .set_vitals_field(VitalsField::ManaStart, 20.0)
        .unwrap();
    draft.set_vitals_field(VitalsField::MaxMana, 30.0).unwrap();
    // Saved: start 80, cap 100. Putting the start back to 80 under a cap of 30 is not allowed.
    let error = draft
        .reset_vitals_field(VitalsField::ManaStart)
        .unwrap_err();
    assert!(error.contains("mana_start"), "{error}");
    assert_eq!(draft.edited_vitals().mana_start, 20.0);
    draft.reset_vitals_field(VitalsField::MaxMana).unwrap();
    draft.reset_vitals_field(VitalsField::ManaStart).unwrap();
    assert!(!draft.is_dirty());
}

#[test]
fn the_vitals_save_on_their_own_and_a_stats_save_leaves_them_unsaved() {
    let dir =
        std::env::temp_dir().join(format!("hollow-steps-draft-vitals-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut draft = PlayerDraft::default();
    draft
        .set_vitals_field(VitalsField::ManaPerKill, 25.0)
        .unwrap();
    draft.set_field(Field::MoveSpeed, 250.0).unwrap();
    draft.save(&dir, &[]).unwrap();
    assert!(!draft.is_stats_dirty());
    assert!(
        draft.is_vitals_dirty(),
        "player.ron does not hold the vitals"
    );
    assert!(!dir.join("vitals.ron").exists());

    let path = draft.save_vitals(&dir).unwrap();
    assert_eq!(path, dir.join("vitals.ron"));
    assert!(!draft.is_dirty());
    assert_eq!(VitalsConfig::load(&dir).unwrap().mana_per_kill, 25.0);
    std::fs::remove_dir_all(&dir).unwrap();
}

#[test]
fn every_vitals_field_can_be_edited_marks_the_draft_and_resets() {
    let mut draft = PlayerDraft::default();
    for field in VitalsField::ALL {
        let spec = field.spec();
        let saved = field.get(draft.saved_vitals());
        // A valid value that differs from the saved one and keeps the starting mana under the cap.
        let value = (saved + spec.step * 5.0).min(spec.max);
        let value = if value == saved { spec.min } else { value };
        draft.set_vitals_field(field, value).unwrap();
        assert!(draft.is_vitals_field_dirty(field), "{}", spec.key);
        assert_eq!(field.get(draft.edited_vitals()), value, "{}", spec.key);
        draft.reset_vitals_field(field).unwrap();
        assert!(!draft.is_vitals_field_dirty(field), "{}", spec.key);
    }
    assert!(!draft.is_dirty());
}
