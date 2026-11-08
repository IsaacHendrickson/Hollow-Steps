use hollow_steps::boss_special::{
    BossSpecials, SpecialAttack, DEATH_EFFECT_SECONDS, RING_RADIUS,
    SLAM_TELEGRAPH_SECONDS, SUMMON_WINDUP_SECONDS,
};
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

#[test]
fn slam_telegraphs_for_half_a_second_and_lands_above_player() {
    let mut boss = BossSpecials::new(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Slam, Vec2::new(240.0, 436.0), BOUNDS);
    boss.update(SLAM_TELEGRAPH_SECONDS - 0.001, Vec2::ZERO, BOUNDS);
    assert!(boss.is_telegraphing());
    boss.update(0.001, Vec2::new(240.0, 436.0), BOUNDS);
    assert_eq!(boss.position().x, 240.0);
    assert!(!boss.is_telegraphing());
}

#[test]
fn slam_damages_only_a_player_under_the_landing_position() {
    let mut boss = BossSpecials::new(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Slam, Vec2::new(240.0, 436.0), BOUNDS);
    assert_eq!(boss.update(0.5, Vec2::new(240.0, 436.0), BOUNDS), 1);
    boss.start_attack(SpecialAttack::Slam, Vec2::new(240.0, 436.0), BOUNDS);
    assert_eq!(boss.update(0.5, Vec2::new(400.0, 436.0), BOUNDS), 0);
}

#[test]
fn summon_waits_one_second_and_spawns_two_patrollers() {
    let mut boss = BossSpecials::new(Vec2::new(700.0, 436.0));
    boss.start_attack(SpecialAttack::Summon, Vec2::ZERO, BOUNDS);
    boss.update(SUMMON_WINDUP_SECONDS - 0.001, Vec2::ZERO, BOUNDS);
    assert!(boss.summoned_enemies().is_empty());
    boss.update(0.001, Vec2::ZERO, BOUNDS);
    assert_eq!(boss.summoned_enemies().len(), 2);
}

#[test]
fn rings_fuse_for_one_second_and_damage_inside_radius() {
    let mut boss = BossSpecials::new(Vec2::new(700.0, 436.0));
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
    let mut boss = BossSpecials::new(Vec2::new(700.0, 436.0));
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
