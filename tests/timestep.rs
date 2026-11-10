use hollow_steps::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use hollow_steps::timestep::{FixedStep, InputLatch};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

fn player() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(200.0, BOUNDS);
    player
}

/// Drives a jump (pressed on the first frame, held throughout) for one simulated
/// second of frames through the fixed step; returns the highest point reached.
fn fixed_apex(fps: f32) -> f32 {
    let mut player = player();
    let mut step = FixedStep::default();
    let mut latch = InputLatch::default();
    let mut apex = player.position().y;
    for frame in 0..fps as usize {
        latch.record(PlayerInput {
            jump_pressed: frame == 0,
            jump_held: true,
            ..PlayerInput::default()
        });
        for _ in 0..step.advance(1.0 / fps) {
            player.update(latch.take_tick(), step.step_seconds(), BOUNDS);
            apex = apex.min(player.position().y);
        }
    }
    apex
}

/// The old behavior: the raw frame time goes straight into the simulation.
fn raw_apex(fps: f32) -> f32 {
    let mut player = player();
    let mut apex = player.position().y;
    for frame in 0..fps as usize {
        player.update(
            PlayerInput {
                jump_pressed: frame == 0,
                jump_held: true,
                ..PlayerInput::default()
            },
            1.0 / fps,
            BOUNDS,
        );
        apex = apex.min(player.position().y);
    }
    apex
}

#[test]
fn raw_frame_times_change_jump_height() {
    // Reproduces the bug the fixed step removes.
    assert!((raw_apex(30.0) - raw_apex(144.0)).abs() > 1.0);
}

#[test]
fn jump_apex_is_identical_at_30_60_and_144_fps() {
    let reference = fixed_apex(60.0);
    assert!(reference < 436.0 - 50.0, "the jump must actually rise");
    assert_eq!(fixed_apex(30.0), reference);
    assert_eq!(fixed_apex(144.0), reference);
}

#[test]
fn a_jump_pressed_on_a_zero_tick_frame_still_happens() {
    // 240 fps frames are shorter than a tick, so most frames run no tick at all.
    let mut player = player();
    let mut step = FixedStep::default();
    let mut latch = InputLatch::default();
    latch.record(PlayerInput {
        jump_pressed: true,
        ..PlayerInput::default()
    });
    assert_eq!(step.advance(1.0 / 240.0), 0);
    latch.record(PlayerInput::default());
    let mut jumped = false;
    for _ in 0..4 {
        for _ in 0..step.advance(1.0 / 240.0) {
            player.update(latch.take_tick(), step.step_seconds(), BOUNDS);
            jumped |= player.velocity().y < 0.0;
        }
    }
    assert!(jumped);
}
