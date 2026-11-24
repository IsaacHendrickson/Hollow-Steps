use macroquad::math::{Rect, Vec2};

use super::*;
use crate::pause_menu::{panel_rect, PauseScreen};
use crate::seed::parse_seed;

// A corner note, not a button: checked when this file compiles.
const _: () = assert!(SEED_FONT_SIZE <= 16);

const VIEW: Vec2 = Vec2::new(960.0, 540.0);
const SCREENS: [PauseScreen; 4] = [
    PauseScreen::Main,
    PauseScreen::RunMain,
    PauseScreen::Settings,
    PauseScreen::Confirm,
];

/// A generous estimate of the line's size: half a font size per character is wider than the
/// default font draws.
fn estimated_size(label: &str) -> Vec2 {
    let size = f32::from(SEED_FONT_SIZE);
    Vec2::new(label.chars().count() as f32 * size * 0.5, size)
}

#[test]
fn a_run_with_a_seed_shows_seed_and_its_sixteen_hex_digits() {
    assert_eq!(
        seed_label(true, Some(0xab)),
        Some("Seed 00000000000000ab".to_owned())
    );
    assert_eq!(
        seed_label(true, Some(u64::MAX)),
        Some("Seed ffffffffffffffff".to_owned())
    );
    assert_eq!(
        seed_label(true, Some(0)),
        Some("Seed 0000000000000000".to_owned())
    );
}

#[test]
fn the_digits_are_what_the_seed_box_reads_back() {
    for seed in [0, 1, 0xdead_beef, 0x0123_4567_89ab_cdef, u64::MAX] {
        let label = seed_label(true, Some(seed)).unwrap();
        let digits = label.strip_prefix("Seed ").unwrap();
        assert_eq!(parse_seed(digits), Ok(seed), "{label}");
    }
}

#[test]
fn there_is_no_line_outside_a_run_or_without_a_seed() {
    assert_eq!(seed_label(false, Some(7)), None, "the hub");
    assert_eq!(seed_label(false, None), None);
    assert_eq!(seed_label(true, None), None, "a run that has no seed");
}

#[test]
fn the_line_is_anchored_to_the_corner_of_whatever_the_view_is() {
    let size = estimated_size("Seed 00000000000000ab");
    for view in [VIEW, Vec2::new(1920.0, 1080.0), Vec2::new(640.0, 360.0)] {
        let origin = seed_origin(view, size);
        assert_eq!(origin, anchored(view, Anchor::TopLeft, size, SEED_MARGIN));
        assert_eq!(origin, SEED_MARGIN);
        assert!(Rect::new(origin.x, origin.y, size.x, size.y).right() < view.x);
    }
}

#[test]
fn the_line_never_covers_a_pause_panel_at_the_virtual_resolution() {
    let size = estimated_size("Seed 00000000000000ab");
    let origin = seed_origin(VIEW, size);
    let line = Rect::new(origin.x, origin.y, size.x, size.y);
    for screen in SCREENS {
        let panel = panel_rect(screen, VIEW);
        assert!(
            !line.overlaps(&panel),
            "{screen:?}: the line {line:?} covers the panel {panel:?}"
        );
    }
}
