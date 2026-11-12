//! Margins, invalid input, the cache and the small helpers.

use macroquad::math::Rect;

use super::tests::{list, platform, style, ROOM};
use super::*;

#[test]
fn the_list_never_extends_past_the_documented_margin() {
    let cases: [&[Rect]; 5] = [
        &[Rect::new(100.0, 300.0, 400.0, 40.0)],
        &[
            Rect::new(100.0, 300.0, 100.0, 40.0),
            Rect::new(100.0, 340.0, 100.0, 40.0),
        ],
        &[
            Rect::new(100.0, 300.0, 50.0, 40.0),
            Rect::new(150.0, 300.0, 50.0, 40.0),
        ],
        &[
            Rect::new(0.0, 100.0, 100.0, 40.0),
            Rect::new(60.0, 80.0, 100.0, 40.0),
        ],
        &[
            Rect::new(10.0, 10.0, 5.0, 3.0),
            Rect::new(40.0, 10.0, 2.0, 2.0),
        ],
    ];
    for solids in cases {
        for shape in list(solids) {
            let (low, high) = shape.bounds();
            assert!(
                solids.iter().any(|s| {
                    low.0 >= s.x - 1e-3
                        && high.0 <= s.right() + 1e-3
                        && low.1 >= s.y - TOP_MARGIN - 1e-3
                        && high.1 <= s.bottom() + BOTTOM_MARGIN + 1e-3
                }),
                "{shape:?} strays from {solids:?}"
            );
        }
    }
}

#[test]
fn tiny_and_invalid_solids_do_not_panic_or_stray() {
    let solids = [
        Rect::new(10.0, 10.0, 1.0, 1.0),
        Rect::new(30.0, 10.0, 0.0, 10.0),
        Rect::new(f32::NAN, 10.0, 10.0, 10.0),
        Rect::new(50.0, 10.0, -5.0, 10.0),
        Rect::new(70.0, f32::INFINITY, 10.0, 10.0),
    ];
    let shapes = list(&solids);

    assert!(!shapes.is_empty(), "the 1x1 solid still draws");
    assert!(shapes.iter().all(|s| {
        let (low, high) = s.bounds();
        [low.0, low.1, high.0, high.1].iter().all(|v| v.is_finite())
    }));
    assert!(list(&solids[1..]).is_empty(), "invalid solids draw nothing");
}

#[test]
fn every_color_is_a_valid_color() {
    let shapes = list(&[platform(100.0, 300.0)]);
    for shape in &shapes {
        let color = match shape {
            Shape::Rect { color, .. }
            | Shape::Line { color, .. }
            | Shape::Triangle { color, .. }
            | Shape::Polygon { color, .. } => color,
        };
        for part in [color.r, color.g, color.b, color.a] {
            assert!((0.0..=1.0).contains(&part), "{shape:?}");
        }
    }
}

#[test]
fn the_cache_rebuilds_only_when_something_changes() {
    let mut look = TerrainLook::default();
    let solids = [platform(100.0, 300.0)];
    let ground = Some(ground_cover(Rect::new(0.0, 0.0, 960.0, 460.0), 460.0));

    let first = look.shapes(ROOM, &solids, ground, &style()).to_vec();
    assert_eq!(look.builds(), 1);
    assert_eq!(look.shapes(ROOM, &solids, ground, &style()), first);
    assert_eq!(look.builds(), 1, "same inputs reuse the list");

    let moved = [platform(120.0, 300.0)];
    assert_ne!(look.shapes(ROOM, &moved, ground, &style()), first);
    assert_eq!(look.builds(), 2, "a moved solid rebuilds");

    look.shapes(RoomId::Second, &moved, ground, &style());
    assert_eq!(look.builds(), 3, "another room rebuilds");

    look.shapes(RoomId::Second, &moved, None, &style());
    assert_eq!(look.builds(), 4, "different ground rebuilds");

    let mossless = TerrainStyle {
        moss: false,
        ..style()
    };
    look.shapes(RoomId::Second, &moved, None, &mossless);
    assert_eq!(look.builds(), 5, "a new palette rebuilds");

    look.shapes(RoomId::Second, &moved, None, &mossless);
    assert_eq!(look.builds(), 5);
}

#[test]
fn the_cache_matches_a_direct_build() {
    let mut look = TerrainLook::default();
    let solids = [platform(100.0, 300.0), Rect::new(520.0, 200.0, 60.0, 160.0)];

    assert_eq!(
        look.shapes(ROOM, &solids, None, &style()),
        list(&solids).as_slice()
    );
}

#[test]
fn an_empty_room_caches_an_empty_list() {
    let mut look = TerrainLook::default();

    assert!(look.shapes(ROOM, &[], None, &style()).is_empty());
    assert!(look.shapes(ROOM, &[], None, &style()).is_empty());
    assert_eq!(look.builds(), 1);
}

#[test]
fn ground_cover_starts_at_the_floor_line_and_reaches_below_it() {
    let cover = ground_cover(Rect::new(0.0, 0.0, 960.0, 460.0), 460.0);

    assert_eq!((cover.x, cover.y, cover.w), (0.0, 460.0, 960.0));
    assert!(
        cover.h > 0.0,
        "a room whose floor is its bottom still has ground"
    );
}

#[test]
fn bounds_cover_every_shape_kind() {
    let color = Rgba::opaque(DEFAULT_FILL);

    assert_eq!(
        Shape::rect(1.0, 2.0, 3.0, 4.0, color).bounds(),
        ((1.0, 2.0), (4.0, 6.0))
    );
    assert_eq!(
        Shape::line((0.0, 0.0), (10.0, 0.0), 2.0, color).bounds(),
        ((-1.0, -1.0), (11.0, 1.0))
    );
    assert_eq!(
        Shape::triangle([(0.0, 5.0), (4.0, 5.0), (2.0, 0.0)], color).bounds(),
        ((0.0, 0.0), (4.0, 5.0))
    );
    assert_eq!(
        Shape::Polygon {
            points: vec![],
            color
        }
        .bounds(),
        ((0.0, 0.0), (0.0, 0.0))
    );
}
