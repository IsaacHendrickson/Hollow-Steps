//! The bird's behavior, flying: it patrols in the air (turning at both ends of its range, bobbing
//! on its hover line), shoots from the hover height instead of the floor, and goes back to
//! patrolling when the player leaves. Built from inline definitions that mirror the shipped
//! bird's shape, so editing the shipped RON never breaks these tests.

use std::sync::Arc;

use hollow_steps::behavior::BehaviorDefinition;
use hollow_steps::enemy::{Enemy, EnemyDefinition, Movement};
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;
const SIZE: (f32, f32) = (32.0, 40.0);
const HOVER: f32 = 56.0;
const BOB: f32 = 6.0;
/// Where the hover line puts the bird's center: floor, minus half its box, minus the hover height.
const HOVER_Y: f32 = 460.0 - 20.0 - HOVER;
const PLAYER_ON_FLOOR_Y: f32 = 436.0;

const PATROL: &str = r#"(initial_state: "patrol", states: [
    (name: "patrol", actions: [patrol(speed: 50.0)], transitions: [])])"#;

/// The shipped Shooter, minus the poses: patrol until the player is within 480 px, then the
/// original distance / retreat / fire loop, and back to patrol once the player is past 560 px.
const SHOOTER: &str = r#"(initial_state: "patrol", states: [
    (name: "patrol", actions: [patrol(speed: 50.0)], transitions: [
        (target: "distance", condition: player_distance_less_than(480.0))]),
    (name: "distance", actions: [], transitions: [
        (target: "patrol", condition: player_distance_greater_than(560.0)),
        (target: "retreat", condition: player_distance_less_than(200.0)),
        (target: "fire", condition: player_distance_less_than(200.0), invert: true)]),
    (name: "retreat", actions: [move_away_from_player(speed: 120.0)], transitions: [
        (target: "fire", condition: player_distance_less_than(200.0), invert: true),
        (target: "fire", condition: action_finished)]),
    (name: "fire", actions: [
        telegraph(seconds: 0.4),
        shoot_projectile(speed: 280.0, damage: 10, lifetime_secs: 3.0, size: (12.0, 12.0),
            aim: AtPlayer, count: 3, spread_degrees: 15.0, telegraph_secs: 0.0,
            telegraph_pose: None, active_pose: None, recovery_pose: None,
            pose_blend_ratio: 0.8, strike_stiffness_multiplier: 3.0),
        wait(seconds: 1.2)], transitions: [
        (target: "retreat", condition: player_distance_less_than(200.0)),
        (target: "distance", condition: action_finished)])])"#;

fn bird(behavior: &str, left: f32, right: f32) -> Enemy {
    let definition = EnemyDefinition {
        name: "scittish".into(),
        // More than one sword hit, so a knocked-back bird is still alive and moving.
        health: 30,
        size: SIZE,
        contact_damage: 10,
        behavior: "behavior".into(),
        body: None,
        death_secs: 0.0,
        movement: Movement::Flying {
            hover_height: HOVER,
            bob_amplitude: BOB,
            bob_speed: 0.6,
            flap: None,
        },
    };
    Enemy::from_definition(
        &definition,
        Arc::new(BehaviorDefinition::from_ron(behavior).unwrap()),
        left,
        right,
        BOUNDS,
    )
    .unwrap()
}

fn in_the_air(enemy: &Enemy) -> bool {
    (enemy.position().y - HOVER_Y).abs() <= BOB + 1e-3
}

#[test]
fn a_flying_patrol_turns_at_both_ends_of_its_range_and_stays_on_its_hover_line() {
    let mut bird = bird(PATROL, 300.0, 400.0);
    let player = Vec2::new(900.0, PLAYER_ON_FLOOR_Y);
    let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
    let (mut min_y, mut max_y) = (f32::MAX, f32::MIN);
    let mut turns_at = Vec::new();
    let mut facing_left = bird.facing_left();
    // 20 s at 50 px/s over a 100 px range: ten legs.
    for _ in 0..2400 {
        bird.update(player, DT, BOUNDS);
        let position = bird.position();
        assert!(
            (300.0 - 1e-3..=400.0 + 1e-3).contains(&position.x),
            "left its range: x = {}",
            position.x
        );
        assert!(in_the_air(&bird), "left its hover line: y = {}", position.y);
        (min_x, max_x) = (min_x.min(position.x), max_x.max(position.x));
        (min_y, max_y) = (min_y.min(position.y), max_y.max(position.y));
        if bird.facing_left() != facing_left {
            facing_left = bird.facing_left();
            turns_at.push(position.x);
        }
    }
    assert!(min_x < 301.0 && max_x > 399.0, "uses the whole range");
    assert!(turns_at.len() >= 8, "turned {} times", turns_at.len());
    assert!(
        turns_at.iter().any(|x| (x - 300.0).abs() < 2.0),
        "turns at the left end"
    );
    assert!(
        turns_at.iter().any(|x| (x - 400.0).abs() < 2.0),
        "turns at the right end"
    );
    assert!(max_y - min_y > 10.0, "it bobs: {min_y}..{max_y}");
}

#[test]
fn a_flying_patrol_returns_to_its_range_after_a_hit_knocks_it_out() {
    let mut bird = bird(PATROL, 300.0, 400.0);
    let player = Vec2::new(900.0, PLAYER_ON_FLOOR_Y);
    // It starts at the left end; a hit from the right pushes it further left, out of range.
    assert!(bird.hit(1, -1.0));
    for _ in 0..24 {
        bird.update(player, DT, BOUNDS);
        assert!(in_the_air(&bird), "knockback keeps the altitude");
    }
    assert!(
        bird.position().x < 300.0,
        "pushed out: {}",
        bird.position().x
    );
    for _ in 0..1200 {
        bird.update(player, DT, BOUNDS);
        assert!(in_the_air(&bird));
    }
    let x = bird.position().x;
    assert!((300.0..=400.0).contains(&x), "back on patrol: {x}");
}

#[test]
fn the_bird_patrols_until_the_player_is_near_shoots_from_the_air_and_resumes_patrolling() {
    let mut bird = bird(SHOOTER, 700.0, 770.0);
    let far = Vec2::new(100.0, PLAYER_ON_FLOOR_Y);
    let near = Vec2::new(400.0, PLAYER_ON_FLOOR_Y);

    // Far away: it just patrols, in the air, and never fires.
    let mut travelled: f32 = 0.0;
    for _ in 0..600 {
        let before = bird.position().x;
        bird.update(far, DT, BOUNDS);
        travelled += (bird.position().x - before).abs();
        assert_eq!(bird.behavior().state_name(), "patrol");
        assert!(bird.behavior().projectiles().is_empty());
        assert!(in_the_air(&bird));
    }
    assert!(travelled > 100.0, "it moved along its range: {travelled}");
    assert!((700.0..=770.0).contains(&bird.position().x));

    // The player comes within range: it stops patrolling and fires a three-shot fan.
    let mut volley = None;
    for _ in 0..600 {
        bird.update(near, DT, BOUNDS);
        assert!(in_the_air(&bird));
        if !bird.behavior().projectiles().is_empty() {
            volley = Some((bird.position(), bird.behavior().projectiles().to_vec()));
            break;
        }
    }
    let (origin, shots) = volley.expect("it fires once the player is near");
    assert_eq!(shots.len(), 3);
    for shot in &shots {
        assert!(
            (shot.position.y - origin.y).abs() < 1.0,
            "shots start at the bird's hover height ({}), not the floor: {}",
            origin.y,
            shot.position.y
        );
        assert!(shot.position.y < 400.0, "well above the floor");
        assert!(shot.velocity.x < 0.0, "aimed left, at the player");
    }
    assert!(
        shots[1].velocity.y > 0.0,
        "the middle shot aims down at a player standing on the floor"
    );

    // The player leaves: once the volley is over it goes back to patrolling, and ends up
    // inside its range again even though retreating took it past the end.
    for _ in 0..1800 {
        bird.update(far, DT, BOUNDS);
        assert!(in_the_air(&bird));
    }
    assert_eq!(bird.behavior().state_name(), "patrol");
    let x = bird.position().x;
    assert!((700.0..=770.0).contains(&x), "back inside its range: {x}");
}

#[test]
fn retreating_from_a_close_player_moves_sideways_only() {
    let mut bird = bird(SHOOTER, 700.0, 770.0);
    // Right underneath it, on the floor: well inside the 200 px retreat distance.
    let player = Vec2::new(730.0, PLAYER_ON_FLOOR_Y);
    let mut moved = 0.0_f32;
    for _ in 0..240 {
        let before = bird.position().x;
        bird.update(player, DT, BOUNDS);
        moved += (bird.position().x - before).abs();
        assert!(in_the_air(&bird), "retreat never drops it to the floor");
    }
    assert!(moved > 20.0, "it backed away: {moved}");
}
