mod support;

use hollow_steps::enemy::Enemy;
use hollow_steps::game::GameState;
use hollow_steps::health::{Health, HIT_DAMAGE, MAX_HEALTH};
use hollow_steps::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

#[test]
fn patrol_turns_at_both_ends_and_preserves_overshoot() {
    let mut enemy = Enemy::new(100.0, 200.0, 100.0, 0.0, BOUNDS);
    for expected in [200.0, 100.0, 200.0, 100.0] {
        enemy.update(Vec2::ZERO, 1.0, BOUNDS);
        assert_eq!(enemy.position().x, expected);
    }
    enemy.update(Vec2::ZERO, 4.25, BOUNDS);
    assert_eq!(enemy.position(), Vec2::new(125.0, 442.0));
    enemy.update(Vec2::ZERO, 1.0, BOUNDS);
    assert_eq!(enemy.position().x, 175.0);
}

#[test]
fn chase_uses_distance_moves_both_ways_and_stays_on_floor() {
    let mut enemy = Enemy::new(100.0, 200.0, 100.0, 150.0, BOUNDS);
    enemy.update(Vec2::new(50.0, 442.0), 0.25, BOUNDS);
    assert_eq!(enemy.position(), Vec2::new(75.0, 442.0));
    enemy.update(Vec2::new(125.0, 442.0), 0.25, BOUNDS);
    assert_eq!(enemy.position().x, 100.0);
    // Above detection range: patrol instead of chasing left.
    enemy.update(Vec2::new(50.0, 0.0), 0.25, BOUNDS);
    assert_eq!(enemy.position(), Vec2::new(125.0, 442.0));
}

#[test]
fn detection_boundary_is_strict_and_return_from_chase_does_not_teleport() {
    let mut enemy = Enemy::new(100.0, 200.0, 100.0, 50.0, BOUNDS);
    enemy.update(Vec2::new(50.0, 442.0), 0.1, BOUNDS);
    assert_eq!(enemy.position().x, 110.0);
    enemy.update(Vec2::new(70.0, 442.0), 0.4, BOUNDS);
    assert_eq!(enemy.position().x, 70.0);
    enemy.update(Vec2::ZERO, 0.1, BOUNDS);
    assert_eq!(enemy.position().x, 80.0);
}

#[test]
fn enemy_handles_zero_invalid_and_large_steps_and_room_walls() {
    let mut enemy = Enemy::new(100.0, 200.0, 100.0, 2000.0, BOUNDS);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        enemy.update(Vec2::new(900.0, 442.0), dt, BOUNDS);
        assert_eq!(enemy.position().x, 100.0);
    }
    enemy.update(Vec2::new(1000.0, 442.0), 100.0, BOUNDS);
    assert_eq!(enemy.position(), Vec2::new(942.0, 442.0));
    let mut stationary = Enemy::new(100.0, 100.0, 100.0, 0.0, BOUNDS);
    stationary.update(Vec2::ZERO, 1000.0, BOUNDS);
    assert_eq!(stationary.position().x, 100.0);
}

#[test]
fn contact_requires_both_axes_including_touching_edges() {
    let enemy = Enemy::new(100.0, 200.0, 100.0, 0.0, BOUNDS);
    let size = Vec2::new(32.0, 48.0);
    assert!(enemy.touches(Vec2::new(134.0, 442.0), size));
    assert!(!enemy.touches(Vec2::new(135.0, 442.0), size));
    assert!(!enemy.touches(Vec2::new(100.0, 399.0), size));
}

#[test]
fn damage_is_one_hit_and_immunity_expires_at_one_second() {
    let mut health = Health::default();
    assert_eq!(health.current(), 50);
    assert!(health.take_damage());
    assert_eq!(health.current(), 40);
    assert!(!health.take_damage());
    let initial_flash = health.flash_on();
    health.update(0.125);
    assert_ne!(health.flash_on(), initial_flash);
    health.update(0.625);
    assert!(!health.take_damage());
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        health.update(dt);
    }
    assert!(health.is_invincible());
    health.update(0.25);
    assert!(!health.is_invincible());
    assert!(!health.flash_on());
    assert!(health.take_damage());
    assert_eq!(health.current(), 30);
}

#[test]
fn dead_health_cannot_underflow_and_respawn_clears_player_motion() {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    for _ in 0..MAX_HEALTH / HIT_DAMAGE {
        player.update(PlayerInput::default(), 1.0, BOUNDS);
        assert!(player.take_damage());
    }
    assert_eq!(player.health().current(), 0);
    player.update(PlayerInput::default(), 10.0, BOUNDS);
    assert!(!player.take_damage());
    player.respawn(Vec2::new(350.0, 436.0), BOUNDS);
    assert_eq!(player.position(), Vec2::new(350.0, 436.0));
    assert_eq!(player.velocity(), Vec2::ZERO);
    assert!(player.is_grounded());
    assert_eq!(player.health().current(), MAX_HEALTH);
    assert!(player.health().is_invincible());
}

#[test]
fn game_contact_damages_once_per_second_then_respawns_at_room_spawn() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    assert_eq!(game.room().enemies.len(), 1);
    assert_eq!(game.player().health().current(), 50);
    // Walk to the enemy in small deterministic steps.
    for _ in 0..400 {
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            0.01,
        );
        if game.player().health().current() == 40 {
            break;
        }
    }
    assert_eq!(game.player().health().current(), 40);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player().health().current(), 40);
    for expected in [30, 20, 10, 50] {
        game.update(PlayerInput::default(), 1.0);
        assert_eq!(game.player().health().current(), expected);
    }
    assert_eq!(game.player().position(), game.room().spawn);
    assert_eq!(game.player().velocity(), Vec2::ZERO);
    assert!(game.player().is_grounded());
    assert!(game.player().health().is_invincible());
}
