//! The air jump of Double Jump in the reach envelope: its two measures and the per-ability
//! intervals. Same rule as the parent module: a fixture config, never the shipped `player.ron`.

use super::*;

// ---- the air jump of Double Jump ---------------------------------------------------------

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "{actual} is not {expected}"
    );
}

/// The two-platform world's answer for the double jump, as `crosses` is for the plain jump and
/// the dash.
fn crosses_with_air_jump(config: &PlayerConfig, gap: f32) -> bool {
    let world = World::new(Some(gap), &[]);
    sampled_landings(&world, config, Second::AirJump)
        .iter()
        .any(|landing| landing.landed)
}

#[test]
fn the_dash_numbers_are_what_they_were_before_the_double_jump_existed() {
    // Measured by this code before the air jump was added, on this fixture and no other.
    let reach = Reach::of(&fixture());
    near(reach.jump_height, 164.58331);
    near(reach.jump_distance, 273.33365);
    near(reach.dash_jump_distance, 485.00012);
    let quick = Reach::of(&faster(fixture(), 1.2));
    near(quick.jump_height, 164.58331);
    near(quick.jump_distance, 322.0);
    near(quick.dash_jump_distance, 541.0003);
}

#[test]
fn the_double_jump_reaches_higher_and_farther_than_one_jump() {
    let reach = Reach::of(&fixture());
    assert!(
        reach.double_jump_height > reach.jump_height + 50.0,
        "{reach:?}"
    );
    assert!(
        reach.double_jump_distance > reach.jump_distance + 50.0,
        "{reach:?}"
    );
    // A second jump is not a dash: it lifts, and a dash does not.
    assert_eq!(reach.jump_height, Reach::of(&fixture()).jump_height);
}

#[test]
fn the_double_jump_height_agrees_with_the_physics_it_measures() {
    // The second jump at the top of the first: each rise is `speed² / (2 · gravity · held
    // multiplier)` and they add. Within the tick-sized error of sampling each apex.
    let config = fixture();
    let rise =
        |speed: f32| speed * speed / (2.0 * config.gravity * config.held_jump_gravity_multiplier);
    let expected = rise(config.jump_speed) + rise(config.double_jump_speed);
    let height = Reach::of(&config).double_jump_height;
    let tick_error = (config.jump_speed + config.double_jump_speed) * FIXED_STEP_SECONDS;
    assert!(
        (height - expected).abs() <= tick_error,
        "measured {height}, expected about {expected}"
    );
}

#[test]
fn the_double_jump_follows_its_speed_and_is_never_below_the_plain_jump() {
    let weak = PlayerConfig {
        double_jump_speed: 1.0,
        ..fixture()
    };
    let strong = PlayerConfig {
        double_jump_speed: 600.0,
        ..fixture()
    };
    let (weak, strong) = (Reach::of(&weak), Reach::of(&strong));
    // An air jump that is slower than the rise it replaces is left out: no second press is
    // one of the samples.
    assert!(weak.double_jump_height >= weak.jump_height, "{weak:?}");
    assert!(weak.double_jump_distance >= weak.jump_distance, "{weak:?}");
    assert!(strong.double_jump_height > weak.double_jump_height + 100.0);
    assert!(strong.double_jump_distance > weak.double_jump_distance + 50.0);
    assert_eq!(
        strong.jump_height, weak.jump_height,
        "the ground jump is not it"
    );
}

#[test]
fn the_double_jump_measure_is_deterministic() {
    let world = World::new(None, &[]);
    assert_eq!(
        double_jump_height(&world, &fixture()),
        double_jump_height(&world, &fixture())
    );
    assert_eq!(
        sampled_landings(&world, &fixture(), Second::AirJump),
        sampled_landings(&world, &fixture(), Second::AirJump)
    );
}

#[test]
fn the_longest_double_jump_gap_is_crossed_and_a_longer_one_is_not() {
    for config in [fixture(), faster(fixture(), 1.4)] {
        let distance = Reach::of(&config).double_jump_distance;
        assert!(distance > 0.0);
        assert!(
            crosses_with_air_jump(&config, distance - 0.01),
            "a gap of {distance} minus a hair must be crossed"
        );
        assert!(
            !crosses_with_air_jump(&config, distance + 0.01),
            "a gap of {distance} plus a hair must not be crossed"
        );
    }
}

#[test]
fn a_gap_between_the_jump_and_the_double_jump_needs_the_second_jump() {
    let reach = Reach::of(&fixture());
    let gap = (reach.jump_distance + reach.double_jump_distance) / 2.0;
    assert!(!crosses(&fixture(), gap, false), "no ability");
    assert!(
        crosses_with_air_jump(&fixture(), gap),
        "Double Jump crosses it"
    );
}

#[test]
fn a_flat_world_with_a_ceiling_leaves_the_gap_measure_alone() {
    let config = fixture();
    let plain = Reach::in_world(&config, &World::new(None, &[]));
    // A ceiling is a long slab whose underside is `rise` px above the top of the body standing
    // on the platform.
    let ceiling = |rise: f32| {
        let underside = TOP - config.size.y - rise;
        [Rect::new(-9_000.0, underside - 20.0, 18_000.0, 20.0)]
    };

    // Clear of the double jump's apex: nothing changes, not even its own numbers.
    let clear = ceiling(plain.double_jump_height + 30.0);
    assert_eq!(plain, Reach::in_world(&config, &World::new(None, &clear)));

    // Between the jump and the double jump: the plain jump and the dash jump are as they were,
    // and only the double jump is cut short.
    let between = ceiling((plain.jump_height + plain.double_jump_height) / 2.0);
    let cut = Reach::in_world(&config, &World::new(None, &between));
    assert_eq!(cut.jump_height, plain.jump_height);
    assert_eq!(cut.jump_distance, plain.jump_distance);
    assert_eq!(cut.dash_jump_distance, plain.dash_jump_distance);
    assert!(
        cut.double_jump_height < plain.double_jump_height - 20.0,
        "{cut:?}"
    );
    assert!(cut.double_jump_height > plain.jump_height, "{cut:?}");
    assert!(
        cut.double_jump_distance <= plain.double_jump_distance,
        "{cut:?}"
    );
    assert!(cut.double_jump_distance >= cut.jump_distance, "{cut:?}");
}

// ---- the interval of each ability ---------------------------------------------------------

#[test]
fn each_ability_has_its_own_gap_and_height_interval() {
    let interval = Interval::from_configs(&fixture(), &fixture());
    let (without, with) = (interval.without, interval.with);

    assert_eq!(
        interval.gap(Ability::Dash),
        Span {
            low: without.jump_distance,
            high: with.dash_jump_distance
        }
    );
    assert_eq!(
        interval.gap(Ability::DoubleJump),
        Span {
            low: without.jump_distance,
            high: with.double_jump_distance
        }
    );
    assert_eq!(
        interval.height(Ability::DoubleJump),
        Span {
            low: without.jump_height,
            high: with.double_jump_height
        }
    );
    for span in [
        interval.gap(Ability::Dash),
        interval.gap(Ability::DoubleJump),
        interval.height(Ability::DoubleJump),
    ] {
        assert!(span.usable(), "{interval:?}");
    }
    assert!(interval.errors().is_empty(), "{:?}", interval.errors());

    // A dash adds no height: no ledge needs it, and that is by design, not an error.
    assert!(!interval.height(Ability::Dash).usable());
    assert!(interval.problems(Ability::Dash).is_empty());

    // The Dash shorthands are the Dash lookup.
    assert_eq!(interval.low(), interval.gap(Ability::Dash).low);
    assert_eq!(interval.high(), interval.gap(Ability::Dash).high);
}

#[test]
fn a_length_needs_an_ability_when_it_is_beyond_the_plain_jump_and_within_the_ability() {
    let interval = Interval::from_configs(&fixture(), &fixture());
    let gap = interval.gap(Ability::DoubleJump);
    let height = interval.height(Ability::DoubleJump);
    let between = |span: Span| (span.low + span.high) / 2.0;

    assert!(gap.contains(between(gap)) && height.contains(between(height)));
    assert!(
        !gap.contains(gap.low),
        "exactly the plain jump needs nothing"
    );
    assert!(!height.contains(height.low));
    assert!(gap.contains(gap.high) && !gap.contains(gap.high + 0.5));
    assert!(height.contains(height.high) && !height.contains(height.high + 0.5));
    assert!(!gap.contains(f32::NAN) && !height.contains(f32::NAN));
    // Each ability has its own upper end: a gap between the two ends is within only one of them.
    let (dash, double) = (
        interval.gap(Ability::Dash),
        interval.gap(Ability::DoubleJump),
    );
    assert_ne!(dash.high, double.high);
    let between_ends = (dash.high + double.high) / 2.0;
    assert_ne!(dash.contains(between_ends), double.contains(between_ends));
}

#[test]
fn a_large_enough_buff_leaves_no_gap_for_the_double_jump_and_the_error_names_both_numbers() {
    let tuning = tuning_of(fixture());
    let vitals = VitalsConfig::from_ron(include_str!("../../../tests/fixtures/vitals.ron"))
        .expect("fixture vitals");
    let charms = CharmCatalog::new(Vec::new(), 100.0).expect("empty catalog");
    let open = Interval::new(&tuning, &vitals, &charms, &MaxTotals::default());
    assert!(open.gap(Ability::DoubleJump).usable(), "{open:?}");
    assert!(open.problems(Ability::DoubleJump).is_empty());

    let big = MaxTotals {
        move_speed_percent: 400.0,
        ..MaxTotals::default()
    };
    let empty = Interval::new(&tuning, &vitals, &charms, &big);
    let gap = empty.gap(Ability::DoubleJump);
    assert!(!gap.usable(), "{empty:?}");
    assert!(!gap.contains(gap.high), "no gap needs it");
    // Speed does not lift: the ledge interval is untouched, so only the gap is reported.
    assert!(empty.height(Ability::DoubleJump).usable());
    let problems = empty.problems(Ability::DoubleJump);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("no gap can need Double Jump"),
        "{problems:?}"
    );
    for distance in [gap.low, gap.high] {
        assert!(
            problems[0].contains(&format!("{distance:.0} px")),
            "{problems:?}"
        );
    }
    // The Dash error is the one it always was, and `errors` lists each ability's.
    let errors = empty.errors();
    assert!(errors
        .iter()
        .any(|(ability, text)| *ability == Ability::DoubleJump && *text == problems[0]));
    assert_eq!(
        empty.error(),
        empty.problems(Ability::Dash).into_iter().next()
    );
}

#[test]
fn a_double_jump_that_lifts_no_higher_leaves_no_ledge_and_the_error_names_both_numbers() {
    let reach = |jump_height, double_jump_height| Reach {
        jump_height,
        jump_distance: 200.0,
        dash_jump_distance: 300.0,
        double_jump_height,
        double_jump_distance: 400.0,
    };
    let interval = Interval {
        without: reach(220.0, 220.0),
        with: reach(200.0, 180.0),
    };
    let height = interval.height(Ability::DoubleJump);
    assert!(!height.usable() && !height.contains(200.0));
    // The gap interval is fine; only the ledge one is reported.
    assert!(interval.gap(Ability::DoubleJump).usable());
    let problems = interval.problems(Ability::DoubleJump);
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(
        problems[0].contains("no ledge can need Double Jump"),
        "{problems:?}"
    );
    assert!(
        problems[0].contains("220 px") && problems[0].contains("180 px"),
        "{problems:?}"
    );
}

#[test]
fn an_ability_with_both_intervals_empty_reports_both() {
    let floating = PlayerConfig {
        held_jump_gravity_multiplier: 0.0,
        ..fixture()
    };
    let interval = Interval::from_configs(&floating, &floating);
    let problems = interval.problems(Ability::DoubleJump);
    assert_eq!(
        problems.len(),
        2,
        "a jump that never ends measures nothing: {problems:?}"
    );
    assert_eq!(interval.errors().len(), 2 + 1, "and the Dash one");
}
