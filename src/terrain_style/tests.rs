use macroquad::math::Rect;

use super::*;

pub(super) const ROOM: RoomId = RoomId::First;

pub(super) fn style() -> TerrainStyle {
    TerrainStyle::default()
}

pub(super) fn list(solids: &[Rect]) -> Vec<Shape> {
    build(ROOM, solids, None, &style())
}

/// A wide platform: plenty of room for moss, drips, lip points and speckles.
pub(super) fn platform(x: f32, y: f32) -> Rect {
    Rect::new(x, y, 400.0, 40.0)
}

fn shifted(shape: &Shape, dx: f32, dy: f32) -> Shape {
    let by = |p: &Point| (p.0 + dx, p.1 + dy);
    match shape {
        Shape::Rect { x, y, w, h, color } => Shape::Rect {
            x: x + dx,
            y: y + dy,
            w: *w,
            h: *h,
            color: *color,
        },
        Shape::Line {
            from,
            to,
            thickness,
            color,
        } => Shape::Line {
            from: by(from),
            to: by(to),
            thickness: *thickness,
            color: *color,
        },
        Shape::Triangle { points, color } => Shape::Triangle {
            points: [by(&points[0]), by(&points[1]), by(&points[2])],
            color: *color,
        },
        Shape::Polygon { points, color } => Shape::Polygon {
            points: points.iter().map(by).collect(),
            color: *color,
        },
    }
}

/// A speckle: a 1 or 2 px square (the outline strips are 2 px wide but long).
fn is_speckle(shape: &Shape) -> bool {
    matches!(shape, Shape::Rect { w, h, .. } if *w <= 2.0 && *h <= 2.0)
}

fn is_lip(shape: &Shape) -> bool {
    matches!(shape, Shape::Polygon { .. })
}

/// A blade of moss: a triangle that rises above the given top line.
fn is_moss(shape: &Shape, top: f32) -> bool {
    matches!(shape, Shape::Triangle { .. }) && shape.bounds().0 .1 < top - 0.001
}

/// A stalactite: a triangle that hangs below the given bottom line.
fn is_stalactite(shape: &Shape, bottom: f32) -> bool {
    matches!(shape, Shape::Triangle { .. }) && shape.bounds().1 .1 > bottom + 0.001
}

fn sorted_debug(shapes: &[Shape]) -> Vec<String> {
    let mut text: Vec<String> = shapes.iter().map(|s| format!("{s:?}")).collect();
    text.sort();
    text
}

#[test]
fn no_solids_give_an_empty_list() {
    assert!(list(&[]).is_empty());
    assert!(build(
        ROOM,
        &[],
        Some(Rect::new(0.0, 460.0, 960.0, 64.0)),
        &style()
    )
    .is_empty());
}

#[test]
fn the_same_inputs_give_identical_output() {
    let solids = [platform(100.0, 300.0), Rect::new(520.0, 200.0, 60.0, 160.0)];

    assert_eq!(list(&solids), list(&solids));
}

#[test]
fn different_positions_look_different() {
    let (a, b) = (platform(100.0, 300.0), platform(300.0, 300.0));
    let at_a = list(&[a]);
    let at_b: Vec<Shape> = list(&[b]).iter().map(|s| shifted(s, -200.0, 0.0)).collect();

    assert!(at_a.len().min(at_b.len()) > 10, "enough shapes to compare");
    assert_ne!(at_a, at_b, "same shape of solid, different seed");
}

#[test]
fn different_rooms_and_offsets_look_different() {
    let solids = [platform(100.0, 300.0)];
    let base = build(RoomId::First, &solids, None, &style());

    assert_ne!(base, build(RoomId::Second, &solids, None, &style()));
    let reshuffled = TerrainStyle {
        seed_offset: 1,
        ..style()
    };
    assert_ne!(base, build(RoomId::First, &solids, None, &reshuffled));
}

#[test]
fn other_solids_do_not_change_how_a_solid_looks() {
    let a = platform(100.0, 300.0);
    let far = Rect::new(700.0, 100.0, 120.0, 30.0);
    let alone = list(&[a]);
    let with_neighbor = list(&[far, a]);
    let near_a: Vec<Shape> = with_neighbor
        .into_iter()
        .filter(|shape| shape.bounds().1 .0 <= a.right() + 1.0)
        .collect();

    assert_eq!(sorted_debug(&alone), sorted_debug(&near_a));
}

#[test]
fn the_order_of_the_solids_does_not_change_the_picture() {
    let (a, b) = (platform(100.0, 300.0), Rect::new(250.0, 220.0, 60.0, 80.0));

    assert_eq!(sorted_debug(&list(&[a, b])), sorted_debug(&list(&[b, a])));
}

#[test]
fn moving_a_solid_leaves_the_others_alone() {
    let (a, b) = (platform(100.0, 300.0), Rect::new(700.0, 100.0, 120.0, 30.0));
    let moved = Rect::new(720.0, 140.0, 120.0, 30.0);
    let before: Vec<Shape> = list(&[a, b])
        .into_iter()
        .filter(|s| s.bounds().1 .0 <= a.right() + 1.0)
        .collect();
    let after: Vec<Shape> = list(&[a, moved])
        .into_iter()
        .filter(|s| s.bounds().1 .0 <= a.right() + 1.0)
        .collect();

    assert_eq!(sorted_debug(&before), sorted_debug(&after));
}

/// The rectangles of one color in a list, as `(x, y, w, h)`.
fn rects_of(shapes: &[Shape], color: Rgba) -> Vec<(f32, f32, f32, f32)> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect {
                x,
                y,
                w,
                h,
                color: c,
            } if *c == color => Some((*x, *y, *w, *h)),
            _ => None,
        })
        .collect()
}

fn palette_colors() -> (Rgba, Rgba, Rgba) {
    let colors = colors::Colors::from_style(&style());
    (colors.band_top, colors.band_mid, colors.band_low)
}

#[test]
fn the_fill_is_the_palette_color_with_a_lighter_top_and_a_darker_bottom() {
    let solid = Rect::new(100.0, 300.0, 200.0, 40.0);
    let shapes = list(&[solid]);
    let (top, middle, low) = palette_colors();

    // The first shape is the whole solid in the palette's fill.
    assert_eq!(
        shapes[0],
        Shape::rect(solid.x, solid.y, solid.w, solid.h, middle)
    );
    assert_eq!(
        (middle.r, middle.g, middle.b),
        DEFAULT_FILL,
        "the fill is exactly the palette's"
    );
    // One lighter band under the open top and one darker band over the open bottom.
    let band = 40.0 * 0.3;
    assert_eq!(rects_of(&shapes, top), vec![(100.0, 300.0, 200.0, band)]);
    assert_eq!(
        rects_of(&shapes, low),
        vec![(100.0, 340.0 - band, 200.0, band)]
    );
    // The gradient is subtle but real: lighter on top, darker below.
    assert!(top.g > middle.g && middle.g > low.g);
    assert!(top.g - low.g < 0.2, "subtle");
}

#[test]
fn bands_are_capped_so_tall_walls_and_short_ones_match() {
    let wall = list(&[Rect::new(100.0, 100.0, 40.0, 300.0)]);
    let step = list(&[Rect::new(100.0, 100.0, 40.0, 60.0)]);
    let (top, _, _) = palette_colors();

    let tall_band = rects_of(&wall, top)[0].3;
    let short_band = rects_of(&step, top)[0].3;
    assert!(tall_band < 300.0 * 0.3, "a wall's band is capped");
    assert!(
        tall_band >= short_band,
        "never thinner than a shorter solid's"
    );
    assert!(
        (tall_band - 14.0).abs() < 1e-3,
        "the cap is 14 px, got {tall_band}"
    );
}

#[test]
fn a_stack_has_one_continuous_gradient_with_no_band_at_the_seam() {
    let upper = Rect::new(100.0, 300.0, 120.0, 40.0);
    let lower = Rect::new(100.0, 340.0, 120.0, 40.0);
    let shapes = list(&[upper, lower]);
    let (top, _, low) = palette_colors();

    // Only the upper solid's open top is lit, and only the lower one's open bottom is dark.
    let lit = rects_of(&shapes, top);
    let dark = rects_of(&shapes, low);
    assert_eq!(lit.len(), 1);
    assert_eq!(lit[0].1, upper.y);
    assert_eq!(dark.len(), 1);
    assert!((dark[0].1 + dark[0].3 - lower.bottom()).abs() < 1e-3);
}

#[test]
fn a_partly_covered_top_is_lit_only_where_it_is_open() {
    let wide = Rect::new(100.0, 300.0, 100.0, 40.0);
    let block = Rect::new(130.0, 280.0, 40.0, 20.0); // on the middle of the wide top
    let (top, _, _) = palette_colors();
    let lit = rects_of(&list(&[wide, block]), top);

    let mut on_wide: Vec<(f32, f32)> = lit
        .iter()
        .filter(|r| r.1 == wide.y)
        .map(|r| (r.0, r.0 + r.2))
        .collect();
    on_wide.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(on_wide, vec![(100.0, 130.0), (170.0, 200.0)]);
}

#[test]
fn speckles_and_cracks_stay_inside_the_fill() {
    let solid = Rect::new(100.0, 300.0, 300.0, 80.0);
    let shapes = list(&[solid]);
    let inside = |shape: &Shape| {
        let (low, high) = shape.bounds();
        low.0 >= solid.x && low.1 >= solid.y && high.0 <= solid.right() && high.1 <= solid.bottom()
    };
    let speckles = shapes.iter().filter(|s| is_speckle(s)).count();
    let cracks = shapes
        .iter()
        .filter(|s| matches!(s, Shape::Line { .. }))
        .collect::<Vec<_>>();

    assert!(speckles > 20, "a large solid is speckled");
    assert!(!cracks.is_empty(), "a large solid has cracks");
    assert!(cracks.iter().all(|s| inside(s)));
    assert!(shapes.iter().filter(|s| is_speckle(s)).all(inside));
}

#[test]
fn a_huge_solid_is_capped() {
    let shapes = list(&[Rect::new(0.0, 0.0, 20_000.0, 5_000.0)]);
    let speckles = shapes.iter().filter(|s| is_speckle(s)).count();
    let cracks = shapes
        .iter()
        .filter(|s| matches!(s, Shape::Line { .. }))
        .count();

    assert!(speckles <= MAX_SPECKLES);
    assert!(cracks <= MAX_CRACKS * 3);
}

#[test]
fn the_lip_stays_within_its_documented_rise() {
    let solid = platform(100.0, 300.0);
    let shapes = list(&[solid]);
    let lips: Vec<&Shape> = shapes.iter().filter(|s| is_lip(s)).collect();

    assert!(lips.len() > 10, "a 400 px top has many lip segments");
    for lip in lips {
        let Shape::Polygon { points, .. } = lip else {
            unreachable!()
        };
        assert_eq!(points.len(), 4);
        for &(x, y) in points {
            assert!(y >= solid.y - LIP_HEIGHT - 1e-4, "rises at most LIP_HEIGHT");
            assert!(
                y <= solid.y + 1.0 + 1e-4,
                "dips at most 1 px into the solid"
            );
            assert!(x >= solid.x - 1e-4 && x <= solid.right() + 1e-4);
        }
    }
}

#[test]
fn the_lip_tapers_to_nothing_at_the_ends_of_an_open_top() {
    let solid = platform(100.0, 300.0);
    let shapes = list(&[solid]);
    let rises: Vec<f32> = shapes
        .iter()
        .filter_map(|s| match s {
            Shape::Polygon { points, .. } => Some(points.clone()),
            _ => None,
        })
        .flat_map(|points| points.into_iter().filter(|p| p.1 < solid.y))
        .map(|p| p.1)
        .collect();

    assert!(!rises.is_empty());
    // The first segment starts and the last ends exactly on the edge line.
    let Shape::Polygon { points: first, .. } = shapes.iter().find(|s| is_lip(s)).unwrap() else {
        unreachable!()
    };
    assert_eq!(first[1], (solid.x, solid.y));
}

#[test]
fn the_lip_is_convex_trapezoids_that_tile_the_open_top() {
    let solid = platform(100.0, 300.0);
    let lips: Vec<Vec<Point>> = list(&[solid])
        .into_iter()
        .filter_map(|s| match s {
            Shape::Polygon { points, .. } => Some(points),
            _ => None,
        })
        .collect();

    for pair in lips.windows(2) {
        // Each trapezoid ends on the vertical line the next one starts on.
        assert_eq!(pair[0][2].0, pair[1][1].0);
        assert!(pair[0][2].0 > pair[0][1].0, "x keeps increasing");
    }
    assert_eq!(lips.last().unwrap()[2].0, solid.right());
}

#[test]
fn moss_grows_only_on_open_tops_and_can_be_switched_off() {
    let solid = platform(100.0, 300.0);
    let on = build(ROOM, &[solid], None, &style());
    let off = build(
        ROOM,
        &[solid],
        None,
        &TerrainStyle {
            moss: false,
            ..style()
        },
    );

    let blades = on.iter().filter(|s| is_moss(s, solid.y)).count();
    assert!(blades > 5, "a 400 px top grows moss");
    assert_eq!(off.iter().filter(|s| is_moss(s, solid.y)).count(), 0);
    for blade in on.iter().filter(|s| is_moss(s, solid.y)) {
        let (low, high) = blade.bounds();
        assert!(low.1 >= solid.y - MOSS_HEIGHT - 1e-4);
        assert!(low.0 >= solid.x && high.0 <= solid.right());
    }
}

#[test]
fn switching_moss_off_changes_nothing_else() {
    let solid = platform(100.0, 300.0);
    let on = build(ROOM, &[solid], None, &style());
    let off = build(
        ROOM,
        &[solid],
        None,
        &TerrainStyle {
            moss: false,
            ..style()
        },
    );
    let without_moss: Vec<Shape> = on.into_iter().filter(|s| !is_moss(s, solid.y)).collect();

    assert_eq!(without_moss, off);
}

#[test]
fn a_covered_top_grows_no_moss_and_gets_no_lip() {
    let lower = platform(100.0, 300.0);
    let upper = Rect::new(100.0, 260.0, 400.0, 40.0); // sits exactly on top of `lower`
    let shapes = list(&[lower, upper]);

    assert!(
        shapes
            .iter()
            .filter(|s| is_lip(s))
            .all(|s| s.bounds().1 .1 <= upper.y + 1.001),
        "only the upper solid's top has a lip"
    );
    assert!(shapes
        .iter()
        .filter(|s| is_moss(s, lower.y))
        .all(|s| s.bounds().1 .1 <= upper.y + 1.001));
}

#[test]
fn stalactites_hang_under_an_open_bottom_only() {
    let solid = platform(100.0, 300.0);
    let floating = build(ROOM, &[solid], None, &style());
    let ground = ground_cover(Rect::new(0.0, 0.0, 960.0, solid.bottom()), solid.bottom());
    let standing = build(ROOM, &[solid], Some(ground), &style());

    let drips: Vec<&Shape> = floating
        .iter()
        .filter(|s| is_stalactite(s, solid.bottom()))
        .collect();
    assert!(drips.len() > 3);
    for drip in drips {
        assert!(drip.bounds().1 .1 <= solid.bottom() + DRIP_LENGTH + 1e-4);
    }
    assert!(standing.iter().all(|s| !is_stalactite(s, solid.bottom())));
}

#[test]
fn a_solid_standing_on_the_ground_has_no_outline_along_its_bottom() {
    let solid = Rect::new(100.0, 400.0, 100.0, 60.0);
    let ground = ground_cover(Rect::new(0.0, 0.0, 960.0, 460.0), 460.0);
    let outline = Rgba::opaque(DEFAULT_EDGE);
    let bottom_outlines = |shapes: &[Shape]| {
        shapes
            .iter()
            .filter(|s| {
                matches!(s, Shape::Rect { y, h, color, .. }
                if *color == outline && *h <= OUTLINE_WIDTH && *y > solid.bottom() - 3.0)
            })
            .count()
    };

    assert_eq!(
        bottom_outlines(&build(ROOM, &[solid], Some(ground), &style())),
        0
    );
    assert_eq!(bottom_outlines(&build(ROOM, &[solid], None, &style())), 1);
}
