use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::game::GameState;
use hollow_steps::item::Item;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomId;

struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-reload-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let shipped = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
        for directory in ["behaviors", "entities", "rooms"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(shipped.join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn write(&self, file: &str, source: &str) {
        std::fs::write(self.0.join(file), source).unwrap();
    }

    fn place_enemy(&self, room: &str, x: f32) {
        self.write(
            &format!("rooms/{room}.ron"),
            &format!("(enemies: [(definition: \"chase\", patrol_left: {x}, patrol_right: {x})])"),
        );
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn walk_right(game: &mut GameState, seconds: f32) {
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        seconds,
    );
}

#[test]
fn reload_updates_all_rooms_and_reloads_referenced_definitions() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    for (room, x) in [("first", 640.0), ("second", 540.0), ("boss", 440.0)] {
        assets.place_enemy(room, x);
    }
    assets.write(
        "entities/chase.ron",
        r#"(name: "chase", health: 9, size: (36.0, 36.0), contact_damage: 0, behavior: "chase")"#,
    );
    assets.write(
        "behaviors/chase.ron",
        r#"(initial_state: "idle", states: [(name: "idle", actions: [], transitions: [])])"#,
    );

    game.reload_rooms().unwrap();

    for (room, x) in [
        (RoomId::First, 640.0),
        (RoomId::Second, 540.0),
        (RoomId::Boss, 440.0),
    ] {
        assert_eq!(game.room().id, room);
        assert_eq!(game.room().enemies.len(), 1);
        let enemy = &game.room().enemies[0];
        assert_eq!(enemy.position().x, x);
        assert_eq!(enemy.health(), 9);
        assert_eq!(enemy.contact_damage(), 0);
        let mut enemy = enemy.clone();
        let position = enemy.position();
        enemy.update(position, 10.0, game.world_bounds());
        assert_eq!(enemy.position(), position);
        walk_right(&mut game, 10.0);
    }
}

#[test]
fn reload_accepts_empty_rooms_and_preserves_player_pickups_and_pause() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    walk_right(&mut game, 120.0 / 220.0);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    walk_right(&mut game, 60.0 / 220.0);
    assert_eq!(game.player().collected_items(), &[Item::AmberShard]);
    assert_eq!(game.coin_count(), 1);
    game.toggle_inventory();
    let player = format!("{:?}", game.player());
    let coins = format!("{:?}", game.coins());
    for room in ["first", "second", "boss"] {
        assets.write(&format!("rooms/{room}.ron"), "(enemies: [])");
    }

    game.reload_rooms().unwrap();
    game.reload_rooms().unwrap();

    assert_eq!(format!("{:?}", game.player()), player);
    assert_eq!(format!("{:?}", game.coins()), coins);
    assert_eq!(game.coin_count(), 1);
    assert_eq!(game.items().len(), 1);
    assert_eq!(game.items()[0].item, Item::Dash);
    assert!(game.inventory_open());
    assert!(game.room().enemies.is_empty());
    game.toggle_inventory();
    walk_right(&mut game, 10.0);
    assert_eq!(game.room().id, RoomId::Second);
    assert!(game.room().enemies.is_empty());
    walk_right(&mut game, 10.0);
    assert_eq!(game.room().id, RoomId::Boss);
    assert!(game.room().enemies.is_empty());
}

#[test]
fn invalid_or_missing_assets_leave_the_entire_game_intact_and_allow_retry() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.0);
    let original = format!("{game:?}");
    assets.place_enemy("first", 640.0);
    for invalid in [
        "not RON",
        r#"(enemies: [(definition: "missing", patrol_left: 200.0, patrol_right: 300.0)])"#,
        r#"(enemies: [(definition: "chase", patrol_left: NaN, patrol_right: 300.0)])"#,
    ] {
        assets.write("rooms/boss.ron", invalid);
        let error = game.reload_rooms().unwrap_err();
        assert!(error.contains("boss.ron"), "{error}");
        assert_eq!(format!("{game:?}"), original);
        game.update(PlayerInput::default(), 0.0);
    }
    std::fs::remove_file(assets.0.join("rooms/boss.ron")).unwrap();
    let error = game.reload_rooms().unwrap_err();
    assert!(error.contains("boss.ron"), "{error}");
    assert_eq!(format!("{game:?}"), original);
    assets.write("rooms/boss.ron", "(enemies: [])");
    assets.write("behaviors/chase.ron", "not RON");
    let error = game.reload_rooms().unwrap_err();
    assert!(error.contains("chase.ron"), "{error}");
    assert_eq!(format!("{game:?}"), original);

    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/behaviors/chase.ron"),
        assets.0.join("behaviors/chase.ron"),
    )
    .unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(game.room().enemies[0].position().x, 640.0);
}

#[test]
fn reload_reports_errors_in_each_room_file_without_partial_changes() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    let original = format!("{game:?}");
    for room in ["first", "second", "boss"] {
        assets.write(&format!("rooms/{room}.ron"), "(typo: [])");
    }

    let error = game.reload_rooms().unwrap_err();

    for room in ["first", "second", "boss"] {
        assert!(error.contains(&format!("{room}.ron")), "{error}");
    }
    assert_eq!(format!("{game:?}"), original);
}

#[test]
fn reload_in_boss_arena_preserves_encounter_and_spawn_tracking() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    walk_right(&mut game, 10.0);
    walk_right(&mut game, 10.0);
    assert_eq!(game.room().id, RoomId::Boss);
    // Advance through Slam and Summon so spawn tracking is nonzero.
    for _ in 0..480 {
        game.update(PlayerInput::default(), 1.0 / 60.0);
    }
    assert!(!game.room().enemies.is_empty());
    let boss = format!("{:?}", game.room().boss_special);
    let player = format!("{:?}", game.player());

    game.reload_rooms().unwrap();

    assert_eq!(format!("{:?}", game.room().boss_special), boss);
    assert_eq!(format!("{:?}", game.player()), player);
    assert!(game.room().exits_locked());
    assert!(game.room().enemies.is_empty());
    for _ in 0..600 {
        game.update(PlayerInput::default(), 1.0 / 60.0);
    }
    assert!(!game.room().enemies.is_empty());
}
