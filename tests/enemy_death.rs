//! The dying state: a defeated enemy stays for its `death_secs`, inert and fading.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hollow_steps::behavior::BehaviorDefinition;
use hollow_steps::combat::SWORD_DAMAGE;
use hollow_steps::definition_editor::{serialize, DefinitionDraft};
use hollow_steps::enemy::{Enemy, EnemyDefinition, EnemyDefinitions, DEFAULT_DEATH_SECS};
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const PATROL: &str = r#"(initial_state: "walk", states: [
    (name: "walk", actions: [patrol(speed: 80.0)], transitions: [])])"#;
const BLAST: &str = r#"(initial_state: "fire", states: [
    (name: "fire", actions: [area_blast(radius: 2000.0, damage: 2, knockback: (100.0, -100.0), active_secs: 5.0, telegraph_secs: 0.0)], transitions: [])])"#;

fn enemy(behavior: &str, health: u16, death_secs: f32) -> Enemy {
    Enemy::from_definition(
        &EnemyDefinition {
            name: "walker".into(),
            health,
            size: (36.0, 36.0),
            visual_size: None,
            contact_damage: 1,
            behavior: "walk".into(),
            body: None,
            death_secs,
        },
        Arc::new(BehaviorDefinition::from_ron(behavior).unwrap()),
        300.0,
        600.0,
        BOUNDS,
    )
    .unwrap()
}

fn kill(enemy: &mut Enemy) {
    for id in 1..=u64::from(enemy.max_health().div_ceil(SWORD_DAMAGE)) {
        assert!(enemy.hit(id, 1.0));
    }
    assert!(enemy.is_dead());
}

#[test]
fn a_defeated_enemy_stays_inert_and_fades_until_its_death_time_is_up() {
    let mut enemy = enemy(PATROL, 2, 0.5);
    enemy.update(Vec2::ZERO, 0.1, BOUNDS);
    let walked = enemy.position();
    assert_ne!(walked.x, 300.0, "it was walking");
    assert_eq!(enemy.opacity(), 1.0);

    kill(&mut enemy);
    assert!(enemy.is_dying());
    assert!(!enemy.is_removable());
    assert_eq!(enemy.opacity(), 1.0, "the fade starts at the killing blow");

    enemy.update(Vec2::ZERO, 0.25, BOUNDS);
    assert!(enemy.is_dying());
    assert_eq!(enemy.opacity(), 0.5);
    assert_eq!(enemy.position(), walked, "a dying enemy does not move");

    enemy.update(Vec2::ZERO, 0.25, BOUNDS);
    assert!(!enemy.is_dying());
    assert!(enemy.is_removable());
    assert_eq!(enemy.opacity(), 0.0);
}

#[test]
fn a_dying_enemy_cannot_be_touched_or_hit_again() {
    let mut enemy = enemy(PATROL, 1, 0.5);
    kill(&mut enemy);
    assert!(enemy.is_dying());
    assert!(!enemy.touches(enemy.position(), Vec2::new(32.0, 48.0)));
    assert!(!enemy.hit(99, 1.0));
    assert_eq!(enemy.health(), 0);
    enemy.update(Vec2::ZERO, 0.1, BOUNDS);
    assert!(!enemy.hit(100, -1.0));
    assert_eq!(enemy.health(), 0);
}

#[test]
fn a_dying_enemy_deals_no_attack_or_blast_damage() {
    let player = Vec2::new(300.0, 436.0);
    let size = Vec2::new(32.0, 48.0);
    let mut enemy = enemy(BLAST, 1, 1.0);
    enemy.update(player, 0.1, BOUNDS);
    assert!(
        enemy.action_damage(player, size) > 0,
        "control: a live enemy's blast hurts"
    );

    kill(&mut enemy);
    assert_eq!(enemy.action_damage(player, size), 0);
    assert!(enemy.attack_hit(player, size).is_none());
    for _ in 0..5 {
        enemy.update(player, 0.1, BOUNDS);
        assert!(enemy.is_dying());
        assert_eq!(enemy.action_damage(player, size), 0);
        assert!(enemy.attack_hit(player, size).is_none());
    }
}

#[test]
fn a_dying_enemy_runs_no_behavior() {
    let mut enemy = enemy(PATROL, 1, hollow_steps::enemy::MAX_DEATH_SECS);
    kill(&mut enemy);
    let at = enemy.position();
    let facing = enemy.facing_left();
    for _ in 0..40 {
        enemy.update(Vec2::new(10.0, 436.0), 0.1, BOUNDS);
    }
    assert!(enemy.is_dying(), "4 s of a 5 s death");
    assert_eq!(enemy.position(), at);
    assert_eq!(enemy.facing_left(), facing);
}

#[test]
fn zero_death_secs_means_gone_the_moment_health_runs_out() {
    let mut enemy = enemy(PATROL, 1, 0.0);
    assert!(!enemy.is_removable());
    kill(&mut enemy);
    assert!(enemy.is_removable());
    assert!(!enemy.is_dying());
    assert_eq!(enemy.opacity(), 0.0);
}

#[test]
fn code_made_enemies_are_removed_at_once_as_before() {
    let mut enemy = Enemy::new(300.0, 600.0, 0.0, 0.0, BOUNDS);
    kill(&mut enemy);
    assert!(enemy.is_removable());
}

#[test]
fn an_enemy_that_is_alive_is_never_dying_or_removable_whatever_the_time() {
    let mut enemy = enemy(PATROL, 3, 0.1);
    for _ in 0..100 {
        enemy.update(Vec2::ZERO, 0.1, BOUNDS);
    }
    assert!(!enemy.is_dead() && !enemy.is_dying() && !enemy.is_removable());
    assert_eq!(enemy.opacity(), 1.0);
}

#[test]
fn zero_negative_and_non_finite_steps_do_not_advance_the_death_and_a_huge_one_ends_it() {
    let mut enemy = enemy(PATROL, 1, 0.5);
    kill(&mut enemy);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        enemy.update(Vec2::ZERO, dt, BOUNDS);
        assert!(enemy.is_dying(), "dt {dt}");
        assert_eq!(enemy.opacity(), 1.0, "dt {dt}");
    }
    enemy.update(Vec2::ZERO, 1_000.0, BOUNDS);
    assert!(enemy.is_removable());
    enemy.update(Vec2::ZERO, 1_000.0, BOUNDS);
    assert!(enemy.is_removable());
    assert_eq!(enemy.opacity(), 0.0);
}

struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-death-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["entities", "behaviors", "rooms", "bosses", "bodies"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn chase(&self) -> PathBuf {
        self.0.join("entities").join("chase.ron")
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn spawn_killed(assets: &Assets) -> Enemy {
    let mut enemy = EnemyDefinitions::load(&assets.0)
        .unwrap()
        .spawn("chase", 560.0, 800.0, BOUNDS)
        .unwrap();
    kill(&mut enemy);
    enemy
}

#[test]
fn a_definition_file_without_death_secs_uses_the_default() {
    let assets = Assets::new();
    let text = std::fs::read_to_string(assets.chase()).unwrap();
    assert!(
        !text.contains("death_secs"),
        "the fixture predates the field"
    );
    let mut enemy = spawn_killed(&assets);
    enemy.update(Vec2::ZERO, DEFAULT_DEATH_SECS - 0.05, BOUNDS);
    assert!(enemy.is_dying());
    enemy.update(Vec2::ZERO, 0.1, BOUNDS);
    assert!(enemy.is_removable());
}

#[test]
fn death_secs_survives_a_save_and_reload() {
    let assets = Assets::new();
    let mut entity: EnemyDefinition =
        ron::from_str(&std::fs::read_to_string(assets.chase()).unwrap()).unwrap();
    entity.death_secs = 1.5;
    let text = serialize(&entity).unwrap();
    assert!(text.contains("death_secs: 1.5"), "{text}");
    std::fs::write(assets.chase(), text).unwrap();

    let mut enemy = spawn_killed(&assets);
    enemy.update(Vec2::ZERO, 1.4, BOUNDS);
    assert!(enemy.is_dying());
    enemy.update(Vec2::ZERO, 0.2, BOUNDS);
    assert!(enemy.is_removable());
}

#[test]
fn a_bad_death_secs_in_a_definition_file_is_refused_with_the_field_named() {
    let assets = Assets::new();
    for bad in ["-1.0", "99.0"] {
        let mut entity: EnemyDefinition =
            ron::from_str(&std::fs::read_to_string(assets.chase()).unwrap()).unwrap();
        entity.death_secs = bad.parse().unwrap();
        let text = serialize(&entity).unwrap_or_else(|_| {
            format!(
                "(name: \"chase\", health: 1, size: (36.0, 36.0), contact_damage: 1, behavior: \"chase\", death_secs: {bad})"
            )
        });
        std::fs::write(assets.chase(), text).unwrap();
        let error = EnemyDefinitions::load(&assets.0).unwrap_err();
        assert!(error.contains("death_secs"), "{error}");
    }
}

#[test]
fn a_new_blank_enemy_starts_with_the_default_death_time() {
    let assets = Assets::new();
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let path = draft.add_blank_entity("fresh").unwrap();
    assert_eq!(draft.entities[&path].death_secs, DEFAULT_DEATH_SECS);
}
