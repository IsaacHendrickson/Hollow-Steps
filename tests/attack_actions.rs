use hollow_steps::enemy_attack::{active_window, AttackBox, AttackShape};
use hollow_steps::player::{Player, PlayerConfig};
use macroquad::math::Vec2;

fn hitbox() -> AttackBox {
    AttackBox {
        shape: AttackShape::Rect,
        offset: (30.0, 0.0),
        size: (20.0, 20.0),
        damage: 20,
        knockback: (40.0, -50.0),
        knockback_secs: 0.2,
        active_secs: 0.5,
    }
}

#[test]
fn attack_geometry_is_independent_and_windows_are_half_open() {
    let attack = hitbox();
    attack.validate().unwrap();
    assert!(attack.overlaps(Vec2::ZERO, Vec2::new(30.0, 0.0), Vec2::splat(2.0)));
    assert!(!attack.overlaps(Vec2::ZERO, Vec2::ZERO, Vec2::splat(2.0)));
    assert!(!active_window(0.49, 0.5, attack.active_secs));
    assert!(active_window(0.5, 0.5, attack.active_secs));
    assert!(!active_window(1.0, 0.5, attack.active_secs));
    assert!(!active_window(0.5, 0.5, 0.0));
}

#[test]
fn attack_damage_and_knockback_respect_existing_invincibility() {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    assert!(player.take_attack_damage(hitbox().hit()));
    assert_eq!(player.health().current(), 30);
    let after = player.clone();
    assert!(!player.take_attack_damage(hitbox().hit()));
    assert_eq!(player, after);
}

use hollow_steps::behavior::{Behavior, BehaviorContext, BehaviorDefinition};
use hollow_steps::player::WorldBounds;
use std::sync::Arc;

fn context() -> BehaviorContext {
    BehaviorContext {
        player_position: Vec2::new(400.0, 442.0),
        health: 3,
        size: Vec2::splat(36.0),
        bounds: WorldBounds::new(0.0, 960.0, 460.0),
        patrol_left: 100.0,
        patrol_right: 200.0,
    }
}
fn machine(actions: &str) -> Behavior {
    Behavior::new(Arc::new(
        BehaviorDefinition::from_ron(&format!(
        "(initial_state: \"run\", states: [(name: \"run\", actions: [{actions}], transitions: [])])"
    ))
        .unwrap(),
    ))
    .unwrap()
}

#[test]
fn lunge_samples_direction_once_and_stands_still_in_recovery() {
    let mut behavior = machine("lunge(speed: 100.0, duration_secs: 1.0, attack: (shape: Rect, offset: (0.0, 0.0), size: (40.0, 40.0), damage: 2, knockback: (10.0, -10.0), active_secs: 1.0), recovery_secs: 0.5, telegraph_secs: 0.5)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(position.x, 100.0);
    assert!(behavior.attack_warning().is_some());
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    let moved_player = BehaviorContext {
        player_position: Vec2::new(0.0, 442.0),
        ..context()
    };
    behavior.update(&mut position, moved_player, 0.5);
    assert_eq!(position.x, 125.0);
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_some());
    behavior.update(&mut position, moved_player, 0.75);
    assert_eq!(position.x, 200.0);
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    assert!(!behavior.action_finished());
    behavior.update(&mut position, moved_player, 0.5);
    assert_eq!(position.x, 200.0);
    assert!(behavior.action_finished());
}

#[test]
fn parabola_lands_exactly_and_jump_target_never_tracks_player() {
    use hollow_steps::enemy_attack::parabolic_position;
    let start = Vec2::new(100.0, 442.0);
    let target = Vec2::new(400.0, 442.0);
    assert_eq!(parabolic_position(start, target, 0.0, 80.0), start);
    assert_eq!(parabolic_position(start, target, 1.0, 80.0), target);
    assert_eq!(
        parabolic_position(start, target, 0.5, 80.0),
        Vec2::new(250.0, 362.0)
    );
    let mut behavior = machine("jump_to(dx: 0.0, dy: 0.0, air_time_secs: 1.0, arc_height: 80.0, attack: None, recovery_secs: 0.5, telegraph_secs: 0.0)");
    let mut position = start;
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position, Vec2::new(250.0, 362.0));
    behavior.update(
        &mut position,
        BehaviorContext {
            player_position: Vec2::new(700.0, 200.0),
            ..context()
        },
        0.5,
    );
    assert_eq!(position, target);
    behavior.update(&mut position, context(), 0.5);
    assert!(behavior.action_finished());
}

#[test]
fn jump_stays_inside_room_with_large_offsets_and_time() {
    let mut behavior = machine("jump_to(dx: 2000.0, dy: 2000.0, air_time_secs: 1.0, arc_height: 10000.0, attack: None, recovery_secs: 0.0, telegraph_secs: 0.0)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.5);
    assert!(position.y >= 18.0 && position.x <= 942.0);
    behavior.update(&mut position, context(), f32::MAX);
    assert_eq!(position, Vec2::new(942.0, 442.0));
}

#[test]
fn slam_opens_only_on_impact_and_then_recovers_without_moving() {
    let mut behavior = machine("slam_down(speed: 200.0, attack: (shape: Rect, offset: (0.0, 0.0), size: (100.0, 32.0), damage: 2, knockback: (0.0, -200.0), active_secs: 0.5), recovery_secs: 0.5, telegraph_secs: 0.5)");
    let mut position = Vec2::new(100.0, 242.0);
    behavior.update(&mut position, context(), 1.0);
    assert_eq!(position, Vec2::new(100.0, 342.0));
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position, Vec2::new(100.0, 442.0));
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_some());
    behavior.update(&mut position, context(), 0.5);
    assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    behavior.update(&mut position, context(), 0.5);
    assert!(behavior.action_finished());
    assert_eq!(position, Vec2::new(100.0, 442.0));
}

#[test]
fn projectile_fan_is_symmetric_and_lifetime_and_walls_remove_it() {
    use hollow_steps::enemy_attack::{spread_angles, Projectile};
    assert_eq!(spread_angles(3, 15.0), vec![-7.5, 0.0, 7.5]);
    assert_eq!(spread_angles(1, 15.0), vec![0.0]);
    let mut projectile = Projectile {
        position: Vec2::new(100.0, 100.0),
        velocity: Vec2::new(100.0, 0.0),
        size: Vec2::splat(10.0),
        damage: 2,
        remaining_secs: 1.0,
    };
    assert!(
        projectile
            .step(0.5, context().bounds, Vec2::splat(-100.0), Vec2::ONE)
            .0
    );
    assert!(
        !projectile
            .step(0.5, context().bounds, Vec2::splat(-100.0), Vec2::ONE)
            .0
    );
    assert_eq!(projectile.position.x, 200.0);
    projectile.remaining_secs = 100.0;
    assert!(
        !projectile
            .step(100.0, context().bounds, Vec2::splat(-100.0), Vec2::ONE)
            .0
    );
    assert_eq!(projectile.position.x, 955.0);
}

#[test]
fn projectile_sweep_hits_player_before_expiry_and_does_not_hit_beyond_it() {
    use hollow_steps::enemy_attack::Projectile;
    let shot = Projectile {
        position: Vec2::new(100.0, 100.0),
        velocity: Vec2::new(1000.0, 0.0),
        size: Vec2::splat(10.0),
        damage: 2,
        remaining_secs: 0.5,
    };
    let (alive, hit) = shot.clone().step(
        10.0,
        context().bounds,
        Vec2::new(400.0, 100.0),
        Vec2::splat(20.0),
    );
    assert!(!alive);
    assert_eq!(hit.unwrap().damage, 2);
    assert!(shot
        .clone()
        .step(
            10.0,
            context().bounds,
            Vec2::new(800.0, 100.0),
            Vec2::splat(20.0)
        )
        .1
        .is_none());
}

#[test]
fn shooting_warns_then_emits_once_with_sampled_aim() {
    let mut behavior = machine("shoot_projectile(speed: 100.0, damage: 1, lifetime_secs: 3.0, size: (10.0, 10.0), aim: AtPlayer, count: 3, spread_degrees: 15.0, telegraph_secs: 0.5)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.projectiles().is_empty());
    behavior.update(
        &mut position,
        BehaviorContext {
            player_position: Vec2::new(100.0, 200.0),
            ..context()
        },
        0.25,
    );
    assert_eq!(behavior.projectiles().len(), 3);
    assert_eq!(behavior.projectiles()[1].velocity, Vec2::new(100.0, 0.0));
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.projectiles().len(), 3);
}

#[test]
fn radial_blast_warns_then_checks_circle_radius_only_while_active() {
    let mut behavior = machine("area_blast(radius: 50.0, damage: 2, knockback: (0.0, -100.0), active_secs: 0.5, telegraph_secs: 0.5)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.attack_warning().is_some());
    assert!(behavior
        .attack_hit(position, position, Vec2::ZERO)
        .is_none());
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior
        .attack_hit(position, position + Vec2::X * 50.0, Vec2::ZERO)
        .is_some());
    assert!(behavior
        .attack_hit(position, position + Vec2::X * 50.01, Vec2::ZERO)
        .is_none());
    assert!(behavior
        .attack_hit(position, position + Vec2::splat(40.0), Vec2::ZERO)
        .is_none());
    behavior.update(&mut position, context(), 0.5);
    assert!(behavior
        .attack_hit(position, position, Vec2::ZERO)
        .is_none());
}

#[test]
fn warning_preview_exists_before_first_tick_and_ends_when_attack_fires() {
    let mut behavior = machine("area_blast(radius: 50.0, damage: 2, knockback: (0.0, -100.0), active_secs: 0.5, telegraph_secs: 0.5)");
    let size = Vec2::splat(36.0);
    let warning = behavior.warning(size).unwrap();
    assert_eq!(warning.feedback.outline_alpha, 0.3);
    assert_eq!(warning.attack.shape, AttackShape::Circle);
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.5);
    assert!(behavior.warning(size).is_none());
    let immediate = machine("area_blast(radius: 50.0, damage: 2, knockback: (0.0, -100.0), active_secs: 0.5, telegraph_secs: 0.0)");
    assert!(immediate.warning(size).is_none());
    let explicit = machine("telegraph(seconds: 0.5), area_blast(radius: 70.0, damage: 2, knockback: (0.0, 0.0), active_secs: 0.5, telegraph_secs: 0.0)");
    assert_eq!(explicit.warning(size).unwrap().attack.size, (140.0, 140.0));
}

#[test]
fn registry_exposes_every_nested_value_as_a_labeled_editor_field() {
    use hollow_steps::behavior::Action;
    use hollow_steps::definition_editor::ActionInput;
    for name in [
        "lunge",
        "jump_to",
        "slam_down",
        "shoot_projectile",
        "area_blast",
    ] {
        let kind = Action::TYPES
            .iter()
            .position(|entry| *entry == name)
            .unwrap();
        let action = Action::defaults(kind).unwrap();
        let input = ActionInput::from_action(&action);
        assert_eq!(input.build().unwrap(), action, "{name}");
        let labels = action.parameter_labels();
        assert_eq!(labels.len(), input.values.len());
        assert!(labels.iter().all(|label| !label.is_empty()));
        assert!(
            input.values.iter().all(|value| !value.contains('(')),
            "raw RON field in {name}"
        );
        assert!(labels.iter().any(|label| label.contains("outline alpha")));
        assert!(labels
            .iter()
            .any(|label| label.contains("wind-up duration")));
        for (field, _) in input.values.iter().enumerate() {
            let mut invalid = input.clone();
            invalid.values[field] = "NaN".into();
            assert!(invalid.build().is_err(), "{name}: {}", labels[field]);
        }
        let encoded = ron::to_string(&action).unwrap();
        assert_eq!(ron::from_str::<Action>(&encoded).unwrap(), action);
    }
}

#[test]
fn optional_landing_attack_can_be_enabled_and_edited_without_raw_ron() {
    use hollow_steps::behavior::Action;
    use hollow_steps::definition_editor::ActionInput;
    let kind = Action::TYPES
        .iter()
        .position(|entry| *entry == "jump_to")
        .unwrap();
    let mut input = ActionInput::new(kind).unwrap();
    let action = input.build().unwrap();
    let labels = action.parameter_labels();
    let enabled = labels
        .iter()
        .position(|label| label.contains("enabled"))
        .unwrap();
    let damage = labels
        .iter()
        .position(|label| label.contains("damage"))
        .unwrap();
    input.values[enabled] = "true".into();
    input.values[damage] = "4".into();
    match input.build().unwrap() {
        Action::JumpTo { attack, .. } => assert_eq!(attack.0.unwrap().damage, 4),
        _ => unreachable!(),
    }
}

#[test]
fn invalid_damage_durations_radius_count_and_aim_report_labeled_errors() {
    use hollow_steps::behavior::Action;
    use hollow_steps::definition_editor::ActionInput;
    for (name, field, value) in [
        ("lunge", "duration", "-1"),
        ("lunge", "damage", "-1"),
        ("lunge", "recovery", "-1"),
        ("lunge", "wind-up", "-1"),
        ("jump_to", "air time", "-1"),
        ("slam_down", "active duration", "-1"),
        ("shoot_projectile", "damage", "-1"),
        ("shoot_projectile", "lifetime", "-1"),
        ("shoot_projectile", "count", "0"),
        ("shoot_projectile", "aim mode", "Unknown"),
        ("area_blast", "radius", "0"),
        ("area_blast", "radius", "-1"),
        ("area_blast", "damage", "-1"),
        ("area_blast", "active duration", "-1"),
    ] {
        let kind = Action::TYPES
            .iter()
            .position(|entry| *entry == name)
            .unwrap();
        let mut input = ActionInput::new(kind).unwrap();
        let labels = input.build().unwrap().parameter_labels();
        let index = labels
            .iter()
            .position(|label| label.contains(field))
            .unwrap();
        input.values[index] = value.into();
        let error = input.build().unwrap_err();
        assert!(
            error.contains(field)
                || (field == "active duration" && error.contains("active seconds")),
            "{name}: {error}"
        );
    }
    let error = ron::from_str::<Action>("shoot_projectile(speed: 1.0, damage: 0, lifetime_secs: 1.0, size: (2.0, 2.0), aim: Unknown, count: 1, spread_degrees: 0.0, telegraph_secs: 0.0)").unwrap_err().to_string();
    assert!(
        error.contains("Unknown") && error.contains("AtPlayer"),
        "{error}"
    );
}

fn fixture_machine(index: usize) -> Behavior {
    use hollow_steps::behavior::{Action, State};
    let actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    Behavior::new(Arc::new(BehaviorDefinition {
        initial_state: "attack".into(),
        states: vec![State {
            name: "attack".into(),
            actions: vec![actions[index].clone()],
            transitions: vec![],
        }],
    }))
    .unwrap()
}

#[test]
fn landing_attack_is_inactive_in_air_and_expires_before_recovery_finishes() {
    let mut behavior = fixture_machine(1);
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.75);
    assert!(position.y < 442.0);
    assert!(behavior
        .attack_hit(position, position, Vec2::ZERO)
        .is_none());
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position, Vec2::new(400.0, 442.0));
    assert!(behavior
        .attack_hit(position, position, Vec2::ZERO)
        .is_some());
    behavior.update(&mut position, context(), 0.5);
    assert!(behavior
        .attack_hit(position, position, Vec2::ZERO)
        .is_none());
    assert!(!behavior.action_finished());
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.action_finished());
}

#[test]
fn projectile_manager_removes_expired_shots_after_action_finishes() {
    let mut behavior = fixture_machine(3);
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(behavior.projectiles().len(), 3);
    assert!(behavior.action_finished());
    behavior.update(&mut position, context(), 1.0);
    assert!(behavior.projectiles().is_empty());
}

#[test]
fn each_action_handles_invalid_and_extreme_time_without_leaking_active_damage() {
    for index in 0..5 {
        let mut behavior = fixture_machine(index);
        let mut position = Vec2::new(100.0, 442.0);
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            behavior.update(&mut position, context(), dt);
            assert_eq!(position, Vec2::new(100.0, 442.0));
            assert!(!behavior.action_finished());
            assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
        }
        behavior.update(&mut position, context(), f32::MAX);
        assert!(behavior.action_finished(), "fixture {index}");
        assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
        assert!(behavior.projectiles().is_empty());
        assert!(position.is_finite());
    }
}

#[test]
fn transition_interrupts_pending_and_active_attack_windows() {
    let source = r#"(initial_state: "attack", states: [
        (name: "attack", actions: [area_blast(radius: 50.0, damage: 2, knockback: (0.0, -100.0), active_secs: 2.0, telegraph_secs: 0.5)],
            transitions: [(target: "idle", condition: player_distance_less_than(10.0))]),
        (name: "idle", actions: [], transitions: [])])"#;
    for elapsed in [0.25, 0.75] {
        let mut behavior =
            Behavior::new(Arc::new(BehaviorDefinition::from_ron(source).unwrap())).unwrap();
        let mut position = Vec2::new(100.0, 442.0);
        behavior.update(&mut position, context(), elapsed);
        let close_player = position;
        behavior.update(
            &mut position,
            BehaviorContext {
                player_position: close_player,
                ..context()
            },
            0.01,
        );
        assert_eq!(behavior.state_name(), "idle");
        assert!(behavior.warning(Vec2::ONE).is_none());
        assert!(behavior.attack_hit(position, position, Vec2::ONE).is_none());
    }
}

#[test]
fn horizontal_knockback_moves_for_authored_duration_and_stops_at_walls() {
    use hollow_steps::player::PlayerInput;
    let config = PlayerConfig {
        gravity: 0.0,
        ..PlayerConfig::default()
    };
    let mut player = Player::new(Vec2::new(100.0, 200.0), config);
    assert!(player.take_attack_damage(hitbox().hit()));
    player.update(PlayerInput::default(), 0.1, context().bounds);
    assert_eq!(player.position().x, 104.0);
    player.update(PlayerInput::default(), 0.4, context().bounds);
    assert_eq!(player.position().x, 108.0);
    player.update(PlayerInput::default(), 0.4, context().bounds);
    assert_eq!(player.position().x, 108.0);
    let mut at_wall = Player::new(Vec2::new(940.0, 200.0), config);
    assert!(at_wall.take_attack_damage(hitbox().hit()));
    at_wall.update(PlayerInput::default(), 1.0, context().bounds);
    assert_eq!(at_wall.position().x, 944.0);
}

#[test]
fn attack_motion_and_projectiles_respect_authored_room_ceiling() {
    use hollow_steps::enemy_attack::Projectile;
    let bounds = context().bounds.with_top(200.0);
    let mut behavior = fixture_machine(1);
    let mut position = Vec2::new(100.0, 242.0);
    behavior.update(
        &mut position,
        BehaviorContext {
            bounds,
            player_position: Vec2::new(400.0, -100.0),
            ..context()
        },
        0.75,
    );
    assert!(position.y >= 218.0);
    let mut projectile = Projectile {
        position: Vec2::new(100.0, 250.0),
        velocity: -Vec2::Y * 100.0,
        size: Vec2::splat(10.0),
        damage: 1,
        remaining_secs: 5.0,
    };
    assert!(!projectile.step(2.0, bounds, Vec2::ZERO, Vec2::ZERO).0);
    assert_eq!(projectile.position.y, 205.0);
}

#[test]
fn mixed_legacy_and_new_attack_fields_are_rejected_instead_of_ignored() {
    use hollow_steps::behavior::Action;
    for source in [
        "jump_to(speed: 100.0, dx: 0.0, dy: 0.0, recovery_secs: -1.0)",
        "slam_down(speed: 100.0, telegraph_secs: 0.0)",
        "area_blast(count: 1, radius: 50.0, fuse_time: 1.0, damage: 1, active_secs: 0.5)",
    ] {
        assert!(ron::from_str::<Action>(source).is_err(), "{source}");
    }
}

#[test]
fn enemy_action_damage_api_includes_attack_boxes_and_closes_at_window_end() {
    use hollow_steps::enemy::{Enemy, EnemyDefinition};
    let definition =
        BehaviorDefinition::from_ron(include_str!("fixtures/attacks/reload_lunge.ron")).unwrap();
    let mut enemy = Enemy::from_definition(
        &EnemyDefinition {
            name: "attacker".into(),
            health: 3,
            size: (36.0, 36.0),
            contact_damage: 0,
            behavior: "attack".into(),
            body: None,
            death_secs: 0.0,
            movement: Default::default(),
        },
        Arc::new(definition),
        100.0,
        200.0,
        context().bounds,
    )
    .unwrap();
    enemy.update(context().player_position, 0.5, context().bounds);
    assert_eq!(enemy.action_damage(enemy.position(), Vec2::ZERO), 20);
    assert_eq!(
        enemy.action_damage(enemy.position() + Vec2::X * 31.0, Vec2::ZERO),
        0
    );
    enemy.update(context().player_position, 0.25, context().bounds);
    assert_eq!(enemy.action_damage(enemy.position(), Vec2::ZERO), 0);
}

#[test]
fn lunge_off_ledge_defers_gravity_until_recovery_then_falls_without_snapping() {
    use hollow_steps::behavior::{Action, State};
    let actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    let mut behavior = Behavior::new(Arc::new(BehaviorDefinition {
        initial_state: "attack".into(),
        states: vec![State {
            name: "attack".into(),
            actions: vec![actions[0].clone(), Action::Wait { seconds: 1.0 }],
            transitions: vec![],
        }],
    }))
    .unwrap();
    let mut position = Vec2::new(100.0, 442.0);
    let dt = 1.0 / 64.0;
    // The room API exposes a single floor: lowering it models leaving a ledge.
    let ledge = BehaviorContext {
        bounds: WorldBounds::new(0.0, 960.0, 1000.0),
        player_position: Vec2::new(400.0, 100.0),
        ..context()
    };
    behavior.update(&mut position, context(), 0.25);
    let mut velocity = 0.0;
    for frame in 0..64 {
        let previous = position;
        if frame >= 32 {
            velocity += 1400.0 * dt;
        }
        behavior.update(&mut position, ledge, dt);
        let expected_y = (previous.y + velocity * dt).min(982.0);
        assert!(
            (position.y - expected_y).abs() < 0.001,
            "frame {frame}: {position:?}, expected {expected_y}"
        );
        if frame < 32 {
            assert!((position.x - previous.x - 100.0 * dt).abs() < 0.001);
        } else {
            assert_eq!(position.x, previous.x);
        }
    }
}

#[test]
fn lunge_flat_ground_speed_distance_and_wall_collision_are_preserved() {
    for (start_x, final_x) in [(100.0, 150.0), (930.0, 942.0)] {
        let mut behavior = fixture_machine(0);
        let mut position = Vec2::new(start_x, 442.0);
        let ctx = BehaviorContext {
            player_position: Vec2::new(960.0, 442.0),
            ..context()
        };
        for frame in 0..64 {
            behavior.update(&mut position, ctx, 1.0 / 64.0);
            let travel = ((frame + 1) as f32 / 64.0 - 0.25).clamp(0.0, 0.5);
            assert_eq!(position.y, 442.0);
            assert!((position.x - (start_x + 100.0 * travel).min(942.0)).abs() < 0.001);
        }
        assert_eq!(position.x, final_x);
        assert!(behavior.action_finished());
    }
}

#[test]
fn elevated_jump_lands_exactly_before_wait_falls_without_snapping() {
    use hollow_steps::behavior::{Action, State};
    let actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    let mut behavior = Behavior::new(Arc::new(BehaviorDefinition {
        initial_state: "attack".into(),
        states: vec![State {
            name: "attack".into(),
            actions: vec![actions[1].clone(), Action::Wait { seconds: 1.0 }],
            transitions: vec![],
        }],
    }))
    .unwrap();
    let ctx = BehaviorContext {
        player_position: Vec2::new(400.0, 200.0),
        ..context()
    };
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, ctx, 1.25);
    assert_eq!(position, ctx.player_position);
    // Landing hit and recovery retain the exact authored target.
    behavior.update(&mut position, ctx, 0.75);
    assert_eq!(position, ctx.player_position);
    behavior.update(&mut position, ctx, 1.0 / 64.0);
    assert!((position.y - (200.0 + 1400.0 / (64.0 * 64.0))).abs() < 0.001);
}

#[test]
fn lunge_aims_up_once_then_falls_during_recovery_and_after_completion() {
    let mut behavior = fixture_machine(0);
    let mut position = Vec2::new(100.0, 442.0);
    let ctx = BehaviorContext {
        player_position: Vec2::new(400.0, 42.0),
        ..context()
    };
    behavior.update(&mut position, ctx, 0.25);
    assert_eq!(position, Vec2::new(100.0, 442.0));
    let moved_player = BehaviorContext {
        player_position: Vec2::new(0.0, 442.0),
        ..ctx
    };
    for frame in 1..=32 {
        behavior.update(&mut position, moved_player, 1.0 / 64.0);
        let travel = frame as f32 / 64.0;
        assert!((position.x - (100.0 + 60.0 * travel)).abs() < 0.001);
        assert!((position.y - (442.0 - 80.0 * travel)).abs() < 0.001);
    }
    let mut velocity = 0.0;
    for _ in 0..32 {
        let previous = position;
        velocity += 1400.0 / 64.0;
        behavior.update(&mut position, moved_player, 1.0 / 64.0);
        assert_eq!(position.x, 130.0);
        assert!((position.y - (previous.y + velocity / 64.0).min(442.0)).abs() < 0.001);
    }
    assert!(behavior.action_finished());
}

#[test]
fn lunge_crossing_end_applies_gravity_only_to_recovery_time() {
    let mut behavior = fixture_machine(0);
    let mut position = Vec2::new(100.0, 442.0);
    let ctx = BehaviorContext {
        player_position: Vec2::new(100.0, 42.0),
        ..context()
    };
    behavior.update(&mut position, ctx, 0.625);
    assert_eq!(position, Vec2::new(100.0, 404.5));
    behavior.update(&mut position, ctx, 0.25);
    assert_eq!(position, Vec2::new(100.0, 392.0 + 1400.0 * 0.125 * 0.125));
}
