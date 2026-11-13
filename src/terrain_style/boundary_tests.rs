//! The boundary slabs: where they sit around the playable area, the doorways cut for exits,
//! and the inputs that must not break them.
use macroquad::math::{Rect, Vec2};

use super::*;
use crate::room::{Exit, RoomEdge};

pub(super) const T: f32 = 32.0;

/// Right edge 900, bottom edge 450.
pub(super) fn bounds() -> Rect {
    Rect::new(100.0, 50.0, 800.0, 400.0)
}

pub(super) fn exit(edge: RoomEdge, offset: f32, length: f32) -> Exit {
    Exit {
        edge,
        offset,
        length,
        target_room: RoomId::Second,
        spawn: Vec2::ZERO,
    }
}

/// The area two rectangles share; 0 when they only touch or are apart.
pub(super) fn overlap(a: &Rect, b: &Rect) -> f32 {
    let w = a.right().min(b.right()) - a.x.max(b.x);
    let h = a.bottom().min(b.bottom()) - a.y.max(b.y);
    if w > 0.0 && h > 0.0 {
        w * h
    } else {
        0.0
    }
}

fn area(rects: &[Rect]) -> f32 {
    rects.iter().map(|r| r.w * r.h).sum()
}

pub(super) fn one_exit_per_edge() -> [Exit; 4] {
    [
        exit(RoomEdge::Left, 100.0, 80.0),
        exit(RoomEdge::Right, 200.0, 96.0),
        exit(RoomEdge::Top, 300.0, 120.0),
        exit(RoomEdge::Bottom, 500.0, 90.0),
    ]
}

#[test]
fn with_the_floor_at_the_bottom_four_slabs_frame_the_room_flush_against_it() {
    let slabs = boundary_slabs(bounds(), 450.0, T, &[]);

    assert_eq!(
        slabs,
        vec![
            Rect::new(68.0, 18.0, 864.0, 32.0),  // ceiling, corners included
            Rect::new(68.0, 50.0, 32.0, 400.0),  // left wall
            Rect::new(900.0, 50.0, 32.0, 400.0), // right wall
            Rect::new(68.0, 450.0, 864.0, 32.0), // floor slab, one thickness below the bounds
        ]
    );
}

#[test]
fn with_the_floor_above_the_bottom_the_walls_stop_at_it_and_the_slab_fills_the_ground() {
    // The floor line is 50 px above the bottom of the bounds.
    let slabs = boundary_slabs(bounds(), 400.0, T, &[]);

    assert_eq!(
        slabs,
        vec![
            Rect::new(68.0, 18.0, 864.0, 32.0),
            Rect::new(68.0, 50.0, 32.0, 350.0), // walls end at the floor line
            Rect::new(900.0, 50.0, 32.0, 350.0),
            // From the floor line to one thickness below the bounds: 50 of ground inside them
            // and 32 below.
            Rect::new(68.0, 400.0, 864.0, 82.0),
        ]
    );
}

#[test]
fn the_slabs_never_enter_the_playable_area_or_each_other() {
    for (floor, thickness) in [
        (450.0, 32.0),
        (400.0, 32.0),
        (450.0, 8.0),
        (420.0, 100.0),
        (98.0, 16.0), // the lowest floor a room may have: 48 px of height
    ] {
        let slabs = boundary_slabs(bounds(), floor, thickness, &[]);
        // Where the player can be: the bounds above the floor line.
        let playable = Rect::new(100.0, 50.0, 800.0, floor - 50.0);
        for (i, slab) in slabs.iter().enumerate() {
            assert_eq!(
                overlap(slab, &playable),
                0.0,
                "slab {i} {slab:?} intrudes (floor {floor}, thickness {thickness})"
            );
            for other in &slabs[i + 1..] {
                assert_eq!(overlap(slab, other), 0.0, "{slab:?} overlaps {other:?}");
            }
        }
        // Together they are the whole ring: the outer box minus the playable area.
        let outer = (800.0 + 2.0 * thickness) * (400.0 + 2.0 * thickness);
        let expected = outer - 800.0 * (floor - 50.0);
        assert!(
            (area(&slabs) - expected).abs() < 0.01,
            "ring area {} != {expected} (floor {floor}, thickness {thickness})",
            area(&slabs)
        );
    }
}

#[test]
fn an_exit_zone_is_never_covered_whichever_edge_it_is_on() {
    for floor in [450.0, 400.0] {
        let exits = one_exit_per_edge();
        let slabs = boundary_slabs(bounds(), floor, T, &exits);
        for exit in &exits {
            let zone = exit.zone(bounds());
            for slab in &slabs {
                assert_eq!(
                    overlap(slab, &zone),
                    0.0,
                    "{:?} exit zone {zone:?} is covered by {slab:?} (floor {floor})",
                    exit.edge
                );
            }
        }
    }
}

#[test]
fn a_side_exit_cuts_a_gap_as_long_as_its_span_through_the_wall() {
    // The exit spans y 150 to 230 (offset 100 from the top of the bounds, length 80).
    let slabs = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Left, 100.0, 80.0)]);

    assert_eq!(
        slabs,
        vec![
            Rect::new(68.0, 18.0, 864.0, 32.0),
            Rect::new(68.0, 50.0, 32.0, 100.0), // wall above the doorway
            Rect::new(68.0, 230.0, 32.0, 220.0), // wall below it
            Rect::new(900.0, 50.0, 32.0, 400.0),
            Rect::new(68.0, 450.0, 864.0, 32.0),
        ]
    );
    let right = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Right, 200.0, 96.0)]);
    assert!(right.contains(&Rect::new(900.0, 50.0, 32.0, 200.0)));
    assert!(right.contains(&Rect::new(900.0, 346.0, 32.0, 104.0)));
}

#[test]
fn top_and_bottom_exits_cut_the_ceiling_and_the_floor_slab() {
    let top = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Top, 300.0, 120.0)]);
    // The zone is x 400 to 520.
    assert!(top.contains(&Rect::new(68.0, 18.0, 332.0, 32.0)));
    assert!(top.contains(&Rect::new(520.0, 18.0, 412.0, 32.0)));
    assert_eq!(top.len(), 5);

    let bottom = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Bottom, 500.0, 90.0)]);
    // The zone is x 600 to 690.
    assert!(bottom.contains(&Rect::new(68.0, 450.0, 532.0, 32.0)));
    assert!(bottom.contains(&Rect::new(690.0, 450.0, 242.0, 32.0)));
    assert_eq!(bottom.len(), 5);
}

#[test]
fn an_exit_at_the_start_of_an_edge_leaves_one_piece_and_one_along_all_of_it_leaves_none() {
    let at_start = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Left, 0.0, 80.0)]);
    assert!(at_start.contains(&Rect::new(68.0, 130.0, 32.0, 320.0)));
    assert_eq!(
        at_start.len(),
        4,
        "ceiling, one wall piece, right wall, floor"
    );

    let whole = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Left, 0.0, 400.0)]);
    assert_eq!(whole.len(), 3, "no left wall at all");
    assert!(whole
        .iter()
        .all(|slab| slab.x >= 68.0 && overlap(slab, &bounds()) == 0.0));
}

#[test]
fn several_exits_on_one_edge_each_get_their_own_gap() {
    let slabs = boundary_slabs(
        bounds(),
        450.0,
        T,
        &[
            exit(RoomEdge::Right, 20.0, 40.0),
            exit(RoomEdge::Right, 200.0, 40.0),
        ],
    );
    let walls: Vec<Rect> = slabs.iter().copied().filter(|s| s.x == 900.0).collect();

    assert_eq!(
        walls,
        vec![
            Rect::new(900.0, 50.0, 32.0, 20.0),   // above the first doorway
            Rect::new(900.0, 110.0, 32.0, 140.0), // between the two
            Rect::new(900.0, 290.0, 32.0, 160.0), // under the second
        ]
    );
}

#[test]
fn overlapping_exits_open_one_gap_and_a_crumb_beside_a_doorway_is_dropped() {
    // Spans y 150 to 230 and y 200 to 280 together make one gap from 150 to 280.
    let overlapping = boundary_slabs(
        bounds(),
        450.0,
        T,
        &[
            exit(RoomEdge::Left, 100.0, 80.0),
            exit(RoomEdge::Left, 150.0, 80.0),
        ],
    );
    let walls: Vec<Rect> = overlapping
        .iter()
        .copied()
        .filter(|s| s.x == 68.0 && s.w == T)
        .collect();
    assert_eq!(
        walls,
        vec![
            Rect::new(68.0, 50.0, 32.0, 100.0),
            Rect::new(68.0, 280.0, 32.0, 170.0),
        ]
    );

    // A doorway half a pixel under the ceiling leaves no wall piece above it.
    let crumb = boundary_slabs(bounds(), 450.0, T, &[exit(RoomEdge::Left, 0.5, 40.0)]);
    let left: Vec<Rect> = crumb
        .iter()
        .copied()
        .filter(|s| s.x == 68.0 && s.w == T)
        .collect();
    assert_eq!(left, vec![Rect::new(68.0, 90.5, 32.0, 359.5)]);
}

#[test]
fn bad_inputs_give_no_slabs_and_a_bad_exit_is_ignored() {
    for thickness in [0.0, -4.0, f32::NAN, f32::INFINITY] {
        assert!(
            boundary_slabs(bounds(), 450.0, thickness, &[]).is_empty(),
            "{thickness}"
        );
    }
    assert!(boundary_slabs(Rect::new(f32::NAN, 0.0, 10.0, 10.0), 5.0, T, &[]).is_empty());
    assert!(boundary_slabs(Rect::new(0.0, 0.0, 0.0, 100.0), 50.0, T, &[]).is_empty());
    assert!(boundary_slabs(bounds(), f32::NAN, T, &[]).is_empty());

    let plain = boundary_slabs(bounds(), 450.0, T, &[]);
    let broken = exit(RoomEdge::Left, f32::NAN, 80.0);
    assert_eq!(boundary_slabs(bounds(), 450.0, T, &[broken]), plain);
    // A floor line outside the bounds is clamped into them.
    assert_eq!(boundary_slabs(bounds(), 9_999.0, T, &[]), plain);
    let high = boundary_slabs(bounds(), -5.0, T, &[]);
    assert_eq!(high.len(), 2, "walls of no height are dropped: {high:?}");
}

#[test]
fn the_same_inputs_give_the_same_slabs_in_the_same_order() {
    let exits = one_exit_per_edge();

    assert_eq!(
        boundary_slabs(bounds(), 430.0, T, &exits),
        boundary_slabs(bounds(), 430.0, T, &exits)
    );
}
