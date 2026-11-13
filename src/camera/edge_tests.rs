//! The camera at the edges of rooms that are larger than the view, and the path from the
//! camera to the window: the visible world rectangle must touch the room edge it rests
//! against, never show anything outside the room, and match what is rendered.
use macroquad::camera::Camera2D;

use super::*;
use crate::game::{VIEW_HEIGHT, VIEW_WIDTH};

const VIEW: Vec2 = Vec2::new(VIEW_WIDTH, VIEW_HEIGHT);
const EPS: f32 = 0.01;
/// Half the player's width and height: how close its center gets to a room edge.
const HALF_PLAYER: Vec2 = Vec2::new(16.0, 24.0);

fn rooms() -> [Rect; 5] {
    [
        // Much wider and taller than the view.
        Rect::new(0.0, 0.0, 4800.0, 3000.0),
        // Bounds that do not start at 0,0, including negative coordinates.
        Rect::new(-500.0, 300.0, 2400.0, 1200.0),
        Rect::new(100.0, 50.0, 2000.0, 800.0),
        // Barely larger than the view, and wide but exactly one view tall.
        Rect::new(0.0, 0.0, 961.0, 541.0),
        Rect::new(-40.0, 20.0, 3000.0, 540.0),
    ]
}

/// Player centers at the extreme corners and edge middles of `room`.
fn limit_targets(room: Rect) -> Vec<Vec2> {
    let xs = [
        room.x + HALF_PLAYER.x,
        room.x + room.w / 2.0,
        room.right() - HALF_PLAYER.x,
    ];
    let ys = [
        room.y + HALF_PLAYER.y,
        room.y + room.h / 2.0,
        room.bottom() - HALF_PLAYER.y,
    ];
    xs.iter()
        .flat_map(|&x| ys.iter().map(move |&y| Vec2::new(x, y)))
        .collect()
}

fn inside(view: Rect, room: Rect) -> bool {
    view.x >= room.x - EPS
        && view.y >= room.y - EPS
        && view.right() <= room.right() + EPS
        && view.bottom() <= room.bottom() + EPS
}

/// The sides of `room` a camera resting on `target` must touch, as (left, right, top, bottom).
fn expected_touches(room: Rect, target: Vec2) -> (bool, bool, bool, bool) {
    (
        target.x <= room.x + HALF_PLAYER.x + EPS,
        target.x >= room.right() - HALF_PLAYER.x - EPS,
        target.y <= room.y + HALF_PLAYER.y + EPS,
        target.y >= room.bottom() - HALF_PLAYER.y - EPS,
    )
}

fn assert_touches(view: Rect, room: Rect, target: Vec2, context: &str) {
    let (left, right, top, bottom) = expected_touches(room, target);
    assert!(
        inside(view, room),
        "{context}: view {view:?} outside {room:?}"
    );
    assert!(!left || (view.x - room.x).abs() < EPS, "{context}: left");
    assert!(
        !right || (view.right() - room.right()).abs() < EPS,
        "{context}: right"
    );
    assert!(!top || (view.y - room.y).abs() < EPS, "{context}: top");
    assert!(
        !bottom || (view.bottom() - room.bottom()).abs() < EPS,
        "{context}: bottom"
    );
}

#[test]
fn a_settled_camera_touches_every_room_edge_and_never_shows_outside() {
    let config = CameraConfig::default();
    for room in rooms() {
        for target in limit_targets(room) {
            for facing in [1.0, -1.0] {
                for step in [1.0 / 120.0, 1.0 / 60.0, 1.0 / 30.0] {
                    // Start from the opposite side so the smoothing has to travel.
                    let start = room.point() + room.size() - target + room.point();
                    let mut camera = Camera::new(config, start, facing, room, VIEW);
                    for tick in 0..((6.0 / step) as u32) {
                        camera.update(target, facing, room, VIEW, step);
                        assert!(
                            inside(camera.view_rect(VIEW), room),
                            "tick {tick}: view left {room:?} heading for {target:?}"
                        );
                    }
                    assert_touches(
                        camera.view_rect(VIEW),
                        room,
                        target,
                        &format!("{room:?} {target:?} facing {facing} step {step}"),
                    );
                }
            }
        }
    }
}

#[test]
fn a_snapped_camera_touches_every_room_edge_too() {
    let config = CameraConfig::default();
    for room in rooms() {
        for target in limit_targets(room) {
            for facing in [1.0, -1.0, 0.0] {
                let camera = Camera::new(config, target, facing, room, VIEW);
                assert_touches(camera.view_rect(VIEW), room, target, "snap");
                // The shaken picture is allowed to leave by the shake, nothing else.
                let shaken = camera.view_rect_shaken(VIEW, Vec2::new(2.0, -2.0));
                assert!((shaken.x - camera.view_rect(VIEW).x - 2.0).abs() < EPS);
            }
        }
    }
}

#[test]
fn look_ahead_cannot_hide_the_player_at_the_edge() {
    let room = Rect::new(-500.0, 300.0, 2400.0, 1200.0);
    let config = CameraConfig::default();
    // Facing the wall the player is pressed against: the look-ahead leads outside the room,
    // the view stays inside it, and the player is still on screen.
    for (target, facing) in [
        (Vec2::new(room.x + 16.0, 900.0), -1.0),
        (Vec2::new(room.right() - 16.0, 900.0), 1.0),
    ] {
        let mut camera = Camera::new(config, target, facing, room, VIEW);
        for _ in 0..1200 {
            camera.update(target, facing, room, VIEW, 1.0 / 120.0);
        }
        let view = camera.view_rect(VIEW);
        assert!(inside(view, room));
        assert!(view.contains(target), "player off screen at {target:?}");
    }
}

#[test]
fn the_view_size_is_the_rendered_size() {
    // `main.rs` renders to `render_target(VIEW_WIDTH as u32, VIEW_HEIGHT as u32)` and passes
    // `vec2(VIEW_WIDTH, VIEW_HEIGHT)` to the camera clamp: the cast must lose nothing.
    assert_eq!(VIEW_WIDTH, (VIEW_WIDTH as u32) as f32);
    assert_eq!(VIEW_HEIGHT, (VIEW_HEIGHT as u32) as f32);
    let camera = Camera::new(
        CameraConfig::default(),
        Vec2::new(1000.0, 800.0),
        1.0,
        rooms()[0],
        VIEW,
    );
    assert_eq!(camera.view_rect(VIEW).size(), VIEW);
}

/// What the `Camera2D` built from the visible rectangle shows, from its `target` and `zoom`.
fn shown_by(camera: &Camera2D) -> Rect {
    let half = Vec2::new(1.0 / camera.zoom.x.abs(), 1.0 / camera.zoom.y.abs());
    Rect::new(
        camera.target.x - half.x,
        camera.target.y - half.y,
        half.x * 2.0,
        half.y * 2.0,
    )
}

/// Window position of a world point: the picture is `view_rect` at 1:1 in the render target,
/// which is then blitted into the letterbox.
fn to_window(point: Vec2, view_rect: Rect, boxed: Letterbox) -> Vec2 {
    boxed.offset + (point - view_rect.point()) * boxed.scale
}

#[test]
fn terrain_touching_a_room_edge_is_fully_visible_at_every_window_size() {
    let config = CameraConfig::default();
    let windows = [
        Vec2::new(960.0, 540.0),
        Vec2::new(1920.0, 1080.0),
        Vec2::new(1600.0, 540.0),
        Vec2::new(960.0, 900.0),
        Vec2::new(400.0, 300.0),
        Vec2::new(700.0, 700.0),
        Vec2::new(3440.0, 1440.0),
        Vec2::new(1283.0, 719.0),
    ];
    for room in rooms() {
        for target in limit_targets(room) {
            let camera = Camera::new(config, target, 1.0, room, VIEW);
            let view_rect = camera.view_rect(VIEW);
            // The camera the front end builds shows exactly the visible rectangle.
            let shown = shown_by(&Camera2D::from_display_rect(view_rect));
            assert!((shown.x - view_rect.x).abs() < 0.05 && (shown.w - VIEW.x).abs() < 0.05);
            assert!((shown.y - view_rect.y).abs() < 0.05 && (shown.h - VIEW.y).abs() < 0.05);
            let (left, right, top, bottom) = expected_touches(room, target);
            for window in windows {
                let boxed = Letterbox::fit(window, VIEW);
                // A 40 px solid hugging each corner of the room, clipped to what is touched.
                let corners = [
                    (left, top, Vec2::new(room.x, room.y)),
                    (right, top, Vec2::new(room.right() - 40.0, room.y)),
                    (left, bottom, Vec2::new(room.x, room.bottom() - 40.0)),
                    (
                        right,
                        bottom,
                        Vec2::new(room.right() - 40.0, room.bottom() - 40.0),
                    ),
                ];
                for (touch_x, touch_y, origin) in corners {
                    if !(touch_x && touch_y) {
                        continue;
                    }
                    let (low, high) = (
                        to_window(origin, view_rect, boxed),
                        to_window(origin + Vec2::splat(40.0), view_rect, boxed),
                    );
                    let inner = Rect::new(
                        boxed.offset.x - 0.01,
                        boxed.offset.y - 0.01,
                        boxed.size.x + 0.02,
                        boxed.size.y + 0.02,
                    );
                    assert!(
                        inner.contains(low) && inner.contains(high),
                        "{room:?} corner {origin:?} cut off at window {window:?}"
                    );
                    assert!(
                        boxed.offset.x + boxed.size.x <= window.x + 0.01
                            && boxed.offset.y + boxed.size.y <= window.y + 0.01
                    );
                }
            }
        }
    }
}

#[test]
fn resizing_the_window_never_changes_what_the_camera_shows() {
    let room = rooms()[1];
    let camera = Camera::new(
        CameraConfig::default(),
        Vec2::new(0.0, 900.0),
        1.0,
        room,
        VIEW,
    );
    let before = camera.view_rect(VIEW);
    for window in [Vec2::new(500.0, 300.0), Vec2::new(2560.0, 1440.0)] {
        let boxed = Letterbox::fit(window, VIEW);
        // The visible rectangle is the camera's alone; the letterbox only places it. The
        // fitted picture is always the whole 960x540 view, uniformly scaled.
        assert_eq!(camera.view_rect(VIEW), before);
        assert!((boxed.size.x / boxed.size.y - VIEW.x / VIEW.y).abs() < 1e-3);
        assert!(
            boxed
                .window_to_virtual(boxed.offset + boxed.size)
                .distance(VIEW)
                < 0.05
        );
    }
}
