use super::*;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DASH: PlayerInput = PlayerInput {
    dash_pressed: true,
    move_axis: 0.0,
    jump_pressed: false,
    pickup_pressed: false,
    jump_held: false,
    attack_pressed: false,
    up_held: false,
    down_held: false,
};

fn equipped_player() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(400.0, BOUNDS);
    player.collect_item(Item::Dash);
    player
}

#[test]
fn dash_requires_pickup() {
    let mut player = equipped_player();
    player.collected_items.clear();
    player.update(DASH, 0.1, BOUNDS);
    assert_eq!(player.position.x, 400.0);
}

#[test]
fn dash_uses_last_facing_and_expires_after_point_two_seconds() {
    let mut player = equipped_player();
    player.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );
    player.update(DASH, 0.1, BOUNDS);
    assert_eq!(player.position, Vec2::new(320.0, 436.0));
    assert_eq!(player.velocity.x, -DASH_SPEED);
    // Opposite input cannot reverse a dash in progress.
    player.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        0.1,
        BOUNDS,
    );
    assert_eq!(player.position.x, 240.0);
    player.update(PlayerInput::default(), 0.1, BOUNDS);
    assert_eq!(player.position.x, 240.0);
}

#[test]
fn cooldown_blocks_another_dash_until_half_a_second_after_start() {
    let mut player = equipped_player();
    player.update(DASH, 0.25, BOUNDS);
    let x = player.position.x;
    player.update(DASH, 0.25, BOUNDS);
    assert_eq!(player.position.x, x);
    player.update(DASH, 0.1, BOUNDS);
    assert_eq!(player.position.x, x + 80.0);
}

#[test]
fn air_dash_is_horizontal_and_only_landing_restores_it() {
    let mut player = equipped_player();
    player.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );
    for _ in 0..20 {
        player.update(PlayerInput::default(), 0.01, BOUNDS);
    }
    let y = player.position.y;
    player.update(DASH, 0.2, BOUNDS);
    assert_eq!(player.position.y, y);
    for _ in 0..30 {
        player.update(PlayerInput::default(), 0.01, BOUNDS);
    }
    assert!(!player.is_grounded());
    let x = player.position.x;
    player.update(DASH, 0.0, BOUNDS);
    assert_eq!(player.dash_remaining, 0.0);
    assert_eq!(player.position.x, x);
    player.update(PlayerInput::default(), 1.0, BOUNDS);
    assert!(player.is_grounded());
    player.update(
        PlayerInput {
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.1,
        BOUNDS,
    );
    player.update(DASH, 0.1, BOUNDS);
    assert_eq!(player.velocity.x, DASH_SPEED);
    assert_eq!(player.velocity.y, 0.0);
    assert!(!player.is_grounded());
}

#[test]
fn buffered_jump_waits_for_ground_dash_to_finish() {
    let mut player = equipped_player();
    player.update(DASH, 0.1, BOUNDS);
    player.update(
        PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..PlayerInput::default()
        },
        0.05,
        BOUNDS,
    );
    assert_eq!(player.position.y, 436.0);
    assert_eq!(player.velocity.y, 0.0);
    player.update(
        PlayerInput {
            jump_held: true,
            ..PlayerInput::default()
        },
        0.05,
        BOUNDS,
    );
    assert_eq!(player.position.x, 560.0);
    assert!(!player.is_grounded());
    assert_eq!(player.velocity.y, -player.config.jump_speed);
}

#[test]
fn attack_during_dash_keeps_dash_and_swing_direction() {
    let mut player = equipped_player();
    player.update(DASH, 0.05, BOUNDS);
    player.update(
        PlayerInput {
            move_axis: -1.0,
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.05,
        BOUNDS,
    );
    assert_eq!(player.position.x, 480.0);
    assert_eq!(player.attack.direction(), 1.0);
    assert!(player
        .attack
        .hitbox(player.position, player.size())
        .is_some());
}

#[test]
fn zero_and_large_steps_preserve_duration_and_bounds() {
    let mut player = equipped_player();
    player.update(DASH, 0.0, BOUNDS);
    assert_eq!(player.position.x, 400.0);
    player.update(PlayerInput::default(), 10.0, BOUNDS);
    assert_eq!(player.position.x, 560.0);
    player.place_on_floor(900.0, BOUNDS);
    player.update(DASH, 0.2, BOUNDS);
    assert_eq!(player.position.x, 944.0);
    player.place_on_floor(30.0, BOUNDS);
    player.update(
        PlayerInput {
            move_axis: -1.0,
            ..DASH
        },
        10.0,
        BOUNDS,
    );
    assert_eq!(player.position.x, 16.0);
    assert!(player.is_grounded());
}
