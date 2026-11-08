use hollow_steps::combat::{Attack, ATTACK_COOLDOWN, ATTACK_REACH, SWING_SECONDS};
use hollow_steps::enemy::{Enemy, ENEMY_MAX_HEALTH, KNOCKBACK_SECONDS};
use hollow_steps::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

#[test]
fn swing_is_short_directional_and_has_a_cooldown() {
    let mut attack = Attack::default();
    let position = Vec2::new(200.0, 436.0);
    let size = Vec2::new(32.0, 48.0);
    assert!(attack.hitbox(position, size).is_none());
    attack.update(true, 1.0, 0.0);
    let first_id = attack.id();
    let hitbox = attack.hitbox(position, size).unwrap();
    assert_eq!(hitbox.x, 216.0);
    assert_eq!(hitbox.w, ATTACK_REACH);
    attack.update(true, -1.0, SWING_SECONDS);
    assert!(attack.hitbox(position, size).is_none());
    assert_eq!(attack.id(), first_id);
    attack.update(true, -1.0, ATTACK_COOLDOWN);
    let hitbox = attack.hitbox(position, size).unwrap();
    assert_eq!(hitbox.x + hitbox.w, 184.0);
    assert_ne!(attack.id(), first_id);
    // A later facing change cannot turn an existing swing around.
    attack.update(false, 1.0, 0.0);
    assert_eq!(attack.direction(), -1.0);
}

#[test]
fn attack_ignores_invalid_time_and_does_not_repeat_without_a_press() {
    let mut attack = Attack::default();
    attack.update(true, 1.0, 0.0);
    let id = attack.id();
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        attack.update(true, -1.0, dt);
        assert_eq!(attack.id(), id);
        assert!(attack.hitbox(Vec2::ZERO, Vec2::ONE).is_some());
    }
    attack.update(false, 1.0, 100.0);
    assert!(attack.hitbox(Vec2::ZERO, Vec2::ONE).is_none());
    assert_eq!(attack.id(), id);
}

#[test]
fn enemy_takes_one_damage_per_swing_and_death_disables_contact_and_motion() {
    let mut enemy = Enemy::new(200.0, 300.0, 100.0, 0.0, BOUNDS);
    for id in 1..=u64::from(ENEMY_MAX_HEALTH) {
        assert!(enemy.hit(id, 1.0));
        assert!(!enemy.hit(id, 1.0));
        assert_eq!(enemy.health(), ENEMY_MAX_HEALTH - id as u8);
    }
    assert!(enemy.is_dead());
    assert!(!enemy.hit(10, -1.0));
    assert!(!enemy.touches(enemy.position(), Vec2::ONE));
    let position = enemy.position();
    enemy.update(Vec2::ZERO, 100.0, BOUNDS);
    assert_eq!(enemy.position(), position);
}

#[test]
fn knockback_pushes_both_ways_stops_at_walls_and_then_patrol_resumes() {
    for direction in [-1.0, 1.0] {
        let mut enemy = Enemy::new(200.0, 300.0, 100.0, 0.0, BOUNDS);
        assert!(!enemy.hit(1, f32::NAN));
        assert!(!enemy.hit(1, 0.0));
        assert!(enemy.hit(1, direction));
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            enemy.update(Vec2::ZERO, dt, BOUNDS);
            assert_eq!(enemy.position().x, 200.0);
        }
        enemy.update(Vec2::ZERO, KNOCKBACK_SECONDS, BOUNDS);
        assert_eq!((enemy.position().x - 200.0).signum(), direction);
        let position = enemy.position();
        enemy.update(Vec2::ZERO, 0.01, BOUNDS);
        assert_ne!(enemy.position(), position);
    }
    for (x, direction, expected) in [(20.0, -1.0, 18.0), (940.0, 1.0, 942.0)] {
        let mut enemy = Enemy::new(x, x, 0.0, 0.0, BOUNDS);
        enemy.hit(1, direction);
        enemy.update(Vec2::ZERO, 100.0, BOUNDS);
        assert_eq!(enemy.position().x, expected);
    }
}

#[test]
fn player_facing_persists_and_placement_cancels_active_attack() {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(200.0, BOUNDS);
    player.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );
    player.update(
        PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );
    assert_eq!(player.attack().direction(), -1.0);
    assert!(player
        .attack()
        .hitbox(player.position(), player.size())
        .is_some());
    let id = player.attack().id();
    player.place_on_floor(200.0, BOUNDS);
    assert!(player
        .attack()
        .hitbox(player.position(), player.size())
        .is_none());
    player.update(
        PlayerInput {
            attack_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
        BOUNDS,
    );
    assert_ne!(player.attack().id(), id);
}
