//! Reach tests. They use a config written here, never the shipped `player.ron`, so tuning the
//! game does not move them.

use macroquad::math::{Rect, Vec2};

use super::*;
use crate::combat::{MeleeConfig, StrikeConfig};

fn fixture() -> PlayerConfig {
    PlayerConfig {
        move_speed: 200.0,
        jump_speed: 500.0,
        double_jump_speed: 400.0,
        gravity: 1500.0,
        max_fall_speed: 900.0,
        held_jump_gravity_multiplier: 0.5,
        coyote_seconds: 0.1,
        jump_buffer_seconds: 0.1,
        size: Vec2::new(30.0, 50.0),
        dash_speed: 700.0,
        dash_duration: 0.25,
        dash_cooldown: 0.5,
        strike: StrikeConfig {
            forward_width: 30.0,
            forward_height: 30.0,
            up_width: 30.0,
            up_height: 30.0,
            down_width: 30.0,
            down_height: 30.0,
            pogo_speed: 400.0,
        },
        melee: MeleeConfig {
            damage: 10,
            swing_seconds: 0.1,
            cooldown: 0.3,
        },
        max_health: 50,
        invincibility_seconds: 1.0,
    }
}

fn faster(config: PlayerConfig, factor: f32) -> PlayerConfig {
    PlayerConfig {
        move_speed: config.move_speed * factor,
        ..config
    }
}

/// True when some sampled attempt lands on the far platform of a real two-platform world whose
/// gap is `gap`: the check that the measured distance means what the game means by "crosses".
fn crosses(config: &PlayerConfig, gap: f32, dash: bool) -> bool {
    let world = World::new(Some(gap), &[]);
    landings(&world, config, dash)
        .iter()
        .any(|landing| landing.landed)
}

#[test]
fn reach_grows_with_move_speed() {
    let slow = Reach::of(&fixture());
    let quick = Reach::of(&faster(fixture(), 1.5));
    assert!(quick.jump_distance > slow.jump_distance + 50.0);
    assert!(quick.dash_jump_distance > slow.dash_jump_distance + 50.0);
    assert_eq!(quick.jump_height, slow.jump_height, "speed does not lift");
}

#[test]
fn a_dash_adds_distance_and_never_height() {
    let reach = Reach::of(&fixture());
    // The dash covers 175 px in 0.25 s where walking covers 50: it must add a good part of that.
    assert!(
        reach.dash_jump_distance > reach.jump_distance + 80.0,
        "{reach:?}"
    );
    assert!(reach.dash_jump_distance >= reach.jump_distance);
    // Even a dash no faster than walking never makes the jump shorter.
    let weak = PlayerConfig {
        dash_speed: 100.0,
        ..fixture()
    };
    let reach = Reach::of(&weak);
    assert!(reach.dash_jump_distance >= reach.jump_distance, "{reach:?}");
}

#[test]
fn the_measure_is_deterministic() {
    assert_eq!(Reach::of(&fixture()), Reach::of(&fixture()));
    let interval = Interval::from_configs(&fixture(), &faster(fixture(), 1.2));
    assert_eq!(
        interval,
        Interval::from_configs(&fixture(), &faster(fixture(), 1.2))
    );
}

#[test]
fn jump_height_agrees_with_the_physics_it_measures() {
    // Held jump: the rise uses `gravity * held_jump_gravity_multiplier`. This is an independent
    // sanity check of the measurement, within the tick-sized error of sampling the apex.
    let config = fixture();
    let expected = config.jump_speed * config.jump_speed
        / (2.0 * config.gravity * config.held_jump_gravity_multiplier);
    let height = Reach::of(&config).jump_height;
    let tick_error = config.jump_speed * FIXED_STEP_SECONDS;
    assert!(
        (height - expected).abs() <= tick_error,
        "measured {height}, expected about {expected}"
    );
}

#[test]
fn the_longest_gap_is_crossed_and_a_longer_one_is_not() {
    for config in [fixture(), faster(fixture(), 1.4)] {
        let reach = Reach::of(&config);
        for (distance, dash) in [
            (reach.jump_distance, false),
            (reach.dash_jump_distance, true),
        ] {
            assert!(distance > 0.0);
            assert!(
                crosses(&config, distance - 0.01, dash),
                "a gap of {distance} minus a hair must be crossed (dash {dash})"
            );
            assert!(
                !crosses(&config, distance + 0.01, dash),
                "a gap of {distance} plus a hair must not be crossed (dash {dash})"
            );
        }
    }
}

#[test]
fn without_a_dash_key_press_no_gap_beyond_the_jump_is_crossed() {
    let reach = Reach::of(&fixture());
    let gap = (reach.jump_distance + reach.dash_jump_distance) / 2.0;
    assert!(!crosses(&fixture(), gap, false), "needs Dash");
    assert!(crosses(&fixture(), gap, true), "Dash crosses it");
}

#[test]
fn a_player_that_cannot_walk_crosses_nothing_but_still_jumps() {
    let still = PlayerConfig {
        move_speed: 0.0,
        ..fixture()
    };
    let reach = Reach::of(&still);
    assert_eq!(reach.jump_distance, 0.0);
    assert_eq!(reach.dash_jump_distance, 0.0);
    assert!(reach.jump_height > 50.0);
}

#[test]
fn walls_and_ceilings_away_from_the_flight_change_nothing() {
    let config = fixture();
    let plain = Reach::in_world(&config, &World::new(None, &[]));
    let far = [
        // A wall well behind the start, one far beyond the landing, a ceiling far above the apex.
        Rect::new(-9_000.0, TOP - 2_000.0, 40.0, 2_000.0),
        Rect::new(5_000.0, TOP - 2_000.0, 40.0, 2_000.0),
        Rect::new(-9_000.0, TOP - 3_000.0, 18_000.0, 40.0),
    ];
    assert_eq!(plain, Reach::in_world(&config, &World::new(None, &far)));
}

#[test]
fn the_world_does_reach_the_flight_when_a_ceiling_is_in_the_way() {
    // Negative control for the test above: a low ceiling over the takeoff does shorten the jump,
    // so equal numbers there mean the extra solids were really in the world.
    let config = fixture();
    let plain = Reach::in_world(&config, &World::new(None, &[]));
    let low = [Rect::new(-400.0, TOP - 120.0, 1_000.0, 20.0)];
    let low = Reach::in_world(&config, &World::new(None, &low));
    assert!(low.jump_distance < plain.jump_distance - 10.0, "{low:?}");
    assert!(low.jump_height < plain.jump_height - 10.0, "{low:?}");
}

#[test]
fn the_interval_is_usable_with_no_buffs_and_empty_with_a_big_one() {
    let base = fixture();
    let charms = CharmCatalog::new(Vec::new(), 100.0).expect("empty catalog");
    let tuning = tuning_of(base);
    let vitals = VitalsConfig::from_ron(include_str!("../../tests/fixtures/vitals.ron"))
        .expect("fixture vitals");

    let open = Interval::new(&tuning, &vitals, &charms, &MaxTotals::default());
    assert!(open.usable(), "{open:?}");
    assert!(open.error().is_none());
    assert!(open.low() < open.high());
    assert!(open.needs_dash((open.low() + open.high()) / 2.0));

    let big = MaxTotals {
        move_speed_percent: 400.0,
        ..MaxTotals::default()
    };
    let empty = Interval::new(&tuning, &vitals, &charms, &big);
    assert!(!empty.usable(), "{empty:?}");
    let text = empty.error().expect("an empty interval has an error");
    for distance in [empty.low(), empty.high()] {
        assert!(text.contains(&format!("{distance:.0} px")), "{text}");
    }
    assert!(!empty.needs_dash(empty.high()), "no gap needs Dash");
}

#[test]
fn an_interval_with_equal_distances_is_not_usable() {
    let reach = Reach {
        jump_height: 100.0,
        jump_distance: 200.0,
        dash_jump_distance: 200.0,
        double_jump_height: 100.0,
        double_jump_distance: 200.0,
    };
    let interval = Interval {
        without: reach,
        with: reach,
    };
    assert!(!interval.usable());
    assert!(!interval.needs_dash(200.0));
    assert!(interval.error().expect("error").contains("200 px"));
}

#[test]
fn needs_dash_is_longer_than_the_jump_and_up_to_the_dash_jump() {
    let interval = Interval {
        without: Reach {
            jump_height: 100.0,
            jump_distance: 180.0,
            dash_jump_distance: 250.0,
            double_jump_height: 100.0,
            double_jump_distance: 180.0,
        },
        with: Reach {
            jump_height: 100.0,
            jump_distance: 160.0,
            dash_jump_distance: 300.0,
            double_jump_height: 100.0,
            double_jump_distance: 160.0,
        },
    };
    assert!(interval.usable());
    assert!(
        !interval.needs_dash(180.0),
        "exactly the jump is not a gap that needs Dash"
    );
    assert!(interval.needs_dash(180.5));
    assert!(interval.needs_dash(300.0));
    assert!(!interval.needs_dash(300.5));
    assert!(!interval.needs_dash(f32::NAN));
}

#[test]
fn the_move_speed_buff_counts_every_charm_and_every_upgrade() {
    let charms = CharmCatalog::from_ron(
        r#"(charms: [
            (id: "swift", name: "Swift", description: "", mana_cost: 10.0,
             effect: MoveSpeedPercent(15.0), color: (1, 2, 3, 255), owned_at_start: false),
            (id: "fleet", name: "Fleet", description: "", mana_cost: 5.0,
             effect: MoveSpeedPercent(10.0), color: (1, 2, 3, 255), owned_at_start: false),
            (id: "heart", name: "Heart", description: "", mana_cost: 5.0,
             effect: MaxHealth(10), color: (1, 2, 3, 255), owned_at_start: false),
        ])"#,
        100.0,
    )
    .expect("charms");
    let upgrades = MaxTotals {
        move_speed_percent: 15.0,
        max_health: 30,
        sword_damage: 9,
    };
    assert_eq!(max_move_speed_percent(&charms, &upgrades), 40.0);
    let none = CharmCatalog::new(Vec::new(), 100.0).expect("empty");
    assert_eq!(max_move_speed_percent(&none, &MaxTotals::default()), 0.0);
}

#[test]
fn the_buffed_scenario_is_the_game_layering_of_the_speed() {
    let base = tuning_of(fixture());
    let vitals = VitalsConfig::from_ron(include_str!("../../tests/fixtures/vitals.ron"))
        .expect("fixture vitals");
    let charms = CharmCatalog::new(Vec::new(), 100.0).expect("empty catalog");
    let upgrades = MaxTotals {
        move_speed_percent: 50.0,
        ..MaxTotals::default()
    };
    let interval = Interval::new(&base, &vitals, &charms, &upgrades);
    let by_hand = faster(fixture(), 1.5);
    assert_eq!(interval.without, Reach::of(&by_hand));
    assert_eq!(interval.with, Reach::of(&fixture()));
}

fn tuning_of(player: PlayerConfig) -> TuningConfig {
    TuningConfig {
        player,
        enemy: crate::tuning::EnemyTuning {
            default_health: 30,
            knockback_speed: 260.0,
            knockback_seconds: 0.16,
        },
    }
}

#[test]
fn a_jump_that_never_comes_down_measures_nothing() {
    // Gravity off while the button is held: valid for the F4 window, and the body keeps rising
    // past the flight budget. It must measure nothing, not hang the editor.
    let floating = PlayerConfig {
        held_jump_gravity_multiplier: 0.0,
        ..fixture()
    };
    let reach = Reach::of(&floating);
    assert_eq!(reach.jump_distance, 0.0);
    assert_eq!(reach.dash_jump_distance, 0.0);
    assert!(Interval::from_configs(&floating, &floating)
        .error()
        .is_some());
}

#[test]
fn the_slowest_allowed_walk_still_measures() {
    let crawl = PlayerConfig {
        move_speed: 1.0,
        ..fixture()
    };
    let reach = Reach::of(&crawl);
    // A body that can only crawl still covers a gap narrower than itself by walking across.
    assert!(reach.jump_distance >= crawl.size.x, "{reach:?}");
    assert!(reach.jump_distance < crawl.size.x + 10.0, "{reach:?}");
}

mod double_jump;
