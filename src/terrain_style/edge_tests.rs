use macroquad::math::Rect;

use super::edges::{exposure, Edge, EdgeSpan, Exposure};

fn span(start: f32, end: f32) -> EdgeSpan {
    EdgeSpan { start, end }
}

fn solid(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

fn of(solids: &[Rect], index: usize) -> Exposure {
    exposure(solids, index, &[])
}

#[test]
fn a_single_solid_is_open_on_every_edge() {
    let open = of(&[solid(100.0, 200.0, 80.0, 30.0)], 0);

    assert_eq!(open.top, vec![span(100.0, 180.0)]);
    assert_eq!(open.bottom, vec![span(100.0, 180.0)]);
    assert_eq!(open.left, vec![span(200.0, 230.0)]);
    assert_eq!(open.right, vec![span(200.0, 230.0)]);
    assert!(Edge::ALL.iter().all(|&edge| open.is_exposed(edge)));
    assert!(!open.is_buried());
}

#[test]
fn a_stack_hides_the_seam_between_the_two_solids() {
    let solids = [
        solid(100.0, 200.0, 80.0, 30.0), // upper
        solid(100.0, 230.0, 80.0, 30.0), // directly below, touching
    ];
    let (upper, lower) = (of(&solids, 0), of(&solids, 1));

    assert!(upper.is_exposed(Edge::Top));
    assert!(!upper.is_exposed(Edge::Bottom), "upper bottom is the seam");
    assert!(!lower.is_exposed(Edge::Top), "lower top is the seam");
    assert!(lower.is_exposed(Edge::Bottom));
    for open in [&upper, &lower] {
        assert!(open.is_exposed(Edge::Left) && open.is_exposed(Edge::Right));
    }
}

#[test]
fn a_narrower_block_on_top_leaves_the_rest_of_the_top_open() {
    let solids = [
        solid(100.0, 200.0, 100.0, 20.0),
        solid(120.0, 180.0, 40.0, 20.0), // stands on the middle of the first
    ];
    let wide = of(&solids, 0);

    assert_eq!(wide.top, vec![span(100.0, 120.0), span(160.0, 200.0)]);
    assert!(wide.is_exposed(Edge::Bottom));
    // The block's own bottom is buried in the wide solid.
    assert!(!of(&solids, 1).is_exposed(Edge::Bottom));
}

#[test]
fn side_by_side_solids_hide_their_touching_sides() {
    let solids = [
        solid(100.0, 200.0, 50.0, 20.0),
        solid(150.0, 200.0, 50.0, 20.0),
    ];
    let (left, right) = (of(&solids, 0), of(&solids, 1));

    assert!(!left.is_exposed(Edge::Right));
    assert!(!right.is_exposed(Edge::Left));
    assert_eq!(left.top, vec![span(100.0, 150.0)]);
    assert_eq!(right.top, vec![span(150.0, 200.0)]);
    assert!(left.is_exposed(Edge::Left) && right.is_exposed(Edge::Right));
    assert!(left.is_exposed(Edge::Bottom) && right.is_exposed(Edge::Bottom));
}

#[test]
fn side_by_side_solids_of_different_heights_show_the_step() {
    let solids = [
        solid(100.0, 200.0, 50.0, 60.0), // tall
        solid(150.0, 230.0, 50.0, 30.0), // lower, touching its right side
    ];

    // The tall one's right side is open above the lower one's top.
    assert_eq!(of(&solids, 0).right, vec![span(200.0, 230.0)]);
    // The lower one's left side is fully against the tall one.
    assert!(!of(&solids, 1).is_exposed(Edge::Left));
}

#[test]
fn overlapping_solids_hide_the_edges_inside_each_other() {
    let solids = [
        solid(0.0, 100.0, 100.0, 40.0), // A: x 0..100, y 100..140
        solid(60.0, 80.0, 100.0, 40.0), // B: x 60..160, y 80..120, over A's top right
    ];
    let (a, b) = (of(&solids, 0), of(&solids, 1));

    assert_eq!(a.top, vec![span(0.0, 60.0)], "B covers A's top from x 60");
    assert_eq!(
        b.bottom,
        vec![span(100.0, 160.0)],
        "A covers B's bottom to x 100"
    );
    // A's right edge is open only below B (y 120..140).
    assert_eq!(a.right, vec![span(120.0, 140.0)]);
    // B's left edge is open only above A (y 80..100).
    assert_eq!(b.left, vec![span(80.0, 100.0)]);
}

#[test]
fn a_gap_between_solids_leaves_the_edge_open() {
    let solids = [
        solid(100.0, 200.0, 80.0, 20.0),
        solid(100.0, 170.0, 80.0, 20.0), // floats 10 px above
    ];

    assert_eq!(of(&solids, 0).top, vec![span(100.0, 180.0)]);
    assert!(of(&solids, 1).is_exposed(Edge::Bottom));
}

#[test]
fn solids_sharing_a_top_line_draw_that_stretch_once() {
    let solids = [
        solid(100.0, 200.0, 100.0, 20.0), // larger
        solid(150.0, 200.0, 100.0, 10.0), // smaller, same top, overlapping x 150..200
    ];

    // The larger solid keeps the shared stretch, the smaller gives it up.
    assert_eq!(of(&solids, 0).top, vec![span(100.0, 200.0)]);
    assert_eq!(of(&solids, 1).top, vec![span(200.0, 250.0)]);
}

#[test]
fn identical_solids_expose_the_shared_edges_on_one_of_them_only() {
    let solids = [
        solid(100.0, 200.0, 80.0, 20.0),
        solid(100.0, 200.0, 80.0, 20.0),
    ];
    let (first, second) = (of(&solids, 0), of(&solids, 1));

    assert!(first.is_exposed(Edge::Top));
    assert!(second.is_buried());
}

#[test]
fn a_cover_rectangle_buries_the_edge_it_holds_up() {
    let solids = [solid(100.0, 400.0, 80.0, 60.0)];
    let ground = Rect::new(0.0, 460.0, 960.0, 64.0);
    let open = exposure(&solids, 0, &[ground]);

    assert!(!open.is_exposed(Edge::Bottom), "stands on the ground");
    assert!(open.is_exposed(Edge::Top));
    assert!(open.is_exposed(Edge::Left) && open.is_exposed(Edge::Right));
}

#[test]
fn a_solid_floating_above_the_ground_keeps_its_bottom_open() {
    let solids = [solid(100.0, 380.0, 80.0, 20.0)];
    let ground = Rect::new(0.0, 460.0, 960.0, 64.0);

    assert!(exposure(&solids, 0, &[ground]).is_exposed(Edge::Bottom));
}

#[test]
fn solids_that_only_meet_at_a_corner_do_not_cover_each_other() {
    let solids = [
        solid(100.0, 200.0, 50.0, 20.0),
        solid(150.0, 220.0, 50.0, 20.0), // touches only at (150, 220)
    ];

    for index in 0..2 {
        let open = of(&solids, index);
        assert!(Edge::ALL.iter().all(|&edge| open.is_exposed(edge)));
        assert_eq!(open.top.len(), 1);
        assert_eq!(open.bottom.len(), 1);
    }
    assert_eq!(of(&solids, 0).top, vec![span(100.0, 150.0)]);
}

#[test]
fn an_unknown_index_has_no_exposure() {
    assert_eq!(exposure(&[], 0, &[]), Exposure::default());
    assert!(of(&[solid(0.0, 0.0, 10.0, 10.0)], 3).is_buried());
}

#[test]
fn exposure_does_not_depend_on_the_order_of_the_solids() {
    let (a, b) = (
        solid(0.0, 100.0, 100.0, 40.0),
        solid(60.0, 80.0, 100.0, 40.0),
    );

    assert_eq!(of(&[a, b], 0), of(&[b, a], 1));
    assert_eq!(of(&[a, b], 1), of(&[b, a], 0));
}

#[test]
fn non_finite_neighbors_cover_nothing() {
    let solids = [
        solid(100.0, 200.0, 80.0, 20.0),
        solid(f32::NAN, 180.0, 80.0, 20.0),
        solid(100.0, f32::INFINITY, 80.0, 20.0),
    ];

    assert_eq!(of(&solids, 0), of(&solids[..1], 0));
}
