use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

fn rules() -> Difficulty {
    Difficulty {
        enemy_health_percent: 10.0,
        enemy_damage_percent: 8.0,
        boss_health_percent: 12.0,
        boss_damage_percent: 8.0,
        coin_percent: 10.0,
        max_effective_tier: 20,
    }
}

#[test]
fn tier_zero_changes_nothing() {
    let scale = rules().scale(0);
    assert_eq!(scale, TierScale::NEUTRAL);
    assert!(scale.is_neutral());
    for multiplier in [
        scale.enemy_health,
        scale.enemy_damage,
        scale.boss_health,
        scale.boss_damage,
        scale.coin,
    ] {
        assert_eq!(multiplier, 1.0);
    }
    // Even with every percent at its largest, tier 0 is untouched.
    let steep = Difficulty {
        enemy_health_percent: 100.0,
        enemy_damage_percent: 100.0,
        boss_health_percent: 100.0,
        boss_damage_percent: 100.0,
        coin_percent: 100.0,
        max_effective_tier: 99,
    };
    assert!(steep.scale(0).is_neutral());
}

#[test]
fn every_tier_adds_the_same_percent_in_a_straight_line() {
    let rules = rules();
    for tier in 0..=20 {
        let scale = rules.scale(tier);
        let tiers = f64::from(tier);
        assert!(close(scale.enemy_health, 1.0 + 0.10 * tiers), "tier {tier}");
        assert!(close(scale.enemy_damage, 1.0 + 0.08 * tiers), "tier {tier}");
        assert!(close(scale.boss_health, 1.0 + 0.12 * tiers), "tier {tier}");
        assert!(close(scale.boss_damage, 1.0 + 0.08 * tiers), "tier {tier}");
        assert!(close(scale.coin, 1.0 + 0.10 * tiers), "tier {tier}");
    }
    // Each number has its own percent: raising one leaves the others alone.
    let only_coins = Difficulty {
        enemy_health_percent: 0.0,
        enemy_damage_percent: 0.0,
        boss_health_percent: 0.0,
        boss_damage_percent: 0.0,
        ..rules
    };
    let scale = only_coins.scale(5);
    assert_eq!(scale.enemy_health, 1.0);
    assert_eq!(scale.boss_damage, 1.0);
    assert!(close(scale.coin, 1.5));
}

#[test]
fn a_tier_above_the_cap_counts_as_the_cap() {
    let rules = Difficulty {
        max_effective_tier: 5,
        ..rules()
    };
    assert_eq!(rules.effective_tier(3), 3);
    assert_eq!(rules.effective_tier(5), 5);
    assert_eq!(rules.effective_tier(6), 5);
    assert_eq!(rules.effective_tier(u32::MAX), 5);
    assert_eq!(rules.scale(5), rules.scale(6));
    assert_eq!(rules.scale(5), rules.scale(u32::MAX));
    assert_ne!(rules.scale(4), rules.scale(5));
    assert!(close(rules.scale(1000).enemy_health, 1.5));
}

#[test]
fn health_is_rounded_and_never_below_one() {
    let scale = rules().scale(3); // enemy x1.3, boss x1.36
    assert_eq!(scale.enemy_health(30), 39);
    assert_eq!(scale.enemy_health(1), 1); // 1.3 rounds down
    assert_eq!(scale.enemy_health(2), 3); // 2.6 rounds up
    assert_eq!(scale.enemy_health(5), 7); // 6.5 rounds up
    assert_eq!(scale.boss_health(200), 272);
    assert_eq!(scale.boss_health(1), 1);
    // Tier 0 and nothing to scale stay exactly as they were.
    assert_eq!(TierScale::NEUTRAL.enemy_health(30), 30);
    assert_eq!(TierScale::NEUTRAL.boss_health(4_294_967_295), 4_294_967_295);
    assert_eq!(scale.enemy_health(0), 0);
    assert_eq!(scale.boss_health(0), 0);
    // A multiplier below 1 (not reachable from the file) still never gives 0.
    let weak = TierScale {
        enemy_health: 0.1,
        ..TierScale::NEUTRAL
    };
    assert_eq!(weak.enemy_health(3), 1);
}

#[test]
fn health_saturates_instead_of_wrapping() {
    let steep = TierScale {
        enemy_health: 100.0,
        boss_health: 100.0,
        ..TierScale::NEUTRAL
    };
    assert_eq!(steep.enemy_health(u16::MAX), u16::MAX);
    assert_eq!(steep.boss_health(u32::MAX), u32::MAX);
}

#[test]
fn damage_is_rounded_and_a_hit_never_vanishes() {
    let scale = rules().scale(3); // enemy x1.24, boss x1.24
    assert_eq!(scale.enemy_damage(10), 12);
    assert_eq!(scale.enemy_damage(1), 1);
    assert_eq!(scale.boss_damage(20), 25);
    assert_eq!(scale.enemy_damage(0), 0, "no damage stays no damage");
    let weak = TierScale {
        enemy_damage: 0.1,
        ..TierScale::NEUTRAL
    };
    assert_eq!(weak.enemy_damage(3), 1, "a hit that hurt still hurts");
    assert_eq!(TierScale::NEUTRAL.enemy_damage(7), 7);
    assert_eq!(TierScale::NEUTRAL.boss_damage(u32::MAX), u32::MAX);
}

#[test]
fn a_signed_hit_scales_only_when_it_hurts() {
    let scale = rules().scale(3);
    assert_eq!(scale.enemy_hit_damage(10), 12);
    assert_eq!(scale.boss_hit_damage(20), 25);
    assert_eq!(scale.enemy_hit_damage(0), 0);
    assert_eq!(scale.enemy_hit_damage(-4), -4);
    assert_eq!(scale.boss_hit_damage(i32::MIN), i32::MIN);
    let steep = TierScale {
        enemy_damage: 100.0,
        ..TierScale::NEUTRAL
    };
    assert_eq!(steep.enemy_hit_damage(i32::MAX), i32::MAX);
}

#[test]
fn coins_are_rounded_and_saturate() {
    let scale = rules().scale(3); // x1.3
    assert_eq!(scale.coins(10), 13);
    assert_eq!(scale.coins(7), 9); // 9.1
    assert_eq!(scale.coins(5), 7); // 6.5 rounds up
    assert_eq!(scale.coins(1), 1);
    assert_eq!(scale.coins(0), 0);
    assert_eq!(TierScale::NEUTRAL.coins(37), 37);
    let steep = TierScale {
        coin: 100.0,
        ..TierScale::NEUTRAL
    };
    assert_eq!(steep.coins(u32::MAX), u32::MAX);
}

#[test]
fn the_embedded_defaults_are_valid_and_are_what_default_gives() {
    let defaults = Difficulty::default();
    defaults.validate().unwrap();
    assert_eq!(Difficulty::from_ron(DEFAULTS), Ok(defaults));
}

#[test]
fn every_percent_must_be_a_finite_number_from_0_to_100() {
    type Set = fn(&mut Difficulty, f32);
    let fields: [(&str, Set); 5] = [
        ("enemy_health_percent", |d, v| d.enemy_health_percent = v),
        ("enemy_damage_percent", |d, v| d.enemy_damage_percent = v),
        ("boss_health_percent", |d, v| d.boss_health_percent = v),
        ("boss_damage_percent", |d, v| d.boss_damage_percent = v),
        ("coin_percent", |d, v| d.coin_percent = v),
    ];
    for (name, set) in fields {
        for value in [0.0, 0.5, 100.0] {
            let mut rules = rules();
            set(&mut rules, value);
            assert_eq!(rules.validate(), Ok(()), "{name} {value}");
        }
        for value in [-0.1, 100.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut rules = rules();
            set(&mut rules, value);
            let error = rules.validate().unwrap_err();
            assert!(error.contains(name), "{name} {value}: {error}");
        }
    }
}

#[test]
fn the_cap_must_be_from_1_to_99() {
    for cap in [1, 20, 99] {
        let rules = Difficulty {
            max_effective_tier: cap,
            ..rules()
        };
        assert_eq!(rules.validate(), Ok(()), "cap {cap}");
    }
    for cap in [0, 100, u32::MAX] {
        let rules = Difficulty {
            max_effective_tier: cap,
            ..rules()
        };
        let error = rules.validate().unwrap_err();
        assert!(error.contains("max_effective_tier"), "cap {cap}: {error}");
    }
}

#[test]
fn text_with_an_unknown_or_missing_field_or_a_bad_number_is_refused() {
    let with = |extra: &str| {
        format!(
            "(enemy_health_percent: 1.0, enemy_damage_percent: 1.0, boss_health_percent: 1.0, \
             boss_damage_percent: 1.0, coin_percent: 1.0, max_effective_tier: 5{extra})"
        )
    };
    assert!(Difficulty::from_ron(&with("")).is_ok());
    let unknown = Difficulty::from_ron(&with(", enemy_speed_percent: 5.0")).unwrap_err();
    assert!(unknown.contains("enemy_speed_percent"), "{unknown}");
    let missing = Difficulty::from_ron("(enemy_health_percent: 1.0)").unwrap_err();
    assert!(missing.contains("difficulty"), "{missing}");
    let too_big =
        Difficulty::from_ron(&with("").replace("coin_percent: 1.0", "coin_percent: 101.0"))
            .unwrap_err();
    assert!(too_big.contains("coin_percent"), "{too_big}");
    assert!(Difficulty::from_ron("not ron").is_err());
}

#[test]
fn the_file_text_round_trips() {
    let rules = Difficulty {
        enemy_health_percent: 12.5,
        coin_percent: 0.0,
        max_effective_tier: 7,
        ..rules()
    };
    assert_eq!(Difficulty::from_ron(&rules.to_ron().unwrap()), Ok(rules));
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-difficulty-{}-{}",
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

#[test]
fn a_file_in_assets_replaces_the_defaults_whole() {
    let dir = TempDir::new();
    assert_eq!(Difficulty::load(&dir.0), Ok(Difficulty::default()));
    let custom = Difficulty {
        enemy_health_percent: 50.0,
        max_effective_tier: 3,
        ..rules()
    };
    std::fs::write(dir.0.join(FILE_NAME), custom.to_ron().unwrap()).unwrap();
    assert_eq!(Difficulty::load(&dir.0), Ok(custom));
}

#[test]
fn a_broken_file_is_an_error_naming_it_never_a_fallback() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(coin_percent: 500.0)").unwrap();
    let error = Difficulty::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
    std::fs::write(dir.0.join(FILE_NAME), "(").unwrap();
    assert!(Difficulty::load(&dir.0).unwrap_err().contains(FILE_NAME));
}
