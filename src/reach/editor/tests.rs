use macroquad::math::{Rect, Vec2};

use super::super::{Interval, Reach};
use super::{outline_label, outlines};
use crate::ability::Ability;

fn interval() -> Interval {
    let reach = |jump_height, jump_distance, dash_jump_distance| Reach {
        jump_height,
        jump_distance,
        dash_jump_distance,
        double_jump_height: jump_height,
        double_jump_distance: jump_distance,
    };
    Interval {
        without: reach(160.0, 200.0, 330.0),
        with: reach(150.0, 180.0, 400.0),
    }
}

/// `interval()` where a double jump goes higher (300 against 160) and farther (250 against 200)
/// than a jump with every buff, so both abilities have something that needs them.
fn with_a_double_jump() -> Interval {
    let mut found = interval();
    found.with.double_jump_height = 300.0;
    found.with.double_jump_distance = 250.0;
    found
}

const ORIGIN: Vec2 = Vec2::new(500.0, 300.0);
const BODY: Vec2 = Vec2::new(30.0, 50.0);

#[test]
fn each_outline_reaches_its_distance_either_side_and_the_top_of_its_jump() {
    let found = outlines(ORIGIN, BODY, &interval());
    // Body 30 x 50 centered at (500, 300): top 275, feet 325. Without Dash: 200 px each way and
    // the top of the body 160 higher (115); the box runs down to the feet.
    assert_eq!(found.without, Rect::new(300.0, 115.0, 400.0, 210.0));
    // With Dash: the longest dash jump, 400 px each way, the top 150 higher (125).
    assert_eq!(
        found.of(Ability::Dash).unwrap(),
        Rect::new(100.0, 125.0, 800.0, 200.0)
    );
}

#[test]
fn a_doorway_between_the_two_outlines_is_one_that_needs_the_dash() {
    let interval = interval();
    let found = outlines(ORIGIN, BODY, &interval);
    let dash = found.of(Ability::Dash).unwrap();
    let doorway_x = |distance: f32| 500.0 + distance;
    let between = |x: f32| x > found.without.right() && x <= dash.right();
    assert!(!between(doorway_x(150.0)), "inside the plain jump");
    assert!(between(doorway_x(300.0)), "needs the dash");
    assert!(!between(doorway_x(450.0)), "beyond even the dash jump");
    // The same rule the interval uses for a gap.
    assert!(interval.needs_dash(300.0) && !interval.needs_dash(150.0));
}

#[test]
fn the_outlines_move_with_the_entrance() {
    let a = outlines(Vec2::new(100.0, 200.0), BODY, &interval());
    let b = outlines(Vec2::new(160.0, 260.0), BODY, &interval());
    assert_eq!(b.without.x - a.without.x, 60.0);
    assert_eq!(
        b.of(Ability::Dash).unwrap().y - a.of(Ability::Dash).unwrap().y,
        60.0
    );
    assert_eq!(b.without.w, a.without.w);
}

#[test]
fn the_double_jump_outline_is_as_tall_as_a_double_jump_and_as_wide_as_its_distance() {
    let interval = with_a_double_jump();
    let found = outlines(ORIGIN, BODY, &interval);
    // 250 px each way, the top of the body (275) 300 higher: -25; the box runs down to the feet.
    let double = found.of(Ability::DoubleJump).unwrap();
    assert_eq!(double, Rect::new(250.0, -25.0, 500.0, 350.0));
    // It rises above the plain jump (115) and the dash jump (125), and the dash is unchanged.
    assert!(double.y < found.without.y && double.y < found.of(Ability::Dash).unwrap().y);
    assert_eq!(
        found.of(Ability::Dash).unwrap(),
        Rect::new(100.0, 125.0, 800.0, 200.0)
    );
    assert_eq!(found, outlines(ORIGIN, BODY, &interval), "a pure function");
}

#[test]
fn an_outline_is_drawn_for_each_ability_something_can_need_in_the_order_of_the_abilities() {
    let found = outlines(ORIGIN, BODY, &with_a_double_jump());
    let drawn: Vec<Ability> = found.with.iter().map(|(ability, _)| *ability).collect();
    assert_eq!(drawn, Ability::ALL);
}

#[test]
fn an_ability_nothing_can_need_gets_no_outline_and_the_others_keep_theirs() {
    // The fixture's double jump goes no higher or farther than the jump: nothing needs it.
    let found = outlines(ORIGIN, BODY, &interval());
    assert_eq!(found.of(Ability::DoubleJump), None);
    assert!(found.of(Ability::Dash).is_some());

    // A dash that goes no farther than the jump with every buff, and a double jump that does.
    let mut dash_short = with_a_double_jump();
    dash_short.with.dash_jump_distance = 190.0;
    let found = outlines(ORIGIN, BODY, &dash_short);
    assert_eq!(found.of(Ability::Dash), None);
    assert!(found.of(Ability::DoubleJump).is_some());
}

#[test]
fn a_double_jump_that_reaches_farther_but_no_higher_has_no_outline() {
    // It adds both kinds of length, so one empty kind is a problem and nothing is drawn for it.
    let mut flat = with_a_double_jump();
    flat.with.double_jump_height = 160.0;
    assert_eq!(outlines(ORIGIN, BODY, &flat).of(Ability::DoubleJump), None);
}

#[test]
fn with_no_ability_anything_can_need_only_the_plain_outline_is_left() {
    let mut none = with_a_double_jump();
    none.with.dash_jump_distance = 190.0;
    none.with.double_jump_distance = 190.0;
    let found = outlines(ORIGIN, BODY, &none);
    assert!(found.with.is_empty());
    assert_eq!(found.without, Rect::new(300.0, 115.0, 400.0, 210.0));
}

#[test]
fn the_labels_say_the_distances_and_heights_in_whole_pixels() {
    let interval = with_a_double_jump();
    assert_eq!(
        outline_label(&interval, None),
        "without any ability 160 px high / 200 px far"
    );
    assert_eq!(
        outline_label(&interval, Some(Ability::Dash)),
        "with Dash 400 px"
    );
    assert_eq!(
        outline_label(&interval, Some(Ability::DoubleJump)),
        "with Double Jump 300 px high / 250 px far"
    );
    let mut fractions = interval;
    fractions.with.double_jump_height = 300.6;
    fractions.with.double_jump_distance = 249.4;
    assert_eq!(
        outline_label(&fractions, Some(Ability::DoubleJump)),
        "with Double Jump 301 px high / 249 px far"
    );
}
