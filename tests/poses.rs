use hollow_steps::body::{BodyInstance, BodyTemplate, BodyVisual};
use hollow_steps::pose::{load_poses, Pose};
use std::{path::Path, sync::Arc};

fn body() -> BodyInstance {
    let mut template: BodyTemplate =
        ron::from_str(include_str!("fixtures/bodies/motion.ron")).unwrap();
    template.pose_map = [
        ("head".into(), "root".into()),
        ("hand_l".into(), "absent".into()),
    ]
    .into();
    let visual: BodyVisual = ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap();
    BodyInstance::new(
        visual.clone(),
        &[(visual.template.clone(), Arc::new(template))].into(),
    )
    .unwrap()
}

#[test]
fn semantic_resolution_ignores_missing_points() {
    let mut body = body();
    let pose: Pose = ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap();
    let targets = pose.resolve(&body.template);
    assert_eq!(targets.len(), 1);
    assert_eq!(
        targets["root"].x,
        body.template.points["root"].rest.0 - 0.20
    );
    pose.apply(&mut body);
    assert_eq!(body.targets["root"], targets["root"]);
    assert_eq!(
        load_poses(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("poses")
        )
        .unwrap()
        .len(),
        1
    );
}

fn machine(action: &str) -> hollow_steps::behavior::Behavior {
    let definition = hollow_steps::behavior::BehaviorDefinition::from_ron(&format!(
        "(initial_state: \"run\", states: [(name: \"run\", actions: [{action}], transitions: [])])"
    ))
    .unwrap();
    hollow_steps::behavior::Behavior::new(Arc::new(definition)).unwrap()
}
fn context() -> hollow_steps::behavior::BehaviorContext {
    hollow_steps::behavior::BehaviorContext {
        player_position: macroquad::math::Vec2::new(400.0, 442.0),
        health: 3,
        size: macroquad::math::Vec2::splat(36.0),
        bounds: hollow_steps::player::WorldBounds::new(0.0, 960.0, 460.0),
        patrol_left: 100.0,
        patrol_right: 200.0,
    }
}
#[test]
fn action_start_sets_targets_and_end_clears_them() {
    use hollow_steps::pose::PoseStep;
    let mut behavior = machine("lunge(speed: 0.0, duration_secs: 0.2, attack: (shape: Rect, offset: (0.0, 0.0), size: (20.0, 20.0), damage: 1, knockback: (0.0, 0.0), active_secs: 0.2), recovery_secs: 0.1, telegraph_secs: 0.5, telegraph_pose: Some(\"windup\"))");
    let mut body = body();
    let poses = [(
        "windup".into(),
        Arc::new(ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap()),
    )]
    .into();
    let mut position = macroquad::math::Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.1);
    for step in behavior.take_pose_steps() {
        if let PoseStep::Apply { name, .. } = step {
            body.set_pose(name.as_deref(), &poses);
        }
    }
    assert_eq!(body.pose_name.as_deref(), Some("windup"));
    assert_eq!(
        body.targets["root"].x,
        body.template.points["root"].rest.0 - 0.2
    );
    behavior.update(&mut position, context(), 1.0);
    for step in behavior.take_pose_steps() {
        match step {
            PoseStep::Apply { name, .. } => body.set_pose(name.as_deref(), &poses),
            PoseStep::Clear => body.clear_pose(),
            _ => {}
        }
    }
    assert!(behavior.action_finished());
    assert!(body.pose_name.is_none());
    assert_ne!(
        body.targets["root"].x,
        body.template.points["root"].rest.0 - 0.2
    );
}

#[test]
fn telegraph_settles_by_blend_ratio_and_strike_is_faster() {
    for duration in [0.01_f32, 0.15, 0.5, 2.0] {
        for steps in [1, 30, 120] {
            let mut body = body();
            let poses = [(
                "windup".into(),
                Arc::new(ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap()),
            )]
            .into();
            body.set_pose(Some("windup"), &poses);
            let target = body.targets["root"];
            body.configure_pose_spring(duration * 0.8, 1.0);
            for _ in 0..steps {
                body.step(duration * 0.8 / steps as f32);
            }
            assert!(
                body.points["root"].distance(target) < 0.003,
                "{duration}, {steps}"
            );
            body.step(duration * 0.2);
            assert!(body.points["root"].distance(target) < 0.001);
            assert!(body.points.values().all(|p| p.is_finite()));
        }
    }
    let mut slow = body();
    let poses = [(
        "windup".into(),
        Arc::new(ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap()),
    )]
    .into();
    slow.set_pose(Some("windup"), &poses);
    let mut fast = slow.clone();
    slow.configure_pose_spring(0.4, 1.0);
    fast.configure_pose_spring(0.4, 3.0);
    slow.step(0.05);
    fast.step(0.05);
    assert!(
        fast.points["root"].distance(fast.targets["root"])
            < slow.points["root"].distance(slow.targets["root"])
    );
}
