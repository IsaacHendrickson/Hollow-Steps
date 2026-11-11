//! Health and damage are ten times the old whole-hit values: hits-to-kill and hits-to-die hold,
//! and values past the old `u8` limit neither overflow nor wrap.
use std::sync::Arc;

use hollow_steps::behavior::BehaviorDefinition;
use hollow_steps::combat::SWORD_DAMAGE;
use hollow_steps::enemy::{Enemy, EnemyDefinition, ENEMY_MAX_HEALTH};
use hollow_steps::health::{Health, HIT_DAMAGE, MAX_HEALTH};
use hollow_steps::hud_layout::masks_for;
use hollow_steps::player::WorldBounds;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const STAND: &str = r#"(initial_state: "wait", states: [
    (name: "wait", actions: [wait(seconds: 1.0)], transitions: [])])"#;

fn enemy(health: u16, contact_damage: u16) -> Enemy {
    Enemy::from_definition(
        &EnemyDefinition {
            name: "dummy".into(),
            health,
            size: (36.0, 36.0),
            visual_size: None,
            contact_damage,
            behavior: "wait".into(),
            body: None,
            death_secs: 0.0,
        },
        Arc::new(BehaviorDefinition::from_ron(STAND).unwrap()),
        300.0,
        300.0,
        BOUNDS,
    )
    .unwrap()
}

#[test]
fn the_standard_numbers_keep_the_old_hit_counts() {
    assert_eq!(HIT_DAMAGE, 10);
    assert_eq!(SWORD_DAMAGE, HIT_DAMAGE);
    assert_eq!(MAX_HEALTH, 5 * HIT_DAMAGE);
    assert_eq!(ENEMY_MAX_HEALTH, 3 * SWORD_DAMAGE);
}

#[test]
fn a_default_enemy_dies_to_exactly_three_sword_hits() {
    let mut enemy = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS);
    for (swing, left) in [(1, 20), (2, 10)] {
        assert!(enemy.hit(swing, 1.0));
        assert_eq!(enemy.health(), left);
        assert!(!enemy.is_dead());
    }
    assert!(enemy.hit(3, 1.0));
    assert!(enemy.is_dead());
    assert!(!enemy.hit(4, 1.0), "a defeated enemy cannot be hit again");
}

#[test]
fn the_player_dies_to_exactly_five_standard_hits() {
    let mut health = Health::default();
    for hit in 1..=5 {
        assert_eq!(health.current(), MAX_HEALTH - (hit - 1) * HIT_DAMAGE);
        assert!(health.take_damage());
        health.update(10.0);
    }
    assert_eq!(health.current(), 0);
    assert!(!health.take_damage(), "no sixth hit lands");
}

#[test]
fn a_sword_hit_on_less_health_than_a_hit_stops_at_zero() {
    let mut enemy = enemy(4, 0);
    assert!(enemy.hit(1, 1.0));
    assert_eq!(enemy.health(), 0);
    assert!(enemy.is_dead());
}

#[test]
fn values_past_the_old_u8_limit_do_not_overflow() {
    let mut big = enemy(2550, 2550);
    assert_eq!(
        (big.health(), big.max_health(), big.contact_damage()),
        (2550, 2550, 2550)
    );
    for swing in 1..=254 {
        assert!(big.hit(swing, 1.0));
    }
    assert_eq!(big.health(), 10);
    assert!(!big.is_dead());
    assert!(big.hit(255, 1.0));
    assert!(big.is_dead());

    let mut health = Health::default();
    assert!(health.take_damage_amount(u16::MAX));
    assert_eq!(health.current(), 0);
}

#[test]
fn health_masks_count_whole_and_part_hits() {
    assert_eq!(masks_for(MAX_HEALTH), 5);
    assert_eq!(masks_for(0), 0);
    assert_eq!(masks_for(1), 1, "a part-used hit still shows a mask");
    assert_eq!(masks_for(HIT_DAMAGE), 1);
    assert_eq!(masks_for(HIT_DAMAGE + 1), 2);
    assert_eq!(masks_for(u16::MAX), u16::MAX.div_ceil(HIT_DAMAGE));
}
