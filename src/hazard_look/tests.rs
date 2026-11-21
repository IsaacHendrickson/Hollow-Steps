use super::*;

const RED: Rgb = (0.8, 0.1, 0.1);
const BLUE: Rgb = (0.1, 0.1, 0.8);

fn rect(x: f32, y: f32, w: f32, h: f32) -> Rect {
    Rect::new(x, y, w, h)
}

/// The spikes of `rect` in the base color `RED`: the triangle of each whole spike (the lit
/// half is another color).
fn spike_triangles(rect: Rect, facing: HazardFacing) -> Vec<[Point; 3]> {
    shapes(rect, HazardStyle::Spikes, facing, RED, 0.0)
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Triangle { points, color } if color == Rgba::opaque(RED) => Some(points),
            _ => None,
        })
        .collect()
}

/// The point of each spike (the corner that is not on the base edge) and its base corners.
fn tip_and_base(triangle: [Point; 3]) -> (Point, [Point; 2]) {
    // Spikes are drawn base corner, tip, base corner.
    let [a, b, c] = triangle;
    (b, [a, c])
}

#[test]
fn a_floor_strip_is_a_row_of_whole_spikes_pointing_up() {
    let spikes = spike_triangles(rect(400.0, 444.0, 64.0, 16.0), HazardFacing::Up);
    assert_eq!(spikes.len(), 4, "64 px wide: four 16 px spikes");
    for triangle in &spikes {
        let (tip, [left, right]) = tip_and_base(*triangle);
        assert_eq!(
            (left.1, right.1),
            (460.0, 460.0),
            "bases on the bottom edge"
        );
        assert_eq!(tip.1, 444.0, "tips on the top edge");
        assert_eq!(tip.0, (left.0 + right.0) / 2.0);
    }
    assert_eq!(spikes[0][0].0, 400.0);
    assert_eq!(spikes[3][2].0, 464.0);
    assert_eq!(
        spikes[1][0].0, spikes[0][2].0,
        "each starts where the last ended"
    );
}

#[test]
fn each_facing_puts_the_base_on_the_opposite_edge_and_the_tips_on_its_own() {
    // 16 wide, 64 tall for the sideways ones; 64 wide, 16 tall for up and down.
    let flat = rect(100.0, 200.0, 64.0, 16.0);
    let tall = rect(100.0, 200.0, 16.0, 64.0);
    for (rect, facing) in [
        (flat, HazardFacing::Up),
        (flat, HazardFacing::Down),
        (tall, HazardFacing::Left),
        (tall, HazardFacing::Right),
    ] {
        let spikes = spike_triangles(rect, facing);
        assert_eq!(spikes.len(), 4, "{facing:?}");
        for triangle in spikes {
            let (tip, base) = tip_and_base(triangle);
            let (tip_edge, base_edge, axis_of_edge): (f32, f32, fn(Point) -> f32) = match facing {
                HazardFacing::Up => (rect.y, rect.bottom(), |p| p.1),
                HazardFacing::Down => (rect.bottom(), rect.y, |p| p.1),
                HazardFacing::Left => (rect.x, rect.right(), |p| p.0),
                HazardFacing::Right => (rect.right(), rect.x, |p| p.0),
            };
            assert_eq!(axis_of_edge(tip), tip_edge, "{facing:?} tip");
            assert!(
                base.iter().all(|corner| axis_of_edge(*corner) == base_edge),
                "{facing:?} base"
            );
        }
    }
}

#[test]
fn a_tall_thin_hazard_is_one_needle_facing_up_and_a_row_of_spikes_facing_a_wall() {
    // The reported case: 16 wide, 400 tall, against a wall.
    let wall = rect(800.0, 60.0, 16.0, 400.0);
    let needle = spike_triangles(wall, HazardFacing::Up);
    assert_eq!(needle.len(), 1, "facing up it is a single spike");
    assert_eq!(needle[0][1].1, wall.y, "400 px tall");

    for facing in [HazardFacing::Left, HazardFacing::Right] {
        let row = spike_triangles(wall, facing);
        assert_eq!(row.len(), 25, "{facing:?}: 400 px of wall, 16 px spikes");
        let top = row.iter().map(|t| t[1].1).fold(f32::MAX, f32::min);
        let bottom = row.iter().map(|t| t[1].1).fold(f32::MIN, f32::max);
        assert!(
            top > wall.y && bottom < wall.bottom(),
            "spread along the whole wall"
        );
        // Every spike is as deep as the hazard is wide.
        for triangle in &row {
            let (tip, [first, second]) = tip_and_base(*triangle);
            assert_eq!((tip.0 - first.0).abs(), 16.0);
            assert_eq!(first.0, second.0);
        }
    }
}

#[test]
fn a_narrow_edge_still_gets_one_spike() {
    assert_eq!(
        spike_triangles(rect(0.0, 0.0, 8.0, 8.0), HazardFacing::Up).len(),
        1
    );
}

#[test]
fn every_style_and_facing_stays_inside_the_rectangle_at_every_size_and_time() {
    let sizes = [
        (8.0, 8.0),
        (16.0, 400.0),
        (400.0, 16.0),
        (123.0, 77.0),
        (64.0, 16.0),
        (2000.0, 40.0),
    ];
    for style in HazardStyle::ALL {
        for facing in HazardFacing::ALL {
            for (w, h) in sizes {
                for time in [0.0, 0.7, 13.37] {
                    let area = rect(100.0, 50.0, w, h);
                    let list = shapes(area, style, facing, RED, time);
                    assert!(!list.is_empty(), "{style:?} {facing:?} {w}x{h}");
                    for shape in &list {
                        let (low, high) = shape.bounds();
                        assert!(
                            low.0 >= area.x - 1e-3
                                && low.1 >= area.y - 1e-3
                                && high.0 <= area.right() + 1e-3
                                && high.1 <= area.bottom() + 1e-3,
                            "{style:?} {facing:?} {w}x{h} t={time}: {shape:?} leaves {area:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn the_color_recolors_every_shape_and_each_style_has_its_own_default() {
    for style in HazardStyle::ALL {
        let colors = |color: Rgb| -> Vec<Rgba> {
            shapes(
                rect(0.0, 0.0, 96.0, 32.0),
                style,
                HazardFacing::Up,
                color,
                0.0,
            )
            .into_iter()
            .map(|shape| match shape {
                Shape::Rect { color, .. }
                | Shape::Line { color, .. }
                | Shape::Triangle { color, .. }
                | Shape::Polygon { color, .. } => color,
            })
            .collect()
        };
        let (red, blue) = (colors(RED), colors(BLUE));
        assert_eq!(red.len(), blue.len(), "{style:?}: same shapes, other color");
        assert!(
            red.iter().all(|color| !blue.contains(color)),
            "{style:?}: no tone of one color is a tone of the other"
        );
    }
    let defaults: Vec<Rgb> = HazardStyle::ALL.iter().map(|s| s.default_color()).collect();
    assert!(defaults[0] != defaults[1] && defaults[1] != defaults[2] && defaults[0] != defaults[2]);
}

#[test]
fn the_styles_look_different_from_each_other() {
    let area = rect(0.0, 0.0, 96.0, 32.0);
    let [spikes, lava, thorns] =
        HazardStyle::ALL.map(|style| shapes(area, style, HazardFacing::Up, RED, 0.0));
    assert_ne!(spikes, lava);
    assert_ne!(lava, thorns);
    assert_ne!(spikes, thorns);
    // Lava is a body, not triangles; spikes and thorns are made of triangles.
    let triangles = |list: &[Shape]| {
        list.iter()
            .filter(|s| matches!(s, Shape::Triangle { .. }))
            .count()
    };
    assert_eq!(triangles(&lava), 0);
    assert!(triangles(&spikes) > 0 && triangles(&thorns) > 0);
}

#[test]
fn drawing_is_deterministic_and_only_lava_moves_with_time() {
    let area = rect(40.0, 40.0, 120.0, 32.0);
    for style in HazardStyle::ALL {
        let now = shapes(area, style, HazardFacing::Left, RED, 2.5);
        assert_eq!(now, shapes(area, style, HazardFacing::Left, RED, 2.5));
        let later = shapes(area, style, HazardFacing::Left, RED, 3.5);
        if style == HazardStyle::Lava {
            assert_ne!(now, later, "lava waves move");
        } else {
            assert_eq!(now, later, "{style:?} stands still");
        }
    }
}

#[test]
fn thorns_vary_in_height_but_keep_their_look_when_the_strip_grows() {
    let tips = |width: f32| -> Vec<f32> {
        shapes(
            rect(0.0, 0.0, width, 30.0),
            HazardStyle::Thorns,
            HazardFacing::Up,
            RED,
            0.0,
        )
        .into_iter()
        .filter_map(|shape| match shape {
            Shape::Triangle { points, .. } => Some(points[2].1),
            _ => None,
        })
        .collect()
    };
    let short = tips(100.0);
    assert!(
        short.windows(2).any(|pair| pair[0] != pair[1]),
        "uneven heights"
    );
    let long = tips(200.0);
    // The thorns of the short strip are the first ones of the long one (same spacing).
    assert_eq!(&long[..short.len()], &short[..]);
}

#[test]
fn an_empty_or_unreadable_rectangle_draws_nothing_and_bad_numbers_are_tamed() {
    for bad in [
        rect(0.0, 0.0, 0.0, 16.0),
        rect(0.0, 0.0, 16.0, -1.0),
        rect(f32::NAN, 0.0, 16.0, 16.0),
        rect(0.0, 0.0, f32::INFINITY, 16.0),
    ] {
        for style in HazardStyle::ALL {
            assert!(shapes(bad, style, HazardFacing::Up, RED, 0.0).is_empty());
        }
    }
    // A color outside 0..1 or not a number, and a time that is not finite, still draw.
    let odd = shapes(
        rect(0.0, 0.0, 32.0, 16.0),
        HazardStyle::Lava,
        HazardFacing::Up,
        (f32::NAN, 7.0, -2.0),
        f32::NAN,
    );
    assert!(!odd.is_empty());
    for shape in odd {
        let color = match shape {
            Shape::Rect { color, .. }
            | Shape::Line { color, .. }
            | Shape::Triangle { color, .. }
            | Shape::Polygon { color, .. } => color,
        };
        assert!([color.r, color.g, color.b]
            .iter()
            .all(|c| (0.0..=1.0).contains(c)));
    }
}

#[test]
fn the_draw_list_stays_small_however_large_the_hazard() {
    for style in HazardStyle::ALL {
        let list = shapes(
            rect(0.0, 0.0, 2000.0, 2000.0),
            style,
            HazardFacing::Up,
            RED,
            0.0,
        );
        assert!(list.len() <= 1000, "{style:?}: {} shapes", list.len());
    }
}

#[test]
fn style_and_facing_names_and_the_default_look() {
    assert_eq!(HazardStyle::default(), HazardStyle::Spikes);
    assert_eq!(HazardFacing::default(), HazardFacing::Up);
    let labels: Vec<_> = HazardStyle::ALL.iter().map(|s| s.label()).collect();
    assert_eq!(labels, ["Spikes", "Lava", "Thorns"]);
    let labels: Vec<_> = HazardFacing::ALL.iter().map(|f| f.label()).collect();
    assert_eq!(labels, ["Up", "Down", "Left", "Right"]);
}
