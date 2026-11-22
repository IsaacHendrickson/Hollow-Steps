//! `gather_scaled`: enemy and boss damage at a tier, hazards untouched, tier 0 as before.
use std::sync::Arc;

use macroquad::math::Vec2;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::boss_definition::BossDefinition;
use crate::boss_special::BossSpecials;
use crate::enemy::EnemyDefinition;
use crate::hazard::{Hazard, HazardPlacement};
use crate::player::WorldBounds;
use crate::room::test_rooms;

const SIZE: Vec2 = Vec2::new(32.0, 48.0);
const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

const STAND: &str = r#"(initial_state: "wait", states: [
    (name: "wait", actions: [wait(seconds: 100.0)], transitions: [])])"#;
const BLAST: &str = r#"(initial_state: "fire", states: [
    (name: "fire", actions: [area_blast(radius: 2000.0, damage: 10, knockback: (100.0, -100.0), active_secs: 50.0, telegraph_secs: 0.0)], transitions: [])])"#;
const LUNGE: &str = r#"(initial_state: "hit", states: [
    (name: "hit", actions: [lunge(speed: 0.0, duration_secs: 50.0,
        attack: (shape: Rect, offset: (0.0, 0.0), size: (2000.0, 2000.0), damage: 10, knockback: (30.0, -40.0), active_secs: 50.0),
        recovery_secs: 0.0, telegraph_secs: 0.0)], transitions: [])])"#;

/// An enemy of 36 by 36 standing at x 300, running `behavior`.
fn enemy(behavior: &str, contact_damage: u16) -> crate::enemy::Enemy {
    enemy_at(300.0, behavior, contact_damage)
}

fn enemy_at(x: f32, behavior: &str, contact_damage: u16) -> crate::enemy::Enemy {
    let mut enemy = crate::enemy::Enemy::from_definition(
        &EnemyDefinition {
            name: "dummy".into(),
            health: 30,
            size: (36.0, 36.0),
            visual_size: None,
            contact_damage,
            behavior: "dummy".into(),
            body: None,
            death_secs: 0.0,
            movement: Default::default(),
        },
        Arc::new(BehaviorDefinition::from_ron(behavior).unwrap()),
        x,
        x,
        BOUNDS,
    )
    .unwrap();
    enemy.update(Vec2::new(x, 436.0), 0.1, BOUNDS);
    enemy
}

fn boss() -> BossSpecials {
    let mut definition = BossDefinition::from_ron(include_str!(
        "../../../tests/fixtures/bosses/ground_pound.ron"
    ))
    .unwrap();
    definition.pause_seconds = 1_000.0;
    BossSpecials::from_definition(Arc::new(definition), Vec2::new(560.0, 436.0)).unwrap()
}

fn spikes() -> Hazard {
    Hazard::from(&HazardPlacement::standard(400.0, 444.0))
}

fn scale(enemy_damage: f64, boss_damage: f64) -> TierScale {
    TierScale {
        enemy_damage,
        boss_damage,
        ..TierScale::NEUTRAL
    }
}

fn gather(room: &Room, at: Vec2, boss_action: u32, scale: &TierScale) -> DamageResolver {
    DamageResolver::gather_scaled(room, at, Vec2::ZERO, SIZE, boss_action, scale)
}

#[test]
fn a_neutral_scale_gathers_exactly_what_gather_does() {
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(BLAST, 10), enemy(LUNGE, 10)];
    room.boss_special = Some(boss());
    room.hazards = vec![spikes()];
    for at in [
        Vec2::new(300.0, 436.0),
        Vec2::new(440.0, 436.0),
        Vec2::new(560.0, 436.0),
        Vec2::new(-500.0, -500.0),
    ] {
        let plain = DamageResolver::gather(&room, at, Vec2::ZERO, SIZE, 7);
        let neutral = gather(&room, at, 7, &TierScale::NEUTRAL);
        assert_eq!(neutral, plain, "at {at:?}");
    }
}

#[test]
fn enemy_contact_damage_is_scaled() {
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(STAND, 10)];
    let at = Vec2::new(300.0, 436.0);
    assert_eq!(gather(&room, at, 0, &TierScale::NEUTRAL).enemy_contact, 10);
    let scaled = gather(&room, at, 0, &scale(1.6, 1.0));
    assert_eq!(scaled.enemy_contact, 16);
    assert_eq!(scaled.resolve().damage, 16);
    // Out of reach there is still nothing to scale.
    let apart = gather(&room, Vec2::new(-500.0, -500.0), 0, &scale(1.6, 1.0));
    assert_eq!(apart.enemy_contact, 0);
    assert_eq!(apart.resolve().damage, 0);
}

#[test]
fn an_enemy_blast_is_scaled() {
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(BLAST, 0)];
    let at = Vec2::new(300.0, 436.0);
    assert_eq!(gather(&room, at, 0, &TierScale::NEUTRAL).enemy_action, 10);
    assert_eq!(gather(&room, at, 0, &scale(1.6, 1.0)).enemy_action, 16);
}

#[test]
fn an_enemy_attack_is_scaled_and_keeps_its_knockback() {
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(LUNGE, 0)];
    let at = Vec2::new(300.0, 436.0);
    let plain = gather(&room, at, 0, &TierScale::NEUTRAL);
    assert_eq!(plain.attacks.len(), 1);
    let scaled = gather(&room, at, 0, &scale(1.6, 1.0));
    assert_eq!(scaled.attacks.len(), 1);
    assert_eq!(scaled.attacks[0].damage, 16);
    assert_eq!(scaled.attacks[0].knockback, plain.attacks[0].knockback);
    assert_eq!(
        scaled.attacks[0].knockback_secs,
        plain.attacks[0].knockback_secs
    );
    assert_eq!(scaled.resolve().damage, 16);
    assert_eq!(scaled.resolve().knockback, plain.resolve().knockback);
}

#[test]
fn boss_contact_and_boss_actions_use_the_boss_multiplier_only() {
    let [_, _, mut room] = test_rooms();
    room.boss_special = Some(boss());
    let on_boss = Vec2::new(560.0, 436.0);
    // Contact is 20 and the action damage handed in is 7.
    let enemy_only = gather(&room, on_boss, 7, &scale(3.0, 1.0));
    assert_eq!((enemy_only.boss_contact, enemy_only.boss_action), (20, 7));
    let boss_only = gather(&room, on_boss, 7, &scale(1.0, 1.5));
    assert_eq!((boss_only.boss_contact, boss_only.boss_action), (30, 11));
    // Out of reach there is no contact; the action damage still counts.
    let apart = gather(&room, Vec2::new(100.0, 436.0), 7, &scale(1.0, 1.5));
    assert_eq!((apart.boss_contact, apart.boss_action), (0, 11));
    // A defeated boss deals nothing, scaled or not.
    room.boss_special.as_mut().unwrap().damage_boss(1_000);
    assert_eq!(gather(&room, on_boss, 0, &scale(1.0, 1.5)).boss_contact, 0);
}

#[test]
fn enemy_damage_does_not_touch_the_boss_and_boss_damage_does_not_touch_enemies() {
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(STAND, 10)];
    room.boss_special = Some(boss());
    let at = Vec2::new(300.0, 436.0);
    assert_eq!(gather(&room, at, 0, &scale(1.0, 5.0)).enemy_contact, 10);
    let at_boss = Vec2::new(560.0, 436.0);
    assert_eq!(gather(&room, at_boss, 0, &scale(5.0, 1.0)).boss_contact, 20);
}

#[test]
fn a_hazard_hurts_the_same_at_every_tier() {
    let [mut room, _, _] = test_rooms();
    room.hazards = vec![spikes()];
    let on_spikes = Vec2::new(440.0, 436.0);
    let plain = DamageResolver::gather(&room, on_spikes, Vec2::ZERO, SIZE, 0);
    let steep = scale(5.0, 5.0);
    let scaled = gather(&room, on_spikes, 0, &steep);
    assert_eq!(scaled, plain);
    assert_eq!(scaled.resolve().damage, i32::from(spikes().damage));
    assert_eq!(scaled.resolve().knockback, plain.resolve().knockback);
}

#[test]
fn a_hazard_next_to_scaled_enemies_keeps_its_own_damage() {
    let [mut room, _, _] = test_rooms();
    room.hazards = vec![spikes()];
    room.enemies = vec![enemy(LUNGE, 0)];
    let on_spikes = Vec2::new(440.0, 436.0);
    let scaled = gather(&room, on_spikes, 0, &scale(2.0, 1.0));
    // Hazards are listed first and are not multiplied; the enemy's attack after them is.
    assert_eq!(scaled.attacks.len(), 2);
    assert_eq!(scaled.attacks[0].damage, i32::from(spikes().damage));
    assert_eq!(scaled.attacks[1].damage, 20);
}

#[test]
fn a_scaled_enemy_can_outweigh_a_hazard_that_tied_with_it() {
    // Spikes against an enemy touch of the same damage: at tier 0 they tie and the spike's
    // knockback is used; with enemy damage scaled up the touch is the strongest source, so the
    // weaker spike gives way (`resolve`) and the hit has no knockback.
    let [mut room, _, _] = test_rooms();
    room.hazards = vec![spikes()];
    room.enemies = vec![enemy_at(440.0, STAND, spikes().damage)];
    let both = Vec2::new(440.0, 436.0);
    let plain = gather(&room, both, 0, &TierScale::NEUTRAL).resolve();
    assert_eq!(plain.damage, i32::from(spikes().damage));
    assert_eq!(plain.knockback, Vec2::new(0.0, -spikes().knockback_speed));
    let scaled = gather(&room, both, 0, &scale(2.0, 1.0)).resolve();
    assert_eq!(scaled.damage, i32::from(spikes().damage) * 2);
    assert_eq!(scaled.knockback, Vec2::ZERO);
}

#[test]
fn damage_that_was_zero_stays_zero_and_huge_damage_saturates() {
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(STAND, 0)];
    let at = Vec2::new(300.0, 436.0);
    assert_eq!(gather(&room, at, 0, &scale(5.0, 5.0)).resolve().damage, 0);
    let [mut room, _, _] = test_rooms();
    room.enemies = vec![enemy(STAND, u16::MAX)];
    assert_eq!(
        gather(&room, at, u32::MAX, &scale(100.0, 100.0)).enemy_contact,
        u16::MAX
    );
    assert_eq!(
        gather(&room, at, u32::MAX, &scale(100.0, 100.0)).boss_action,
        u32::MAX
    );
}

#[test]
fn a_boss_attack_is_scaled_by_the_boss_multiplier_and_keeps_its_knockback() {
    let definition = BossDefinition::from_ron(
        r#"(name: "Striker", health: 100, size: (48.0, 48.0), contact_damage: 0,
            selection: cycle, pause_seconds: 1000.0,
            attacks: [(name: "Hit", actions: [
                lunge(speed: 0.0, duration_secs: 50.0,
                    attack: (shape: Rect, offset: (0.0, 0.0), size: (2000.0, 2000.0), damage: 10,
                        knockback: (30.0, -40.0), active_secs: 50.0),
                    recovery_secs: 0.0, telegraph_secs: 0.0),
            ])])"#,
    )
    .unwrap();
    let mut boss =
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(560.0, 436.0)).unwrap();
    let player = Vec2::new(400.0, 436.0);
    boss.start_attack("Hit", player, BOUNDS);
    boss.update(0.1, player, BOUNDS);
    let [_, _, mut room] = test_rooms();
    room.boss_special = Some(boss);
    let plain = gather(&room, player, 0, &TierScale::NEUTRAL);
    assert_eq!(plain.attacks.len(), 1, "the boss's attack is landing");
    let scaled = gather(&room, player, 0, &scale(1.0, 1.5));
    assert_eq!(scaled.attacks[0].damage, 15);
    assert_eq!(scaled.attacks[0].knockback, plain.attacks[0].knockback);
    assert_eq!(scaled.resolve().damage, 15);
    let enemy_only = gather(&room, player, 0, &scale(5.0, 1.0));
    assert_eq!(enemy_only.attacks[0].damage, 10);
}
