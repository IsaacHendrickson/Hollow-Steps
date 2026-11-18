use super::*;
use crate::charm_menu::DESCRIPTION_LINES;

/// Ten pixels a character.
fn ten(text: &str) -> f32 {
    text.chars().count() as f32 * 10.0
}

#[test]
fn text_that_fits_is_left_alone() {
    assert_eq!(fit_line("Heart Charm", 110.0, &ten), "Heart Charm");
    assert_eq!(fit_line("", 0.0, &ten), "");
}

#[test]
fn text_that_does_not_fit_is_cut_with_dots_and_then_fits() {
    let cut = fit_line("A very long charm name", 120.0, &ten);
    assert_eq!(cut, "A very lo...");
    assert!(ten(&cut) <= 120.0);
    // The dots never follow a space.
    assert_eq!(fit_line("ab cdefghijkl", 60.0, &ten), "ab...");
}

#[test]
fn text_in_a_width_that_cannot_hold_even_the_dots_is_empty() {
    assert_eq!(fit_line("Heart Charm", 20.0, &ten), "");
    assert_eq!(fit_line("Heart Charm", 0.0, &ten), "");
    assert_eq!(fit_line("Heart Charm", f32::NAN, &ten), "");
}

#[test]
fn cutting_never_splits_a_character() {
    let cut = fit_line("ééééééééé", 60.0, &ten);
    assert_eq!(cut, "ééé...");
}

#[test]
fn a_description_is_wrapped_at_spaces() {
    let lines = wrap_lines("one two three four five six", 100.0, 5, &ten);
    assert_eq!(lines, ["one two", "three four", "five six"]);
    for line in &lines {
        assert!(ten(line) <= 100.0, "{line}");
    }
}

#[test]
fn a_description_that_needs_more_lines_than_there_are_ends_in_dots() {
    let lines = wrap_lines("one two three four five six", 100.0, 2, &ten);
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0], "one two");
    assert_eq!(lines[1], "three f...");
    assert!(lines.iter().all(|line| ten(line) <= 100.0));
}

#[test]
fn one_word_wider_than_a_line_is_cut_and_the_rest_goes_on() {
    let lines = wrap_lines("abcdefghijklmnop ok", 80.0, 3, &ten);
    assert_eq!(lines, ["abcde...", "ok"]);
}

#[test]
fn empty_text_or_no_lines_wrap_to_nothing() {
    assert!(wrap_lines("", 100.0, 3, &ten).is_empty());
    assert!(wrap_lines("   ", 100.0, 3, &ten).is_empty());
    assert!(wrap_lines("some words", 100.0, 0, &ten).is_empty());
    // A width that holds nothing still ends, with empty lines.
    assert!(wrap_lines("some words", 0.0, 2, &ten).len() <= 2);
}

#[test]
fn a_description_of_the_longest_allowed_length_wraps_in_the_panel() {
    let text = "word ".repeat(40);
    let text = text.trim();
    assert!(text.chars().count() <= 200);
    let lines = wrap_lines(text, 500.0, DESCRIPTION_LINES, &ten);
    assert_eq!(lines.len(), DESCRIPTION_LINES);
    assert!(lines.iter().all(|line| ten(line) <= 500.0));
}
