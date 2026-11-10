use hollow_steps::enemy_attack::{active_window, AttackBox, AttackShape};
use hollow_steps::player::{Player, PlayerConfig};
use macroquad::math::Vec2;

fn hitbox() -> AttackBox {
    AttackBox {
        shape: AttackShape::Rect,
        offset: (30.0, 0.0),
        size: (20.0, 20.0),
        damage: 2,
        knockback: (40.0, -50.0),
        active_secs: 0.5,
    }
}

#[test]
fn attack_geometry_is_independent_and_windows_are_half_open() {
    let attack = hitbox();
    attack.validate().unwrap();
    assert!(attack.overlaps(Vec2::ZERO, Vec2::new(30.0, 0.0), Vec2::splat(2.0)));
    assert!(!attack.overlaps(Vec2::ZERO, Vec2::ZERO, Vec2::splat(2.0)));
    assert!(!active_window(0.49, 0.5, attack.active_secs));
    assert!(active_window(0.5, 0.5, attack.active_secs));
    assert!(!active_window(1.0, 0.5, attack.active_secs));
    assert!(!active_window(0.5, 0.5, 0.0));
}

#[test]
fn attack_damage_and_knockback_respect_existing_invincibility() {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    assert!(player.take_attack_damage(hitbox().hit()));
    assert_eq!(player.health().current(), 3);
    let after = player.clone();
    assert!(!player.take_attack_damage(hitbox().hit()));
    assert_eq!(player, after);
}

use hollow_steps::behavior::{Behavior, BehaviorContext, BehaviorDefinition};
use hollow_steps::player::WorldBounds;
use std::sync::Arc;

fn context() -> BehaviorContext {
    BehaviorContext {
        player_position: Vec2::new(400.0, 442.0),
        health: 3,
        size: Vec2::splat(36.0),
        bounds: WorldBounds::new(0.0, 960.0, 460.0),
        patrol_left: 100.0,
        patrol_right: 200.0,
    }
}
fn machine(actions: &str) -> Behavior {
    Behavior::new(Arc::new(
        BehaviorDefinition::from_ron(&format!(
        "(initial_state: \"run\", states: [(name: \"run\", actions: [{actions}], transitions: [])])"
    ))
        .unwrap(),
    ))
    .unwrap()
}

#[test]
fn lunge_samples_direction_once_and_stands_still_in_recovery() {
    let mut behavior = machine("lunge(speed: 100.0, duration_secs: 1.0, attack: (shape: Rect, offset: (0.0, 0.0), size: (40.0, 40.0), damage: 2, knockback: (10.0, -10.0), active_secs: 1.0), recovery_secs: 0.5, telegraph_secs: 0.5)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(position.x, 100.0);
    assert!(behavior.attack_warning().is_some());
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    let moved_player = BehaviorContext {
        player_position: Vec2::new(0.0, 442.0),
        ..context()
    };
    behavior.update(&mut position, moved_player, 0.5);
    assert_eq!(position.x, 125.0);
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_some());
    behavior.update(&mut position, moved_player, 0.75);
    assert_eq!(position.x, 200.0);
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    assert!(!behavior.action_finished());
    behavior.update(&mut position, moved_player, 0.5);
    assert_eq!(position.x, 200.0);
    assert!(behavior.action_finished());
}
