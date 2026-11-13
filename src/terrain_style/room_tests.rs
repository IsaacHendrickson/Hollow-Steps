//! Whole rooms: the boundary and the solids drawn together, the density settings and the
//! room-level cache.

use macroquad::math::Rect;

use super::boundary_tests::{bounds, one_exit_per_edge, overlap, T};
use super::colors::Colors;
use super::tests::{platform, ROOM};
use super::*;

/// The room of `boundary_tests`: x 100 to 900, y 50 to 450, with the floor at the bottom.
fn inputs<'a>(solids: &'a [Rect], exits: &'a [Exit]) -> RoomInputs<'a> {
    RoomInputs {
        room: ROOM,
        bounds: bounds(),
        floor_y: 450.0,
        exits,
        solids,
    }
}

/// The default look with slabs `T` thick.
fn look() -> Look {
    Look {
        boundary_thickness: T,
        ..Look::default()
    }
}

fn room(solids: &[Rect]) -> Vec<Shape> {
    build_room(&inputs(solids, &[]), &look())
}

fn colors() -> Colors {
    Colors::from_style(&TerrainStyle::default())
}

/// The rectangles of one color, in draw order.
fn rects_of(shapes: &[Shape], color: Rgba) -> Vec<Rect> {
    shapes
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect {
                x,
                y,
                w,
                h,
                color: c,
            } if *c == color => Some(Rect::new(*x, *y, *w, *h)),
            _ => None,
        })
        .collect()
}

fn sorted(items: &[impl std::fmt::Debug]) -> Vec<String> {
    let mut text: Vec<String> = items.iter().map(|item| format!("{item:?}")).collect();
    text.sort();
    text
}

fn box_of(shape: &Shape) -> Rect {
    let (low, high) = shape.bounds();
    Rect::new(low.0, low.1, high.0 - low.0, high.1 - low.1)
}

/// A blade of moss: a triangle standing on the line `y` (its base one pixel inside).
fn is_moss_on(shape: &Shape, y: f32) -> bool {
    let (low, high) = shape.bounds();
    matches!(shape, Shape::Triangle { .. })
        && (high.1 - (y + 1.0)).abs() < 0.001
        && low.1 < y - 0.001
}

/// A stalactite: a triangle hanging from the line `y` (its base one pixel inside).
fn is_drip_from(shape: &Shape, y: f32) -> bool {
    let (low, high) = shape.bounds();
    matches!(shape, Shape::Triangle { .. })
        && (low.1 - (y - 1.0)).abs() < 0.001
        && high.1 > y + 0.001
}

/// How many speckles and crack segments (two or three per crack) a list holds.
fn counts(shapes: &[Shape]) -> (usize, usize) {
    let c = colors();
    let specks = [c.speck_light, c.speck_dark, c.speck_accent];
    let speckles = shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Rect { color, .. } if specks.contains(color)))
        .count();
    let cracks = shapes
        .iter()
        .filter(|shape| matches!(shape, Shape::Line { color, .. } if *color == c.crack))
        .count();
    (speckles, cracks)
}

#[test]
fn a_bare_frame_outlines_only_its_inner_and_outer_faces() {
    let (w, lit) = (OUTLINE_WIDTH, HIGHLIGHT_HEIGHT);
    let outlines = rects_of(&room(&[]), colors().outline);

    let expected = [
        // Ceiling: its outer top (under the lit strip), its underside between the walls, and
        // its two ends.
        Rect::new(68.0, 18.0 + lit, 864.0, w),
        Rect::new(100.0, 50.0 - w, 800.0, w),
        Rect::new(68.0, 18.0, w, 32.0),
        Rect::new(932.0 - w, 18.0, w, 32.0),
        // Walls: the outer and the inner face of each, but not where they meet the ceiling
        // and the floor slab.
        Rect::new(68.0, 50.0, w, 400.0),
        Rect::new(100.0 - w, 50.0, w, 400.0),
        Rect::new(900.0, 50.0, w, 400.0),
        Rect::new(932.0 - w, 50.0, w, 400.0),
        // Floor slab: its top between the walls, its underside and its two ends.
        Rect::new(100.0, 450.0 + lit, 800.0, w),
        Rect::new(68.0, 482.0 - w, 864.0, w),
        Rect::new(68.0, 450.0, w, 32.0),
        Rect::new(932.0 - w, 450.0, w, 32.0),
    ];
    assert_eq!(sorted(&outlines), sorted(&expected));
}

#[test]
fn the_floor_is_lit_along_the_open_part_of_its_top() {
    let lit = rects_of(&room(&[]), colors().highlight);

    // The ceiling's outer top, and the floor between the two walls that stand on it.
    assert_eq!(
        sorted(&lit),
        sorted(&[
            Rect::new(68.0, 18.0, 864.0, HIGHLIGHT_HEIGHT),
            Rect::new(100.0, 450.0, 800.0, HIGHLIGHT_HEIGHT),
        ])
    );
}

#[test]
fn the_ceiling_drips_into_the_room_and_the_floor_grows_moss() {
    let shapes = room(&[]);
    let drips = shapes.iter().filter(|s| is_drip_from(s, 50.0)).count();
    let moss = shapes.iter().filter(|s| is_moss_on(s, 450.0)).count();

    assert!(drips >= 5, "{drips} stalactites under the ceiling");
    assert!(moss >= 5, "{moss} moss blades on the floor");
    // The moss can be switched off for the whole room, the boundary included.
    let bare = Look {
        palette: TerrainStyle {
            moss: false,
            ..TerrainStyle::default()
        },
        ..look()
    };
    let mossless = build_room(&inputs(&[], &[]), &bare);
    assert!(!mossless.iter().any(|s| is_moss_on(s, 450.0)));
    assert!(mossless.iter().filter(|s| is_drip_from(s, 50.0)).count() >= 5);
}

#[test]
fn decoration_reaches_into_the_room_by_no_more_than_the_documented_margins() {
    // The playable area less the strip under the ceiling and the strip over the floor that
    // stalactites and moss may use.
    let clear = Rect::new(
        100.0,
        50.0 + BOTTOM_MARGIN,
        800.0,
        400.0 - BOTTOM_MARGIN - TOP_MARGIN,
    );
    // Everything the room may touch: the frame plus the margins above and below it.
    let reach = Rect::new(
        68.0,
        18.0 - TOP_MARGIN,
        864.0,
        464.0 + TOP_MARGIN + BOTTOM_MARGIN,
    );
    for shape in room(&[]) {
        assert_eq!(
            overlap(&box_of(&shape), &clear),
            0.0,
            "{shape:?} reaches too far into the room"
        );
        let (low, high) = shape.bounds();
        assert!(
            low.0 >= reach.x - 0.001
                && low.1 >= reach.y - 0.001
                && high.0 <= reach.right() + 0.001
                && high.1 <= reach.bottom() + 0.001,
            "{shape:?} strays outside {reach:?}"
        );
    }
}

#[test]
fn no_shape_reaches_into_an_exit_zone() {
    let exits = one_exit_per_edge();
    let shapes = build_room(&inputs(&[], &exits), &look());

    for exit in &exits {
        let zone = exit.zone(bounds());
        for shape in &shapes {
            assert_eq!(
                overlap(&box_of(shape), &zone),
                0.0,
                "{shape:?} covers the {:?} exit",
                exit.edge
            );
        }
    }
    // The doorways did not take the rest of the boundary with them.
    assert!(shapes.len() > room(&[]).len() / 2);
}

#[test]
fn an_empty_room_still_draws_its_boundary() {
    assert!(!room(&[]).is_empty());

    let off = Look {
        draw_boundary: false,
        ..look()
    };
    assert!(
        build_room(&inputs(&[], &[]), &off).is_empty(),
        "no solids and no boundary draw nothing"
    );
}

#[test]
fn a_platform_in_mid_air_looks_the_same_with_or_without_the_boundary() {
    // x 250 to 650, y 200 to 240: well clear of the slabs.
    let solids = [platform(250.0, 200.0)];
    let off = Look {
        draw_boundary: false,
        ..look()
    };
    let around_platform = |shapes: Vec<Shape>| -> Vec<Shape> {
        shapes
            .into_iter()
            .filter(|shape| {
                let (low, high) = shape.bounds();
                low.0 > 249.0 && high.0 < 651.0 && low.1 > 150.0 && high.1 < 300.0
            })
            .collect()
    };

    let with = around_platform(room(&solids));
    let without = around_platform(build_room(&inputs(&solids, &[]), &off));

    assert!(without.len() > 20, "the platform has a look to compare");
    assert_eq!(sorted(&with), sorted(&without));
}

#[test]
fn a_platform_does_not_change_how_the_boundary_looks() {
    let bare = sorted(&room(&[]));
    let with_platform = sorted(&room(&[platform(250.0, 200.0)]));

    assert!(
        bare.iter().all(|shape| with_platform.contains(shape)),
        "every shape of the bare boundary is still drawn"
    );
}

#[test]
fn a_platform_standing_on_the_floor_hides_the_seam_and_leaves_the_rest_lit() {
    // y 410 to 450: its foot is on the floor line.
    let solids = [Rect::new(300.0, 410.0, 100.0, 40.0)];
    let shapes = room(&solids);
    let lit = rects_of(&shapes, colors().highlight);
    let outlines = rects_of(&shapes, colors().outline);

    assert!(lit.contains(&Rect::new(100.0, 450.0, 200.0, HIGHLIGHT_HEIGHT)));
    assert!(lit.contains(&Rect::new(400.0, 450.0, 500.0, HIGHLIGHT_HEIGHT)));
    assert!(lit.contains(&Rect::new(300.0, 410.0, 100.0, HIGHLIGHT_HEIGHT)));
    assert_eq!(lit.len(), 4, "plus the ceiling's outer top: {lit:?}");
    // Neither the platform's foot nor the floor under it is outlined (between its sides).
    let w = OUTLINE_WIDTH;
    let seam = Rect::new(300.0 + w, 446.0, 100.0 - 2.0 * w, 8.0);
    assert!(
        outlines.iter().all(|line| overlap(line, &seam) == 0.0),
        "{outlines:?}"
    );
    // Its sides are.
    assert!(outlines.contains(&Rect::new(300.0, 410.0, w, 40.0)));
    assert!(outlines.contains(&Rect::new(400.0 - w, 410.0, w, 40.0)));
}

#[test]
fn a_platform_against_a_wall_hides_the_seam_on_both_sides() {
    // Touches the left wall's inner face over y 300 to 320.
    let solids = [Rect::new(100.0, 300.0, 200.0, 20.0)];
    let outlines = rects_of(&room(&solids), colors().outline);
    let w = OUTLINE_WIDTH;

    // The wall's inner face is outlined above the platform and below it, not along it.
    assert!(outlines.contains(&Rect::new(100.0 - w, 50.0, w, 250.0)));
    assert!(outlines.contains(&Rect::new(100.0 - w, 320.0, w, 130.0)));
    assert!(!outlines
        .iter()
        .any(|line| (line.x - (100.0 - w)).abs() < 0.001
            && overlap(line, &Rect::new(98.0, 300.0, 2.0, 20.0)) > 0.0));
    // The platform has no outline on the side that meets the wall.
    assert!(!outlines.contains(&Rect::new(100.0, 300.0, w, 20.0)));
    assert!(outlines.contains(&Rect::new(300.0 - w, 300.0, w, 20.0)));
}

#[test]
fn the_same_room_gives_the_same_list_and_other_seeds_give_other_lists() {
    let solids = [platform(250.0, 200.0)];
    let base = room(&solids);

    assert_eq!(base, room(&solids));
    let other_room = RoomInputs {
        room: RoomId::Second,
        ..inputs(&solids, &[])
    };
    assert_ne!(base, build_room(&other_room, &look()));
    let reshuffled = Look {
        palette: TerrainStyle {
            seed_offset: 1,
            ..TerrainStyle::default()
        },
        ..look()
    };
    assert_ne!(base, build_room(&inputs(&solids, &[]), &reshuffled));
}

#[test]
fn the_boundary_thickness_comes_from_the_look() {
    for thickness in [16.0, 48.0, 64.0] {
        let wide = Look {
            boundary_thickness: thickness,
            ..look()
        };
        let shapes = build_room(&inputs(&[], &[]), &wide);

        let left = shapes
            .iter()
            .map(|s| s.bounds().0 .0)
            .fold(f32::MAX, f32::min);
        let right = shapes
            .iter()
            .map(|s| s.bounds().1 .0)
            .fold(f32::MIN, f32::max);
        assert_eq!(left, 100.0 - thickness);
        assert_eq!(right, 900.0 + thickness);
    }
}

#[test]
fn a_boundary_without_a_usable_thickness_falls_back_to_the_ground_cover() {
    // Standing on the floor: only buried if something counts as ground.
    let solids = [Rect::new(100.0, 410.0, 80.0, 40.0)];
    let off = Look {
        draw_boundary: false,
        ..look()
    };
    let plain = build_room(&inputs(&solids, &[]), &off);

    for thickness in [0.0, -3.0, f32::NAN] {
        let broken = Look {
            boundary_thickness: thickness,
            ..look()
        };
        assert_eq!(build_room(&inputs(&solids, &[]), &broken), plain);
    }
}

#[test]
fn without_the_boundary_a_room_is_drawn_as_the_platform_look_always_was() {
    let solids = [platform(250.0, 200.0), Rect::new(100.0, 410.0, 80.0, 40.0)];
    let off = Look {
        draw_boundary: false,
        ..look()
    };

    assert_eq!(
        build_room(&inputs(&solids, &[]), &off),
        build(
            ROOM,
            &solids,
            Some(ground_cover(bounds(), 450.0)),
            &TerrainStyle::default()
        )
    );
}

#[test]
fn density_scales_speckles_and_cracks_and_the_caps_hold() {
    let solids = [Rect::new(250.0, 200.0, 300.0, 60.0)];
    let at = |speckles: f32, cracks: f32| {
        let look = Look {
            speckle_density: speckles,
            crack_density: cracks,
            draw_boundary: false,
            ..look()
        };
        counts(&build_room(&inputs(&solids, &[]), &look))
    };

    let (speckles, cracks) = at(1.0, 1.0);
    assert!(
        speckles > 20 && cracks >= 3,
        "{speckles} speckles, {cracks} cracks"
    );
    assert_eq!(at(0.0, 0.0), (0, 0));
    assert_eq!(
        at(0.0, 1.0),
        (0, cracks),
        "the two settings are independent"
    );
    assert_eq!(at(1.0, 0.0), (speckles, 0));
    let (more_speckles, more_cracks) = at(2.0, 2.0);
    assert!(more_speckles > speckles && more_cracks > cracks);
    let (most_speckles, most_segments) = at(1000.0, 1000.0);
    assert_eq!(most_speckles, MAX_SPECKLES);
    assert!(
        (2 * MAX_CRACKS..=3 * MAX_CRACKS).contains(&most_segments),
        "{most_segments} crack segments for at most {MAX_CRACKS} cracks"
    );
    assert_eq!(at(-1.0, f32::NAN), (0, 0), "nonsense densities draw none");
}

#[test]
fn a_denser_look_keeps_the_speckles_it_already_had() {
    let solids = [Rect::new(250.0, 200.0, 300.0, 60.0)];
    let with = |density: f32| {
        let look = Look {
            speckle_density: density,
            draw_boundary: false,
            ..look()
        };
        build_room(&inputs(&solids, &[]), &look)
    };
    let (thin, dense) = (with(1.0), with(2.0));

    // The speckles come from one stream, so doubling the density only appends to it.
    let c = colors();
    let specks = [c.speck_light, c.speck_dark, c.speck_accent];
    let first: Vec<&Shape> = thin
        .iter()
        .filter(|shape| matches!(shape, Shape::Rect { color, .. } if specks.contains(color)))
        .collect();
    let second: Vec<&Shape> = dense
        .iter()
        .filter(|shape| matches!(shape, Shape::Rect { color, .. } if specks.contains(color)))
        .collect();
    assert!(second.len() > first.len());
    assert_eq!(first[..], second[..first.len()]);
}

#[test]
fn the_room_cache_rebuilds_only_when_something_changes() {
    let solids = [platform(250.0, 200.0)];
    let moved = [platform(260.0, 200.0)];
    let exits = one_exit_per_edge();
    let base = inputs(&solids, &exits);
    let variants = [
        (base, look()),
        (
            RoomInputs {
                room: RoomId::Second,
                ..base
            },
            look(),
        ),
        (
            RoomInputs {
                bounds: Rect::new(100.0, 50.0, 800.0, 360.0),
                ..base
            },
            look(),
        ),
        (
            RoomInputs {
                floor_y: 420.0,
                ..base
            },
            look(),
        ),
        (inputs(&solids, &[]), look()),
        (inputs(&moved, &exits), look()),
        (
            base,
            Look {
                boundary_thickness: 40.0,
                ..look()
            },
        ),
        (
            base,
            Look {
                draw_boundary: false,
                ..look()
            },
        ),
        (
            base,
            Look {
                speckle_density: 2.0,
                ..look()
            },
        ),
        (
            base,
            Look {
                palette: TerrainStyle {
                    moss: false,
                    ..TerrainStyle::default()
                },
                ..look()
            },
        ),
    ];
    let mut cache = TerrainLook::default();

    for (count, (inputs, look)) in variants.iter().enumerate() {
        let shapes = cache.room_shapes(inputs, look).shapes.to_vec();
        assert_eq!(cache.builds(), count + 1, "variant {count} rebuilds");
        assert_eq!(shapes, build_room(inputs, look), "variant {count}");
        assert_eq!(cache.room_shapes(inputs, look).shapes, shapes);
        assert_eq!(cache.builds(), count + 1, "asking again reuses the list");
    }
}

#[test]
fn the_platform_cache_and_the_room_cache_do_not_serve_each_other() {
    let solids = [platform(250.0, 200.0)];
    let mut cache = TerrainLook::default();

    let platforms_only = cache
        .shapes(ROOM, &solids, None, &TerrainStyle::default())
        .to_vec();
    let whole_room = cache
        .room_shapes(&inputs(&solids, &[]), &look())
        .shapes
        .to_vec();
    assert_eq!(cache.builds(), 2);
    assert_ne!(platforms_only, whole_room);

    let again = cache.shapes(ROOM, &solids, None, &TerrainStyle::default());
    assert_eq!(again, platforms_only.as_slice());
    assert_eq!(cache.builds(), 3);
}

#[test]
fn the_cache_says_whether_the_boundary_is_in_the_list() {
    let mut cache = TerrainLook::default();
    let mut boundary = |look: Look| cache.room_shapes(&inputs(&[], &[]), &look).boundary;

    assert!(boundary(look()));
    assert!(
        boundary(look()),
        "the answer survives a cache hit, not just a rebuild"
    );
    assert!(!boundary(Look {
        draw_boundary: false,
        ..look()
    }));
    for thickness in [0.0, -1.0, f32::NAN] {
        assert!(
            !boundary(Look {
                boundary_thickness: thickness,
                ..look()
            }),
            "no usable thickness: {thickness}"
        );
    }
    assert!(boundary(look()));
}

#[test]
fn the_default_look_is_the_default_palette_with_a_boundary() {
    let look = Look::default();

    assert_eq!(look.palette, TerrainStyle::default());
    assert!(look.draw_boundary);
    assert_eq!(look.boundary_thickness, DEFAULT_BOUNDARY_THICKNESS);
    assert_eq!((look.speckle_density, look.crack_density), (1.0, 1.0));
}
