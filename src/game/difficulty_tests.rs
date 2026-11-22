//! The difficulty rules in `GameState` (`tier_link`): what F4's Difficulty section sets for the
//! next run, saves to `difficulty.ron`, and the tier the HUD shows.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, world};
use super::tier_tests::{begin, first_enemy, rules, tiered, with_tier, RULES};
use super::*;
use crate::difficulty::Difficulty;

fn changed(rules: Difficulty) -> Difficulty {
    Difficulty {
        enemy_health_percent: 50.0,
        ..rules
    }
}

// ---- set ------------------------------------------------------------------------------------------

#[test]
fn new_rules_apply_to_the_next_run_and_not_to_the_one_in_progress() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    assert_eq!(first_enemy(&game), (39, 39), "10 percent a tier at tier 3");
    let held = game.tier_scale();

    game.set_difficulty(changed(rules())).unwrap();

    assert_eq!(game.difficulty().enemy_health_percent, 50.0);
    assert_eq!(
        game.tier_scale(),
        held,
        "the run keeps the layer it began with"
    );
    assert_eq!(first_enemy(&game), (39, 39));
    game.reload_rooms().unwrap();
    assert_eq!(first_enemy(&game), (39, 39), "so does an F5 rebuild");

    game.abandon_run();
    game.continue_after_result();
    begin(&mut game);
    assert_eq!(first_enemy(&game), (75, 75), "50 percent a tier: 30 x 2.5");
}

#[test]
fn rules_that_fail_validation_are_refused_and_change_nothing() {
    let (_scratch, mut game) = tiered(0);
    let before = *game.difficulty();
    let bad = [
        Difficulty {
            enemy_health_percent: f32::NAN,
            ..before
        },
        Difficulty {
            enemy_damage_percent: 100.5,
            ..before
        },
        Difficulty {
            boss_health_percent: -1.0,
            ..before
        },
        Difficulty {
            boss_damage_percent: f32::INFINITY,
            ..before
        },
        Difficulty {
            coin_percent: 1000.0,
            ..before
        },
        Difficulty {
            max_effective_tier: 0,
            ..before
        },
        Difficulty {
            max_effective_tier: 100,
            ..before
        },
    ];
    for rules in bad {
        let error = game.set_difficulty(rules).unwrap_err();
        assert!(error.contains("must be"), "{error}");
        assert_eq!(game.difficulty(), &before);
    }
    assert!(!game.difficulty_unsaved());
}

#[test]
fn the_limits_themselves_are_accepted() {
    let (_scratch, mut game) = tiered(0);
    let edge = Difficulty {
        enemy_health_percent: 0.0,
        enemy_damage_percent: 100.0,
        boss_health_percent: 0.0,
        boss_damage_percent: 100.0,
        coin_percent: 100.0,
        max_effective_tier: 99,
    };
    game.set_difficulty(edge).unwrap();
    game.set_difficulty(Difficulty {
        max_effective_tier: 1,
        ..edge
    })
    .unwrap();
}

// ---- save -----------------------------------------------------------------------------------------

#[test]
fn unsaved_follows_the_rules_against_the_file_and_save_clears_it() {
    let (scratch, mut game) = tiered(0);
    assert!(!game.difficulty_unsaved());
    let original = *game.difficulty();

    game.set_difficulty(changed(original)).unwrap();
    assert!(game.difficulty_unsaved());
    game.set_difficulty(original).unwrap();
    assert!(!game.difficulty_unsaved(), "put back, nothing differs");

    game.set_difficulty(changed(original)).unwrap();
    let path = game.save_difficulty().unwrap();
    assert_eq!(path, scratch.0.join("difficulty.ron"));
    assert!(!game.difficulty_unsaved());
    assert!(
        !scratch.0.join("difficulty.ron.tmp").exists(),
        "no staging file left"
    );
}

#[test]
fn a_saved_file_loads_back_as_the_same_rules_and_a_new_game_starts_with_them() {
    let (scratch, mut game) = tiered(0);
    let rules = Difficulty {
        enemy_health_percent: 12.5,
        enemy_damage_percent: 3.0,
        boss_health_percent: 40.0,
        boss_damage_percent: 0.0,
        coin_percent: 99.0,
        max_effective_tier: 7,
    };
    game.set_difficulty(rules).unwrap();
    game.save_difficulty().unwrap();

    let text = std::fs::read_to_string(scratch.0.join("difficulty.ron")).unwrap();
    assert!(
        text.starts_with("// "),
        "the file keeps an explaining comment:\n{text}"
    );
    assert_eq!(Difficulty::from_ron(&text).unwrap(), rules);
    assert_eq!(Difficulty::load(&scratch.0).unwrap(), rules);
    assert_eq!(game_in(&scratch).difficulty(), &rules);
}

#[test]
fn save_writes_a_file_where_there_was_none_and_replaces_one_that_was_there() {
    let scratch = world(100.0);
    assert!(
        !scratch.0.join("difficulty.ron").exists(),
        "the fixtures have none"
    );
    let mut game = game_in(&scratch);
    assert_eq!(
        game.difficulty(),
        &Difficulty::default(),
        "the embedded defaults"
    );
    game.set_difficulty(changed(Difficulty::default())).unwrap();
    game.save_difficulty().unwrap();
    assert!(scratch.0.join("difficulty.ron").exists());

    game.set_difficulty(Difficulty::default()).unwrap();
    game.save_difficulty().unwrap();
    assert_eq!(Difficulty::load(&scratch.0).unwrap(), Difficulty::default());
}

#[test]
fn a_failed_save_says_where_and_leaves_the_rules_unsaved() {
    let (scratch, mut game) = tiered(0);
    // A directory where the file belongs: the staged file cannot be moved over it.
    std::fs::remove_file(scratch.0.join("difficulty.ron")).unwrap();
    std::fs::create_dir(scratch.0.join("difficulty.ron")).unwrap();
    game.set_difficulty(changed(rules())).unwrap();

    let error = game.save_difficulty().unwrap_err();

    assert!(error.contains("difficulty.ron"), "{error}");
    assert!(game.difficulty_unsaved(), "still unsaved");
    assert_eq!(
        game.difficulty().enemy_health_percent,
        50.0,
        "still in effect"
    );
    assert!(
        !scratch.0.join("difficulty.ron.tmp").exists(),
        "no staging file left"
    );
}

#[test]
fn saving_never_touches_the_profile_or_the_rooms() {
    let (scratch, mut game) = tiered(3);
    let rooms = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();
    let profile = std::fs::read_to_string(scratch.0.join("profile.ron")).ok();
    game.set_difficulty(changed(rules())).unwrap();
    game.save_difficulty().unwrap();
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap(),
        rooms
    );
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("profile.ron")).ok(),
        profile
    );
    assert_eq!(game.profile().tier, 3);
}

#[test]
fn the_shipped_rules_text_is_a_file_the_loader_accepts() {
    // The helper's own header must not break the format it is written in.
    let text = Difficulty::from_ron(RULES).unwrap().to_file_text().unwrap();
    assert_eq!(
        Difficulty::from_ron(&text).unwrap(),
        Difficulty::from_ron(RULES).unwrap()
    );
    let bad = Difficulty {
        coin_percent: 500.0,
        ..Difficulty::default()
    };
    assert!(
        bad.to_file_text().is_err(),
        "an invalid rule set is never written"
    );
}

// ---- the HUD's tier -------------------------------------------------------------------------------

#[test]
fn the_hud_shows_the_profile_tier_in_the_hub_and_the_runs_tier_in_a_run() {
    let (_scratch, mut game) = tiered(4);
    assert_eq!(game.hud_tier(), Some(4), "the hub");

    with_tier(&mut game, 5);
    assert_eq!(game.hud_tier(), Some(5), "the hub follows F4");

    begin(&mut game);
    assert_eq!(game.hud_tier(), Some(5), "the run");
    with_tier(&mut game, 9);
    assert_eq!(
        game.hud_tier(),
        Some(5),
        "F4 changes the next run, not this one"
    );
}

#[test]
fn the_hud_shows_the_real_tier_even_above_the_cap() {
    let rules = RULES.replace("max_effective_tier: 20", "max_effective_tier: 3");
    let scratch = world(100.0);
    scratch.write("difficulty.ron", &rules);
    let mut game = game_in(&scratch);
    with_tier(&mut game, 8);
    begin(&mut game);
    assert_eq!(game.world_tier(), 3);
    assert_eq!(
        game.hud_tier(),
        Some(8),
        "the player's tier, not the capped one"
    );
}

#[test]
fn the_hud_has_no_tier_on_the_result_screen_or_with_no_hub() {
    let (_scratch, mut game) = tiered(2);
    begin(&mut game);
    game.abandon_run();
    assert!(game.run_result().is_some());
    assert_eq!(game.hud_tier(), None);
    game.continue_after_result();
    assert_eq!(game.hud_tier(), Some(2));

    let scratch = Scratch::new();
    let mut plain = GameState::try_new(&scratch.0).unwrap();
    with_tier(&mut plain, 5);
    assert_eq!(plain.hud_tier(), None);
}

#[test]
fn a_test_room_shows_the_profile_tier() {
    let (_scratch, mut game) = tiered(0);
    begin(&mut game);
    with_tier(&mut game, 6);
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(game.hud_tier(), Some(6));
    game.end_test_room().unwrap();
    assert_eq!(
        game.hud_tier(),
        Some(0),
        "back in the run, which began at tier 0"
    );
}
