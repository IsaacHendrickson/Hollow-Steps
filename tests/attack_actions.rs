use hollow_steps::enemy_attack::{active_window, AttackBox, AttackShape};
use hollow_steps::player::{Player, PlayerConfig};
use macroquad::math::Vec2;

fn hitbox() -> AttackBox {
    AttackBox {
        shape: AttackShape::Rect,
        offset: (30.0, 0.0),
        size: (20.0, 20.0),
        damage: 2,
        knockback: (40.0, -50.0),
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
    assert_eq!(player.health().current(), 3);
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
