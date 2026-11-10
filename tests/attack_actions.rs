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
