use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::enemy::{EnemyDefinition, EnemyDefinitions};
use hollow_steps::game::GameState;
use hollow_steps::health::Health;
use hollow_steps::player::{PlayerInput, WorldBounds};
use hollow_steps::room::{load_room_enemies, load_rooms};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const PATROL: &str = r#"(initial_state: "patrol", states: [(name: "patrol", actions: [patrol(speed: 100.0)], transitions: [])])"#;
const ENTITY: &str =
    r#"(name: "guard", health: 8, size: (60.0, 80.0), contact_damage: 2, behavior: "patrol")"#;

struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-assets-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["behaviors", "entities", "rooms"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
        }
        let assets = Self(path);
        assets.write("behaviors/patrol.ron", PATROL);
        assets.write("entities/guard.ron", ENTITY);
        assets
    }

    fn write(&self, file: &str, source: &str) {
        std::fs::write(self.0.join(file), source).unwrap();
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn shipped_assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

fn error_for_path<'a>(error: &'a str, path: &Path) -> &'a str {
    error
        .strip_prefix(&format!("{}: ", path.display()))
        .unwrap_or_else(|| panic!("expected error for {}: {error}", path.display()))
}

#[test]
fn entity_fields_control_health_size_contact_damage_and_patrol() {
    let assets = Assets::new();
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let mut enemy = definitions.spawn("guard", 100.0, 200.0, BOUNDS).unwrap();
    assert_eq!(enemy.name(), "guard");
    assert_eq!(enemy.health(), 8);
    assert_eq!(enemy.max_health(), 8);
    assert_eq!(enemy.size(), Vec2::new(60.0, 80.0));
    assert_eq!(enemy.contact_damage(), 2);
    assert_eq!(enemy.position(), Vec2::new(100.0, 420.0));
    assert!(enemy.touches(Vec2::new(130.0, 460.0), Vec2::ZERO));
    assert!(!enemy.touches(Vec2::new(131.0, 460.0), Vec2::ZERO));
    enemy.update(Vec2::ZERO, 0.25, BOUNDS);
    assert_eq!(enemy.position(), Vec2::new(125.0, 420.0));
    assert!(enemy.hit(1, -1.0));
    assert_eq!(enemy.health(), 7);
}

#[test]
fn variable_contact_damage_is_atomic_respects_immunity_and_saturates() {
    let mut health = Health::default();
    assert!(!health.take_damage_amount(0));
    assert!(!health.is_invincible());
    assert!(health.take_damage_amount(2));
    assert_eq!(health.current(), 3);
    assert!(!health.take_damage_amount(2));
    health.update(1.0);
    assert!(health.take_damage_amount(u8::MAX));
    assert_eq!(health.current(), 0);
    health.update(1.0);
    assert!(!health.take_damage_amount(1));
}

#[test]
fn game_loads_room_enemies_and_uses_definition_contact_damage() {
    let assets = Assets::new();
    assets.write(
        "rooms/first.ron",
        r#"(enemies: [(definition: "guard", patrol_left: 180.0, patrol_right: 180.0)])"#,
    );
    assets.write("rooms/second.ron", "(enemies: [])");
    assets.write("rooms/boss.ron", "(enemies: [])");
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player().health().current(), 3);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player().health().current(), 3);
    game.update(PlayerInput::default(), 1.0);
    assert_eq!(game.player().health().current(), 1);
}

#[test]
fn shipped_chase_preserves_detection_boundary_return_and_repeated_pursuit() {
    let definitions = EnemyDefinitions::load(&shipped_assets()).unwrap();
    let mut enemy = definitions.spawn("chase", 560.0, 800.0, BOUNDS).unwrap();
    // Exactly 180 units is patrol; just inside it immediately chases left.
    enemy.update(Vec2::new(380.0, 442.0), 0.5, BOUNDS);
    assert_eq!(enemy.position().x, 605.0);
    enemy.update(Vec2::new(500.0, 442.0), 1.0, BOUNDS);
    assert_eq!(enemy.position().x, 515.0);
    enemy.update(Vec2::ZERO, 0.25, BOUNDS);
    assert_eq!(enemy.position().x, 537.5);
    // Reaching the player must not permanently stop future pursuit.
    enemy.update(Vec2::new(550.0, 442.0), 0.5, BOUNDS);
    assert_eq!(enemy.position().x, 550.0);
    enemy.update(Vec2::new(600.0, 442.0), 0.5, BOUNDS);
    assert_eq!(enemy.position().x, 595.0);
}

#[test]
fn shipped_patrol_ignores_player_and_all_rooms_load_definition_references() {
    let definitions = EnemyDefinitions::load(&shipped_assets()).unwrap();
    let mut enemy = definitions.spawn("patrol", 100.0, 200.0, BOUNDS).unwrap();
    enemy.update(Vec2::new(90.0, 442.0), 0.5, BOUNDS);
    assert_eq!(enemy.position().x, 145.0);
    assert_eq!(enemy.behavior().state_name(), "patrol");
    let rooms = load_rooms(&shipped_assets()).unwrap();
    assert_eq!(rooms[0].enemies.len(), 1);
    assert_eq!(rooms[0].enemies[0].name(), "chase");
    assert!(rooms[1].enemies.is_empty());
    assert!(rooms[2].enemies.is_empty());
}

#[test]
fn shared_definitions_have_independent_runtime_state() {
    let definitions = EnemyDefinitions::load(&shipped_assets()).unwrap();
    let mut first = definitions.spawn("chase", 100.0, 200.0, BOUNDS).unwrap();
    let second = definitions.spawn("chase", 100.0, 200.0, BOUNDS).unwrap();
    first.update(Vec2::new(50.0, 442.0), 0.25, BOUNDS);
    assert_eq!(first.behavior().state_name(), "chase");
    assert_eq!(second.behavior().state_name(), "patrol");
    assert_eq!(second.position().x, 100.0);
    assert_eq!(second.behavior().state_elapsed(), 0.0);
}

#[test]
fn entity_validation_rejects_empty_names_health_size_and_behavior() {
    let valid: EnemyDefinition = ron::from_str(ENTITY).unwrap();
    for (definition, expected) in [
        (
            EnemyDefinition {
                name: " ".into(),
                ..valid.clone()
            },
            "name must not be empty",
        ),
        (
            EnemyDefinition {
                health: 0,
                ..valid.clone()
            },
            "health must be positive",
        ),
        (
            EnemyDefinition {
                size: (0.0, 1.0),
                ..valid.clone()
            },
            "size must be finite and positive",
        ),
        (
            EnemyDefinition {
                size: (1.0, -1.0),
                ..valid.clone()
            },
            "size must be finite and positive",
        ),
        (
            EnemyDefinition {
                size: (f32::NAN, 1.0),
                ..valid.clone()
            },
            "size must be finite and positive",
        ),
        (
            EnemyDefinition {
                size: (1.0, f32::INFINITY),
                ..valid.clone()
            },
            "size must be finite and positive",
        ),
        (
            EnemyDefinition {
                behavior: "".into(),
                ..valid.clone()
            },
            "missing behavior name",
        ),
    ] {
        assert!(definition.validate().unwrap_err().contains(expected));
    }
}

#[test]
fn missing_behavior_and_duplicate_entity_errors_include_file_and_name() {
    let assets = Assets::new();
    assets.write(
        "entities/guard.ron",
        &ENTITY.replace("\"patrol\"", "\"absent\""),
    );
    let error = EnemyDefinitions::load(&assets.0).unwrap_err();
    let message = error_for_path(&error, &assets.0.join("entities").join("guard.ron"));
    assert!(
        message.contains("missing behavior name \"absent\""),
        "{error}"
    );
    assets.write("entities/guard.ron", ENTITY);
    assets.write("entities/duplicate.ron", ENTITY);
    let error = EnemyDefinitions::load(&assets.0).unwrap_err();
    // Sorted loading encounters duplicate.ron first, then rejects guard.ron.
    let message = error_for_path(&error, &assets.0.join("entities").join("guard.ron"));
    assert!(
        message.contains("duplicate entity name \"guard\""),
        "{error}"
    );
}

#[test]
fn behavior_validation_and_parse_errors_identify_the_file() {
    let assets = Assets::new();
    assets.write(
        "behaviors/patrol.ron",
        &PATROL.replace(
            "transitions: []",
            "transitions: [(target: \"absent\", condition: action_finished)]",
        ),
    );
    let error = EnemyDefinitions::load(&assets.0).unwrap_err();
    let path = assets.0.join("behaviors").join("patrol.ron");
    let message = error_for_path(&error, &path);
    assert!(
        message.contains("transition to missing state \"absent\""),
        "{error}"
    );
    assets.write("behaviors/patrol.ron", "malformed");
    let error = EnemyDefinitions::load(&assets.0).unwrap_err();
    let message = error_for_path(&error, &path);
    assert!(message.starts_with("1:"), "{error}");
    assert!(
        message.contains("Expected struct `BehaviorDefinition`"),
        "{error}"
    );
}

#[test]
fn entity_missing_fields_unknown_fields_and_malformed_files_are_errors() {
    let assets = Assets::new();
    for (source, expected, detail) in [
        (
            ENTITY.replace(", behavior: \"patrol\"", ""),
            "missing field",
            "behavior",
        ),
        (
            ENTITY.replace("behavior:", "typo:"),
            "Unexpected field",
            "typo",
        ),
        (
            ENTITY.replace("health: 8", "health: 0"),
            "health must be positive",
            "guard",
        ),
        ("not RON".into(), "Expected struct", "EnemyDefinition"),
    ] {
        assets.write("entities/guard.ron", &source);
        let error = EnemyDefinitions::load(&assets.0).unwrap_err();
        let message = error_for_path(&error, &assets.0.join("entities").join("guard.ron"));
        assert!(message.contains(expected), "{error}");
        assert!(message.contains(detail), "{error}");
    }
}

#[test]
fn room_references_and_placement_ranges_are_validated_with_file_context() {
    let assets = Assets::new();
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let path = assets.0.join("rooms").join("first.ron");
    for (source, expected) in [
        (
            r#"(enemies: [(definition: "unknown", patrol_left: 100.0, patrol_right: 200.0)])"#,
            "missing enemy definition \"unknown\"",
        ),
        (
            r#"(enemies: [(definition: "guard", patrol_left: 10.0, patrol_right: 200.0)])"#,
            "invalid patrol interval",
        ),
        (
            r#"(enemies: [(definition: "guard", patrol_left: 200.0, patrol_right: 100.0)])"#,
            "invalid patrol interval",
        ),
        (
            r#"(enemies: [(definition: "guard", patrol_left: NaN, patrol_right: 200.0)])"#,
            "invalid patrol interval",
        ),
        (
            r#"(enemies: [(definition: "guard", patrol_left: 100.0, patrol_right: 950.0)])"#,
            "invalid patrol interval",
        ),
    ] {
        assets.write("rooms/first.ron", source);
        let error = load_room_enemies(&path, &definitions, BOUNDS).unwrap_err();
        let message = error_for_path(&error, &path);
        assert!(message.starts_with("enemy 0:"), "{error}");
        assert!(message.contains(expected), "{error}");
    }
    assets.write(
        "rooms/first.ron",
        r#"(enemies: [(definition: "guard", patrol_left: 30.0, patrol_right: 930.0)])"#,
    );
    assert_eq!(
        load_room_enemies(&path, &definitions, BOUNDS)
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn missing_directories_and_room_files_return_paths() {
    let assets = Assets::new();
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let path = assets.0.join("rooms").join("missing.ron");
    let error = load_room_enemies(&path, &definitions, BOUNDS).unwrap_err();
    assert!(!error_for_path(&error, &path).is_empty(), "{error}");
    let error = EnemyDefinitions::load(&assets.0.join("missing")).unwrap_err();
    let path = assets.0.join("missing").join("behaviors");
    assert!(!error_for_path(&error, &path).is_empty(), "{error}");
}
