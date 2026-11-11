//! A flier's wing flap: two poses alternate, faster the faster it flies; a pose the behavior
//! plays (wind-up) and the hurt pose are never overridden; ground enemies never flap.
//! Everything is built in code: a tiny body template, four poses and a handful of behaviors.

use std::collections::BTreeMap;
use std::sync::Arc;

use hollow_steps::behavior::BehaviorDefinition;
use hollow_steps::body::{BodyTemplate, BodyTemplates, BodyVisual};
use hollow_steps::combat::SWORD_DAMAGE;
use hollow_steps::enemy::{Enemy, EnemyDefinition, EnemyDefinitions, Flap, Movement};
use hollow_steps::player::WorldBounds;
use hollow_steps::pose::{Pose, Poses};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;
const SIZE: (f32, f32) = (32.0, 40.0);
const PLAYER_FAR: Vec2 = Vec2::new(900.0, 440.0);

const STAND: &str = r#"(initial_state: "stand", states: [
    (name: "stand", actions: [wait(seconds: 100.0)], transitions: [])])"#;
/// Walks 200 px/s along its patrol range (always inside 0..960).
const PATROL: &str = r#"(initial_state: "walk", states: [
    (name: "walk", actions: [patrol(speed: 200.0)], transitions: [])])"#;
/// Holds the wind-up pose for 0.3 s, then returns to idle and waits.
const WINDUP: &str = r#"(initial_state: "s", states: [(name: "s", actions: [
    pose(pose_name: Some("windup"), blend_secs: 0.05),
    wait(seconds: 0.3),
    pose(pose_name: None, blend_secs: 0.05),
    wait(seconds: 100.0)], transitions: [])])"#;

const TEMPLATE: &str = r#"(
    pose_map: {"hand_l": "wing", "head": "head"},
    points: {"head": (rest: (0.2, -0.2)), "wing": (rest: (-0.2, -0.1))},
    parts: [
        (name: "spar", shape: Line, from: Some("head"), to: Some("wing"), size: (0.1, 0.1), rotation: 0.0, color_slot: secondary),
        (name: "head", shape: Ellipse, anchor: Some("head"), offset: Some((0.1, 0.0)), size: (0.3, 0.3), rotation: 20.0, color_slot: primary),
    ],
)"#;

fn poses() -> Poses {
    let pose = |text: &str| Arc::new(ron::from_str::<Pose>(text).unwrap());
    [
        (
            "fly_up".to_owned(),
            pose(r#"(offsets: {"hand_l": (0.0, -0.4)})"#),
        ),
        (
            "fly_down".to_owned(),
            pose(r#"(offsets: {"hand_l": (0.0, 0.4)})"#),
        ),
        (
            "windup".to_owned(),
            pose(r#"(offsets: {"hand_l": (-0.3, 0.0)})"#),
        ),
        (
            "hurt".to_owned(),
            pose(r#"(offsets: {"hand_l": (0.1, 0.1)})"#),
        ),
    ]
    .into()
}

fn templates() -> BodyTemplates {
    [(
        "flier".to_owned(),
        Arc::new(ron::from_str::<BodyTemplate>(TEMPLATE).unwrap()),
    )]
    .into()
}

fn visual() -> BodyVisual {
    let mut visual = BodyVisual::for_template("flier");
    visual.hurt.pose = Some("hurt".into());
    visual
}

fn flap() -> Flap {
    Flap {
        up_pose: "fly_up".into(),
        down_pose: "fly_down".into(),
        hover_rate: 2.0,
        rate_per_speed: 0.01,
        max_rate: 8.0,
    }
}

fn flier(flap: Option<Flap>) -> Movement {
    Movement::Flying {
        hover_height: 56.0,
        bob_amplitude: 0.0,
        bob_speed: 1.0,
        flap,
    }
}

fn entity(movement: Movement, body: Option<BodyVisual>) -> EnemyDefinition {
    EnemyDefinition {
        name: "bird".into(),
        health: 50,
        size: SIZE,
        contact_damage: 10,
        behavior: "b".into(),
        body,
        death_secs: 0.0,
        movement,
    }
}

fn catalog(entity: EnemyDefinition, behavior: &str) -> Result<EnemyDefinitions, String> {
    let behaviors: BTreeMap<String, Arc<BehaviorDefinition>> = [(
        "b".to_owned(),
        Arc::new(BehaviorDefinition::from_ron(behavior).unwrap()),
    )]
    .into();
    EnemyDefinitions::from_definitions_with_poses([entity], behaviors, templates(), poses())
}

fn spawn(movement: Movement, behavior: &str) -> Enemy {
    catalog(entity(movement, Some(visual())), behavior)
        .unwrap()
        .spawn("bird", 100.0, 800.0, BOUNDS)
        .unwrap()
}

fn pose_of(enemy: &Enemy) -> Option<String> {
    enemy.body().unwrap().pose_name.clone()
}

/// Steps for `seconds` and records every change of the body's pose, in order.
fn pose_changes(enemy: &mut Enemy, seconds: f32) -> Vec<Option<String>> {
    let mut seen = vec![pose_of(enemy)];
    for _ in 0..(seconds / DT).round() as usize {
        enemy.update(PLAYER_FAR, DT, BOUNDS);
        let pose = pose_of(enemy);
        if seen.last() != Some(&pose) {
            seen.push(pose);
        }
    }
    seen
}

#[test]
fn a_hovering_flier_alternates_its_two_poses_starting_with_the_upstroke() {
    let mut bird = spawn(flier(Some(flap())), STAND);
    let seen = pose_changes(&mut bird, 2.0);
    let up = Some("fly_up".to_owned());
    let down = Some("fly_down".to_owned());
    assert_eq!(seen[0], None, "nothing before the first tick");
    assert_eq!(seen[1], up, "each beat starts with the upstroke");
    assert_eq!(seen[2], down);
    assert_eq!(seen[3], up);
    // 2 beats per second for 2 seconds: 4 beats = 8 strokes (the last may just be starting).
    let strokes = seen.len() - 1;
    assert!((7..=9).contains(&strokes), "{strokes} strokes: {seen:?}");
    assert!(seen[1..].iter().all(|p| p == &up || p == &down));
}

#[test]
fn the_wings_beat_faster_the_faster_the_flier_moves() {
    // Strokes = pose changes after the first (the first entry is the body before any pose).
    let strokes = |movement, behavior| pose_changes(&mut spawn(movement, behavior), 2.0).len() - 1;
    let hovering = strokes(flier(Some(flap())), STAND);
    let cruising = strokes(flier(Some(flap())), PATROL);
    // Standing: 2 beats/s. Patrolling at 200 px/s: 2 + 0.01 * 200 = 4 beats/s: twice the strokes.
    assert!(
        cruising + 2 >= hovering * 2 && cruising > hovering + 5,
        "hovering {hovering} strokes, cruising {cruising}"
    );
}

#[test]
fn a_pose_the_behavior_plays_is_never_overridden_and_the_flap_resumes_after_it() {
    let mut bird = spawn(flier(Some(flap())), WINDUP);
    // The wind-up pose is applied on the first tick and held for 0.3 s.
    for tick in 0..30 {
        bird.update(PLAYER_FAR, DT, BOUNDS);
        assert_eq!(
            pose_of(&bird).as_deref(),
            Some("windup"),
            "tick {tick}: the flap must not take over"
        );
    }
    // The pose action then clears it and the wings go back to flapping.
    for _ in 0..60 {
        bird.update(PLAYER_FAR, DT, BOUNDS);
    }
    let after = pose_of(&bird);
    assert!(
        matches!(after.as_deref(), Some("fly_up" | "fly_down")),
        "flapping again, not stuck on a stale pose: {after:?}"
    );
}

#[test]
fn the_hurt_pose_and_flash_still_work_and_the_flap_comes_back_afterwards() {
    let mut bird = spawn(flier(Some(flap())), STAND);
    for _ in 0..30 {
        bird.update(PLAYER_FAR, DT, BOUNDS);
    }
    assert!(bird.hit(1, 1.0));
    assert_eq!(bird.health(), 50 - SWORD_DAMAGE);
    assert_eq!(pose_of(&bird).as_deref(), Some("hurt"));
    assert!(
        bird.body().unwrap().flash_strength() > 0.0,
        "the white flash plays"
    );
    // The flap keeps counting under the hurt pose but never replaces it...
    for _ in 0..12 {
        bird.update(PLAYER_FAR, DT, BOUNDS);
        assert_eq!(pose_of(&bird).as_deref(), Some("hurt"));
    }
    // ...and once the hurt time (0.15 s) and the knockback are over, the wings flap again.
    for _ in 0..60 {
        bird.update(PLAYER_FAR, DT, BOUNDS);
    }
    assert!(matches!(
        pose_of(&bird).as_deref(),
        Some("fly_up" | "fly_down")
    ));
    assert_eq!(bird.body().unwrap().flash_strength(), 0.0);
}

#[test]
fn without_a_flap_or_without_flying_the_body_is_left_alone() {
    let mut no_flap = spawn(flier(None), PATROL);
    assert_eq!(
        pose_changes(&mut no_flap, 1.0),
        vec![None],
        "a flier with no flap"
    );

    let mut walker = spawn(Movement::Ground, PATROL);
    assert_eq!(pose_changes(&mut walker, 1.0), vec![None], "a ground enemy");
}

#[test]
fn a_dead_flier_stops_beating_its_wings() {
    let mut bird = catalog(
        EnemyDefinition {
            death_secs: 2.0,
            ..entity(flier(Some(flap())), Some(visual()))
        },
        STAND,
    )
    .unwrap()
    .spawn("bird", 100.0, 800.0, BOUNDS)
    .unwrap();
    for id in 1..=5 {
        bird.hit(id, 1.0);
    }
    assert!(bird.is_dying());
    let frozen = pose_of(&bird);
    let seen = pose_changes(&mut bird, 1.0);
    assert_eq!(
        seen,
        vec![frozen],
        "a dying body keeps the pose of the killing blow"
    );
}

#[test]
fn the_body_mirrors_with_the_direction_of_flight() {
    let mut bird = spawn(flier(Some(flap())), PATROL);
    let mut seen = (false, false);
    for _ in 0..1200 {
        bird.update(PLAYER_FAR, DT, BOUNDS);
        if bird.facing_left() {
            seen.0 = true;
        } else {
            seen.1 = true;
        }
    }
    assert_eq!(seen, (true, true), "it turned at the ends of its patrol");

    // The head part sits on opposite sides of the body for the two facings.
    let body = bird.body().unwrap();
    let head = body
        .template
        .parts
        .iter()
        .find(|part| part.name == "head")
        .unwrap()
        .clone();
    let center = Vec2::new(500.0, 300.0);
    let size = Vec2::new(SIZE.0, SIZE.1);
    let right = body.part_transform(&head, center, size, false).unwrap();
    let left = body.part_transform(&head, center, size, true).unwrap();
    assert!(!right.mirrored && left.mirrored);
    assert!((right.center.x - center.x + (left.center.x - center.x)).abs() < 1e-3);
    assert!(right.center.x > center.x && left.center.x < center.x);
    assert_eq!(right.center.y, left.center.y);
    assert_eq!(right.rotation, -left.rotation, "tilt mirrors too");
}

#[test]
fn a_flap_needs_a_body_and_existing_poses() {
    let no_body = catalog(entity(flier(Some(flap())), None), STAND).unwrap_err();
    assert!(
        no_body.contains("bird") && no_body.contains("body"),
        "{no_body}"
    );

    let missing = Flap {
        down_pose: "no_such_pose".into(),
        ..flap()
    };
    let error = catalog(entity(flier(Some(missing)), Some(visual())), STAND).unwrap_err();
    assert!(
        error.contains("bird") && error.contains("down_pose") && error.contains("no_such_pose"),
        "{error}"
    );

    let bad_rate = Flap {
        hover_rate: 0.0,
        ..flap()
    };
    let error = catalog(entity(flier(Some(bad_rate)), Some(visual())), STAND).unwrap_err();
    assert!(error.contains("hover_rate"), "{error}");

    assert!(catalog(entity(flier(Some(flap())), Some(visual())), STAND).is_ok());
}

#[test]
fn a_flap_survives_the_editors_save_and_reload_and_old_fliers_still_load() {
    let flapping = entity(flier(Some(flap())), Some(visual()));
    let text = hollow_steps::definition_editor::serialize(&flapping).unwrap();
    assert!(text.contains("flap: Some("), "{text}");
    assert_eq!(ron::from_str::<EnemyDefinition>(&text).unwrap(), flapping);

    // A flier written before the flap existed has no `flap` key and saves without one.
    let old = r#"(name: "bird", health: 3, size: (32.0, 40.0), contact_damage: 1, behavior: "b",
        movement: Flying(hover_height: 56.0, bob_amplitude: 5.0, bob_speed: 0.7))"#;
    let parsed: EnemyDefinition = ron::from_str(old).unwrap();
    assert_eq!(parsed.movement.flap(), None);
    let saved = hollow_steps::definition_editor::serialize(&parsed).unwrap();
    assert!(!saved.contains("flap"), "{saved}");
}
