//! The looks of the puzzle pieces as pure draw lists: no window, nothing fixed to the screen.
use super::*;
use hollow_steps::puzzle::LAST_SWITCH;

const SIZES: [(f32, f32); 5] = [
    (8.0, 8.0),
    (24.0, 24.0),
    (32.0, 64.0),
    (64.0, 16.0),
    (200.0, 320.0),
];

fn rect(width: f32, height: f32) -> Rect {
    Rect::new(100.0, 50.0, width, height)
}

fn lines(shapes: &[Shape]) -> usize {
    shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Line { .. }))
        .count()
}

/// Every shape lies inside `rect` (lines count by their end points; their thickness may reach
/// half a thickness beyond).
fn assert_inside(shapes: &[Shape], rect: Rect, what: &str) {
    let slack = 0.01;
    for shape in shapes {
        let (low, high) = shape.bounds();
        assert!(
            low.0 >= rect.x - slack - CRACK_THICKNESS
                && low.1 >= rect.y - slack - CRACK_THICKNESS
                && high.0 <= rect.right() + slack + CRACK_THICKNESS
                && high.1 <= rect.bottom() + slack + CRACK_THICKNESS,
            "{what}: {shape:?} leaves {rect:?}"
        );
    }
}

#[test]
fn a_wall_has_one_crack_line_for_each_hit_it_has_left() {
    for remaining in 0..=9 {
        let shapes = breakable_shapes(rect(32.0, 64.0), remaining);
        assert_eq!(lines(&shapes), usize::from(remaining), "{remaining} left");
    }
}

#[test]
fn a_cracked_wall_looks_different_for_each_number_of_hits_left() {
    let looks: Vec<_> = (1..=9)
        .map(|remaining| breakable_shapes(rect(32.0, 64.0), remaining))
        .collect();
    for (i, a) in looks.iter().enumerate() {
        for b in &looks[i + 1..] {
            assert_ne!(a, b);
        }
    }
}

#[test]
fn every_piece_is_drawn_inside_its_own_rectangle_at_any_size() {
    for (width, height) in SIZES {
        let at = rect(width, height);
        assert_inside(&breakable_shapes(at, 9), at, "wall");
        for on in [false, true] {
            assert_inside(&switch_shapes(at, 8, on), at, "switch");
        }
        assert_inside(&barrier_shapes(at, 3), at, "barrier");
    }
}

#[test]
fn a_switch_looks_different_on_and_off_and_the_lever_leans_the_other_way() {
    let at = rect(24.0, 24.0);
    let (off, on) = (switch_shapes(at, 2, false), switch_shapes(at, 2, true));
    assert_ne!(off, on);
    assert_eq!((lines(&off), lines(&on)), (1, 1));
    let tip = |shapes: &[Shape]| {
        shapes.iter().find_map(|shape| match shape {
            Shape::Line { from, to, .. } => Some((from.0, to.0)),
            _ => None,
        })
    };
    let (pivot_off, tip_off) = tip(&off).unwrap();
    let (pivot_on, tip_on) = tip(&on).unwrap();
    assert!(tip_off < pivot_off && tip_on > pivot_on);
}

#[test]
fn the_same_piece_always_draws_the_same_way() {
    let at = rect(24.0, 96.0);
    assert_eq!(barrier_shapes(at, 4), barrier_shapes(at, 4));
    assert_eq!(switch_shapes(at, 4, true), switch_shapes(at, 4, true));
    assert_eq!(breakable_shapes(at, 5), breakable_shapes(at, 5));
}

#[test]
fn each_switch_number_has_its_own_tint_and_a_barrier_wears_its_switchs() {
    let tints: Vec<_> = (FIRST_SWITCH..=LAST_SWITCH).map(switch_tint).collect();
    for (i, a) in tints.iter().enumerate() {
        for b in &tints[i + 1..] {
            assert_ne!(a, b);
        }
    }
    let at = rect(24.0, 96.0);
    let looks: Vec<_> = (FIRST_SWITCH..=LAST_SWITCH)
        .map(|id| barrier_shapes(at, id))
        .collect();
    for (i, a) in looks.iter().enumerate() {
        for b in &looks[i + 1..] {
            assert_ne!(a, b, "barriers of two switches differ");
        }
    }
    let wears = |shapes: &[Shape], id: u8| {
        let tint = switch_tint(id);
        shapes.iter().any(|shape| {
            matches!(shape, Shape::Rect { color, .. }
                if (color.r, color.g, color.b) == tint)
        })
    };
    for id in FIRST_SWITCH..=LAST_SWITCH {
        assert!(wears(&barrier_shapes(at, id), id), "barrier {id}");
        assert!(wears(&switch_shapes(at, id, true), id), "switch {id} on");
    }
    assert!(!wears(&barrier_shapes(at, 2), 3));
}

#[test]
fn a_barrier_is_a_block_of_rectangles_with_at_least_two_bars() {
    for (width, height) in SIZES {
        let shapes = barrier_shapes(rect(width, height), 1);
        assert_eq!(lines(&shapes), 0);
        assert!(shapes.len() >= 3 + 2, "{width}x{height}: {}", shapes.len());
    }
    let wide = barrier_shapes(rect(80.0, 96.0), 1);
    let narrow = barrier_shapes(rect(8.0, 96.0), 1);
    assert!(wide.len() > narrow.len(), "a wider barrier has more bars");
}

#[test]
fn a_switch_number_outside_the_range_still_draws_something() {
    for id in [0, 9, 255] {
        let at = rect(24.0, 24.0);
        assert!(!switch_shapes(at, id, true).is_empty());
        assert!(!barrier_shapes(at, id).is_empty());
    }
}
