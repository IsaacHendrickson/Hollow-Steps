use super::super::{Interval, Reach};
use super::line;

fn interval(without_distance: f32, with_dash: f32) -> Interval {
    let reach = |jump_distance, dash_jump_distance| Reach {
        jump_height: 160.0,
        jump_distance,
        dash_jump_distance,
        double_jump_height: 250.0,
        double_jump_distance: jump_distance + 100.0,
    };
    Interval {
        without: reach(without_distance, without_distance + 10.0),
        with: reach(without_distance - 20.0, with_dash),
    }
}

#[test]
fn the_line_names_both_distances_in_whole_pixels_and_says_what_to_run() {
    assert_eq!(
        line(&interval(183.4, 296.2)),
        "jump 183 px, dash jump 296 px; run Check dungeon"
    );
}

#[test]
fn an_empty_interval_says_why_instead() {
    let text = line(&interval(300.0, 250.0));
    assert!(text.contains("300 px") && text.contains("250 px"), "{text}");
    assert!(text.contains("no gap can need Dash"), "{text}");
    assert!(!text.contains("run Check dungeon"), "{text}");
}

#[test]
fn distances_that_are_equal_leave_no_gap_for_the_dash() {
    assert!(line(&interval(250.0, 250.0)).contains("no gap can need Dash"));
}
