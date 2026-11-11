use super::*;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);

fn wide_room() -> Rect {
    Rect::new(0.0, 0.0, 1920.0, 1080.0)
}

fn layer(depth: f32) -> BackgroundLayer {
    BackgroundLayer {
        depth,
        color: (10, 20, 30, 255),
        shapes: vec![Shape::Circle {
            x: 100.0,
            y: 100.0,
            radius: 20.0,
        }],
    }
}

/// Where a point of a layer lands on the screen: its place in the room, moved by the
/// layer's offset, relative to the top-left of the view.
fn screen_x(point_x: f32, view_left: f32, depth: f32) -> f32 {
    let offset = parallax_offset(Vec2::new(view_left, 0.0), wide_room(), VIEW, depth);
    point_x + offset.x - view_left
}

#[test]
fn depth_one_moves_with_the_world_for_every_camera_position() {
    for left in [0.0, 1.0, 333.3, 960.0, 5000.0, -50.0] {
        assert_eq!(
            parallax_offset(Vec2::new(left, 40.0), wide_room(), VIEW, 1.0),
            Vec2::ZERO
        );
    }
}

#[test]
fn depth_zero_stays_fixed_on_the_screen() {
    let at_start = screen_x(700.0, 0.0, 0.0);
    for left in [10.0, 250.0, 700.0, 960.0] {
        assert!(
            (screen_x(700.0, left, 0.0) - at_start).abs() < 1e-3,
            "{left}"
        );
    }
}

#[test]
fn in_between_a_layer_slides_by_its_depth_times_the_cameras_travel() {
    // Moving the camera right by 400 slides a depth-0.25 layer left by 100 on the screen.
    let before = screen_x(700.0, 100.0, 0.25);
    let after = screen_x(700.0, 500.0, 0.25);
    assert!((before - after - 100.0).abs() < 1e-3, "{before} {after}");
    // A nearer layer slides more than a farther one for the same camera move.
    let slide = |depth: f32| screen_x(700.0, 100.0, depth) - screen_x(700.0, 500.0, depth);
    assert!(slide(0.2) < slide(0.5) && slide(0.5) < slide(0.9));
}

#[test]
fn every_layer_sits_at_its_authored_place_when_the_camera_rests_at_the_room_corner() {
    for depth in [0.0, 0.25, 0.5, 1.0] {
        assert_eq!(
            parallax_offset(Vec2::new(0.0, 0.0), wide_room(), VIEW, depth),
            Vec2::ZERO
        );
    }
    // Also in a room that does not start at the origin.
    let room = Rect::new(-400.0, 200.0, 1920.0, 1080.0);
    assert_eq!(
        parallax_offset(Vec2::new(-400.0, 200.0), room, VIEW, 0.3),
        Vec2::ZERO
    );
}

#[test]
fn the_offset_is_clamped_at_the_room_edges() {
    // The camera can travel 960 right and 540 down in this room.
    let furthest = parallax_offset(Vec2::new(960.0, 540.0), wide_room(), VIEW, 0.0);
    assert_eq!(furthest, Vec2::new(960.0, 540.0));
    // Past the right and bottom edges (e.g. screen shake pushed the view out) it stops there.
    assert_eq!(
        parallax_offset(Vec2::new(5000.0, 9000.0), wide_room(), VIEW, 0.0),
        furthest
    );
    // Past the left and top edges nothing moves.
    assert_eq!(
        parallax_offset(Vec2::new(-300.0, -300.0), wide_room(), VIEW, 0.0),
        Vec2::ZERO
    );
    // The clamp scales with depth.
    assert_eq!(
        parallax_offset(Vec2::new(5000.0, 0.0), wide_room(), VIEW, 0.25).x,
        960.0 * 0.75
    );
}

#[test]
fn a_room_no_larger_than_the_view_gives_no_offset_on_that_axis() {
    // The real first room: as wide as the view, shorter than it.
    let small = Rect::new(0.0, 0.0, 960.0, 460.0);
    // The camera centers the room, so the view starts above it.
    assert_eq!(
        parallax_offset(Vec2::new(0.0, -40.0), small, VIEW, 0.0),
        Vec2::ZERO
    );
    // A wide room that is shorter than the view: only x travels.
    let low = Rect::new(0.0, 0.0, 1920.0, 400.0);
    assert_eq!(
        parallax_offset(Vec2::new(500.0, -70.0), low, VIEW, 0.5),
        Vec2::new(250.0, 0.0)
    );
}

#[test]
fn bad_inputs_never_produce_nan_or_a_jump() {
    for left in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let offset = parallax_offset(Vec2::new(left, left), wide_room(), VIEW, 0.5);
        assert!(offset.is_finite(), "{left}");
    }
    assert_eq!(
        parallax_offset(Vec2::new(f32::NAN, 0.0), wide_room(), VIEW, 0.0),
        Vec2::ZERO
    );
    // A non-finite depth counts as 1; one outside 0 to 1 is clamped.
    assert_eq!(
        parallax_offset(Vec2::new(500.0, 0.0), wide_room(), VIEW, f32::NAN),
        Vec2::ZERO
    );
    let at = |depth| parallax_offset(Vec2::new(500.0, 0.0), wide_room(), VIEW, depth);
    assert_eq!(at(-3.0), at(0.0));
    assert_eq!(at(7.0), at(1.0));
    // A degenerate room or view does not divide or overflow.
    assert!(parallax_offset(
        Vec2::new(5.0, 5.0),
        Rect::new(0.0, 0.0, 0.0, 0.0),
        VIEW,
        0.2
    )
    .is_finite());
    assert!(parallax_offset(Vec2::new(5.0, 5.0), wide_room(), Vec2::ZERO, 0.2).is_finite());
}

#[test]
fn a_good_set_of_layers_validates_and_none_is_fine() {
    assert!(validate(&[]).is_ok());
    assert!(validate(&[layer(0.0), layer(0.5), layer(1.0)]).is_ok());
    let shapes = BackgroundLayer {
        depth: 0.4,
        color: (0, 0, 0, 0),
        shapes: vec![
            Shape::Rect {
                x: -50.0,
                y: 10.0,
                w: 100.0,
                h: 5.0,
            },
            Shape::Triangle {
                a: (0.0, 0.0),
                b: (10.0, 0.0),
                c: (5.0, -8.0),
            },
        ],
    };
    assert!(validate(&[shapes]).is_ok());
}

#[test]
fn depth_must_be_between_zero_and_one_and_finite() {
    for depth in [-0.01, 1.01, f32::NAN, f32::INFINITY] {
        let error = validate(&[layer(depth)]).unwrap_err();
        assert!(
            error.contains("background layer 0") && error.contains("depth"),
            "{error}"
        );
    }
}

#[test]
fn a_layer_needs_shapes_but_not_too_many_and_a_room_not_too_many_layers() {
    let mut empty = layer(0.5);
    empty.shapes.clear();
    assert!(validate(&[empty])
        .unwrap_err()
        .contains("at least one shape"));

    let mut crowded = layer(0.5);
    crowded.shapes = vec![crowded.shapes[0]; MAX_SHAPES];
    assert!(validate(&[crowded.clone()]).is_ok());
    crowded.shapes.push(crowded.shapes[0]);
    assert!(validate(&[crowded]).unwrap_err().contains("more than"));

    let layers = vec![layer(0.5); MAX_LAYERS];
    assert!(validate(&layers).is_ok());
    let layers = vec![layer(0.5); MAX_LAYERS + 1];
    assert!(validate(&layers).unwrap_err().contains("more than"));
}

#[test]
fn each_bad_shape_is_rejected_and_named_by_layer_and_index() {
    let bad = [
        Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 10.0,
        },
        Shape::Rect {
            x: 0.0,
            y: 0.0,
            w: 10.0,
            h: -1.0,
        },
        Shape::Rect {
            x: f32::NAN,
            y: 0.0,
            w: 10.0,
            h: 10.0,
        },
        Shape::Rect {
            x: 0.0,
            y: MAX_EXTENT + 1.0,
            w: 10.0,
            h: 10.0,
        },
        Shape::Circle {
            x: 0.0,
            y: 0.0,
            radius: 0.0,
        },
        Shape::Circle {
            x: 0.0,
            y: f32::INFINITY,
            radius: 5.0,
        },
        Shape::Triangle {
            a: (0.0, 0.0),
            b: (f32::NAN, 1.0),
            c: (2.0, 2.0),
        },
        Shape::Triangle {
            a: (0.0, 0.0),
            b: (1.0, 1.0),
            c: (2.0, -MAX_EXTENT - 1.0),
        },
    ];
    for shape in bad {
        let mut broken = layer(0.5);
        broken.shapes.push(shape);
        let error = validate(&[layer(0.1), broken]).unwrap_err();
        assert!(
            error.contains("background layer 1") && error.contains("shape 1"),
            "{shape:?}: {error}"
        );
    }
}

#[test]
fn layers_round_trip_through_ron_in_the_documented_form() {
    let text = r#"[
        (depth: 0.5, color: (1, 2, 3, 255), shapes: [
            Rect(x: 0.0, y: 10.0, w: 20.0, h: 30.0),
            Circle(x: 5.0, y: 6.0, radius: 7.0),
            Triangle(a: (0.0, 0.0), b: (4.0, 0.0), c: (2.0, 3.0)),
        ]),
    ]"#;
    let layers: Vec<BackgroundLayer> = ron::from_str(text).unwrap();
    assert_eq!(layers.len(), 1);
    assert_eq!(layers[0].shapes.len(), 3);
    assert!(validate(&layers).is_ok());
    let again: Vec<BackgroundLayer> = ron::from_str(&ron::to_string(&layers).unwrap()).unwrap();
    assert_eq!(layers, again);
}

#[test]
fn unknown_fields_and_unknown_shapes_are_parse_errors() {
    assert!(ron::from_str::<BackgroundLayer>(
        "(depth: 0.5, color: (1, 2, 3, 255), shapes: [], glow: 1.0)"
    )
    .is_err());
    assert!(ron::from_str::<BackgroundLayer>(
        "(depth: 0.5, color: (1, 2, 3, 255), shapes: [Star(x: 0.0)])"
    )
    .is_err());
    assert!(ron::from_str::<Shape>("Circle(x: 0.0, y: 0.0, radius: 1.0, spikes: 5)").is_err());
    // A color channel outside 0 to 255 does not parse.
    assert!(ron::from_str::<BackgroundLayer>(
        "(depth: 0.5, color: (1, 2, 300, 255), shapes: [Circle(x: 0.0, y: 0.0, radius: 1.0)])"
    )
    .is_err());
}
