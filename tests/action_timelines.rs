use hollow_steps::behavior::{Action, Behavior, BehaviorContext, BehaviorDefinition, State};
use hollow_steps::boss_definition::BossDefinition;
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;
use std::fmt::Write;
use std::sync::Arc;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 60.0;
fn player(frame: usize) -> Vec2 {
    Vec2::new(240.0 + (frame % 120) as f32 * 2.0, 436.0)
}
fn point(p: Vec2) -> String {
    format!("{:.3},{:.3}", p.x, p.y)
}
fn golden(name: &str, actual: String) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("timelines")
        .join(format!("{name}.txt"));
    assert_eq!(
        actual,
        std::fs::read_to_string(path).unwrap().replace("\r\n", "\n"),
        "{name}"
    );
}
#[test]
fn ash_warden_attacks() {
    let definition = Arc::new(
        BossDefinition::from_ron(include_str!("fixtures/timelines/ash_warden.ron")).unwrap(),
    );
    for name in ["Slam", "Summon", "Rings", "Dash"] {
        let mut boss =
            BossSpecials::from_definition(definition.clone(), Vec2::new(700.0, 436.0)).unwrap();
        boss.set_random_seed(12345);
        boss.start_attack(name, player(0), BOUNDS);
        let mut timeline = String::new();
        for frame in 0..180 {
            let damage = boss.update(DT, player(frame), BOUNDS);
            write!(
                timeline,
                "{frame} pos={} attack={:?} damage={damage} preview={:?}",
                point(boss.position()),
                boss.active_attack_name(),
                boss.landing_preview()
            )
            .unwrap();
            for ring in boss.rings() {
                write!(
                    timeline,
                    " circle={} radius={:.3} exploded={}",
                    point(ring.position()),
                    ring.radius(),
                    ring.exploded()
                )
                .unwrap();
            }
            for enemy in boss.summoned_enemies() {
                write!(timeline, " spawn={}", point(enemy.position())).unwrap();
            }
            writeln!(timeline).unwrap();
        }
        golden(&format!("warden_{name}"), timeline);
    }
}
#[test]
fn enemy_attack_types() {
    let mut actions: Vec<Action> = ron::from_str(include_str!("fixtures/attacks/all.ron")).unwrap();
    actions.extend(
        ron::from_str::<Vec<Action>>(include_str!("fixtures/timelines/legacy_actions.ron"))
            .unwrap(),
    );
    for (name, action) in [
        "charger_lunge",
        "hopper_jump",
        "hopper_slam",
        "shooter",
        "radial_blast",
        "legacy_jump",
        "legacy_slam",
        "legacy_blast",
    ]
    .into_iter()
    .zip(actions)
    {
        let mut behavior = Behavior::new(Arc::new(BehaviorDefinition {
            initial_state: "attack".into(),
            states: vec![State {
                name: "attack".into(),
                actions: vec![action],
                transitions: vec![],
            }],
        }))
        .unwrap();
        behavior.set_random_seed(12345);
        let mut position = Vec2::new(500.0, 300.0);
        let mut timeline = String::new();
        for frame in 0..180 {
            let context = BehaviorContext {
                player_position: player(frame),
                health: 5,
                size: Vec2::new(32.0, 32.0),
                bounds: BOUNDS,
                patrol_left: 100.0,
                patrol_right: 800.0,
            };
            behavior.update_with_player_size(&mut position, context, DT, Vec2::new(24.0, 48.0));
            write!(
                timeline,
                "{frame} pos={} finished={}",
                point(position),
                behavior.action_finished()
            )
            .unwrap();
            if let Some(area) = behavior.active_damage_area() {
                write!(
                    timeline,
                    " area={:?} center={} size={} damage={} knockback={} secs={:.3}",
                    area.shape,
                    point(area.center(position)),
                    point(area.size.into()),
                    area.damage,
                    point(area.knockback.into()),
                    area.active_secs
                )
                .unwrap();
            }
            for projectile in behavior.projectiles() {
                write!(
                    timeline,
                    " projectile={} velocity={} size={} damage={} remaining={:.3}",
                    point(projectile.position),
                    point(projectile.velocity),
                    point(projectile.size),
                    projectile.damage,
                    projectile.remaining_secs
                )
                .unwrap();
            }
            for blast in behavior.blasts() {
                write!(
                    timeline,
                    " blast={} radius={:.3} damage={} exploded={}",
                    point(blast.position),
                    blast.radius,
                    blast.damage,
                    blast.exploded
                )
                .unwrap();
            }
            writeln!(timeline).unwrap();
        }
        golden(name, timeline);
    }
}
