use hollow_steps::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const LOWER_FLOOR: WorldBounds = WorldBounds::new(0.0, 960.0, 800.0);

fn player() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(200.0, BOUNDS);
    player
}

#[test]
fn holding_jump_reaches_higher_than_tapping_and_both_land() {
    let mut tap = player();
    let mut hold = player();
    for player in [&mut tap, &mut hold] {
        player.update(
            PlayerInput {
                jump_pressed: true,
                ..PlayerInput::default()
            },
            0.0,
            BOUNDS,
        );
    }
    let mut tap_apex = tap.position().y;
    let mut hold_apex = hold.position().y;
    for _ in 0..150 {
        tap.update(PlayerInput::default(), 0.01, BOUNDS);
        hold.update(
            PlayerInput {
                jump_held: true,
                ..PlayerInput::default()
            },
            0.01,
            BOUNDS,
        );
        tap_apex = tap_apex.min(tap.position().y);
        hold_apex = hold_apex.min(hold.position().y);
    }
    assert!(hold_apex < tap_apex - 50.0);
    assert!(tap.is_grounded() && hold.is_grounded());
    assert_eq!(tap.position().y, 436.0);
    assert_eq!(hold.position().y, 436.0);
    // Holding alone never starts another jump.
    hold.update(
        PlayerInput {
            jump_held: true,
            ..PlayerInput::default()
        },
        0.01,
        BOUNDS,
    );
    assert!(hold.is_grounded());
}

#[test]
fn release_restores_normal_gravity_and_invalid_time_does_not_move_player() {
    let mut player = player();
    player.update(
        PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..PlayerInput::default()
        },
        0.01,
        BOUNDS,
    );
    let before = player.velocity().y;
    player.update(PlayerInput::default(), 0.01, BOUNDS);
    assert!((player.velocity().y - before - 14.0).abs() < 0.001);
    let position = player.position();
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        player.update(PlayerInput::default(), dt, BOUNDS);
        assert_eq!(player.position(), position);
    }
    player.update(PlayerInput::default(), 100.0, BOUNDS);
    assert!(player.is_grounded());
    assert_eq!(player.velocity().y, 0.0);
}

#[test]
fn coyote_jump_works_once_after_leaving_support_and_expires() {
    let mut early = player();
    early.update(PlayerInput::default(), 0.01, LOWER_FLOOR);
    early.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.05,
        LOWER_FLOOR,
    );
    assert!(early.velocity().y < 0.0);
    early.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        LOWER_FLOOR,
    );
    assert!(early.velocity().y > -PlayerConfig::default().jump_speed);

    let mut late = player();
    late.update(PlayerInput::default(), 0.01, LOWER_FLOOR);
    late.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        PlayerConfig::default().coyote_seconds,
        LOWER_FLOOR,
    );
    assert!(late.velocity().y > 0.0);
}

#[test]
fn buffered_jump_fires_on_landing_but_old_presses_and_air_spawns_do_not() {
    let mut buffered = player();
    buffered.place_at(Vec2::new(200.0, 435.0), BOUNDS);
    buffered.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.01,
        BOUNDS,
    );
    assert!(buffered.velocity().y > 0.0);
    for _ in 0..4 {
        buffered.update(PlayerInput::default(), 0.01, BOUNDS);
    }
    assert!(buffered.velocity().y < 0.0);
    assert!(!buffered.is_grounded());

    let mut expired = player();
    expired.place_at(Vec2::new(200.0, 100.0), BOUNDS);
    expired.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.01,
        BOUNDS,
    );
    assert!(expired.velocity().y > 0.0);
    expired.update(PlayerInput::default(), 1.0, BOUNDS);
    assert!(expired.is_grounded());
    assert_eq!(expired.velocity().y, 0.0);
}

#[test]
fn placement_clears_buffer_and_coyote_time() {
    let mut player = player();
    player.update(PlayerInput::default(), 0.01, LOWER_FLOOR);
    player.place_at(Vec2::new(200.0, 435.0), BOUNDS);
    player.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );
    assert_eq!(player.velocity().y, 0.0);
    player.place_on_floor(200.0, BOUNDS);
    player.update(PlayerInput::default(), 0.01, BOUNDS);
    assert!(player.is_grounded());
}
