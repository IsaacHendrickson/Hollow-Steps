//! The `pose` action: a zero-time action that holds a named pose (or returns to idle) in enemy
//! behaviors and boss attacks. Uses fixtures only, never `assets/`.

use std::sync::Arc;

use hollow_steps::behavior::{Action, Behavior, BehaviorContext, BehaviorDefinition};
use hollow_steps::body::{BodyTemplate, BodyTemplates, BodyVisual};
use hollow_steps::boss_definition::{BossDefinition, BossDefinitions};
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::enemy::EnemyDefinition;
use hollow_steps::player::WorldBounds;
use hollow_steps::pose::{
    default_pose_action_blend_secs, is_pose_parameter, validate_behavior, validate_entity, Pose,
    PoseStep, Poses,
};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const START: Vec2 = Vec2::new(400.0, 436.0);

fn poses() -> Poses {
    let pose = |source: &str| Arc::new(ron::from_str::<Pose>(source).unwrap());
    Poses::from([
        (
            "windup".into(),
            pose(include_str!("fixtures/poses/windup.ron")),
        ),
        ("hurt".into(), pose(include_str!("fixtures/poses/hurt.ron"))),
    ])
}

fn templates() -> BodyTemplates {
    let mut template: BodyTemplate =
        ron::from_str(include_str!("fixtures/bodies/small.ron")).unwrap();
    template.pose_map = [
        ("head".into(), "root".into()),
        ("hand_l".into(), "tip".into()),
    ]
    .into();
    BodyTemplates::from([("small".into(), Arc::new(template))])
}

fn visual() -> BodyVisual {
    ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap()
}

fn action(text: &str) -> Action {
    ron::from_str(text).unwrap()
}

// ---- the action itself ----

#[test]
fn it_loads_with_a_default_blend_and_with_no_name() {
    let named = action(r#"pose(pose_name: Some("windup"))"#);
    assert_eq!(
        named,
        Action::Pose {
            pose_name: Some("windup".into()),
            blend_secs: default_pose_action_blend_secs(),
        }
    );
    let cleared = action("pose(pose_name: None, blend_secs: 0.3)");
    assert_eq!(
        cleared,
        Action::Pose {
            pose_name: None,
            blend_secs: 0.3
        }
    );
    named.validate().unwrap();
    cleared.validate().unwrap();
    let text = ron::to_string(&named).unwrap();
    assert_eq!(action(&text), named);
}

#[test]
fn it_rejects_bad_names_blends_and_unknown_fields() {
    for bad in [0.0, -0.1, f32::NAN, f32::INFINITY] {
        let mut pose = action(r#"pose(pose_name: Some("windup"))"#);
        if let Action::Pose { blend_secs, .. } = &mut pose {
            *blend_secs = bad;
        }
        assert!(pose.validate().is_err(), "blend {bad}");
    }
    assert!(action(r#"pose(pose_name: Some("Not Valid"))"#)
        .validate()
        .is_err());
    assert!(ron::from_str::<Action>(r#"pose(pose_name: None, speed: 1.0)"#).is_err());
}

#[test]
fn the_editor_catalog_lists_it_with_a_pose_picker() {
    let kind = Action::TYPES.iter().position(|n| *n == "pose").unwrap();
    let mut pose = Action::defaults(kind).expect("a catalog default exists");
    assert_eq!(pose.kind(), kind);
    pose.validate().unwrap();
    assert_eq!(pose.parameter_labels().len(), 2);
    let (field, _) = pose.parameters()[1];
    assert!(is_pose_parameter(field));
    pose.set_parameter(1, "windup").unwrap();
    assert_eq!(pose.pose_names(), vec![("pose_name", &"windup".to_owned())]);
    pose.set_parameter(1, "").unwrap();
    assert!(pose.pose_names().is_empty());
    assert!(pose.set_parameter(0, "0").is_err());
    assert!(pose.set_parameter(1, "Not Valid").is_err());
}

// ---- enemy behaviors ----

fn machine(actions: &str) -> Behavior {
    let definition = BehaviorDefinition::from_ron(&format!(
        "(initial_state: \"run\", states: [(name: \"run\", actions: [{actions}], transitions: [])])"
    ))
    .unwrap();
    Behavior::new(Arc::new(definition)).unwrap()
}

fn context() -> BehaviorContext {
    BehaviorContext {
        player_position: Vec2::new(400.0, 442.0),
        health: 3,
        size: Vec2::splat(36.0),
        bounds: BOUNDS,
        patrol_left: 100.0,
        patrol_right: 200.0,
    }
}

#[test]
fn an_enemy_pose_action_takes_no_time_and_hands_over_the_pose() {
    let mut behavior = machine(r#"pose(pose_name: Some("windup")), wait(seconds: 0.5)"#);
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.1);
    let steps = behavior.take_pose_steps();
    assert!(steps.iter().any(|s| matches!(
        s,
        PoseStep::Apply { name: Some(name), seconds, blend_secs, .. }
            if name == "windup" && *seconds == 0.0 && *blend_secs == default_pose_action_blend_secs()
    )));
    // The wait still got the whole tick: the pose action cost nothing.
    let idle: f32 = steps
        .iter()
        .map(|s| match s {
            PoseStep::Idle(seconds) => *seconds,
            _ => 0.0,
        })
        .sum();
    assert!((idle - 0.1).abs() < 1e-6, "{idle}");
}

#[test]
fn a_pose_action_without_a_name_clears_the_pose() {
    let mut behavior = machine("pose(pose_name: None), wait(seconds: 0.5)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.1);
    assert!(behavior
        .take_pose_steps()
        .iter()
        .any(|s| matches!(s, PoseStep::Clear)));
}

#[test]
fn enemy_validation_checks_the_pose_action() {
    let behavior =
        BehaviorDefinition::from_ron(r#"(initial_state: "run", states: [(name: "run", actions: [pose(pose_name: Some("missing"))], transitions: [])])"#)
            .unwrap();
    let error = validate_behavior(&behavior, &poses()).unwrap_err();
    assert!(
        error.contains("pose_name") && error.contains("unknown pose"),
        "{error}"
    );
    let known =
        BehaviorDefinition::from_ron(r#"(initial_state: "run", states: [(name: "run", actions: [pose(pose_name: Some("windup"))], transitions: [])])"#)
            .unwrap();
    validate_behavior(&known, &poses()).unwrap();
    let entity: EnemyDefinition =
        ron::from_str(include_str!("fixtures/entities/chase.ron")).unwrap();
    assert!(entity.body.is_none());
    assert!(validate_entity(&entity, &known, &poses())
        .unwrap_err()
        .contains("requires a body template"));
}

// ---- boss attacks ----

fn boss_definition(actions: &str, body: bool) -> BossDefinition {
    let source = format!(
        r#"(name: "Ash Warden", health: 100, size: (48.0, 48.0), contact_damage: 0, selection: cycle, pause_seconds: 1.0, attacks: [(name: "Test", actions: [{actions}])])"#
    );
    let mut definition = BossDefinition::from_ron(&source).unwrap();
    definition.body = body.then(visual);
    definition
}

fn catalog(definition: BossDefinition) -> Result<BossDefinitions, String> {
    BossDefinitions::from_definitions_with_assets([definition], templates(), poses())
}

fn spawn(actions: &str, body: bool) -> BossSpecials {
    catalog(boss_definition(actions, body))
        .unwrap()
        .spawn("Ash Warden", START)
        .unwrap()
}

fn pose_of(boss: &BossSpecials) -> Option<String> {
    boss.body().unwrap().pose_name.clone()
}

#[test]
fn boss_validation_checks_the_pose_action() {
    let error = catalog(boss_definition(
        r#"pose(pose_name: Some("missing")), wait(seconds: 0.1)"#,
        true,
    ))
    .unwrap_err();
    assert!(
        error.contains("pose_name") && error.contains("unknown pose"),
        "{error}"
    );
    let error = catalog(boss_definition(
        r#"pose(pose_name: Some("windup")), wait(seconds: 0.1)"#,
        false,
    ))
    .unwrap_err();
    assert!(error.contains("requires a body"), "{error}");
    // Returning to idle names no pose, so it needs no body.
    catalog(boss_definition(
        "pose(pose_name: None), wait(seconds: 0.1)",
        false,
    ))
    .unwrap();
}

#[test]
fn a_pose_holds_through_a_telegraph_and_the_next_pose_replaces_it() {
    let mut boss = spawn(
        r#"pose(pose_name: Some("windup")), telegraph(seconds: 0.5), pose(pose_name: Some("hurt")), wait(seconds: 0.5)"#,
        true,
    );
    let player = Vec2::new(400.0, 436.0);
    assert_eq!(pose_of(&boss), None);
    boss.start_attack("Test", player, BOUNDS);
    assert_eq!(pose_of(&boss).as_deref(), Some("windup"));
    boss.update(0.3, player, BOUNDS);
    assert_eq!(pose_of(&boss).as_deref(), Some("windup"));
    boss.update(0.3, player, BOUNDS);
    assert_eq!(pose_of(&boss).as_deref(), Some("hurt"));
}

#[test]
fn a_pose_outlasts_an_attack_that_ends_in_a_glide_and_clears_on_arrival() {
    // The glide is instantaneous to start but moves over later ticks, like Ash Warden's dash.
    let mut boss = spawn(
        r#"pose(pose_name: Some("windup")), telegraph(seconds: 0.1), pose(pose_name: Some("hurt")), glide_to_player(speed: 100.0, above_floor: 0.0)"#,
        true,
    );
    let player = Vec2::new(700.0, 436.0);
    boss.start_attack("Test", player, BOUNDS);
    let mut held_while_gliding = false;
    for _ in 0..600 {
        boss.update(1.0 / 60.0, player, BOUNDS);
        let gliding = !boss.is_telegraphing() && boss.position().x != player.x;
        if gliding {
            assert_eq!(pose_of(&boss).as_deref(), Some("hurt"));
            held_while_gliding = true;
        }
    }
    assert!(held_while_gliding, "the glide outlasted the attack");
    assert_eq!(boss.position().x, player.x);
    assert_eq!(pose_of(&boss), None, "idle again once it arrived");
}

#[test]
fn pose_actions_change_neither_timing_nor_damage() {
    let slam = |poses: bool| {
        let (windup, slam) = if poses {
            (
                r#"pose(pose_name: Some("windup")),"#,
                r#"pose(pose_name: Some("hurt")),"#,
            )
        } else {
            ("", "")
        };
        format!(
            "{windup} glide_to_player(speed: 1000.0, above_floor: 180.0), telegraph(seconds: 0.5), {slam} glide_to_floor(speed: 1000.0), damage_target(radius: 56.0, damage: 10)"
        )
    };
    let run = |poses: bool| {
        let mut boss = spawn(&slam(poses), poses);
        let player = Vec2::new(500.0, 436.0);
        boss.start_attack("Test", player, BOUNDS);
        let mut trace = Vec::new();
        for _ in 0..90 {
            let damage =
                boss.update_with_player_size(1.0 / 60.0, player, Vec2::new(20.0, 40.0), BOUNDS);
            trace.push((damage, boss.position(), boss.is_telegraphing()));
        }
        trace
    };
    let plain = run(false);
    assert!(plain.iter().any(|(damage, ..)| *damage > 0));
    assert_eq!(plain, run(true));
}

#[test]
fn a_boss_without_a_body_runs_an_unnamed_pose_action_harmlessly() {
    let mut boss = spawn("pose(pose_name: None), wait(seconds: 0.2)", false);
    boss.start_attack("Test", START, BOUNDS);
    for _ in 0..30 {
        boss.update(1.0 / 60.0, START, BOUNDS);
    }
    assert!(boss.body().is_none());
    assert!(!boss.is_telegraphing());
}
