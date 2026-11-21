use macroquad::math::{Rect, Vec2};

use super::super::{Interval, Reach};
use super::{outline_labels, outlines};

fn interval() -> Interval {
    let reach = |jump_height, jump_distance, dash_jump_distance| Reach {
        jump_height,
        jump_distance,
        dash_jump_distance,
    };
    Interval {
        without: reach(160.0, 200.0, 330.0),
        with: reach(150.0, 180.0, 400.0),
    }
}

#[test]
fn each_outline_reaches_its_distance_either_side_and_the_top_of_its_jump() {
    let found = outlines(Vec2::new(500.0, 300.0), Vec2::new(30.0, 50.0), &interval());
    // Body 30 x 50 centered at (500, 300): top 275, feet 325. Without Dash: 200 px each way and
    // the top of the body 160 higher (115); the box runs down to the feet.
    assert_eq!(found.without, Rect::new(300.0, 115.0, 400.0, 210.0));
    // With Dash: the longest dash jump, 400 px each way, the top 150 higher (125).
    assert_eq!(found.with, Rect::new(100.0, 125.0, 800.0, 200.0));
}

#[test]
fn a_doorway_between_the_two_outlines_is_one_that_needs_the_dash() {
    let interval = interval();
    let found = outlines(Vec2::new(500.0, 300.0), Vec2::new(30.0, 50.0), &interval);
    let doorway_x = |distance: f32| 500.0 + distance;
    let between = |x: f32| x > found.without.right() && x <= found.with.right();
    assert!(!between(doorway_x(150.0)), "inside the plain jump");
    assert!(between(doorway_x(300.0)), "needs the dash");
    assert!(!between(doorway_x(450.0)), "beyond even the dash jump");
    // The same rule the interval uses for a gap.
    assert!(interval.needs_dash(300.0) && !interval.needs_dash(150.0));
}

#[test]
fn the_outlines_move_with_the_entrance() {
    let a = outlines(Vec2::new(100.0, 200.0), Vec2::new(30.0, 50.0), &interval());
    let b = outlines(Vec2::new(160.0, 260.0), Vec2::new(30.0, 50.0), &interval());
    assert_eq!(b.without.x - a.without.x, 60.0);
    assert_eq!(b.with.y - a.with.y, 60.0);
    assert_eq!(b.without.w, a.without.w);
}

#[test]
fn the_labels_say_the_distances_in_whole_pixels() {
    assert_eq!(
        outline_labels(&interval()),
        ["without Dash 200 px", "with Dash 400 px"]
    );
}
