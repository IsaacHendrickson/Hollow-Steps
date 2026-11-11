mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::enemy::Enemy;
use hollow_steps::game::GameState;
use hollow_steps::item::Item;
use hollow_steps::player::{PlayerConfig, PlayerInput, WorldBounds};
use hollow_steps::tuning::{self, Field, TuningConfig, FILE_NAME};
use macroquad::math::Vec2;

const SHIPPED: &str = include_str!("../data/player.ron");
const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);

/// A private copy of the fixture assets, so tests can add and change `player.ron`.
struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-tuning-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let fixtures = support::fixture_assets();
        for directory in ["behaviors", "entities", "rooms", "bosses"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(fixtures.join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn write_tuning(&self, text: &str) {
        std::fs::write(self.0.join(FILE_NAME), text).unwrap();
    }

    fn write_shipped_with(&self, old: &str, new: &str) {
        assert!(SHIPPED.contains(old), "{old}");
        self.write_tuning(&SHIPPED.replace(old, new));
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn walk(game: &mut GameState, seconds: f32) {
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        seconds,
    );
}

#[test]
fn without_a_player_file_the_game_uses_the_embedded_defaults() {
    let game = GameState::try_new(&support::fixture_assets()).unwrap();
    assert_eq!(*game.tuning(), TuningConfig::default());
    assert_eq!(*game.player().config(), PlayerConfig::default());
    assert!(!game.tuning_unsaved());
}

#[test]
fn a_player_file_in_assets_replaces_the_defaults() {
    let assets = Assets::new();
    assets.write_shipped_with("move_speed: 220.0", "move_speed: 330.0");
    let mut game = assets.game();
    assert_eq!(game.player().config().move_speed, 330.0);
    let start = game.player().position().x;
    walk(&mut game, 0.5);
    assert!((game.player().position().x - start - 165.0).abs() < 0.01);
}

#[test]
fn an_invalid_player_file_is_an_error_naming_the_file_not_a_silent_fallback() {
    for (old, new) in [
        ("move_speed: 220.0", "move_speed: 0.0"),
        ("gravity: 1400.0", "gravity: 1400.0, wind: 1.0"),
        ("dash_speed: 800.0,", ""),
        ("default_health: 3", "default_health: 0"),
    ] {
        let assets = Assets::new();
        assets.write_shipped_with(old, new);
        let error = GameState::try_new(&assets.0).unwrap_err();
        assert!(error.contains(FILE_NAME), "{error}");
    }
    let assets = Assets::new();
    assets.write_tuning("not ron at all");
    assert!(GameState::try_new(&assets.0)
        .unwrap_err()
        .contains(FILE_NAME));
}

#[test]
fn the_dash_uses_the_configured_speed() {
    let assets = Assets::new();
    assets.write_shipped_with("dash_speed: 800.0", "dash_speed: 400.0");
    let mut game = assets.game();
    walk(&mut game, 320.0 / 220.0);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert_eq!(game.player().collected_items(), &[Item::Dash]);
    game.update(
        PlayerInput {
            dash_pressed: true,
            ..PlayerInput::default()
        },
        0.1,
    );
    // 180 start + 320 walked + 400 px/s for 0.1 s.
    assert!((game.player().position().x - 540.0).abs() < 0.001);
}

#[test]
fn dash_duration_and_cooldown_come_from_the_config() {
    let assets = Assets::new();
    assets.write_tuning(
        &SHIPPED
            .replace("dash_duration: 0.2", "dash_duration: 0.05")
            .replace("dash_cooldown: 0.5", "dash_cooldown: 0.3"),
    );
    let mut game = assets.game();
    walk(&mut game, 320.0 / 220.0);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    let dash = PlayerInput {
        dash_pressed: true,
        ..PlayerInput::default()
    };
    let idle = PlayerInput::default();
    let x = |game: &GameState| game.player().position().x;
    let start = x(&game);
    game.update(dash, 0.05);
    assert!(
        (x(&game) - start - 40.0).abs() < 0.001,
        "0.05 s at 800 px/s"
    );
    game.update(idle, 0.1);
    let after = x(&game);
    game.update(dash, 0.05);
    assert_eq!(
        x(&game),
        after,
        "still cooling down 0.2 s after the first dash"
    );
    game.update(idle, 0.2);
    game.update(dash, 0.05);
    assert!(
        x(&game) > after,
        "cooldown is over 0.4 s after the first dash"
    );
}

#[test]
fn reload_rereads_the_file_without_resetting_the_player() {
    let assets = Assets::new();
    let mut game = assets.game();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: true,
            jump_held: true,
            ..PlayerInput::default()
        },
        0.05,
    );
    let (position, velocity) = (game.player().position(), game.player().velocity());
    assert!(velocity.y < 0.0, "mid-jump");

    assets.write_shipped_with("gravity: 1400.0", "gravity: 2800.0");
    game.reload_rooms().unwrap();
    assert_eq!(game.player().position(), position);
    assert_eq!(game.player().velocity(), velocity);
    assert_eq!(game.player().config().gravity, 2800.0);
    assert_eq!(game.tuning().player.gravity, 2800.0);
    assert!(!game.tuning_unsaved());

    // The new gravity is in effect on the next tick (held jump halves it).
    game.update(
        PlayerInput {
            jump_held: true,
            ..PlayerInput::default()
        },
        0.05,
    );
    assert!((game.player().velocity().y - (velocity.y + 2800.0 * 0.5 * 0.05)).abs() < 0.01);
}

#[test]
fn a_bad_file_on_reload_changes_nothing() {
    let assets = Assets::new();
    let mut game = assets.game();
    walk(&mut game, 0.1);
    let (position, tuning) = (game.player().position(), *game.tuning());
    assets.write_shipped_with("jump_speed: 480.0", "jump_speed: 99999.0");
    let error = game.reload_rooms().unwrap_err();
    assert!(
        error.contains(FILE_NAME) && error.contains("jump_speed"),
        "{error}"
    );
    assert_eq!(game.player().position(), position);
    assert_eq!(*game.tuning(), tuning);
    assert_eq!(*game.player().config(), tuning.player);
}

#[test]
fn removing_the_file_and_reloading_restores_the_defaults() {
    let assets = Assets::new();
    assets.write_shipped_with("move_speed: 220.0", "move_speed: 330.0");
    let mut game = assets.game();
    std::fs::remove_file(assets.0.join(FILE_NAME)).unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(*game.tuning(), TuningConfig::default());
}

#[test]
fn set_tuning_applies_at_once_refuses_invalid_values_and_keeps_the_player() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    walk(&mut game, 0.2);
    let position = game.player().position();
    let faster = game.tuning().with_field(Field::MoveSpeed, 440.0).unwrap();
    game.set_tuning(faster).unwrap();
    assert_eq!(game.player().position(), position);
    assert!(game.tuning_unsaved());
    walk(&mut game, 0.25);
    assert!((game.player().position().x - position.x - 110.0).abs() < 0.01);

    let mut broken = faster;
    broken.player.gravity = f32::NAN;
    assert!(game.set_tuning(broken).is_err());
    broken.player.gravity = -5.0;
    assert!(game.set_tuning(broken).is_err());
    assert_eq!(*game.tuning(), faster);
    assert_eq!(game.player().config().gravity, faster.player.gravity);
}

#[test]
fn a_taller_player_keeps_its_feet_on_the_floor() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    walk(&mut game, 0.1);
    let feet = |game: &GameState| game.player().position().y + game.player().size().y / 2.0;
    let before = feet(&game);
    let tall = game.tuning().with_field(Field::Height, 96.0).unwrap();
    game.set_tuning(tall).unwrap();
    assert_eq!(game.player().size().y, 96.0);
    assert!((feet(&game) - before).abs() < 0.001);
    let short = tall.with_field(Field::Height, 24.0).unwrap();
    game.set_tuning(short).unwrap();
    assert!((feet(&game) - before).abs() < 0.001);
}

#[test]
fn enemies_in_the_room_take_the_knockback_and_a_reload_keeps_them_current() {
    let assets = Assets::new();
    let mut game = assets.game();
    let slow = game
        .tuning()
        .with_field(Field::EnemyKnockbackSeconds, 0.5)
        .unwrap();
    game.set_tuning(slow).unwrap();
    assert!(!game.room().enemies.is_empty());
    assert!(game
        .room()
        .enemies
        .iter()
        .all(|enemy| enemy.tuning() == slow.enemy));

    assets.write_shipped_with("knockback_speed: 260.0", "knockback_speed: 100.0");
    game.reload_rooms().unwrap();
    assert!(game
        .room()
        .enemies
        .iter()
        .all(|enemy| enemy.tuning().knockback_speed == 100.0));
}

#[test]
fn enemy_knockback_uses_the_configured_speed_and_time() {
    let mut enemy = Enemy::new(500.0, 500.0, 0.0, 0.0, BOUNDS);
    let tuning = TuningConfig::default()
        .with_field(Field::EnemyKnockbackSpeed, 100.0)
        .unwrap()
        .with_field(Field::EnemyKnockbackSeconds, 0.5)
        .unwrap()
        .enemy;
    enemy.set_tuning(tuning);
    let start = enemy.position().x;
    assert!(enemy.hit(1, 1.0));
    enemy.update(Vec2::new(0.0, 0.0), 0.25, BOUNDS);
    assert!((enemy.position().x - start - 25.0).abs() < 0.001);
    enemy.update(Vec2::new(0.0, 0.0), 0.25, BOUNDS);
    assert!((enemy.position().x - start - 50.0).abs() < 0.001);
}

#[test]
fn spawned_enemies_take_the_configured_health_but_definition_enemies_keep_theirs() {
    let tuning = TuningConfig::default()
        .with_field(Field::EnemyDefaultHealth, 7.0)
        .unwrap()
        .enemy;
    let spawned = Enemy::new(300.0, 300.0, 0.0, 0.0, BOUNDS).spawned_with(tuning);
    assert_eq!((spawned.health(), spawned.max_health()), (7, 7));
    assert_eq!(spawned.tuning(), tuning);

    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    let authored = game.room().enemies[0].health();
    let mut changed = *game.tuning();
    changed.enemy = tuning;
    game.set_tuning(changed).unwrap();
    assert_eq!(game.room().enemies[0].health(), authored);
}

#[test]
fn saving_writes_a_file_that_loads_back_and_clears_the_unsaved_flag() {
    let assets = Assets::new();
    let mut game = assets.game();
    let edited = game
        .tuning()
        .with_field(Field::DashCooldown, 1.25)
        .unwrap()
        .with_field(Field::EnemyDefaultHealth, 5.0)
        .unwrap();
    game.set_tuning(edited).unwrap();
    assert!(game.tuning_unsaved());
    let path = game.save_tuning().unwrap();
    assert_eq!(path.file_name(), Some(FILE_NAME.as_ref()));
    assert!(!game.tuning_unsaved());
    assert_eq!(TuningConfig::load(&assets.0).unwrap(), edited);
    assert_eq!(*assets.game().tuning(), edited);
    assert!(
        !assets.0.join("player.ron.tmp").exists(),
        "staging file is cleaned up"
    );
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains('\r'), "LF line endings");

    // Saving again replaces the file.
    game.set_tuning(edited.with_field(Field::DashCooldown, 0.75).unwrap())
        .unwrap();
    game.save_tuning().unwrap();
    assert_eq!(
        TuningConfig::load(&assets.0).unwrap().player.dash_cooldown,
        0.75
    );
}

#[test]
fn a_failed_save_reports_the_file_and_leaves_the_flag_and_old_file_alone() {
    let assets = Assets::new();
    assets.write_tuning(SHIPPED);
    let mut game = assets.game();
    let edited = game.tuning().with_field(Field::Gravity, 2000.0).unwrap();
    game.set_tuning(edited).unwrap();

    // A directory where the staging file belongs makes the write fail part way.
    std::fs::create_dir(assets.0.join("player.ron.tmp")).unwrap();
    let error = game.save_tuning().unwrap_err();
    assert!(error.contains("player.ron"), "{error}");
    assert!(game.tuning_unsaved());
    assert_eq!(
        std::fs::read_to_string(assets.0.join(FILE_NAME)).unwrap(),
        SHIPPED
    );
    std::fs::remove_dir(assets.0.join("player.ron.tmp")).unwrap();

    let missing = assets.0.join("nowhere");
    assert!(tuning::save(&missing, &edited)
        .unwrap_err()
        .contains("player.ron"));
}

#[test]
fn an_invalid_config_is_never_written() {
    let assets = Assets::new();
    let mut broken = TuningConfig::default();
    broken.player.move_speed = f32::INFINITY;
    assert!(tuning::save(&assets.0, &broken).is_err());
    assert!(!assets.0.join(FILE_NAME).exists());
}
