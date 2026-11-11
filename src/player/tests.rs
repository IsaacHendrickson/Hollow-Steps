use super::*;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

fn player_on_floor() -> Player {
    let config = PlayerConfig::default();
    let mut player = Player::new(Vec2::ZERO, config);
    player.place_on_floor(200.0, BOUNDS);
    player
}

#[test]
fn horizontal_input_moves_at_configured_speed() {
    let mut player = player_on_floor();

    player.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        0.5,
        BOUNDS,
    );

    assert!((player.position().x - 310.0).abs() < f32::EPSILON);
    assert!((player.velocity().x - 220.0).abs() < f32::EPSILON);
}

#[test]
fn horizontal_input_is_clamped_to_the_world() {
    let mut player = player_on_floor();

    player.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        10.0,
        BOUNDS,
    );

    assert_eq!(player.position().x, 944.0);
}

#[test]
fn jump_applies_upward_velocity_and_gravity() {
    let mut player = player_on_floor();

    player.update(
        PlayerInput {
            move_axis: 0.0,
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );

    assert!(!player.is_grounded());
    assert!((player.velocity().y + 480.0).abs() < f32::EPSILON);

    player.update(PlayerInput::default(), 0.1, BOUNDS);

    assert!((player.velocity().y + 340.0).abs() < f32::EPSILON);
    assert!(player.position().y < 436.0);
}

#[test]
fn jump_input_is_ignored_while_airborne() {
    let mut player = player_on_floor();
    player.update(
        PlayerInput {
            move_axis: 0.0,
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );

    player.update(
        PlayerInput {
            move_axis: 0.0,
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.1,
        BOUNDS,
    );

    assert!((player.velocity().y + 340.0).abs() < f32::EPSILON);
}

#[test]
fn falling_player_lands_on_the_floor() {
    let mut player = player_on_floor();
    player.update(PlayerInput::default(), 1.0, BOUNDS);

    assert_eq!(player.position().y, 436.0);
    assert_eq!(player.velocity().y, 0.0);
    assert!(player.is_grounded());
}

#[test]
fn room_spawn_preserves_height_and_inventory_but_clears_movement() {
    let mut player = player_on_floor();
    player.collect_item(Item::AmberShard);
    player.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.1,
        BOUNDS,
    );

    player.place_at(Vec2::new(100.0, 200.0), BOUNDS);

    assert_eq!(player.position(), Vec2::new(100.0, 200.0));
    assert_eq!(player.velocity(), Vec2::ZERO);
    assert!(!player.is_grounded());
    assert_eq!(player.collected_items(), &[Item::AmberShard]);
}
