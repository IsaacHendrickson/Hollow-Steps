//! Permanent upgrades through `GameState`: that the levels in the profile are in effect (from the
//! start of the game and the moment the profile changes), that they stack with charms, that the
//! base numbers and their files never see them, and that the bars show bought health like a
//! charm's (a shorter base part, the added health at the end). The rules themselves are tested in `crate::upgrades`.
//!
//! The world is a private copy of `tests/fixtures` with a hub (`run_tests::world`) and an
//! `upgrades.ron` written here, so no number of the shipped file matters.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, profile_path, world};
use super::*;
use crate::charms::Toggle;
use crate::hud_bars::{layout_bars, HudConfig};
use crate::profile::{self, Profile};
use crate::tuning::TuningConfig;
use crate::vitals::VitalsConfig;

const UPGRADES: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "More health.",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "Faster.",
     effect: MoveSpeedPercent(5.0), costs: [40, 80, 140]),
    (id: "keen_edge", name: "Keen Edge", description: "Sharper.",
     effect: SwordDamage(3), costs: [50, 100, 180]),
])"#;

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-2, "{a} is not close to {b}");
}

fn bought(vitality: u32, swift_feet: u32, keen_edge: u32) -> Profile {
    let mut profile = Profile::default();
    for (id, level) in [
        ("vitality", vitality),
        ("swift_feet", swift_feet),
        ("keen_edge", keen_edge),
    ] {
        if level > 0 {
            profile.upgrades.insert(id.into(), level);
        }
    }
    profile
}

/// A hub world with the test upgrades, whose profile file already holds `saved`.
fn scratch_with(saved: &Profile) -> Scratch {
    let scratch = world(100.0);
    scratch.write("upgrades.ron", UPGRADES);
    profile::save(&profile_path(&scratch), saved).unwrap();
    scratch
}

#[test]
fn the_levels_in_the_saved_profile_are_in_effect_when_the_game_starts() {
    let scratch = scratch_with(&bought(2, 3, 1));
    let game = game_in(&scratch);
    let base = *game.tuning();

    let config = game.player().config();
    assert_eq!(config.max_health, base.player.max_health + 20);
    close(config.move_speed, base.player.move_speed * 1.15);
    assert_eq!(config.melee.damage, base.player.melee.damage + 3);
    assert_eq!(
        game.vitals().max_health(),
        f32::from(base.player.max_health + 20)
    );
}

#[test]
fn a_game_without_upgrades_plays_exactly_as_before() {
    let scratch = scratch_with(&Profile::default());
    let game = game_in(&scratch);
    assert_eq!(game.player().config(), &game.tuning().player);
}

#[test]
fn a_game_with_no_hub_ignores_the_profile_s_upgrades() {
    // No run.ron: the loop is off, the profile is never used and the game is as it always was.
    let scratch = Scratch::new();
    scratch.write("upgrades.ron", UPGRADES);
    let mut game = GameState::try_new(&scratch.0).unwrap();
    let base = game.tuning().player;
    game.set_profile(bought(3, 3, 3)).unwrap();
    assert_eq!(game.player().config(), &base);
}

#[test]
fn a_changed_profile_applies_at_once_and_the_player_keeps_place_and_timers() {
    let scratch = scratch_with(&Profile::default());
    let mut game = game_in(&scratch);
    game.player.place_on_floor(321.0, game.world_bounds());
    let (position, health) = (game.player().position(), game.vitals().health());
    let base = game.tuning().player.max_health;

    game.set_profile(bought(2, 0, 0)).unwrap();

    assert_eq!(game.player().config().max_health, base + 20);
    assert_eq!(game.player().position(), position, "not moved");
    // The existing max-health rule: a raised maximum adds the same health, nothing is refilled.
    close(game.vitals().health(), health + 20.0);
    assert_eq!(game.player().health().max(), base + 20);

    // Taking the levels away (F4's Reset profile) cuts the excess again.
    game.set_profile(Profile::default()).unwrap();
    assert_eq!(game.player().config().max_health, base);
    close(game.vitals().health(), health);
}

#[test]
fn upgrades_and_charms_add_up_and_each_leaves_when_taken_off() {
    let scratch = scratch_with(&bought(1, 2, 0));
    let mut game = game_in(&scratch);
    let base = *game.tuning();

    // Two levels of 5 percent and the swift charm's 15 percent.
    assert_eq!(game.toggle_charm("swift"), Toggle::Equipped);
    close(
        game.player().config().move_speed,
        base.player.move_speed * 1.25,
    );
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);
    assert_eq!(
        game.player().config().max_health,
        base.player.max_health + 20
    );

    assert_eq!(game.toggle_charm("swift"), Toggle::Unequipped);
    close(
        game.player().config().move_speed,
        base.player.move_speed * 1.10,
    );
    assert_eq!(game.toggle_charm("heart"), Toggle::Unequipped);
    assert_eq!(
        game.player().config().max_health,
        base.player.max_health + 10
    );
}

#[test]
fn an_upgrade_costs_no_mana_and_a_charm_budget_is_the_base_mana() {
    let scratch = scratch_with(&bought(3, 3, 3));
    let mut game = game_in(&scratch);
    let base_mana = game.vitals_config().max_mana;
    close(game.vitals().max_mana(), base_mana);
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);
    close(game.vitals().max_mana(), base_mana - 15.0);
}

#[test]
fn the_base_numbers_and_their_files_never_see_the_upgrades() {
    let scratch = scratch_with(&bought(3, 3, 3));
    let mut game = game_in(&scratch);
    let (tuning, vitals) = (*game.tuning(), *game.vitals_config());
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);

    // What F4 and the Player panel read is the base, and nothing looks edited.
    assert_eq!(*game.tuning(), tuning);
    assert_eq!(*game.player_draft().edited(), tuning);
    assert!(!game.tuning_unsaved() && !game.vitals_unsaved());
    assert_ne!(game.player().config().max_health, tuning.player.max_health);

    // Saving writes the base values, not the ones in effect.
    game.save_tuning().unwrap();
    game.save_vitals().unwrap();
    assert_eq!(TuningConfig::load(&scratch.0).unwrap(), tuning);
    assert_eq!(VitalsConfig::load(&scratch.0).unwrap(), vitals);
}

#[test]
fn bought_health_shows_on_the_bar_like_a_charms_added_health() {
    let scratch = scratch_with(&bought(2, 0, 0));
    let mut game = game_in(&scratch);
    let base = f32::from(game.tuning().player.max_health);
    let anchor = Rect::new(20.0, 20.0, 200.0, 20.0);
    let layout_of =
        |game: &GameState| layout_bars(&HudConfig::default(), game.bar_values(), anchor);

    // The baseline stays the number from player.ron; the bought 20 are above it.
    let values = game.bar_values();
    close(values.max_health, base + 20.0);
    close(values.base_max_health, base);
    close(game.base_max_health(), base);

    // So the original bar gets shorter and the bought health follows it at the end, behind the
    // divider, exactly as a charm's added health does.
    let layout = layout_of(&game);
    close(layout.health_base.w, anchor.w * base / (base + 20.0));
    close(layout.health_extension.w, anchor.w * 20.0 / (base + 20.0));
    close(layout.health_extension.x, layout.health_base.right());
    close(layout.health_extension.right(), anchor.right());
    assert!(layout.health_edge.w > 0.0, "the divider is drawn");

    // A charm on top grows the same added part at the end.
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);
    close(game.bar_values().max_health, base + 30.0);
    let layout = layout_of(&game);
    close(layout.health_base.w, anchor.w * base / (base + 30.0));
    close(layout.health_extension.w, anchor.w * 30.0 / (base + 30.0));
    assert_eq!(game.bar_values().base_max_mana, game.base_max_mana());
}

#[test]
fn bought_health_and_a_charms_health_look_the_same_on_the_bar() {
    // 10 bought (one level) and 10 from the heart charm give the same bar.
    let bought_scratch = scratch_with(&bought(1, 0, 0));
    let bought_game = game_in(&bought_scratch);
    let charm_scratch = scratch_with(&Profile::default());
    let mut charm_game = game_in(&charm_scratch);
    assert_eq!(charm_game.toggle_charm("heart"), Toggle::Equipped);

    let anchor = Rect::new(20.0, 20.0, 200.0, 20.0);
    let layout = |game: &GameState| layout_bars(&HudConfig::default(), game.bar_values(), anchor);
    let (bought_bars, charm_bars) = (layout(&bought_game), layout(&charm_game));
    assert!(bought_bars.health_extension.w > 0.0);
    for (bought, charm) in [
        (bought_bars.health_base, charm_bars.health_base),
        (bought_bars.health_extension, charm_bars.health_extension),
        (bought_bars.health_edge, charm_bars.health_edge),
        (bought_bars.health_locked, charm_bars.health_locked),
        (bought_bars.health, charm_bars.health),
    ] {
        assert_eq!(bought, charm);
    }
    // The charm also costs mana, which locks the end of the mana bar; an upgrade costs none.
    assert_eq!(bought_bars.mana_locked.w, 0.0);
    assert!(charm_bars.mana_locked.w > 0.0);
}

#[test]
fn without_upgrades_the_whole_bar_is_the_base_part() {
    let scratch = scratch_with(&Profile::default());
    let game = game_in(&scratch);
    let anchor = Rect::new(20.0, 20.0, 200.0, 20.0);
    let layout = layout_bars(&HudConfig::default(), game.bar_values(), anchor);
    close(layout.health_base.w, anchor.w);
    close(layout.health_extension.w, 0.0);
    close(layout.health_edge.w, 0.0);
}

#[test]
fn an_id_the_catalog_lacks_has_no_effect_and_survives_a_save() {
    let mut saved = bought(1, 0, 0);
    saved.upgrades.insert("long_gone".into(), 3);
    let scratch = scratch_with(&saved);
    let mut game = game_in(&scratch);
    let base = game.tuning().player.max_health;
    assert_eq!(
        game.player().config().max_health,
        base + 10,
        "only vitality"
    );

    game.set_profile(game.profile().clone()).unwrap();
    let (reloaded, message) = profile::load(&profile_path(&scratch));
    assert_eq!(message, None);
    assert_eq!(reloaded.upgrades.get("long_gone"), Some(&3));
    assert_eq!(reloaded, saved);
}

#[test]
fn f5_reads_a_changed_upgrades_file_and_a_broken_one_changes_nothing() {
    let scratch = scratch_with(&bought(2, 0, 0));
    let mut game = game_in(&scratch);
    let base = game.tuning().player.max_health;
    assert_eq!(game.player().config().max_health, base + 20);

    scratch.write(
        "upgrades.ron",
        &UPGRADES.replace("MaxHealth(10)", "MaxHealth(25)"),
    );
    game.reload_rooms().unwrap();
    assert_eq!(game.player().config().max_health, base + 50);

    scratch.write("upgrades.ron", "(upgrades: [(id: ");
    assert!(game.reload_rooms().unwrap_err().contains("upgrades.ron"));
    assert_eq!(
        game.player().config().max_health,
        base + 50,
        "nothing changed"
    );
}

#[test]
fn a_broken_upgrades_file_stops_the_game_from_loading_and_names_the_file() {
    let scratch = world(100.0);
    scratch.write("upgrades.ron", "(upgrades: [(id: ");
    let error = GameState::try_new(&scratch.0).unwrap_err();
    assert!(error.contains("upgrades.ron"), "{error}");
}
