//! Hit-stop and screen shake through `GameState`: who freezes, for how long, and how it
//! interacts with pausing.
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::feel::{FeelConfig, HitStopConfig, ShakeBurst, ShakeConfig};

/// A binary fraction, so hit-stop timers count down exactly.
const DT: f32 = 1.0 / 128.0;

fn config() -> FeelConfig {
    FeelConfig {
        hit_stop: HitStopConfig {
            enemy_hit_secs: 8.0 * DT,
            player_damaged_secs: 16.0 * DT,
        },
        shake: ShakeConfig {
            enemy_hit: ShakeBurst {
                amplitude: 2.0,
                secs: 32.0 * DT,
            },
            player_damaged: ShakeBurst {
                amplitude: 8.0,
                secs: 64.0 * DT,
            },
            frequency_hz: 16.0,
        },
    }
}

/// The fixtures turn feel off (see `tests/fixtures/feel.ron`); these tests turn it on.
fn game() -> GameState {
    let mut game =
        GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
            .expect("fixed test assets must load");
    assert!(!game.feel().hit_stop_active());
    game.feel = Feel::new(config());
    game
}

/// An enemy that walks right at `speed` px/s from `x`, with `contact_damage`.
fn walker(x: f32, speed: f32, contact_damage: u16, bounds: WorldBounds) -> Enemy {
    let behavior = BehaviorDefinition::from_ron(&format!(
        r#"(initial_state: "walk", states: [
            (name: "walk", actions: [patrol(speed: {speed})], transitions: [])])"#
    ))
    .unwrap();
    Enemy::from_definition(
        &EnemyDefinition {
            name: "walker".into(),
            health: 50,
            size: (36.0, 36.0),
            contact_damage,
            behavior: "walk".into(),
            body: None,
            death_secs: 0.0,
        },
        Arc::new(behavior),
        x,
        x + 600.0,
        bounds,
    )
    .unwrap()
}

fn input() -> PlayerInput {
    PlayerInput::default()
}

fn swing() -> PlayerInput {
    PlayerInput {
        attack_pressed: true,
        ..input()
    }
}

/// A game with the player at x = 200 and a walking enemy in swing reach.
fn fight() -> GameState {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].coins.clear();
    game.rooms[0].enemies = vec![walker(235.0, 64.0, 0, bounds)];
    game
}

fn enemy_x(game: &GameState) -> f32 {
    game.room().enemies[0].position().x
}

#[test]
fn a_hit_freezes_the_enemy_for_exactly_the_configured_ticks() {
    let mut game = fight();
    game.update(swing(), DT);
    assert_eq!(game.take_events().len(), 2, "swing and hit");
    let hit_x = enemy_x(&game);
    assert!(game.feel().hit_stop_active());

    // Eight frozen ticks: no walking and no knockback slide.
    for tick in 0..8 {
        game.update(input(), DT);
        assert_eq!(enemy_x(&game), hit_x, "tick {tick} should be frozen");
    }
    assert!(!game.feel().hit_stop_active());

    // Then the knockback slide and walk resume.
    game.update(input(), DT);
    assert_ne!(enemy_x(&game), hit_x);
}

#[test]
fn the_player_keeps_moving_during_hit_stop() {
    let mut game = fight();
    game.update(swing(), DT);
    assert!(game.feel().hit_stop_active());
    let walk = PlayerInput {
        move_axis: -1.0,
        ..input()
    };
    let before = game.player.position().x;
    for _ in 0..4 {
        game.update(walk, DT);
    }
    assert!(game.feel().hit_stop_active());
    assert!(
        game.player.position().x < before,
        "the player is not frozen"
    );
}

#[test]
fn the_swing_and_the_shake_keep_their_own_clocks_during_hit_stop() {
    let mut game = fight();
    game.update(swing(), DT);
    assert!(game.player.strike_hitbox().is_some());
    assert_eq!(game.feel().shake_amplitude(), 2.0);
    // 16 ticks is longer than a swing lasts, while the enemy stood still for only 8 of them.
    for _ in 0..16 {
        game.update(input(), DT);
    }
    assert!(game.player.strike_hitbox().is_none());
    assert_eq!(
        game.feel().shake_amplitude(),
        1.0,
        "half of 32 ticks have passed"
    );
}

#[test]
fn taking_damage_freezes_enemies_for_the_damage_time() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].coins.clear();
    game.rooms[0].enemies = vec![walker(205.0, 64.0, 10, bounds)];
    game.update(input(), DT);
    assert_eq!(game.player.health().current(), 40);
    assert_eq!(game.feel().hit_stop_remaining(), 16.0 * DT);
    let x = enemy_x(&game);
    for _ in 0..16 {
        game.update(input(), DT);
        assert_eq!(enemy_x(&game), x);
    }
    game.update(input(), DT);
    assert_ne!(enemy_x(&game), x);
}

#[test]
fn the_boss_stands_still_during_hit_stop() {
    let timer = |game: &GameState| game.rooms[2].boss_special.as_ref().unwrap().attack_timer();
    let boss_game = || {
        let mut game = game();
        game.current = RoomId::Boss.index();
        game.player.place_on_floor(100.0, game.world_bounds());
        game
    };
    let mut control = boss_game();
    let mut frozen = boss_game();
    frozen.feel.apply(&[GameEvent::EnemyHit]);
    let start = timer(&frozen);
    for _ in 0..8 {
        control.update(input(), DT);
        frozen.update(input(), DT);
    }
    assert!(timer(&control) > start, "control: the boss is running");
    assert_eq!(timer(&frozen), start, "the boss waited out the hit-stop");
    frozen.update(input(), DT);
    assert!(timer(&frozen) > start, "and then resumed");
}

#[test]
fn a_boss_hit_freezes_enemies_too() {
    let mut game = game();
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(520.0, game.world_bounds());
    game.update(swing(), DT);
    assert!(game.take_events().contains(&GameEvent::EnemyHit));
    assert!(game.feel().hit_stop_active());
}

#[test]
fn pausing_neither_spends_nor_blocks_hit_stop() {
    let mut game = fight();
    game.update(swing(), DT);
    let remaining = game.feel().hit_stop_remaining();
    assert!(remaining > 0.0);

    // The inventory opens in the middle of a hit-stop, and time stands still.
    game.toggle_inventory();
    assert!(game.inventory_open());
    for _ in 0..100 {
        game.update(input(), DT);
    }
    assert_eq!(game.feel().hit_stop_remaining(), remaining);
    assert_eq!(game.shake_offset(), Vec2::ZERO, "no shake while paused");

    // It closes again, and the remaining hit-stop plays out.
    game.toggle_inventory();
    assert!(!game.inventory_open());
    let x = enemy_x(&game);
    game.update(input(), DT);
    assert_eq!(enemy_x(&game), x, "still frozen after the pause");
    assert!(game.feel().hit_stop_remaining() < remaining);
}

#[test]
fn the_editor_pauses_hit_stop_too() {
    let mut game = fight();
    game.update(swing(), DT);
    let remaining = game.feel().hit_stop_remaining();
    game.toggle_editor()
        .expect("the fixture room opens in the editor");
    assert!(game.editor().is_some());
    for _ in 0..100 {
        game.update(input(), DT);
    }
    assert_eq!(game.feel().hit_stop_remaining(), remaining);
    game.toggle_editor().unwrap();
    assert!(game.editor().is_none());
    game.update(input(), DT);
    assert!(game.feel().hit_stop_remaining() < remaining);
}

#[test]
fn zero_and_non_finite_steps_spend_no_hit_stop_and_stay_finite() {
    let mut game = fight();
    game.update(swing(), DT);
    let remaining = game.feel().hit_stop_remaining();
    let x = enemy_x(&game);
    for dt in [0.0, f32::NAN, f32::INFINITY, -1.0] {
        game.update(input(), dt);
    }
    assert_eq!(game.feel().hit_stop_remaining(), remaining);
    assert_eq!(enemy_x(&game), x);
    assert!(game.shake_offset().is_finite());
}

#[test]
fn a_very_large_step_ends_the_hit_stop_without_panicking() {
    let mut game = fight();
    game.update(swing(), DT);
    game.update(input(), 10.0);
    assert!(!game.feel().hit_stop_active());
    assert!(enemy_x(&game).is_finite());
}

#[test]
fn damage_shakes_the_screen_and_the_shake_fades_to_nothing() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].coins.clear();
    game.rooms[0].enemies = vec![walker(205.0, 0.0, 10, bounds)];
    assert_eq!(game.shake_offset(), Vec2::ZERO);
    game.update(input(), DT);
    let mut peak: f32 = 0.0;
    for _ in 0..64 {
        game.update(input(), DT);
        let offset = game.shake_offset();
        assert!(offset.length() <= 8.0 * 2.0_f32.sqrt());
        peak = peak.max(offset.length());
    }
    assert!(peak > 1.0, "the shake is visible");
    for _ in 0..8 {
        game.update(input(), DT);
    }
    assert_eq!(game.shake_offset(), Vec2::ZERO, "and it ends");
}

#[test]
fn shipped_feel_values_load_and_fixtures_turn_feel_off() {
    let shipped = FeelConfig::load(Path::new("definitely-not-an-assets-folder")).unwrap();
    assert_eq!(shipped, FeelConfig::default());
    let fixture =
        FeelConfig::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")).unwrap();
    assert_eq!(fixture.hit_stop.enemy_hit_secs, 0.0);
    assert_eq!(fixture.shake.player_damaged.amplitude, 0.0);
}

#[test]
fn a_bad_feel_file_is_an_error_naming_the_file_and_reload_keeps_the_old_values() {
    let dir = std::env::temp_dir().join(format!("hollow-steps-feel-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    // A scratch copy of the fixtures with a broken feel.ron.
    copy_dir(&fixtures, &dir);
    std::fs::write(dir.join("feel.ron"), "(hit_stop: (enemy_hit_secs: -1.0))").unwrap();
    let error = FeelConfig::load(&dir).unwrap_err();
    assert!(error.contains("feel.ron"), "{error}");
    assert!(GameState::try_new(&dir).is_err());

    // A good file reloads on F5; a broken one afterwards is refused and changes nothing.
    std::fs::write(
        dir.join("feel.ron"),
        std::fs::read_to_string(fixtures.join("feel.ron")).unwrap(),
    )
    .unwrap();
    let mut game = GameState::try_new(&dir).unwrap();
    assert_eq!(game.feel().config().hit_stop.enemy_hit_secs, 0.0);
    std::fs::write(dir.join("feel.ron"), include_str!("../../data/feel.ron")).unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(*game.feel().config(), FeelConfig::default());
    std::fs::write(dir.join("feel.ron"), "(oops").unwrap();
    assert!(game.reload_rooms().is_err());
    assert_eq!(*game.feel().config(), FeelConfig::default());
    let _ = std::fs::remove_dir_all(&dir);
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
