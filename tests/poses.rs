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
        2
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

#[test]
fn hurt_pose_and_flash_expire_on_game_time_and_resume_action() {
    let poses = load_poses(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("poses"),
    )
    .unwrap();
    let mut body = body();
    body.visual.hurt.pose = Some("hurt".into());
    body.set_pose(Some("windup"), &poses);
    body.configure_pose_spring(0.4, 1.0);
    body.hurt(&poses);
    assert_eq!(body.pose_name.as_deref(), Some("hurt"));
    assert_eq!(body.flash_strength(), 1.0);
    body.step(f32::NAN);
    body.step(0.0);
    body.step(-1.0);
    assert_eq!(body.pose_name.as_deref(), Some("hurt"));
    body.step(0.08);
    assert_eq!(body.flash_strength(), 0.0);
    assert_eq!(body.pose_name.as_deref(), Some("hurt"));
    body.step(0.069);
    assert_eq!(body.pose_name.as_deref(), Some("hurt"));
    body.step(0.002);
    assert_eq!(body.pose_name.as_deref(), Some("windup"));
    body.hurt(&poses);
    body.clear_pose(); // Attack ending during hurt must not cancel the damage response.
    assert_eq!(body.pose_name.as_deref(), Some("hurt"));
    body.step(10.0);
    assert!(body.pose_name.is_none());
    assert!(body.points.values().all(|p| p.is_finite()));
}

#[test]
fn separate_telegraph_uses_following_action_binding_and_duration() {
    use hollow_steps::pose::PoseStep;
    let mut behavior = machine("telegraph(seconds: 0.5), shoot_projectile(speed: 280.0, damage: 1, lifetime_secs: 3.0, size: (12.0, 12.0), aim: AtPlayer, count: 1, spread_degrees: 0.0, telegraph_secs: 0.0, telegraph_pose: Some(\"windup\"), active_pose: Some(\"recoil\"))");
    let mut position = macroquad::math::Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.take_pose_steps().iter().any(|s| matches!(s, PoseStep::Apply { name: Some(name), seconds, blend_secs, .. } if name == "windup" && *seconds == 0.25 && *blend_secs == 0.4)));
    assert!(behavior.projectiles().is_empty());
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.projectiles().len(), 1);
    assert!(behavior.take_pose_steps().iter().any(
        |s| matches!(s, PoseStep::Apply { name: Some(name), instant: true, .. } if name == "recoil")
    ));
}

#[test]
fn validation_reports_unknown_pose_semantic_and_missing_body() {
    use hollow_steps::{
        behavior::BehaviorDefinition,
        enemy::EnemyDefinition,
        pose::{validate_behavior, validate_entity},
    };
    let poses = load_poses(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("poses"),
    )
    .unwrap();
    let mut behavior =
        BehaviorDefinition::from_ron(include_str!("fixtures/attacks/posed.ron")).unwrap();
    validate_behavior(&behavior, &poses).unwrap();
    let action = &mut behavior.states[0].actions[0];
    let index = action
        .parameters()
        .iter()
        .position(|(key, _)| *key == "telegraph_pose")
        .unwrap();
    action.set_parameter(index, "missing").unwrap();
    let error = validate_behavior(&behavior, &poses).unwrap_err();
    assert!(
        error.contains("telegraph_pose")
            && error.contains("unknown pose")
            && error.contains("missing")
    );
    let entity: EnemyDefinition =
        ron::from_str(include_str!("fixtures/entities/chase.ron")).unwrap();
    assert!(validate_entity(&entity, &behavior, &poses)
        .unwrap_err()
        .contains("requires a body template"));
    let mut template = body().template.as_ref().clone();
    template.pose_map.insert("elbow".into(), "root".into());
    let error = template.validate().unwrap_err();
    assert!(
        error.contains("pose_map")
            && error.contains("unknown semantic point")
            && error.contains("elbow")
    );
    let mut pose: Pose = ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap();
    pose.offsets.insert("elbow".into(), (0.0, 0.0));
    assert!(pose
        .validate()
        .unwrap_err()
        .contains("unknown semantic point"));
}

struct Assets(std::path::PathBuf);
impl Assets {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-poses-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        for directory in [
            "entities",
            "behaviors",
            "rooms",
            "bosses",
            "bodies",
            "poses",
        ] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(fixture.join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
    fn posed(&self) {
        use hollow_steps::{definition_editor::serialize, enemy::EnemyDefinition};
        let mut entity: EnemyDefinition =
            ron::from_str(include_str!("fixtures/entities/chase.ron")).unwrap();
        entity.body = Some(body().visual);
        std::fs::write(
            self.0.join("entities").join("chase.ron"),
            serialize(&entity).unwrap(),
        )
        .unwrap();
        std::fs::write(
            self.0.join("bodies").join("small.ron"),
            serialize(body().template.as_ref()).unwrap(),
        )
        .unwrap();
        std::fs::write(
            self.0.join("behaviors").join("chase.ron"),
            include_str!("fixtures/attacks/posed.ron"),
        )
        .unwrap();
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn staged_saves_validate_pose_catalog_and_leave_files_untouched_on_errors() {
    use hollow_steps::{
        behavior::BehaviorDefinition,
        definition_editor::{write_files, DefinitionDraft},
    };
    let assets = Assets::new();
    assets.posed();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = assets.0.join("behaviors").join("chase.ron");
    let before = std::fs::read(&path).unwrap();
    let original = draft.behaviors["chase"].clone();
    draft.behaviors.insert(
        "chase".into(),
        BehaviorDefinition::from_ron(
            &include_str!("fixtures/attacks/posed.ron").replace("windup", "missing"),
        )
        .unwrap(),
    );
    let error = draft.plan(&[]).unwrap_err();
    let (reported, _) = error.split_once(": ").unwrap();
    assert_eq!(Path::new(reported), path);
    assert!(error.contains("unknown pose"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    draft.behaviors.insert("chase".into(), original);
    let entity_path = draft.entity_path("chase").unwrap();
    draft.entities.get_mut(&entity_path).unwrap().body = None;
    assert!(draft
        .plan(&[])
        .unwrap_err()
        .contains("requires a body template"));
    draft.entities.get_mut(&entity_path).unwrap().body = Some(body().visual);
    Arc::make_mut(draft.bodies.get_mut("small").unwrap())
        .pose_map
        .insert("elbow".into(), "root".into());
    assert!(draft
        .plan(&[])
        .unwrap_err()
        .contains("unknown semantic point"));
    draft.bodies.insert("small".into(), body().template);
    Arc::make_mut(draft.poses.get_mut("windup").unwrap())
        .offsets
        .get_mut("head")
        .unwrap()
        .0 = -0.3;
    let plan = draft.plan(&[]).unwrap();
    assert!(plan
        .iter()
        .any(|(p, _)| p == &assets.0.join("poses").join("windup.ron")));
    write_files(&plan).unwrap();
    let saved = DefinitionDraft::load(&assets.0).unwrap();
    assert_eq!(saved.poses["windup"].offsets["head"].0, -0.3);
}

#[test]
fn hot_reload_reads_new_poses_atomically_and_keeps_point_positions() {
    use hollow_steps::{definition_editor::serialize, game::GameState, player::PlayerInput};
    let assets = Assets::new();
    assets.posed();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.1);
    let old = game.room().enemies[0].body().unwrap().points.clone();
    let path = assets.0.join("poses").join("windup.ron");
    let mut pose: Pose = ron::from_str(include_str!("fixtures/poses/windup.ron")).unwrap();
    pose.offsets.insert("head".into(), (-0.4, 0.0));
    std::fs::write(&path, serialize(&pose).unwrap()).unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(game.room().enemies[0].body().unwrap().points, old);
    game.update(PlayerInput::default(), 0.01);
    let body = game.room().enemies[0].body().unwrap();
    assert_eq!(
        body.targets["root"].x,
        body.template.points["root"].rest.0 - 0.4
    );
    let before = body.points.clone();
    std::fs::write(&path, "invalid pose").unwrap();
    assert!(game.reload_rooms().unwrap_err().contains("windup.ron"));
    assert_eq!(game.room().enemies[0].body().unwrap().points, before);
}

fn enemy_with_action(action: hollow_steps::behavior::Action) -> hollow_steps::enemy::Enemy {
    use hollow_steps::{
        behavior::{BehaviorDefinition, State},
        enemy::{EnemyDefinition, EnemyDefinitions},
    };
    let mut entity: EnemyDefinition =
        ron::from_str(include_str!("fixtures/entities/chase.ron")).unwrap();
    entity.body = Some(body().visual);
    entity.body.as_mut().unwrap().hurt.pose = Some("hurt".into());
    let behavior = BehaviorDefinition {
        initial_state: "run".into(),
        states: vec![State {
            name: "run".into(),
            actions: vec![action],
            transitions: vec![],
        }],
    };
    let poses = load_poses(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("poses"),
    )
    .unwrap();
    let definitions = EnemyDefinitions::from_definitions_with_poses(
        [entity.clone()],
        [(entity.behavior, Arc::new(behavior))].into(),
        [(body().visual.template, body().template)].into(),
        poses,
    )
    .unwrap();
    definitions
        .spawn(&entity.name, 100.0, 200.0, context().bounds)
        .unwrap()
}

#[test]
fn all_five_attacks_set_and_clear_pose_targets_without_changing_gameplay() {
    use hollow_steps::{behavior::Action, pose::is_pose_parameter};
    let actions: Vec<Action> =
        ron::from_str(include_str!("fixtures/attacks/pose_actions.ron")).unwrap();
    assert_eq!(actions.len(), 5);
    for action in actions {
        let mut plain_action = action.clone();
        for (index, (field, _)) in action.parameters().iter().enumerate() {
            if is_pose_parameter(field) {
                plain_action.set_parameter(index, "None").unwrap();
            }
        }
        let mut posed = enemy_with_action(action.clone());
        let mut plain = enemy_with_action(plain_action);
        for (step, dt) in [0.1, 0.15, 0.2, 0.5, 1.0, 10.0].into_iter().enumerate() {
            posed.update(context().player_position, dt, context().bounds);
            plain.update(context().player_position, dt, context().bounds);
            if step == 0 {
                assert_eq!(posed.body().unwrap().pose_name.as_deref(), Some("windup"));
            }
            assert_eq!(posed.position(), plain.position(), "{action:?}");
            assert_eq!(posed.size(), plain.size());
            assert_eq!(posed.health(), plain.health());
            assert_eq!(
                posed.attack_hit(context().player_position, context().size),
                plain.attack_hit(context().player_position, context().size)
            );
            assert_eq!(
                posed.behavior().projectiles(),
                plain.behavior().projectiles()
            );
        }
        let body = posed.body().unwrap();
        assert!(posed.behavior().action_finished());
        assert!(body.pose_name.is_none());
        assert!(body.points.values().all(|p| p.is_finite()));
        assert_ne!(
            body.targets["root"].x,
            body.template.points["root"].rest.0 - 0.2
        );
    }
}

#[test]
fn pose_offsets_mirror_once_with_facing_and_do_not_mutate_local_targets() {
    use hollow_steps::body::resolve_point;
    use macroquad::math::Vec2;
    let mut body = body();
    let poses = load_poses(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("poses"),
    )
    .unwrap();
    body.set_pose(Some("windup"), &poses);
    let targets = body.targets.clone();
    let rest = body.template.rest_positions();
    let offset = |points, left| {
        resolve_point(
            "root",
            points,
            Vec2::new(100.0, 200.0),
            Vec2::new(40.0, 60.0),
            1.3,
            left,
        )
        .unwrap()
    };
    let right_delta = offset(&body.targets, false) - offset(&rest, false);
    let left_delta = offset(&body.targets, true) - offset(&rest, true);
    assert!((left_delta.x + right_delta.x).abs() < 0.0001);
    assert!((left_delta.y - right_delta.y).abs() < 0.0001);
    assert!(right_delta.x < 0.0 && left_delta.x > 0.0);
    assert_eq!(body.targets, targets);
}

#[test]
fn damage_starts_hurt_once_and_does_not_restart_on_duplicate_hit() {
    let actions: Vec<hollow_steps::behavior::Action> =
        ron::from_str(include_str!("fixtures/attacks/pose_actions.ron")).unwrap();
    let mut enemy = enemy_with_action(actions[0].clone());
    assert!(enemy.hit(1, 1.0));
    assert_eq!(enemy.body().unwrap().pose_name.as_deref(), Some("hurt"));
    enemy.update(context().player_position, 0.1, context().bounds);
    assert!(!enemy.hit(1, 1.0));
    enemy.update(context().player_position, 0.051, context().bounds);
    assert!(enemy.body().unwrap().pose_name.is_none());
    assert_eq!(enemy.body().unwrap().flash_strength(), 0.0);
}

#[test]
fn transition_interrupt_clears_targets_in_the_same_update() {
    use hollow_steps::{
        behavior::{Behavior, BehaviorDefinition, Condition, State, Transition},
        pose::PoseStep,
    };
    let mut definition =
        BehaviorDefinition::from_ron(include_str!("fixtures/attacks/posed.ron")).unwrap();
    definition.states[0].transitions.push(Transition {
        target: "idle".into(),
        condition: Condition::TimerElapsed(0.15),
        invert: false,
    });
    definition.states.push(State {
        name: "idle".into(),
        actions: vec![],
        transitions: vec![],
    });
    let mut behavior = Behavior::new(Arc::new(definition)).unwrap();
    let poses = load_poses(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("poses"),
    )
    .unwrap();
    let mut body = body();
    let mut position = macroquad::math::Vec2::new(100.0, 442.0);
    for dt in [0.1, 0.1] {
        behavior.update(&mut position, context(), dt);
        for step in behavior.take_pose_steps() {
            match step {
                PoseStep::Apply { name, .. } => body.set_pose(name.as_deref(), &poses),
                PoseStep::Clear => body.clear_pose(),
                _ => {}
            }
        }
    }
    assert_eq!(behavior.state_name(), "idle");
    assert!(body.pose_name.is_none());
}
