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

#[derive(Clone)]
struct MockActor {
    position: Vec2,
    motion: hollow_steps::action_runtime::ActorMotion,
    seed: u32,
    outputs: Vec<hollow_steps::action_runtime::ActionOutput>,
}
impl MockActor {
    fn new() -> Self {
        Self {
            position: Vec2::new(500.0, 444.0),
            motion: hollow_steps::action_runtime::ActorMotion {
                facing: 1.0,
                velocity: (0.0, 0.0),
                airborne: false,
            },
            seed: 12345,
            outputs: vec![],
        }
    }
}
impl hollow_steps::action_runtime::ActionActor for MockActor {
    fn position(&self) -> Vec2 {
        self.position
    }
    fn set_position(&mut self, position: Vec2) {
        self.position = position;
    }
    fn context(&self) -> BehaviorContext {
        context(Vec2::new(520.0, 444.0))
    }
    fn player_size(&self) -> Vec2 {
        Vec2::new(24.0, 48.0)
    }
    fn motion(&self) -> hollow_steps::action_runtime::ActorMotion {
        self.motion
    }
    fn set_motion(&mut self, motion: hollow_steps::action_runtime::ActorMotion) {
        self.motion = motion;
    }
    fn random_arena_position(&mut self) -> Vec2 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        Vec2::new((self.seed % 900) as f32, 300.0)
    }
    fn emit(&mut self, output: hollow_steps::action_runtime::ActionOutput) {
        self.outputs.push(output);
    }
}

#[test]
fn every_registered_variant_runs_in_the_single_exhaustive_runtime() {
    use hollow_steps::action_runtime::{ActionOutput, ActionRuntime};
    let mut actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    actions.extend(
        ron::from_str::<Vec<Action>>(include_str!("fixtures/timelines/legacy_actions.ron"))
            .unwrap(),
    );
    actions.extend(
        ron::from_str::<Vec<Action>>(include_str!("fixtures/actions/runtime.ron")).unwrap(),
    );
    let kinds: std::collections::BTreeSet<_> = actions.iter().map(Action::kind).collect();
    assert_eq!(kinds, (0..Action::TYPES.len()).collect());
    assert_eq!(
        actions.len(),
        Action::TYPES.len(),
        "one fixture for every variant"
    );
    for action in actions {
        let mut runtime = ActionRuntime::default();
        let mut actor = MockActor::new();
        let mut completed = false;
        for _ in 0..600 {
            let (used, finished) = runtime.step(&action, &action, &mut actor, 0.025);
            assert!((0.0..=0.025).contains(&used), "{action:?}");
            assert!(actor.position.is_finite(), "{action:?}");
            if finished {
                completed = true;
                break;
            }
        }
        assert_eq!(
            completed,
            !matches!(action, Action::Patrol { .. }),
            "{action:?}"
        );
        match action {
            Action::ShootProjectile { count, .. } => assert_eq!(
                actor
                    .outputs
                    .iter()
                    .filter(|output| matches!(output, ActionOutput::Projectile(_)))
                    .count(),
                usize::from(count)
            ),
            Action::Summon { .. } => assert_eq!(
                actor
                    .outputs
                    .iter()
                    .filter(|output| matches!(output, ActionOutput::Spawn(_)))
                    .count(),
                2
            ),
            Action::SpawnShockwave { damage, .. } | Action::DamageTarget { damage, .. } => {
                assert!(actor
                    .outputs
                    .contains(&ActionOutput::Damage(u32::from(damage))))
            }
            Action::Lunge { .. }
            | Action::JumpTo { .. }
            | Action::ImpactSlam { .. }
            | Action::RadialBlast { .. } => assert!(actor
                .outputs
                .iter()
                .any(|output| matches!(output, ActionOutput::DamageArea { .. }))),
            _ => {}
        }
        runtime.finish_action();
    }
}

#[test]
fn invalid_time_cannot_mutate_an_actor_or_emit_effects() {
    let mut runtime = hollow_steps::action_runtime::ActionRuntime::default();
    let action = Action::SpawnShockwave {
        radius: 80.0,
        damage: 2,
    };
    let mut actor = MockActor::new();
    for dt in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(runtime.step(&action, &action, &mut actor, dt), (0.0, false));
        assert_eq!(actor.position, Vec2::new(500.0, 444.0));
        assert!(actor.outputs.is_empty());
    }
    assert_eq!(runtime.step(&action, &action, &mut actor, 0.0), (0.0, true));
    assert_eq!(actor.outputs.len(), 1);
}

#[test]
fn real_enemy_and_boss_entities_share_modern_outputs() {
    use hollow_steps::enemy::{Enemy, EnemyDefinition};
    let actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    let player = Vec2::new(540.0, 444.0);
    let player_size = Vec2::new(24.0, 48.0);
    for action in actions {
        let mut definition = BehaviorDefinition::blank();
        definition.states[0].actions.push(action.clone());
        let mut enemy = Enemy::from_definition(
            &EnemyDefinition {
                name: "Fixture enemy".into(),
                health: 20,
                size: (32.0, 32.0),
                contact_damage: 0,
                behavior: "fixture".into(),
                body: None,
            },
            Arc::new(definition),
            500.0,
            500.0,
            BOUNDS,
        )
        .unwrap();
        let mut boss = boss(vec![action], enemy.position());
        boss.start_attack("Test", player, BOUNDS);
        for _ in 0..180 {
            enemy.update_with_player_size(player, player_size, 1.0 / 60.0, BOUNDS);
            boss.update_with_player_size(1.0 / 60.0, player, player_size, BOUNDS);
            assert_eq!(enemy.position(), boss.position());
            assert_eq!(
                enemy.behavior().active_damage_area(),
                boss.active_damage_area()
            );
            assert_eq!(enemy.behavior().projectiles(), boss.projectiles());
            assert_eq!(
                enemy.attack_hit(player, player_size),
                boss.attack_hit(player, player_size)
            );
        }
    }
}
