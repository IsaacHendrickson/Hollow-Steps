mod support;

use hollow_steps::boss_special::{
    BossSpecials, SpecialAttack, BOSS_MOVE_SPEED, DEATH_EFFECT_SECONDS, RING_RADIUS,
    SLAM_TELEGRAPH_SECONDS, SUMMON_WINDUP_SECONDS,
};
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

#[test]
fn starting_special_attacks_does_not_teleport_the_boss() {
    let start = Vec2::new(700.0, 436.0);
    for kind in [
        SpecialAttack::Slam,
        SpecialAttack::Summon,
        SpecialAttack::Rings,
    ] {
        let mut boss = warden(start);
        boss.start_attack(kind, Vec2::new(240.0, 436.0), BOUNDS);
        assert_eq!(boss.position(), start);
        for dt in [0.0, -1.0] {
            boss.update(dt, Vec2::ZERO, BOUNDS);
            assert_eq!(boss.position(), start);
        }
    }
}

#[test]
fn special_attacks_glide_at_a_fixed_speed_using_delta_time() {
    let start = Vec2::new(700.0, 436.0);
    for kind in [
        SpecialAttack::Slam,
        SpecialAttack::Summon,
        SpecialAttack::Rings,
    ] {
        let mut boss = warden(start);
        boss.start_attack(kind, Vec2::new(240.0, 436.0), BOUNDS);
        for dt in [0.02, 0.03] {
            let previous = boss.position();
            boss.update(dt, Vec2::ZERO, BOUNDS);
            assert!((boss.position().distance(previous) - BOSS_MOVE_SPEED * dt).abs() < 0.0001);
            assert!(boss.position().x < previous.x);
        }
        assert!((boss.position().distance(start) - BOSS_MOVE_SPEED * 0.05).abs() < 0.0001);
        assert!(boss.is_telegraphing());
    }
}

#[test]
fn gliding_stops_at_the_clamped_target_without_overshooting() {
    for (target_x, expected_x) in [(-960.0, BOUNDS.left), (1920.0, BOUNDS.right)] {
        let mut boss = warden(Vec2::new(700.0, 436.0));
        boss.start_attack(SpecialAttack::Summon, Vec2::new(target_x, 436.0), BOUNDS);
        boss.update(10.0, Vec2::ZERO, BOUNDS);
        assert_eq!(boss.position(), Vec2::new(expected_x, 436.0));
        boss.update(1.0, Vec2::ZERO, BOUNDS);
        assert_eq!(boss.position(), Vec2::new(expected_x, 436.0));
    }
}

#[test]
fn slam_glides_down_after_the_telegraph_without_a_landing_teleport() {
    let target = Vec2::new(240.0, 436.0);
    let mut boss = warden(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Slam, target, BOUNDS);
    boss.update(SLAM_TELEGRAPH_SECONDS - 0.001, Vec2::ZERO, BOUNDS);
    let hovering = boss.position();
    assert_eq!(boss.update(0.001, target, BOUNDS), 1);
    assert_eq!(boss.position(), hovering);
    assert!(!boss.is_telegraphing());
    assert_eq!(boss.update(0.01, target, BOUNDS), 0);
    assert!((boss.position().y - hovering.y - BOSS_MOVE_SPEED * 0.01).abs() < 0.0001);
    boss.update(1.0, target, BOUNDS);
    assert_eq!(boss.position(), target);
}

#[test]
fn distant_slam_hits_its_target_x_even_while_still_gliding() {
    let target = Vec2::new(960.0, 436.0);
    let mut boss = warden(Vec2::new(0.0, 436.0));
    boss.start_attack(SpecialAttack::Slam, target, BOUNDS);
    assert_eq!(boss.update(SLAM_TELEGRAPH_SECONDS, target, BOUNDS), 1);
    assert!(boss.position().x < target.x);
    assert_eq!(boss.update(1.0, target, BOUNDS), 0);
    assert_eq!(boss.position(), target);
}

#[test]
fn slam_telegraphs_for_half_a_second_and_lands_above_player() {
    let mut boss = warden(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Slam, Vec2::new(240.0, 436.0), BOUNDS);
    boss.update(SLAM_TELEGRAPH_SECONDS - 0.001, Vec2::ZERO, BOUNDS);
    assert!(boss.is_telegraphing());
    boss.update(0.001, Vec2::new(240.0, 436.0), BOUNDS);
    assert_eq!(boss.position().x, 240.0);
    assert!(!boss.is_telegraphing());
}

#[test]
fn slam_damages_only_a_player_under_the_landing_position() {
    let mut boss = warden(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Slam, Vec2::new(240.0, 436.0), BOUNDS);
    assert_eq!(boss.update(0.5, Vec2::new(240.0, 436.0), BOUNDS), 1);
    boss.start_attack(SpecialAttack::Slam, Vec2::new(240.0, 436.0), BOUNDS);
    assert_eq!(boss.update(0.5, Vec2::new(400.0, 436.0), BOUNDS), 0);
}

#[test]
fn summon_waits_one_second_and_spawns_two_patrollers() {
    let mut boss = warden(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Summon, Vec2::ZERO, BOUNDS);
    boss.update(SUMMON_WINDUP_SECONDS - 0.001, Vec2::ZERO, BOUNDS);
    assert!(boss.summoned_enemies().is_empty());
    boss.update(0.001, Vec2::ZERO, BOUNDS);
    assert_eq!(boss.summoned_enemies().len(), 2);
}

#[test]
fn rings_fuse_for_one_second_and_damage_inside_radius() {
    let mut boss = warden(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Rings, Vec2::ZERO, BOUNDS);
    let ring_position = boss.rings()[0].position();
    boss.update(0.999, ring_position, BOUNDS);
    assert!(!boss.rings()[0].exploded());
    assert_eq!(boss.update(0.001, ring_position, BOUNDS), 1);
    assert!(boss.rings().iter().all(|ring| ring.exploded()));
    assert!(ring_position.distance(boss.rings()[0].position()) <= RING_RADIUS);
}

#[test]
fn death_effect_precedes_twenty_coins_inside_the_arena() {
    let mut boss = warden(Vec2::new(700.0, 436.0));
    boss.damage_boss(boss.health());
    assert!(boss.is_death_effect_active());
    assert!(boss.coins().is_empty());
    boss.update(DEATH_EFFECT_SECONDS, Vec2::ZERO, BOUNDS);
    assert_eq!(boss.coins().len(), 20);
    assert!(boss.coins().iter().all(|coin| {
        coin.position.x >= BOUNDS.left
            && coin.position.x <= BOUNDS.right
            && coin.position.y >= 80.0
            && coin.position.y <= BOUNDS.floor_y
    }));
}

fn warden(position: Vec2) -> BossSpecials {
    hollow_steps::boss_definition::BossDefinitions::load(&support::fixture_assets())
        .unwrap()
        .spawn("Ash Warden", position)
        .unwrap()
}
