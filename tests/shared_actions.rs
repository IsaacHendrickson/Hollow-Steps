use hollow_steps::behavior::{Action, Behavior, BehaviorContext, BehaviorDefinition, State};
use hollow_steps::boss_definition::{AttackDefinition, AttackSelection, BossDefinition};
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;
use std::sync::Arc;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
fn boss(actions: Vec<Action>, position: Vec2) -> BossSpecials {
    BossSpecials::from_definition(
        Arc::new(BossDefinition {
            name: "Fixture boss".into(),
            health: 20,
            size: (32.0, 32.0),
            contact_damage: 0,
            selection: AttackSelection::Cycle,
            pause_seconds: 1.0,
            attacks: vec![AttackDefinition {
                name: "Test".into(),
                actions,
            }],
        }),
        position,
    )
    .unwrap()
}
fn behavior(actions: Vec<Action>) -> Behavior {
    Behavior::new(Arc::new(BehaviorDefinition {
        initial_state: "test".into(),
        states: vec![State {
            name: "test".into(),
            actions,
            transitions: vec![],
        }],
    }))
    .unwrap()
}
fn context(player: Vec2) -> BehaviorContext {
    BehaviorContext {
        player_position: player,
        health: 20,
        size: Vec2::splat(32.0),
        bounds: BOUNDS,
        patrol_left: 16.0,
        patrol_right: 944.0,
    }
}
#[test]
fn legacy_forms_remain_distinct_and_execute_without_data_conversion() {
    let actions: Vec<Action> =
        ron::from_str(include_str!("fixtures/timelines/legacy_actions.ron")).unwrap();
    assert!(matches!(actions[0], Action::JumpToPlayer { .. }));
    assert!(matches!(actions[1], Action::SlamDown { .. }));
    assert!(matches!(actions[2], Action::AreaBlast { .. }));
    let serialized = ron::to_string(&actions).unwrap();
    assert_eq!(ron::from_str::<Vec<Action>>(&serialized).unwrap(), actions);
    let start = Vec2::new(400.0, 444.0);
    let mut enemy_position = start;
    let mut enemy = behavior(actions.clone());
    let mut boss = boss(actions, start);
    enemy.set_random_seed(12345);
    boss.set_random_seed(12345);
    boss.start_attack("Test", start, BOUNDS);
    for _ in 0..150 {
        enemy.update(&mut enemy_position, context(start), 1.0 / 60.0);
        boss.update(1.0 / 60.0, start, BOUNDS);
        assert_eq!(enemy_position, boss.position());
    }
    assert!(enemy.action_finished());
    assert!(!boss.is_telegraphing());
    assert_eq!(boss.rings().len(), 3);
    assert!(boss.rings().iter().all(|ring| ring.exploded()));
}

#[test]
fn bosses_accept_and_execute_every_modern_action_with_enemy_geometry() {
    let actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    let player = Vec2::new(540.0, 444.0);
    let player_size = Vec2::new(24.0, 48.0);
    for action in actions {
        let start = Vec2::new(500.0, 300.0);
        let mut position = start;
        let mut enemy = behavior(vec![action.clone()]);
        let mut boss = boss(vec![action.clone()], start);
        boss.start_attack("Test", player, BOUNDS);
        for frame in 0..180 {
            enemy.update_with_player_size(&mut position, context(player), 1.0 / 60.0, player_size);
            boss.update_with_player_size(1.0 / 60.0, player, player_size, BOUNDS);
            assert_eq!(position, boss.position(), "{action:?} frame {frame}");
            assert_eq!(
                enemy.active_damage_area(),
                boss.active_damage_area(),
                "{action:?} frame {frame}"
            );
            assert_eq!(
                enemy.projectiles(),
                boss.projectiles(),
                "{action:?} frame {frame}"
            );
            assert_eq!(
                enemy.attack_hit(position, player, player_size),
                boss.attack_hit(player, player_size)
            );
        }
    }
}

#[test]
fn shockwaves_are_shared_immediate_horizontal_checks_even_high_above_player() {
    for (x, expected) in [(480.0, 2), (520.0, 2), (479.0, 0), (521.0, 0)] {
        let action = Action::SpawnShockwave {
            radius: 20.0,
            damage: 2,
        };
        let start = Vec2::new(500.0, 40.0);
        let player = Vec2::new(x, 444.0);
        let mut position = start;
        let mut enemy = behavior(vec![action.clone()]);
        let mut boss = boss(vec![action], start);
        boss.start_attack("Test", player, BOUNDS);
        enemy.update(&mut position, context(player), 0.01);
        assert_eq!(enemy.blast_damage(player, Vec2::splat(200.0)), expected);
        assert_eq!(boss.update(0.01, player, BOUNDS), u32::from(expected));
        enemy.update(&mut position, context(player), 0.01);
        assert_eq!(enemy.blast_damage(player, Vec2::splat(200.0)), 0);
        assert_eq!(boss.update(0.01, player, BOUNDS), 0);
    }
}

#[test]
fn f3_geometry_is_identical_for_shared_enemy_and_boss_damage() {
    let action: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    let player = Vec2::new(540.0, 444.0);
    let mut enemy = behavior(vec![action[4].clone()]);
    let mut position = Vec2::new(500.0, 300.0);
    let mut boss = boss(vec![action[4].clone()], position);
    boss.start_attack("Test", player, BOUNDS);
    enemy.update(&mut position, context(player), 0.6);
    boss.update(0.6, player, BOUNDS);
    let enemy_geometry = hollow_steps::body_debug::action_primitives(
        position,
        enemy.active_damage_area(),
        enemy.projectiles(),
    );
    let boss_geometry = hollow_steps::body_debug::action_primitives(
        boss.position(),
        boss.active_damage_area(),
        boss.projectiles(),
    );
    assert_eq!(enemy_geometry, boss_geometry);
    assert!(matches!(
        enemy_geometry.as_slice(),
        [hollow_steps::body_debug::OverlayPrimitive::Circle { radius: 50.0, .. }]
    ));
}
