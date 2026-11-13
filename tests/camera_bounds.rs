//! The follow camera shows the room's edges: it may travel out to the outer face of the
//! boundary slabs (the walls, ceiling and floor), which stand just outside the room bounds.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::camera::{Camera, CameraConfig};
use hollow_steps::game::{GameState, VIEW_HEIGHT, VIEW_WIDTH};
use hollow_steps::terrain_style::boundary_slabs;
use macroquad::math::{Rect, Vec2};

const VIEW: Vec2 = Vec2::new(VIEW_WIDTH, VIEW_HEIGHT);

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-camera-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn game() -> GameState {
    GameState::try_new(&support::fixture_assets()).unwrap()
}

fn thickness(game: &GameState) -> f32 {
    game.terrain_styles()
        .resolve_or_default(&game.room().terrain_style)
        .boundary_thickness
}

/// A camera resting on `target`, as `new_camera` in the front end makes it.
fn camera_on(game: &GameState, target: Vec2) -> Camera {
    Camera::new(
        CameraConfig::default(),
        target,
        0.0,
        game.camera_bounds(),
        VIEW,
    )
}

#[test]
fn the_camera_bounds_are_the_room_widened_by_the_wall_thickness() {
    let game = game();
    let room = game.room().bounds;
    let t = thickness(&game);
    assert!(t > 0.0);
    assert_eq!(
        game.camera_bounds(),
        Rect::new(room.x - t, room.y - t, room.w + 2.0 * t, room.h + 2.0 * t)
    );
}

#[test]
fn at_the_left_edge_the_whole_left_wall_is_in_view() {
    let game = game();
    let room = game.room().bounds;
    let t = thickness(&game);
    let camera = camera_on(&game, Vec2::new(room.x, room.y + room.h / 2.0));
    let view = camera.view_rect(VIEW);
    assert!(
        view.left() <= room.x - t + 0.001,
        "the view starts at {} but the wall's outer face is at {}",
        view.left(),
        room.x - t
    );
    let slabs = boundary_slabs(room, game.world_bounds().floor_y, t, &game.room().exits);
    let wall = slabs
        .iter()
        .find(|slab| slab.right() <= room.x + 0.001 && slab.h < room.h + 2.0 * t - 1.0)
        .expect("a left wall slab");
    assert!(
        view.left() <= wall.x + 0.001 && view.right() >= wall.right(),
        "the wall {wall:?} is inside the view {view:?}"
    );
}

#[test]
fn at_the_right_edge_the_whole_right_wall_is_in_view() {
    let game = game();
    let room = game.room().bounds;
    let t = thickness(&game);
    let camera = camera_on(&game, Vec2::new(room.right(), room.y + room.h / 2.0));
    let view = camera.view_rect(VIEW);
    assert!(
        view.right() >= room.right() + t - 0.001,
        "the view ends at {} but the wall's outer face is at {}",
        view.right(),
        room.right() + t
    );
}

#[test]
fn the_ceiling_and_the_floor_are_reachable_too() {
    let game = game();
    let room = game.room().bounds;
    let t = thickness(&game);
    // Tall enough view to need scrolling vertically: use a camera over a taller world.
    let top = Camera::new(
        CameraConfig::default(),
        Vec2::new(room.x + room.w / 2.0, room.y),
        0.0,
        Rect::new(room.x - t, room.y - t, room.w + 2.0 * t, 2000.0),
        VIEW,
    );
    assert!((top.view_rect(VIEW).top() - (room.y - t)).abs() < 0.001);
    let bottom_edge = Rect::new(room.x - t, -1500.0, room.w + 2.0 * t, 1500.0 + room.h + t);
    let bottom = Camera::new(
        CameraConfig::default(),
        Vec2::new(room.x + room.w / 2.0, room.bottom()),
        0.0,
        bottom_edge,
        VIEW,
    );
    assert!((bottom.view_rect(VIEW).bottom() - (room.bottom() + t)).abs() < 0.001);
}

#[test]
fn a_following_camera_never_leaves_the_widened_bounds() {
    let game = game();
    let bounds = game.camera_bounds();
    let room = game.room().bounds;
    let mut camera = camera_on(
        &game,
        Vec2::new(room.x + room.w / 2.0, room.y + room.h / 2.0),
    );
    for target_x in [room.x - 500.0, room.right() + 500.0, room.x - 500.0] {
        for _ in 0..600 {
            camera.update(
                Vec2::new(target_x, room.bottom()),
                -1.0,
                bounds,
                VIEW,
                1.0 / 120.0,
            );
            let view = camera.view_rect(VIEW);
            // A view wider than the bounds is centered on them; otherwise it stays inside.
            if bounds.w >= VIEW.x {
                assert!(view.left() >= bounds.x - 0.001 && view.right() <= bounds.right() + 0.001);
            }
            if bounds.h >= VIEW.y {
                assert!(view.top() >= bounds.y - 0.001 && view.bottom() <= bounds.bottom() + 0.001);
            }
        }
    }
}

#[test]
fn without_a_boundary_the_camera_keeps_to_the_room() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    game.editor_mut().unwrap().draft.set_draw_boundary(false);
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    let restarted = GameState::try_new(&temp.0).unwrap();
    assert!(
        !restarted
            .terrain_styles()
            .resolve_or_default(&restarted.room().terrain_style)
            .draw_boundary
    );
    assert_eq!(restarted.camera_bounds(), restarted.room().bounds);
}
