use std::sync::Arc;

use hollow_steps::behavior::{Action, Behavior, BehaviorContext, BehaviorDefinition};
use hollow_steps::behavior_values::KeyValues;
use hollow_steps::definition_editor::ActionInput;
use hollow_steps::enemy::{Enemy, EnemyDefinition};
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

fn context() -> BehaviorContext {
    BehaviorContext {
        player_position: Vec2::new(400.0, 442.0),
        health: 3,
        size: Vec2::new(36.0, 36.0),
        bounds: WorldBounds::new(0.0, 960.0, 460.0),
        patrol_left: 100.0,
        patrol_right: 200.0,
    }
}

fn definition(actions: &str) -> BehaviorDefinition {
    BehaviorDefinition::from_ron(&format!(
        "(initial_state: \"run\", states: [(name: \"run\", actions: [{actions}], transitions: [])])"
    ))
    .unwrap()
}

fn machine(actions: &str) -> Behavior {
    Behavior::new(Arc::new(definition(actions))).unwrap()
}

#[test]
fn jump_samples_once_and_moves_at_configured_speed_without_overshoot() {
    let mut behavior = machine("jump_to_player(dx: 0.0, dy: -200.0, speed: 200.0)");
    let mut position = Vec2::new(100.0, 442.0);
    let input = BehaviorContext {
        player_position: Vec2::new(100.0, 442.0),
        ..context()
    };
    behavior.update(&mut position, input, 0.25);
    assert_eq!(position, Vec2::new(100.0, 392.0));
    assert!(!behavior.action_finished());
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position, Vec2::new(100.0, 292.0));
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(position, Vec2::new(100.0, 242.0));
    assert!(behavior.action_finished());
    behavior.update(&mut position, context(), f32::MAX);
    assert_eq!(position, Vec2::new(100.0, 242.0));
    assert_eq!(behavior.blast_damage(position, context().size), 0);

    let mut diagonal = machine("jump_to_player(dx: 0.0, dy: 0.0, speed: 100.0)");
    position = Vec2::new(100.0, 442.0);
    let input = BehaviorContext {
        player_position: Vec2::new(160.0, 362.0),
        ..context()
    };
    diagonal.update(&mut position, input, 0.5);
    assert_eq!(position, Vec2::new(130.0, 402.0));
    diagonal.update(&mut position, input, 100.0);
    assert_eq!(position, input.player_position);
}

#[test]
fn jump_clamps_the_whole_body_to_every_arena_edge_even_with_extreme_offsets() {
    for (dx, dy, expected) in [
        (-f32::MAX, -f32::MAX, Vec2::new(18.0, 18.0)),
        (f32::MAX, f32::MAX, Vec2::new(942.0, 442.0)),
    ] {
        let mut behavior = machine(&format!("jump_to_player(dx: {dx}, dy: {dy}, speed: 100.0)"));
        let mut position = Vec2::new(100.0, 442.0);
        behavior.update(&mut position, context(), f32::MAX);
        assert_eq!(position, expected);
        assert!(behavior.action_finished());
    }
    let mut stopped = machine("jump_to_player(dx: 0.0, dy: -200.0, speed: 0.0)");
    let mut position = Vec2::new(100.0, 442.0);
    stopped.update(&mut position, context(), f32::MAX);
    assert_eq!(position, Vec2::new(100.0, 442.0));
    assert!(!stopped.action_finished());
}

#[test]
fn hover_holds_both_axes_for_exact_duration_and_carries_time_to_slam() {
    let mut behavior = machine("hover(seconds: 0.5), slam_down(speed: 100.0)");
    let mut position = Vec2::new(18.0, 18.0);
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(position, Vec2::new(18.0, 18.0));
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position, Vec2::new(18.0, 43.0));
    assert!(!behavior.action_finished());
    assert_eq!(behavior.blast_damage(position, context().size), 0);

    let mut zero = machine("hover(seconds: 0.0)");
    zero.update(&mut position, context(), 0.0);
    assert!(!zero.action_finished());
    zero.update(&mut position, context(), 0.01);
    assert!(zero.action_finished());
    assert_eq!(position, Vec2::new(18.0, 43.0));
}

#[test]
fn slam_moves_only_down_at_speed_stops_at_floor_and_has_no_implicit_damage() {
    let mut behavior = machine("slam_down(speed: 200.0), hover(seconds: 0.5)");
    let mut position = Vec2::new(942.0, 242.0);
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position, Vec2::new(942.0, 342.0));
    assert_eq!(behavior.blast_damage(position, context().size), 0);
    behavior.update(&mut position, context(), 0.75);
    assert_eq!(position, Vec2::new(942.0, 442.0));
    assert!(!behavior.action_finished());
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.action_finished());
    assert_eq!(position, Vec2::new(942.0, 442.0));

    let mut stopped = machine("slam_down(speed: 0.0)");
    position.y = 18.0;
    stopped.update(&mut position, context(), f32::MAX);
    assert_eq!(position.y, 18.0);
    assert!(!stopped.action_finished());
    let mut landing = machine("slam_down(speed: 200.0)");
    landing.update(&mut position, context(), f32::MAX);
    assert_eq!(position.y, 442.0);
    assert!(landing.action_finished());
    let mut on_floor = machine("slam_down(speed: 0.0)");
    on_floor.update(&mut position, context(), 0.1);
    assert!(on_floor.action_finished());
}

#[test]
fn area_blast_samples_once_waits_for_fuse_and_hits_circle_rectangle_boundary_once() {
    let mut behavior = machine("area_blast(count: 3, radius: 20.0, fuse_time: 1.0, damage: 4)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    let warnings = behavior.blasts().to_vec();
    assert_eq!(warnings.len(), 3);
    assert!(warnings.iter().all(|blast| !blast.exploded
        && blast.radius == 20.0
        && blast.damage == 4
        && blast.position.x >= 0.0
        && blast.position.x <= 960.0
        && blast.position.y >= 0.0
        && blast.position.y <= 460.0));
    assert_ne!(warnings[0].position, warnings[1].position);
    let center = warnings[0].position;
    assert_eq!(behavior.blast_damage(center, Vec2::splat(10.0)), 0);

    let mut single = machine("area_blast(count: 1, radius: 20.0, fuse_time: 0.0, damage: 2)");
    single.update(&mut position, context(), 0.01);
    let center = single.blasts()[0].position;
    assert_eq!(
        single.blast_damage(center + Vec2::new(25.0, 0.0), Vec2::splat(10.0)),
        2
    );
    assert_eq!(
        single.blast_damage(center + Vec2::new(26.0, 0.0), Vec2::splat(10.0)),
        0
    );
    assert_eq!(
        single.blast_damage(center + Vec2::splat(25.0), Vec2::splat(10.0)),
        0
    );
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(behavior.blasts(), warnings);
    assert!(!behavior.action_finished());
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.action_finished());
    assert_eq!(position, Vec2::new(100.0, 442.0));
    assert!(behavior.blasts().iter().all(|blast| blast.exploded));
    assert_eq!(behavior.blast_damage(center, Vec2::splat(10.0)), 4);
    assert_eq!(
        behavior.blast_damage(center + Vec2::new(25.0, 0.0), Vec2::splat(10.0)),
        4
    );
    assert_eq!(
        behavior.blast_damage(Vec2::splat(-1000.0), Vec2::splat(10.0)),
        0
    );
    behavior.update(&mut position, context(), 0.01);
    assert!(behavior.blasts().is_empty());
    assert_eq!(behavior.blast_damage(center, Vec2::splat(10.0)), 0);
}

#[test]
fn area_blast_zero_values_large_steps_and_seeded_positions_are_safe() {
    let mut empty = machine("area_blast(count: 0, radius: 0.0, fuse_time: 0.0, damage: 0)");
    let mut position = Vec2::new(100.0, 442.0);
    empty.update(&mut position, context(), 0.1);
    assert!(empty.action_finished());
    assert!(empty.blasts().is_empty());
    let source =
        "area_blast(count: 1, radius: 0.0, fuse_time: 0.0, damage: 0), hover(seconds: 1.0)";
    let mut one = machine(source);
    let mut two = machine(source);
    one.update(&mut position, context(), f32::MAX);
    two.update(&mut position, context(), f32::MAX);
    assert!(one.action_finished());
    assert_eq!(one.blasts(), two.blasts());
    assert!(one.blasts()[0].exploded);
    assert_eq!(
        one.blast_damage(one.blasts()[0].position, Vec2::splat(10.0)),
        0
    );
}

#[test]
fn new_actions_ignore_invalid_time_and_do_not_sample_until_started() {
    for action in [
        "jump_to_player(dx: 0.0, dy: -200.0, speed: 200.0)",
        "hover(seconds: 1.0)",
        "slam_down(speed: 200.0)",
        "area_blast(count: 2, radius: 20.0, fuse_time: 1.0, damage: 3)",
    ] {
        let mut behavior = machine(action);
        let mut position = Vec2::new(100.0, 242.0);
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            behavior.update(&mut position, context(), dt);
            assert_eq!(position, Vec2::new(100.0, 242.0));
            assert_eq!(behavior.state_elapsed(), 0.0);
            assert!(!behavior.action_finished());
            assert!(behavior.blasts().is_empty());
        }
    }
    let mut behavior =
        machine("hover(seconds: 0.5), jump_to_player(dx: -20.0, dy: -200.0, speed: 200.0)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    let input = BehaviorContext {
        player_position: Vec2::new(120.0, 442.0),
        ..context()
    };
    behavior.update(&mut position, input, 0.25);
    behavior.update(&mut position, context(), 1.0);
    assert_eq!(position, Vec2::new(100.0, 242.0));
}

#[test]
fn transitions_cancel_fuses_and_resample_jump_on_reentry() {
    let source = r#"(initial_state: "jump", states: [
        (name: "jump", actions: [jump_to_player(dx: 0.0, dy: -100.0, speed: 100.0)],
         transitions: [(target: "blast", condition: action_finished)]),
        (name: "blast", actions: [area_blast(count: 2, radius: 20.0, fuse_time: 5.0, damage: 2)],
         transitions: [(target: "jump", condition: timer_elapsed(0.5))])])"#;
    let mut behavior =
        Behavior::new(Arc::new(BehaviorDefinition::from_ron(source).unwrap())).unwrap();
    let mut position = Vec2::new(400.0, 442.0);
    behavior.update(&mut position, context(), 1.0);
    assert_eq!(behavior.state_name(), "blast");
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.blasts().len(), 2);
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.state_name(), "jump");
    assert!(behavior.blasts().is_empty());
    let input = BehaviorContext {
        player_position: Vec2::new(600.0, 442.0),
        ..context()
    };
    behavior.update(&mut position, input, 1.0);
    assert_eq!(position, Vec2::new(500.0, 342.0));
}

#[test]
fn every_enemy_can_run_all_actions_and_knockback_pauses_airborne_behavior() {
    let definition = EnemyDefinition {
        name: "flyer".into(),
        health: 3,
        size: (36.0, 36.0),
        contact_damage: 0,
        behavior: "all".into(),
        body: None,
    };
    let actions = "jump_to_player(dx: 0.0, dy: -200.0, speed: 200.0), hover(seconds: 0.5), slam_down(speed: 400.0), area_blast(count: 1, radius: 2000.0, fuse_time: 0.5, damage: 3)";
    let mut enemy = Enemy::from_definition(
        &definition,
        Arc::new(self::definition(actions)),
        100.0,
        200.0,
        context().bounds,
    )
    .unwrap();
    let player = Vec2::new(100.0, 442.0);
    enemy.update(player, 0.5, context().bounds);
    assert_eq!(enemy.position(), Vec2::new(100.0, 342.0));
    enemy.hit(1, 1.0);
    enemy.update(player, 0.16, context().bounds);
    assert_eq!(enemy.position().y, 342.0);
    assert_eq!(enemy.behavior().state_elapsed(), 0.5);
    enemy.update(player, 3.0, context().bounds);
    assert!(enemy.behavior().action_finished());
    assert_eq!(enemy.position().y, 442.0);
    assert_eq!(enemy.action_damage(player, Vec2::splat(36.0)), 3);
    enemy.hit(2, 1.0);
    enemy.update(player, 0.01, context().bounds);
    assert_eq!(enemy.action_damage(player, Vec2::splat(36.0)), 0);
    enemy.hit(3, 1.0);
    assert!(enemy.is_dead());
    assert_eq!(enemy.action_damage(player, Vec2::splat(36.0)), 0);
}

#[test]
fn catalog_labels_validation_and_editor_round_trip_cover_all_new_parameters() {
    for name in ["jump_to_player", "hover", "slam_down", "area_blast"] {
        assert!(Action::TYPES.contains(&name), "missing action {name}");
    }
    let actions = "jump_to_player(dx: -25.0, dy: -200.0, speed: 300.0), hover(seconds: 0.75), slam_down(speed: 600.0), area_blast(count: 4, radius: 70.0, fuse_time: 1.5, damage: 2)";
    let mut behavior = definition(actions);
    let mut values = KeyValues::from_behavior(&behavior);
    assert_eq!(values.values.len(), 9);
    assert!(values.values.iter().all(|v| v.label.contains("pixels")
        || v.label.contains("seconds")
        || v.label.contains("integer")));
    assert!(values.values[2].label.contains("negative above"));
    values.values[1].value = "-50".into();
    values.apply(&mut behavior).unwrap();
    let before = behavior.clone();
    values.values[5].value = "1.5".into();
    assert!(values.apply(&mut behavior).is_err());
    assert_eq!(behavior, before);
    let encoded = ron::ser::to_string(&behavior).unwrap();
    assert_eq!(BehaviorDefinition::from_ron(&encoded).unwrap(), behavior);
    for action in &behavior.states[0].actions {
        let kind = action.kind();
        assert_eq!(ActionInput::from_action(action).build().unwrap(), *action);
        let template = Action::defaults(kind).unwrap();
        assert_eq!(
            template.parameters().len(),
            template.parameter_labels().len()
        );
        template.validate().unwrap();
    }
}

#[test]
fn new_action_validation_rejects_nonfinite_negative_and_fractional_parameters() {
    for action in [
        "jump_to_player(dx: NaN, dy: 0.0, speed: 1.0)",
        "jump_to_player(dx: 0.0, dy: inf, speed: 1.0)",
        "jump_to_player(dx: 0.0, dy: 0.0, speed: -1.0)",
        "hover(seconds: -1.0)",
        "slam_down(speed: inf)",
        "area_blast(count: 1.5, radius: 1.0, fuse_time: 1.0, damage: 1)",
        "area_blast(count: 65536, radius: 1.0, fuse_time: 1.0, damage: 1)",
        "area_blast(count: 1, radius: -1.0, fuse_time: 1.0, damage: 1)",
        "area_blast(count: 1, radius: 1.0, fuse_time: NaN, damage: 1)",
        "area_blast(count: 1, radius: 1.0, fuse_time: 1.0, damage: 256)",
    ] {
        let source = format!("(initial_state: \"run\", states: [(name: \"run\", actions: [{action}], transitions: [])])");
        assert!(BehaviorDefinition::from_ron(&source).is_err(), "{action}");
    }
    let mut blast = ActionInput::from_action(
        &definition("area_blast(count: 1, radius: 1.0, fuse_time: 1.0, damage: 1)").states[0]
            .actions[0],
    );
    for (index, value) in [
        (0, "-1"),
        (0, "2.5"),
        (0, "65536"),
        (1, "NaN"),
        (2, "-1"),
        (3, "1.5"),
        (3, "256"),
    ] {
        let before = blast.values.clone();
        blast.values[index] = value.into();
        assert!(blast.build().is_err());
        blast.values = before;
    }
}
