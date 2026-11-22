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
fn the_line_names_every_ability_in_whole_pixels_and_says_what_to_run() {
    assert_eq!(
        line(&interval(183.4, 296.2)),
        "jump 183 px, dash jump 296 px, double jump 250 px high / 263 px far; run Check dungeon"
    );
}

#[test]
fn the_line_reads_as_the_example_in_the_docs() {
    let mut numbers = interval(183.4, 296.2);
    numbers.with.double_jump_height = 270.4;
    numbers.with.double_jump_distance = 240.2;
    assert_eq!(
        line(&numbers),
        "jump 183 px, dash jump 296 px, double jump 270 px high / 240 px far; run Check dungeon"
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

#[test]
fn a_double_jump_no_ledge_needs_says_why_and_the_dash_numbers_are_not_repeated() {
    let mut flat = interval(183.4, 296.2);
    flat.with.double_jump_height = 160.0;
    let text = line(&flat);
    assert!(text.contains("no ledge can need Double Jump"), "{text}");
    assert!(text.contains("160 px"), "both numbers: {text}");
    assert!(!text.contains("no gap can need"), "the gap is fine: {text}");
    assert!(!text.contains("run Check dungeon"), "{text}");
}

#[test]
fn every_ability_with_nothing_to_need_it_is_named_once_each_in_the_line() {
    // Dash adds nothing (250 against 300) and neither does the double jump (a shorter jump,
    // 150 px lower): two messages, each naming its ability, one line.
    let mut nothing = interval(300.0, 250.0);
    nothing.with.double_jump_distance = 290.0;
    nothing.with.double_jump_height = 100.0;
    let text = line(&nothing);
    assert_eq!(text.matches("no gap can need Dash").count(), 1, "{text}");
    assert_eq!(
        text.matches("no gap can need Double Jump").count(),
        1,
        "{text}"
    );
    assert_eq!(
        text.matches("no ledge can need Double Jump").count(),
        1,
        "{text}"
    );
}
