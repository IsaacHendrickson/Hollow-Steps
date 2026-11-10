use hollow_steps::editor_view::*;
use macroquad::math::{Rect, Vec2};

fn view() -> CanvasView {
    CanvasView {
        offset: Vec2::new(37.0, -12.0),
        scale: 0.8,
    }
}

fn close(a: Vec2, b: Vec2) -> bool {
    (a - b).length() < 1e-3
}

#[test]
fn screen_and_world_round_trip() {
    for view in [
        view(),
        CanvasView {
            offset: Vec2::new(-400.0, 900.0),
            scale: MAX_ZOOM,
        },
        CanvasView {
            offset: Vec2::ZERO,
            scale: MIN_ZOOM,
        },
    ] {
        for world in [
            Vec2::ZERO,
            Vec2::new(960.0, 460.0),
            Vec2::new(-35.5, 1234.25),
        ] {
            assert!(close(view.world(view.screen(world)), world));
        }
        let rect = view.rect(Rect::new(10.0, 20.0, 100.0, 50.0));
        assert!(close(view.world(rect.point()), Vec2::new(10.0, 20.0)));
        assert!((rect.w - 100.0 * view.scale).abs() < 1e-3);
    }
}

#[test]
fn zoom_keeps_the_point_under_the_cursor_fixed() {
    for cursor in [
        Vec2::new(300.0, 200.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(599.0, 420.0),
    ] {
        for steps in [1.0, -1.0, 2.5, -3.0] {
            let mut zoomed = view();
            let before = zoomed.world(cursor);
            zoomed.zoom_at(cursor, steps);
            assert!(close(zoomed.world(cursor), before), "{cursor:?} {steps}");
            assert_ne!(zoomed.scale, view().scale);
        }
    }
}

#[test]
fn zooming_in_grows_and_zooming_out_shrinks() {
    let mut zoomed = view();
    zoomed.zoom_at(Vec2::new(100.0, 100.0), 1.0);
    assert!((zoomed.scale - 0.8 * ZOOM_PER_STEP).abs() < 1e-4);
    zoomed.zoom_at(Vec2::new(100.0, 100.0), -1.0);
    assert!((zoomed.scale - 0.8).abs() < 1e-4);
}

#[test]
fn zoom_is_clamped_and_keeps_the_cursor_fixed_at_the_limits() {
    let cursor = Vec2::new(250.0, 150.0);
    let mut zoomed = view();
    for _ in 0..200 {
        zoomed.zoom_at(cursor, 3.0);
    }
    assert_eq!(zoomed.scale, MAX_ZOOM);
    let anchor = zoomed.world(cursor);
    zoomed.zoom_at(cursor, 3.0);
    assert!(close(zoomed.world(cursor), anchor));
    for _ in 0..200 {
        zoomed.zoom_at(cursor, -3.0);
    }
    assert_eq!(zoomed.scale, MIN_ZOOM);
}

#[test]
fn one_huge_wheel_spin_counts_as_a_few_steps() {
    let mut spun = view();
    spun.zoom_at(Vec2::ZERO, 1000.0);
    let mut limited = view();
    limited.zoom_at(Vec2::ZERO, MAX_WHEEL_STEPS);
    assert_eq!(spun, limited);
}

#[test]
fn non_finite_zoom_and_pan_input_is_ignored() {
    let mut v = view();
    v.zoom_at(Vec2::new(f32::NAN, 0.0), 1.0);
    v.zoom_at(Vec2::ZERO, f32::NAN);
    v.zoom_at(Vec2::ZERO, f32::INFINITY);
    v.pan_by(Vec2::new(f32::INFINITY, 0.0));
    assert_eq!(v, view());
}

#[test]
fn fit_places_the_room_at_the_area_corner_and_never_exceeds_the_limits() {
    let room = Rect::new(80.0, 40.0, 800.0, 360.0);
    let area = Rect::new(20.0, 90.0, 560.0, 300.0);
    let fitted = CanvasView::fit(room, area);
    assert!(close(fitted.screen(room.point()), area.point()));
    let fitted_room = fitted.rect(room);
    assert!(fitted_room.right() <= area.right() + 1e-3);
    assert!(fitted_room.bottom() <= area.bottom() + 1e-3);
    let tiny = CanvasView::fit(
        Rect::new(0.0, 0.0, 32.0, 48.0),
        Rect::new(0.0, 0.0, 5000.0, 5000.0),
    );
    assert_eq!(tiny.scale, MAX_ZOOM);
    let degenerate = CanvasView::fit(room, Rect::new(0.0, 0.0, 0.0, 0.0));
    assert!(degenerate.scale.is_finite() && degenerate.scale >= MIN_ZOOM);
}

#[test]
fn panning_moves_the_view_and_cannot_lose_the_room() {
    let room = Rect::new(0.0, 0.0, 960.0, 460.0);
    let viewport = Rect::new(0.0, 80.0, 600.0, 380.0);
    let mut v = CanvasView::fit(room, viewport);
    let start = v.offset;
    v.pan_by(Vec2::new(15.0, -5.0));
    assert!(close(v.offset, start + Vec2::new(15.0, -5.0)));

    v.pan_by(Vec2::new(-100_000.0, 50_000.0));
    v.keep_visible(room, viewport);
    let r = v.rect(room);
    assert!(r.right() >= viewport.x + KEEP_VISIBLE - 1e-3);
    assert!(r.y <= viewport.bottom() - KEEP_VISIBLE + 1e-3);

    v.pan_by(Vec2::new(100_000.0, -100_000.0));
    v.keep_visible(room, viewport);
    let r = v.rect(room);
    assert!(r.x <= viewport.right() - KEEP_VISIBLE + 1e-3);
    assert!(r.bottom() >= viewport.y + KEEP_VISIBLE - 1e-3);
}

#[test]
fn keep_visible_leaves_a_visible_room_alone() {
    let room = Rect::new(0.0, 0.0, 960.0, 460.0);
    let viewport = Rect::new(0.0, 80.0, 600.0, 380.0);
    let mut v = CanvasView::fit(room, viewport);
    let before = v;
    v.keep_visible(room, viewport);
    assert_eq!(v, before);
}

#[test]
fn pan_drag_reports_frame_to_frame_movement_only_while_active() {
    let mut pan = PanDrag::default();
    assert!(!pan.is_panning());
    assert_eq!(pan.update(false, Vec2::new(10.0, 10.0)), Vec2::ZERO);
    // The first active frame only records the start point.
    assert_eq!(pan.update(true, Vec2::new(10.0, 10.0)), Vec2::ZERO);
    assert!(pan.is_panning());
    assert_eq!(pan.update(true, Vec2::new(14.0, 7.0)), Vec2::new(4.0, -3.0));
    assert_eq!(pan.update(true, Vec2::new(14.0, 9.0)), Vec2::new(0.0, 2.0));
    assert_eq!(pan.update(false, Vec2::new(50.0, 50.0)), Vec2::ZERO);
    assert!(!pan.is_panning());
    // A new drag does not jump by the distance moved while idle.
    assert_eq!(pan.update(true, Vec2::new(90.0, 90.0)), Vec2::ZERO);
}

fn grid(size: f32) -> Grid {
    Grid {
        enabled: true,
        size,
    }
}

#[test]
fn grid_defaults_to_an_eight_pixel_snap_and_is_off() {
    let grid = Grid::default();
    assert!(!grid.enabled);
    assert_eq!(grid.size, 8.0);
    assert_eq!(grid.step(), 8.0);
}

#[test]
fn snap_rounds_to_the_nearest_line_and_ties_away_from_the_origin() {
    let g = grid(8.0);
    assert_eq!(
        g.snap(Vec2::new(11.9, 12.0), Vec2::ZERO),
        Vec2::new(8.0, 16.0)
    );
    assert_eq!(g.snap(Vec2::new(3.9, 4.0), Vec2::ZERO), Vec2::new(0.0, 8.0));
    assert_eq!(
        g.snap(Vec2::new(-3.9, -4.0), Vec2::ZERO),
        Vec2::new(0.0, -8.0)
    );
    assert_eq!(
        g.snap(Vec2::new(16.0, -24.0), Vec2::ZERO),
        Vec2::new(16.0, -24.0)
    );
}

#[test]
fn snap_is_measured_from_the_room_corner() {
    let g = grid(10.0);
    let origin = Vec2::new(83.0, 41.0);
    assert_eq!(
        g.snap(Vec2::new(100.0, 52.0), origin),
        Vec2::new(103.0, 51.0)
    );
    assert_eq!(g.snap(origin, origin), origin);
}

#[test]
fn snap_does_nothing_when_off_or_given_bad_numbers() {
    let p = Vec2::new(11.9, 12.3);
    let off = Grid {
        enabled: false,
        size: 8.0,
    };
    assert_eq!(off.snap(p, Vec2::ZERO), p);
    assert!(grid(8.0)
        .snap(Vec2::new(f32::NAN, 1.0), Vec2::ZERO)
        .x
        .is_nan());
}

#[test]
fn snap_size_is_clamped_to_a_usable_range() {
    assert_eq!(grid(0.0).step(), MIN_SNAP);
    assert_eq!(grid(-5.0).step(), MIN_SNAP);
    assert_eq!(grid(100_000.0).step(), MAX_SNAP);
    assert_eq!(grid(f32::NAN).step(), DEFAULT_SNAP);
    // A zero size must not divide by zero.
    assert_eq!(
        grid(0.0).snap(Vec2::new(2.4, 2.6), Vec2::ZERO),
        Vec2::new(2.0, 3.0)
    );
}

#[test]
fn drawn_grid_lines_thin_out_when_zoomed_out() {
    let g = grid(8.0);
    assert_eq!(g.line_step(1.0), Some(8.0));
    // 8 * 0.5 = 4 px is too dense, so lines double to 16 world units (8 px).
    assert_eq!(g.line_step(0.5), Some(16.0));
    let sparse = g.line_step(MIN_ZOOM).unwrap();
    assert!(sparse * MIN_ZOOM >= MIN_GRID_PIXELS);
    assert_eq!(Grid::default().line_step(1.0), None);
    assert_eq!(g.line_step(0.0), None);
    assert_eq!(g.line_step(f32::NAN), None);
}

#[test]
fn grid_lines_sit_on_multiples_from_the_origin_inside_the_range() {
    let lines = grid_lines(85.0, 120.0, 80.0, 10.0).collect::<Vec<_>>();
    assert_eq!(lines, vec![90.0, 100.0, 110.0, 120.0]);
    assert_eq!(grid_lines(0.0, 30.0, 0.0, 10.0).count(), 4);
    assert_eq!(grid_lines(5.0, 5.0, 0.0, 10.0).count(), 0);
    assert_eq!(grid_lines(0.0, 100.0, 0.0, 0.0).count(), 0);
    assert_eq!(grid_lines(0.0, 100.0, 0.0, f32::NAN).count(), 0);
    assert_eq!(grid_lines(f32::NAN, 100.0, 0.0, 10.0).count(), 0);
    assert!(grid_lines(0.0, 1.0e9, 0.0, 1.0).count() <= 1000);
}

#[test]
fn regions_follow_the_window_and_panel() {
    let small = CanvasRegions::new(Vec2::new(960.0, 540.0), 600.0, 1);
    let large = CanvasRegions::new(Vec2::new(1600.0, 1000.0), 1240.0, 1);
    for r in [small, large] {
        assert_eq!(r.header.bottom(), r.viewport.y);
        assert!((r.viewport.bottom() - r.footer.y).abs() < 1e-3);
    }
    assert_eq!(small.viewport.w, 600.0);
    assert_eq!(small.footer.bottom(), 540.0);
    assert_eq!(large.footer.bottom(), 1000.0);
    assert!(large.viewport.h > small.viewport.h);
    assert!(large.footer_wrap_width() > small.footer_wrap_width());
    assert!(small.footer_wrap_width() < small.footer.w);
}

#[test]
fn footer_reserves_a_fixed_height_until_a_message_needs_more() {
    let window = Vec2::new(960.0, 540.0);
    let none = CanvasRegions::new(window, 600.0, 0);
    let one = CanvasRegions::new(window, 600.0, 1);
    let reserved = CanvasRegions::new(window, 600.0, FOOTER_RESERVED_LINES);
    assert_eq!(none, one);
    assert_eq!(one, reserved);
    let long = CanvasRegions::new(window, 600.0, FOOTER_RESERVED_LINES + 4);
    assert!(long.footer.h > reserved.footer.h);
    assert!(long.viewport.h < reserved.viewport.h);
    // Baselines stay inside the footer, one line apart.
    let last = long.footer_baseline(FOOTER_RESERVED_LINES + 3);
    assert!(last <= long.footer.bottom());
    assert!((long.footer_baseline(1) - long.footer_baseline(0) - FOOTER_LINE_HEIGHT).abs() < 1e-4);
    assert!(long.footer_baseline(0) >= long.footer.y);
}

#[test]
fn tiny_windows_give_positive_sizes_instead_of_nan() {
    for window in [
        Vec2::new(10.0, 10.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(300.0, 90.0),
    ] {
        let r = CanvasRegions::new(window, 5.0, 2);
        for rect in [r.header, r.viewport, r.footer, r.fit_area()] {
            assert!(rect.w >= 1.0 && rect.h >= 1.0, "{window:?} {rect:?}");
            assert!(rect.x.is_finite() && rect.y.is_finite());
        }
        assert!(r.footer_wrap_width() >= 1.0);
    }
}

// Fixed-width text: 8 pixels per character.
fn measure(text: &str) -> f32 {
    text.chars().count() as f32 * 8.0
}

#[test]
fn wrapping_breaks_between_words_at_the_width() {
    // 72 px is nine characters: "three four" (ten) does not fit, "one two" (seven) does.
    let lines = wrap_words("one two three four", 72.0, measure);
    assert_eq!(lines, vec!["one two", "three", "four"]);
    assert!(lines.iter().all(|l| measure(l) <= 72.0));
    // Exactly the width still fits on one line.
    assert_eq!(wrap_words("three four", 80.0, measure), vec!["three four"]);
}

#[test]
fn wrapping_changes_with_the_width_and_keeps_every_word() {
    let text = "exit spawn outside destination Second";
    let narrow = wrap_words(text, 100.0, measure);
    let wide = wrap_words(text, 800.0, measure);
    assert!(narrow.len() > wide.len());
    assert_eq!(wide, vec![text.to_owned()]);
    assert_eq!(narrow.join(" "), text);
}

#[test]
fn wrapping_handles_empty_text_extra_spaces_and_no_leading_blank_line() {
    assert!(wrap_words("", 80.0, measure).is_empty());
    assert!(wrap_words("   \n\t ", 80.0, measure).is_empty());
    assert_eq!(wrap_words("  a   b  ", 80.0, measure), vec!["a b"]);
    // A long first word must not push out an empty line.
    let lines = wrap_words("abcdefghijklmnopqrst", 80.0, measure);
    assert!(lines.iter().all(|l| !l.is_empty()));
}

#[test]
fn wrapping_breaks_words_longer_than_a_line() {
    let path = "assets/rooms/a_very_long_room_file_name.ron:";
    let lines = wrap_words(path, 80.0, measure);
    assert!(lines.len() > 1);
    assert!(lines.iter().all(|l| measure(l) <= 80.0));
    assert_eq!(lines.concat(), path);
    // A line narrower than one character still terminates and keeps every character.
    assert_eq!(wrap_words("abc", 1.0, measure).concat(), "abc");
}
