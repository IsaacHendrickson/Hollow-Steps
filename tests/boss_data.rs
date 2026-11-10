mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hollow_steps::behavior::{Action, BehaviorDefinition};
use hollow_steps::boss_definition::{AttackSelection, BossDefinition, BossDefinitions};
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::editor::{load_extra_rooms, RoomDraft, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::player::{PlayerInput, WorldBounds};
use hollow_steps::room::{load_rooms, RoomEnemies};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const WARDEN: &str = include_str!("fixtures/bosses/ash_warden.ron");
const POUND: &str = include_str!("fixtures/bosses/ground_pound.ron");

struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-boss-data-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["bosses", "rooms", "entities", "behaviors"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
    fn write(&self, directory: &str, file: &str, source: &str) -> PathBuf {
        let path = self.0.join(directory).join(file);
        std::fs::write(&path, source).unwrap();
        path
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn error_at<'a>(error: &'a str, path: &Path) -> &'a str {
    error
        .strip_prefix(&format!("{}: ", path.display()))
        .unwrap_or_else(|| panic!("expected {}: {error}", path.display()))
}
fn runtime(source: &str) -> BossSpecials {
    BossSpecials::from_definition(
        Arc::new(BossDefinition::from_ron(source).unwrap()),
        Vec2::new(400.0, 436.0),
    )
    .unwrap()
}

#[test]
fn loads_fields_attacks_and_room_placements_without_a_hardcoded_arena_boss() {
    let definitions = BossDefinitions::load(&support::fixture_assets()).unwrap();
    let boss = definitions
        .spawn("Ash Warden", Vec2::new(300.0, 300.0))
        .unwrap();
    assert_eq!(boss.name(), "Ash Warden");
    assert_eq!(boss.health(), 100);
    assert_eq!(boss.max_health(), 100);
    assert_eq!(boss.size(), Vec2::splat(48.0));
    assert_eq!(boss.contact_damage(), 0);
    let definition = BossDefinition::from_ron(WARDEN).unwrap();
    assert_eq!(definition.selection, AttackSelection::Cycle);
    assert_eq!(definition.pause_seconds, 3.0);
    assert_eq!(
        definition
            .attacks
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["Slam", "Summon", "Rings"]
    );
    let assets = Assets::new();
    assets.write("rooms", "first.ron", "(enemies: [], boss: Some((definition: \"Ground Pound Example\", position: (300.0, 436.0))))");
    assets.write("rooms", "boss.ron", "(enemies: [])");
    let rooms = load_rooms(&assets.0).unwrap();
    assert_eq!(rooms[0].boss_special.as_ref().unwrap().health(), 10);
    assert_eq!(
        rooms[0].boss_special.as_ref().unwrap().position(),
        Vec2::new(300.0, 436.0)
    );
    assert!(rooms[0].exits_locked());
    assert!(rooms[2].boss_special.is_none());
    assert!(!rooms[2].exits_locked());
}

#[test]
fn custom_rooms_and_room_saves_preserve_placements_without_editor_controls() {
    let assets = Assets::new();
    let [_, _, room] = load_rooms(&assets.0).unwrap();
    let mut document: RoomEnemies = ron::from_str(
        "(enemies: [], boss: Some((definition: \"Ash Warden\", position: (560.0, 436.0))))",
    )
    .unwrap();
    let mut layout = RoomLayout::from_room(&room);
    layout.exits.clear();
    document.layout = Some(layout.clone());
    assets.write("rooms", "room_7.ron", &ron::to_string(&document).unwrap());
    let enemies = EnemyDefinitions::load(&assets.0).unwrap();
    let rooms = load_extra_rooms(&assets.0, &enemies).unwrap();
    assert_eq!(rooms.len(), 1);
    assert_eq!(rooms[0].boss_special.as_ref().unwrap().name(), "Ash Warden");
    let draft = RoomDraft::from_document(document.clone(), &room);
    let saved: RoomEnemies = ron::from_str(&draft.serialize(&enemies, &[]).unwrap()).unwrap();
    assert_eq!(saved.boss, document.boss);
    // The existing staged save path validates and rebuilds boss placements too.
    let content = hollow_steps::definition_editor::DefinitionDraft::load(&assets.0).unwrap();
    let (_, rebuilt) = content.prepare(&content.plan(&[]).unwrap()).unwrap();
    assert_eq!(
        rebuilt.iter().filter(|r| r.boss_special.is_some()).count(),
        2
    );
}

#[test]
fn load_errors_identify_file_boss_attack_action_and_parameter() {
    let assets = Assets::new();
    for (source, expected) in [
        (
            WARDEN.replace("health: 100", "health: 0"),
            "health must be positive",
        ),
        (
            WARDEN.replace("name: \"Ash Warden\"", "name: \" \""),
            "name must not be empty",
        ),
        (
            WARDEN.replace("size: (48.0, 48.0)", "size: (0.0, 48.0)"),
            "size must be finite and positive",
        ),
        (
            WARDEN.replace("size: (48.0, 48.0)", "size: (48.0, inf)"),
            "size must be finite and positive",
        ),
        (
            WARDEN.replace("pause_seconds: 3.0", "pause_seconds: -1.0"),
            "pause_seconds",
        ),
        (
            WARDEN.replace("pause_seconds: 3.0", "pause_seconds: NaN"),
            "pause_seconds",
        ),
        (
            WARDEN.replace("name: \"Summon\"", "name: \"Slam\""),
            "duplicate attack name",
        ),
        (
            WARDEN.replace("name: \"Summon\"", "name: \" \""),
            "empty or duplicate attack name",
        ),
        (WARDEN.replace("speed: 1000.0", "speed: 0.0"), "positive"),
        (
            WARDEN.replace("above_floor: 180.0", "above_floor: -1.0"),
            "above_floor",
        ),
        (WARDEN.replace("seconds: 0.5", "seconds: NaN"), "seconds"),
        (WARDEN.replace("radius: 56.0", "radius: -1.0"), "radius"),
        (WARDEN.replace("damage: 1", "damage: 1.5"), "Expected comma"),
        (WARDEN.replace("damage: 1", "damage: 256"), "out of bounds"),
        (WARDEN.replace("count: 3", "count: 0"), "count"),
        (WARDEN.replace("count: 3", "count: 257"), "count"),
        (WARDEN.replace("count: 3", "count: 1.5"), "count"),
        (WARDEN.replace("inset: 120.0", "inset: -1.0"), "inset"),
        (
            WARDEN.replace("selection: cycle", "selection: typo"),
            "typo",
        ),
        (WARDEN.replace("pause_seconds:", "typo:"), "typo"),
        (
            WARDEN.replace(
                "glide_to_floor(speed: 1000.0)",
                "unknown_action(speed: 1000.0)",
            ),
            "unknown_action",
        ),
        (
            WARDEN.replace(
                "glide_to_floor(speed: 1000.0)",
                "glide_to_floor(typo: 1000.0)",
            ),
            "typo",
        ),
        (
            WARDEN.replace("glide_to_floor(speed: 1000.0)", "glide_to_floor()"),
            "speed",
        ),
        (
            WARDEN.replace("glide_to_floor(speed: 1000.0)", "patrol(speed: 1000.0)"),
            "enemy-only",
        ),
        (WARDEN.replace("health: 100,", ""), "health"),
        (
            WARDEN.replace("contact_damage: 0", "contact_damage: 256"),
            "out of bounds",
        ),
        (WARDEN.replace("selection: cycle,", ""), "selection"),
        ("malformed".into(), "Expected struct"),
    ] {
        let path = assets.write("bosses", "ash_warden.ron", &source);
        let error = BossDefinitions::load(&assets.0).unwrap_err();
        assert!(error_at(&error, &path).contains(expected), "{error}");
    }
    let path = assets.write(
        "bosses",
        "ash_warden.ron",
        &WARDEN.replace("speed: 1000.0", "speed: -1.0"),
    );
    let error = BossDefinitions::load(&assets.0).unwrap_err();
    let detail = error_at(&error, &path);
    for expected in ["Ash Warden", "Slam", "action 0", "speed"] {
        assert!(detail.contains(expected), "{error}");
    }
    let mut definition = BossDefinition::from_ron(WARDEN).unwrap();
    definition.attacks.clear();
    assert!(definition
        .validate()
        .unwrap_err()
        .contains("attacks must not be empty"));
    definition = BossDefinition::from_ron(WARDEN).unwrap();
    definition.attacks[0].actions.clear();
    assert!(definition
        .validate()
        .unwrap_err()
        .contains("actions must not be empty"));
    for source in [
        POUND.replace("dy: -200.0", "dy: NaN"),
        POUND.replace("seconds: 0.5", "seconds: inf"),
        POUND.replace("speed: 100.0", "speed: 0.0"),
    ] {
        assert!(BossDefinition::from_ron(&source).is_err());
    }
}

#[test]
fn duplicate_definitions_missing_references_and_invalid_positions_are_load_errors() {
    let assets = Assets::new();
    assets.write("bosses", "z_duplicate.ron", WARDEN);
    let error = BossDefinitions::load(&assets.0).unwrap_err();
    assert!(
        error_at(&error, &assets.0.join("bosses").join("z_duplicate.ron"))
            .contains("duplicate boss name \"Ash Warden\"")
    );
    std::fs::remove_file(assets.0.join("bosses").join("z_duplicate.ron")).unwrap();
    for (reference, position, expected) in [
        (
            "absent",
            "(100.0, 100.0)",
            "missing boss definition \"absent\"",
        ),
        ("Ash Warden", "(NaN, 100.0)", "position must be finite"),
        ("Ash Warden", "(10.0, 100.0)", "position/body must fit"),
        ("Ash Warden", "(950.0, 100.0)", "position/body must fit"),
        ("Ash Warden", "(100.0, 10.0)", "position/body must fit"),
        ("Ash Warden", "(100.0, 450.0)", "position/body must fit"),
    ] {
        let path = assets.write(
            "rooms",
            "boss.ron",
            &format!(
                "(enemies: [], boss: Some((definition: {reference:?}, position: {position})))"
            ),
        );
        let error = load_rooms(&assets.0).unwrap_err();
        assert!(error_at(&error, &path).contains(expected), "{error}");
    }
    assets.write(
        "rooms",
        "boss.ron",
        "(enemies: [], boss: Some((definition: \"Ash Warden\", position: (24.0, 436.0))))",
    );
    assert!(load_rooms(&assets.0).is_ok());
}

#[test]
fn selection_cycles_wraps_and_random_is_seeded_and_can_repeat() {
    let mut cycle = runtime(WARDEN);
    for expected in ["Slam", "Summon", "Rings", "Slam"] {
        cycle.start_next_attack(Vec2::ZERO, BOUNDS);
        assert_eq!(cycle.active_attack_name(), Some(expected));
        cycle.start_next_attack(Vec2::ZERO, BOUNDS); // An active attack cannot be replaced.
        assert_eq!(cycle.active_attack_name(), Some(expected));
        cycle.update(1.0, Vec2::ZERO, BOUNDS);
    }
    let source = WARDEN.replace("selection: cycle", "selection: random");
    let mut first = runtime(&source);
    let mut second = runtime(&source);
    first.set_random_seed(1);
    second.set_random_seed(1);
    let mut selected = Vec::new();
    for _ in 0..30 {
        first.start_next_attack(Vec2::ZERO, BOUNDS);
        second.start_next_attack(Vec2::ZERO, BOUNDS);
        assert_eq!(first.active_attack_name(), second.active_attack_name());
        selected.push(first.active_attack_name().unwrap().to_owned());
        first.update(1.0, Vec2::ZERO, BOUNDS);
        second.update(1.0, Vec2::ZERO, BOUNDS);
    }
    assert_eq!(&selected[..2], ["Slam", "Slam"]);
    for name in ["Slam", "Summon", "Rings"] {
        assert!(selected.iter().any(|s| s == name));
    }
}

#[test]
fn ground_pound_runs_jump_hover_slam_then_shockwave_in_order_once() {
    let mut boss = runtime(POUND);
    let target = Vec2::new(400.0, 436.0);
    boss.start_next_attack(target, BOUNDS);
    assert_eq!(boss.position(), target);
    assert_eq!(boss.update(0.1, target, BOUNDS), 0);
    assert_eq!(boss.position(), Vec2::new(400.0, 336.0));
    assert_eq!(boss.update(0.1, target, BOUNDS), 0);
    assert_eq!(boss.position(), Vec2::new(400.0, 236.0));
    for _ in 0..2 {
        assert_eq!(boss.update(0.25, target, BOUNDS), 0);
        assert_eq!(boss.position(), Vec2::new(400.0, 236.0));
    }
    assert_eq!(boss.update(1.0, target, BOUNDS), 0);
    assert_eq!(boss.position(), Vec2::new(400.0, 336.0));
    assert_eq!(boss.update(1.0, target, BOUNDS), 2);
    assert_eq!(boss.position(), target);
    assert!(!boss.is_telegraphing());
    assert_eq!(boss.update(10.0, target, BOUNDS), 0);
    let mut overshoot = runtime(POUND);
    overshoot.start_next_attack(target, BOUNDS);
    assert_eq!(overshoot.update(10.0, target, BOUNDS), 2);
    assert_eq!(overshoot.position(), target);
}

#[test]
fn sequences_carry_unused_time_and_instant_actions_obey_file_order() {
    let mut definition = BossDefinition::from_ron(POUND).unwrap();
    definition.attacks[0].actions = vec![
        Action::Wait { seconds: 0.25 },
        Action::SpawnRings { count: 1 },
        Action::Wait { seconds: 0.25 },
        Action::ExplodeRings {
            radius: 10000.0,
            damage: 3,
        },
        Action::Summon { inset: 100.0 },
    ];
    let mut boss =
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(400.0, 436.0)).unwrap();
    boss.start_next_attack(Vec2::ZERO, BOUNDS);
    assert!(boss.rings().is_empty());
    assert_eq!(boss.update(0.375, Vec2::ZERO, BOUNDS), 0);
    assert_eq!(boss.rings().len(), 1);
    assert!(!boss.rings()[0].exploded());
    assert!(boss.summoned_enemies().is_empty());
    assert_eq!(boss.update(0.125, Vec2::ZERO, BOUNDS), 3);
    assert!(boss.rings()[0].exploded());
    assert_eq!(boss.summoned_enemies().len(), 2);
    assert_eq!(boss.update(10.0, Vec2::ZERO, BOUNDS), 0);
    assert_eq!(boss.summoned_enemies().len(), 2);
}

#[test]
fn independent_instances_invalid_time_death_and_zero_time_sequences_are_safe() {
    let definitions = BossDefinitions::load(&support::fixture_assets()).unwrap();
    let mut first = definitions
        .spawn("Ground Pound Example", Vec2::new(400.0, 436.0))
        .unwrap();
    let second = definitions
        .spawn("Ground Pound Example", Vec2::new(400.0, 436.0))
        .unwrap();
    first.start_next_attack(Vec2::new(400.0, 436.0), BOUNDS);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(first.update(dt, Vec2::ZERO, BOUNDS), 0);
        first.schedule(dt, Vec2::ZERO, BOUNDS);
        assert_eq!(first.attack_timer(), 0.0);
        assert_eq!(first.position(), second.position());
    }
    first.update(0.1, Vec2::ZERO, BOUNDS);
    assert_ne!(first.position(), second.position());
    first.damage_boss(10);
    assert_eq!(first.contact_damage(), 0);
    first.start_next_attack(Vec2::ZERO, BOUNDS);
    assert_eq!(first.update(f32::MAX, Vec2::ZERO, BOUNDS), 0);
    assert!(!first.is_telegraphing());
    assert_eq!(second.health(), 10);
    let mut definition = BossDefinition::from_ron(POUND).unwrap();
    definition.pause_seconds = 0.0;
    definition.attacks[0].actions = vec![Action::Wait { seconds: 0.0 }];
    let mut instant = BossSpecials::from_definition(Arc::new(definition), Vec2::ZERO).unwrap();
    instant.schedule(f32::MAX, Vec2::ZERO, BOUNDS);
    assert!(!instant.is_telegraphing());
    assert!(instant.attack_timer().is_finite());
}

#[test]
fn contact_damage_and_failed_boss_reload_use_the_normal_game_boundaries() {
    let assets = Assets::new();
    assets.write("rooms", "first.ron", "(enemies: [], boss: Some((definition: \"Ground Pound Example\", position: (180.0, 436.0))))");
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player().health().current(), 3);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.player().health().current(), 3);
    let before = format!("{:?}", game.room().boss_special);
    let path = assets.write(
        "bosses",
        "ground_pound.ron",
        &POUND.replace("speed: 100.0", "speed: 0.0"),
    );
    let error = game.reload_rooms().unwrap_err();
    assert!(error_at(&error, &path).contains("positive"));
    assert_eq!(format!("{:?}", game.room().boss_special), before);
    assets.write("bosses", "ground_pound.ron", POUND);
    game.reload_rooms().unwrap();
    assert_eq!(format!("{:?}", game.room().boss_special), before);
    assert_eq!(game.player().health().current(), 3);
}

#[test]
fn boss_only_actions_do_not_become_enemy_behaviors_or_editor_actions() {
    let error = BehaviorDefinition::from_ron("(initial_state: \"idle\", states: [(name: \"idle\", actions: [glide_to_floor(speed: 100.0)], transitions: [])])").unwrap_err();
    assert!(error.contains("boss-only action"));
    assert_eq!(
        Action::TYPES,
        [
            "shoot_projectile",
            "lunge",
            "move_toward_player",
            "move_away_from_player",
            "patrol",
            "wait",
            "telegraph",
            "jump_to",
            "jump_to_player",
            "hover",
            "slam_down",
            "legacy_slam_down",
            "area_blast"
        ]
    );
}

#[test]
fn configured_pause_delays_selection_and_long_sequences_are_not_replaced() {
    let mut definition = BossDefinition::from_ron(POUND).unwrap();
    definition.pause_seconds = 0.25;
    let mut boss =
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(400.0, 436.0)).unwrap();
    let player = Vec2::new(400.0, 436.0);
    boss.schedule(0.125, player, BOUNDS);
    assert!(!boss.is_telegraphing());
    boss.schedule(0.125, player, BOUNDS);
    assert_eq!(boss.active_attack_name(), Some("Ground pound"));
    boss.update(0.1, player, BOUNDS);
    boss.schedule(10.0, player, BOUNDS);
    assert_eq!(boss.position(), Vec2::new(400.0, 336.0));
    assert_eq!(boss.active_attack_name(), Some("Ground pound"));
}

#[test]
fn jump_target_is_captured_clamped_and_a_distant_player_avoids_the_shockwave() {
    let mut boss = runtime(POUND);
    boss.start_next_attack(Vec2::new(-1000.0, 436.0), BOUNDS);
    let distant_player = Vec2::new(900.0, 436.0);
    assert_eq!(boss.update(1.0, distant_player, BOUNDS), 0);
    assert_eq!(boss.position().x, 24.0);
    assert_eq!(boss.update(f32::MAX, distant_player, BOUNDS), 0);
    assert_eq!(boss.position(), Vec2::new(24.0, 436.0));
    assert!(!boss.is_telegraphing());
}

#[test]
fn shared_area_blast_uses_its_count_fuse_radius_and_damage_in_boss_sequences() {
    let mut definition = BossDefinition::from_ron(POUND).unwrap();
    definition.attacks[0].actions = vec![Action::AreaBlast {
        count: 2,
        radius: 70.0,
        fuse_time: 0.5,
        damage: 4,
    }];
    let mut boss =
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(400.0, 436.0)).unwrap();
    boss.start_next_attack(Vec2::ZERO, BOUNDS);
    assert_eq!(boss.rings().len(), 2);
    assert_eq!(boss.rings()[0].radius(), 70.0);
    let target = boss.rings()[0].position();
    assert_eq!(boss.update(0.25, target, BOUNDS), 0);
    assert!(!boss.rings()[0].exploded());
    assert_eq!(boss.update(0.25, target, BOUNDS), 4);
    assert!(boss.rings().iter().all(|r| r.exploded()));
    assert_eq!(boss.update(0.5, target, BOUNDS), 0);
}
