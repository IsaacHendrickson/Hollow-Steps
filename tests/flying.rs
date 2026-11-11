//! Flying enemies: a hover line plus a sine bob, no gravity, no terrain, always inside the
//! room. Ground enemies (the default) must behave exactly as before.

use std::sync::Arc;

use hollow_steps::behavior::BehaviorDefinition;
use hollow_steps::combat::SWORD_DAMAGE;
use hollow_steps::definition_editor::serialize;
use hollow_steps::enemy::{Enemy, EnemyDefinition, Movement};
use hollow_steps::player::WorldBounds;
use hollow_steps::terrain::Terrain;
use macroquad::math::{Rect, Vec2};

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;
const SIZE: (f32, f32) = (32.0, 40.0);
/// Floor 460 minus half the 40 px box: where a ground enemy's center rests.
const REST_Y: f32 = 440.0;

const PATROL: &str = r#"(initial_state: "walk", states: [
    (name: "walk", actions: [patrol(speed: 80.0)], transitions: [])])"#;
const STAND: &str = r#"(initial_state: "stand", states: [
    (name: "stand", actions: [wait(seconds: 100.0)], transitions: [])])"#;
const FLEE: &str = r#"(initial_state: "flee", states: [
    (name: "flee", actions: [move_away_from_player(speed: 400.0)], transitions: [])])"#;
const CHASE: &str = r#"(initial_state: "chase", states: [
    (name: "chase", actions: [move_toward_player(speed: 400.0)], transitions: [])])"#;

fn flying(hover_height: f32, bob_amplitude: f32, bob_speed: f32) -> Movement {
    Movement::Flying {
        hover_height,
        bob_amplitude,
        bob_speed,
    }
}

fn definition(movement: Movement) -> EnemyDefinition {
    EnemyDefinition {
        name: "bird".into(),
        health: 20,
        size: SIZE,
        contact_damage: 10,
        behavior: "behavior".into(),
        body: None,
        death_secs: 0.0,
        movement,
    }
}

fn enemy_in(
    behavior: &str,
    movement: Movement,
    left: f32,
    right: f32,
    bounds: WorldBounds,
) -> Enemy {
    Enemy::from_definition(
        &definition(movement),
        Arc::new(BehaviorDefinition::from_ron(behavior).unwrap()),
        left,
        right,
        bounds,
    )
    .unwrap()
}

fn enemy(behavior: &str, movement: Movement) -> Enemy {
    enemy_in(behavior, movement, 300.0, 600.0, BOUNDS)
}

#[test]
fn a_ground_enemy_is_unchanged_by_the_new_field() {
    // Movement::Ground spelled out and the default are the same enemy, tick for tick.
    let mut explicit = enemy(PATROL, Movement::Ground);
    let mut default = enemy(PATROL, Movement::default());
    assert_eq!(explicit.position().y, REST_Y, "starts on the floor");
    for _ in 0..600 {
        explicit.update(Vec2::new(0.0, REST_Y), DT, BOUNDS);
        default.update(Vec2::new(0.0, REST_Y), DT, BOUNDS);
        assert_eq!(explicit.position(), default.position());
        assert_eq!(
            explicit.position().y,
            REST_Y,
            "gravity keeps it on the floor"
        );
    }
    assert!(explicit.position().x > 300.0, "it patrolled");
    assert!(!explicit.movement().is_flying());
}

#[test]
fn a_ground_enemy_is_still_stopped_by_terrain() {
    let wall = Terrain::new(vec![Rect::new(400.0, 300.0, 20.0, 160.0)]);
    let mut walker = enemy(PATROL, Movement::Ground);
    for _ in 0..1200 {
        walker.update_with_terrain(Vec2::new(0.0, REST_Y), Vec2::ZERO, DT, BOUNDS, &wall);
    }
    assert!(
        walker.position().x <= 400.0 - 16.0 + 0.01,
        "{:?}",
        walker.position()
    );
}

#[test]
fn a_flying_enemy_starts_on_its_hover_line() {
    let bird = enemy(STAND, flying(60.0, 8.0, 0.5));
    assert_eq!(bird.position().x, 300.0);
    assert_eq!(bird.position().y, REST_Y - 60.0);
    assert!(bird.movement().is_flying());
}

#[test]
fn a_flying_enemy_holds_its_hover_height_plus_the_bob() {
    let movement = flying(60.0, 8.0, 0.5);
    let mut bird = enemy(STAND, movement);
    let mut seconds = 0.0_f64;
    let (mut lowest, mut highest) = (f32::MAX, f32::MIN);
    for _ in 0..480 {
        bird.update(Vec2::new(900.0, REST_Y), DT, BOUNDS);
        seconds += f64::from(DT);
        let expected = movement.altitude(Vec2::new(SIZE.0, SIZE.1), BOUNDS, seconds);
        let y = bird.position().y;
        assert!(
            (y - expected.unwrap()).abs() < 1e-3,
            "t={seconds}: {y} vs {expected:?}"
        );
        lowest = lowest.min(y);
        highest = highest.max(y);
    }
    // Four seconds at 0.5 cycles/s is two full bobs of +-8 px around 380.
    assert!(
        highest <= 388.001 && lowest >= 371.999,
        "{lowest}..{highest}"
    );
    assert!(
        highest - lowest > 15.9,
        "the bob is really there: {lowest}..{highest}"
    );
}

#[test]
fn a_flying_enemy_ignores_gravity_while_it_patrols_sideways() {
    let mut bird = enemy(PATROL, flying(80.0, 0.0, 1.0));
    for _ in 0..1200 {
        bird.update(Vec2::new(0.0, REST_Y), DT, BOUNDS);
        assert_eq!(bird.position().y, REST_Y - 80.0, "no bob, no fall");
    }
    assert!(
        bird.position().x > 300.0,
        "it still patrolled: {:?}",
        bird.position()
    );
}

#[test]
fn a_flying_enemy_does_not_collide_with_terrain() {
    let wall = Terrain::new(vec![Rect::new(400.0, 300.0, 20.0, 160.0)]);
    let mut bird = enemy(PATROL, flying(20.0, 0.0, 1.0));
    for _ in 0..1200 {
        bird.update_with_terrain(Vec2::new(0.0, REST_Y), Vec2::ZERO, DT, BOUNDS, &wall);
    }
    assert!(
        bird.position().x > 420.0,
        "it flew straight through the wall: {:?}",
        bird.position()
    );
}

#[test]
fn a_flying_enemy_never_leaves_the_room() {
    let size = Vec2::new(SIZE.0, SIZE.1);
    // Flee to a wall, chase a player far outside the room, hover far above the ceiling.
    let cases = [
        (FLEE, flying(30.0, 10.0, 2.0), Vec2::new(500.0, 400.0)),
        (CHASE, flying(30.0, 10.0, 2.0), Vec2::new(-5000.0, 300.0)),
        (CHASE, flying(30.0, 10.0, 2.0), Vec2::new(5000.0, 300.0)),
        (STAND, flying(900.0, 150.0, 3.0), Vec2::new(500.0, 400.0)),
        (STAND, flying(0.0, 150.0, 3.0), Vec2::new(500.0, 400.0)),
    ];
    for (behavior, movement, player) in cases {
        let mut bird = enemy(behavior, movement);
        for tick in 0..2400 {
            bird.update(player, DT, BOUNDS);
            let p = bird.position();
            assert!(
                p.x >= BOUNDS.left + size.x / 2.0 - 0.01
                    && p.x <= BOUNDS.right - size.x / 2.0 + 0.01
                    && p.y >= BOUNDS.top + size.y / 2.0 - 0.01
                    && p.y <= BOUNDS.floor_y - size.y / 2.0 + 0.01,
                "{movement:?} tick {tick}: {p:?}"
            );
        }
    }
}

#[test]
fn a_hit_knocks_a_flier_sideways_without_dropping_it() {
    let mut bird = enemy(STAND, flying(60.0, 0.0, 1.0));
    bird.update(Vec2::new(900.0, REST_Y), DT, BOUNDS);
    assert!(bird.hit(1, -1.0));
    assert_eq!(bird.health(), 20 - SWORD_DAMAGE);
    let x = bird.position().x;
    for _ in 0..30 {
        bird.update(Vec2::new(900.0, REST_Y), DT, BOUNDS);
        assert_eq!(
            bird.position().y,
            REST_Y - 60.0,
            "knockback keeps the altitude"
        );
    }
    assert!(bird.position().x < x, "pushed in the hit direction");
}

#[test]
fn zero_nan_and_huge_deltas_are_safe_and_the_bob_clock_only_runs_forward() {
    let mut bird = enemy(STAND, flying(60.0, 8.0, 0.5));
    let start = bird.position();
    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        bird.update(Vec2::new(900.0, REST_Y), bad, BOUNDS);
        assert_eq!(bird.position(), start, "delta {bad}");
    }
    // A huge step is a long wait, not a crash or an escape from the room.
    bird.update(Vec2::new(900.0, REST_Y), 1.0e6, BOUNDS);
    let p = bird.position();
    assert!(p.is_finite() && p.y >= 20.0 && p.y <= REST_Y, "{p:?}");
}

#[test]
fn the_same_inputs_give_the_same_flight() {
    let mut a = enemy(PATROL, flying(60.0, 8.0, 0.7));
    let mut b = enemy(PATROL, flying(60.0, 8.0, 0.7));
    for _ in 0..1000 {
        a.update(Vec2::new(0.0, REST_Y), DT, BOUNDS);
        b.update(Vec2::new(0.0, REST_Y), DT, BOUNDS);
        assert_eq!(a.position(), b.position());
    }
}

#[test]
fn validation_rejects_a_bad_flying_definition_naming_the_entity_and_field() {
    for (movement, field) in [
        (flying(-1.0, 4.0, 1.0), "hover_height"),
        (flying(10.0, -4.0, 1.0), "bob_amplitude"),
        (flying(10.0, 4.0, 0.0), "bob_speed"),
        (flying(f32::NAN, 4.0, 1.0), "hover_height"),
    ] {
        let error = definition(movement).validate().unwrap_err();
        assert!(error.contains("bird") && error.contains(field), "{error}");
        let built = Enemy::from_definition(
            &definition(movement),
            Arc::new(BehaviorDefinition::from_ron(STAND).unwrap()),
            300.0,
            600.0,
            BOUNDS,
        );
        assert!(built.is_err(), "{movement:?} must not spawn");
    }
    assert!(definition(flying(0.0, 0.0, 0.1)).validate().is_ok());
}

#[test]
fn an_old_entity_file_without_movement_loads_as_ground_and_saves_without_it() {
    let old = r#"(name: "walker", health: 3, size: (36.0, 36.0), contact_damage: 1, behavior: "patrol", death_secs: 0.4)"#;
    let parsed: EnemyDefinition = ron::from_str(old).unwrap();
    assert_eq!(parsed.movement, Movement::Ground);
    assert!(parsed.validate().is_ok());
    let saved = ron::to_string(&parsed).unwrap();
    assert!(!saved.contains("movement"), "{saved}");

    let bird = r#"(name: "bird", health: 3, size: (32.0, 40.0), contact_damage: 1, behavior: "x",
        movement: Flying(hover_height: 56.0, bob_amplitude: 5.0, bob_speed: 0.7))"#;
    let parsed: EnemyDefinition = ron::from_str(bird).unwrap();
    assert_eq!(parsed.movement, flying(56.0, 5.0, 0.7));
    let again: EnemyDefinition = ron::from_str(&ron::to_string(&parsed).unwrap()).unwrap();
    assert_eq!(again, parsed);

    let unknown = r#"(name: "bird", health: 3, size: (32.0, 40.0), contact_damage: 1, behavior: "x",
        movement: Flying(hover_height: 56.0, bob_amplitude: 5.0, bob_speed: 0.7, wings: 2))"#;
    assert!(ron::from_str::<EnemyDefinition>(unknown).is_err());
}

#[test]
fn movement_survives_the_editors_save_and_reload_and_ground_saves_clean() {
    let flier = definition(flying(56.0, 5.0, 0.7));
    let text = serialize(&flier).unwrap();
    assert!(text.contains("movement: Flying("), "{text}");
    let reloaded: EnemyDefinition = ron::from_str(&text).unwrap();
    assert_eq!(reloaded, flier);

    let walker = definition(Movement::Ground);
    let text = serialize(&walker).unwrap();
    assert!(
        !text.contains("movement"),
        "ground files stay as they were: {text}"
    );
    assert_eq!(ron::from_str::<EnemyDefinition>(&text).unwrap(), walker);
}
