use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

fn bank(profile: &mut Profile, earned: u32, percent: f32, outcome: RunOutcome) -> RunResult {
    profile.bank(earned, percent, outcome)
}

#[test]
fn banking_keeps_the_configured_share_of_the_run_coins() {
    for (percent, earned, banked) in [
        (100.0, 37, 37),
        (50.0, 40, 20),
        (50.0, 7, 3),
        (0.0, 40, 0),
        (25.0, 3, 0),
        (100.0, 0, 0),
    ] {
        let mut profile = Profile::default();
        let result = bank(&mut profile, earned, percent, RunOutcome::Died);
        assert_eq!(result.coins_earned, earned, "{percent}% of {earned}");
        assert_eq!(result.coins_banked, banked, "{percent}% of {earned}");
        assert_eq!(profile.coins, banked, "{percent}% of {earned}");
    }
}

#[test]
fn banked_coins_add_to_what_the_profile_already_has() {
    let mut profile = Profile {
        coins: 10,
        ..Profile::default()
    };
    bank(&mut profile, 30, 100.0, RunOutcome::Abandoned);
    bank(&mut profile, 15, 100.0, RunOutcome::Died);
    assert_eq!(profile.coins, 55);
}

#[test]
fn only_a_clear_raises_the_tier_and_the_cleared_count() {
    let start = Profile {
        coins: 5,
        tier: 1,
        cleared: 4,
    };
    for (outcome, tier, cleared) in [
        (RunOutcome::Cleared, 2, 5),
        (RunOutcome::Died, 1, 4),
        (RunOutcome::Abandoned, 1, 4),
    ] {
        let mut profile = start;
        let result = bank(&mut profile, 10, 100.0, outcome);
        assert_eq!(
            (profile.tier, profile.cleared),
            (tier, cleared),
            "{outcome:?}"
        );
        assert_eq!(result.outcome, outcome);
        assert_eq!(result.tier_before, 1);
        assert_eq!(result.tier_after, tier, "{outcome:?}");
        assert_eq!(profile.coins, 15, "{outcome:?} banks the coins");
    }
}

#[test]
fn a_clear_with_no_coins_still_raises_the_tier() {
    let mut profile = Profile::default();
    let result = bank(&mut profile, 0, 0.0, RunOutcome::Cleared);
    assert_eq!((result.tier_before, result.tier_after), (0, 1));
    assert_eq!((profile.coins, profile.cleared), (0, 1));
}

#[test]
fn banking_saturates_instead_of_overflowing() {
    let mut profile = Profile {
        coins: u32::MAX - 3,
        tier: u32::MAX,
        cleared: u32::MAX,
    };
    let result = bank(&mut profile, 100, 100.0, RunOutcome::Cleared);
    assert_eq!(profile.coins, u32::MAX);
    assert_eq!(profile.tier, u32::MAX);
    assert_eq!(profile.cleared, u32::MAX);
    assert_eq!(result.coins_banked, 100, "the result says what was kept");
    assert_eq!(result.tier_after, u32::MAX);
}

#[test]
fn a_huge_run_total_is_kept_exactly_at_one_hundred_percent_and_never_exceeded() {
    assert_eq!(kept_coins(u32::MAX, 100.0), u32::MAX);
    assert_eq!(kept_coins(u32::MAX, 50.0), u32::MAX / 2);
    assert_eq!(kept_coins(u32::MAX, 0.0), 0);
    // A bigger share never banks fewer coins, and no share banks more than was earned.
    let mut last = 0;
    for percent in [0.0, 0.1, 33.3, 50.0, 99.99, 100.0] {
        let kept = kept_coins(u32::MAX, percent);
        assert!(kept >= last, "{percent}%");
        last = kept;
        assert!(kept_coins(1_000, percent) <= 1_000);
    }
}

#[test]
fn a_share_outside_the_range_is_clamped_and_never_pays_more_than_was_earned() {
    assert_eq!(kept_coins(10, 250.0), 10);
    assert_eq!(kept_coins(10, -5.0), 0);
    assert_eq!(kept_coins(10, f32::INFINITY), 10, "clamped to 100");
    assert_eq!(kept_coins(10, f32::NEG_INFINITY), 0, "clamped to 0");
    assert_eq!(
        kept_coins(10, f32::NAN),
        10,
        "not a number counts as the default"
    );
}

#[test]
fn the_embedded_default_has_no_hub_and_the_old_rooms() {
    let config = RunConfig::default();
    assert_eq!(config.hub_room, None);
    assert!(!config.has_hub());
    assert_eq!(config.start_room, RoomId::First);
    assert_eq!(config.final_boss_room, RoomId::Boss);
    assert_eq!(config.coin_keep_percent, 100.0);
}

#[test]
fn an_empty_file_means_all_the_defaults() {
    assert_eq!(RunConfig::from_ron("()").unwrap(), RunConfig::default());
}

#[test]
fn a_hub_can_be_written_with_or_without_some() {
    let plain = RunConfig::from_ron("(hub_room: Custom(0))").unwrap();
    let explicit = RunConfig::from_ron("(hub_room: Some(Custom(0)))").unwrap();
    assert_eq!(plain, explicit);
    assert_eq!(plain.hub_room, Some(RoomId::Custom(0)));
    assert!(plain.has_hub());
    assert_eq!(
        RunConfig::from_ron("(hub_room: None)").unwrap().hub_room,
        None
    );
}

#[test]
fn every_field_can_be_set() {
    let config = RunConfig::from_ron(
        "(hub_room: Custom(2), start_room: Second, final_boss_room: Custom(5), \
         coin_keep_percent: 40.5)",
    )
    .unwrap();
    assert_eq!(config.hub_room, Some(RoomId::Custom(2)));
    assert_eq!(config.start_room, RoomId::Second);
    assert_eq!(config.final_boss_room, RoomId::Custom(5));
    assert_eq!(config.coin_keep_percent, 40.5);
}

#[test]
fn the_keep_percent_must_be_a_finite_number_from_0_to_100() {
    for text in [
        "(coin_keep_percent: -0.5)",
        "(coin_keep_percent: 100.5)",
        "(coin_keep_percent: 1000.0)",
    ] {
        let error = RunConfig::from_ron(text).unwrap_err();
        assert!(error.contains("coin_keep_percent"), "{text}: {error}");
    }
    for edge in ["0.0", "100.0"] {
        RunConfig::from_ron(&format!("(coin_keep_percent: {edge})")).unwrap();
    }
    // NaN and infinity cannot be written in a file, but a config built in code is checked too.
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let config = RunConfig {
            coin_keep_percent: bad,
            ..RunConfig::default()
        };
        assert!(config.validate().is_err(), "{bad}");
    }
}

#[test]
fn the_hub_must_differ_from_the_start_room() {
    let error = RunConfig::from_ron("(hub_room: First)").unwrap_err();
    assert!(
        error.contains("hub_room must differ from start_room"),
        "{error}"
    );
    let error = RunConfig::from_ron("(hub_room: Custom(1), start_room: Custom(1))").unwrap_err();
    assert!(error.contains("hub_room must differ"), "{error}");
    // Without a hub, any start room is fine, and a hub elsewhere is fine.
    RunConfig::from_ron("(start_room: Second)").unwrap();
    RunConfig::from_ron("(hub_room: Custom(0), start_room: First)").unwrap();
}

#[test]
fn an_unknown_field_or_a_bad_value_is_refused() {
    assert!(RunConfig::from_ron("(shop: (1.0, 2.0))").is_err());
    assert!(RunConfig::from_ron("(hub_rom: Custom(0))").is_err());
    assert!(RunConfig::from_ron("(start_room: Nowhere)").is_err());
    assert!(RunConfig::from_ron("(coin_keep_percent: \"all\")").is_err());
    assert!(RunConfig::from_ron("not ron").is_err());
}

#[test]
fn rooms_the_file_names_must_exist() {
    let rooms = [
        RoomId::First,
        RoomId::Second,
        RoomId::Boss,
        RoomId::Custom(0),
    ];
    let config = RunConfig::from_ron("(hub_room: Custom(0))").unwrap();
    config.check_rooms(&rooms).unwrap();
    RunConfig::default().check_rooms(&rooms[..3]).unwrap();

    let error = config.check_rooms(&rooms[..3]).unwrap_err();
    assert!(
        error.contains("hub_room") && error.contains("room_0.ron"),
        "{error}"
    );
    let config = RunConfig::from_ron("(final_boss_room: Custom(9))").unwrap();
    let error = config.check_rooms(&rooms).unwrap_err();
    assert!(
        error.contains("final_boss_room") && error.contains("room_9.ron"),
        "{error}"
    );
    let config = RunConfig::from_ron("(start_room: Second)").unwrap();
    let error = config
        .check_rooms(&[RoomId::First, RoomId::Boss])
        .unwrap_err();
    assert!(error.contains("start_room"), "{error}");
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-run-{}-{}",
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
fn an_assets_file_replaces_the_embedded_default_whole() {
    let dir = TempDir::new();
    assert_eq!(RunConfig::load(&dir.0).unwrap(), RunConfig::default());

    std::fs::write(
        dir.0.join(FILE_NAME),
        "(hub_room: Custom(0), coin_keep_percent: 50.0)",
    )
    .unwrap();
    let config = RunConfig::load(&dir.0).unwrap();
    assert_eq!(config.hub_room, Some(RoomId::Custom(0)));
    assert_eq!(config.coin_keep_percent, 50.0);
}

#[test]
fn a_bad_assets_file_is_an_error_naming_it_not_a_fallback() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(coin_keep_percent: 300.0)").unwrap();
    let error = RunConfig::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "names the file: {error}");
    assert!(error.contains("coin_keep_percent"), "{error}");
}
