//! The player numbers of `player.ron` and `vitals.ron` as the running game uses them: max health,
//! immunity after a hit, sword damage, swing time and cooldown, and mana per kill, changed through
//! the draft (the F4 path) and through the editor's copy of it (the Player panel's path).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::tuning::Field;
use crate::vitals::VitalsField;

const DT: f32 = 0.01;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A game in the first room with nothing in it, the player standing at x = 200.
fn game_at(assets: &Path) -> GameState {
    let mut game = GameState::try_new(assets).expect("fixed test assets must load");
    game.rooms[0].enemies.clear();
    game.player.place_on_floor(200.0, game.world_bounds());
    game
}

fn game() -> GameState {
    game_at(&fixtures())
}

/// A private copy of the fixtures, so saving never touches the repository.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-player-stats-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(&fixtures(), &path);
        Self(path)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

/// An enemy that stays where it is.
fn dummy(x: f32, health: u16, contact_damage: u16, bounds: WorldBounds) -> Enemy {
    let behavior = BehaviorDefinition::from_ron(
        r#"(initial_state: "idle", states: [
            (name: "idle", actions: [patrol(speed: 0.0)], transitions: [])])"#,
    )
    .unwrap();
    Enemy::from_definition(
        &EnemyDefinition {
            name: "dummy".into(),
            health,
            size: (36.0, 36.0),
            visual_size: None,
            contact_damage,
            behavior: "idle".into(),
            body: None,
            death_secs: 0.0,
            movement: Default::default(),
        },
        Arc::new(behavior),
        x,
        x,
        bounds,
    )
    .unwrap()
}

fn idle() -> PlayerInput {
    PlayerInput::default()
}

fn swing() -> PlayerInput {
    PlayerInput {
        attack_pressed: true,
        ..idle()
    }
}

fn run(game: &mut GameState, seconds: f32) {
    for _ in 0..(seconds / DT).round() as usize {
        game.update(idle(), DT);
    }
}

/// Puts a `health`-strong enemy in reach of the sword and swings once.
fn swing_at(game: &mut GameState, health: u16) {
    let bounds = game.world_bounds();
    game.rooms[0].enemies = vec![dummy(250.0, health, 0, bounds)];
    game.update(swing(), 0.0);
}

/// A `damage`-strong enemy on top of the player, for one tick.
fn hit_player(game: &mut GameState, damage: u16) {
    let bounds = game.world_bounds();
    let x = game.player.position().x;
    game.rooms[0].enemies = vec![dummy(x, 30, damage, bounds)];
    game.update(idle(), 0.0);
    game.rooms[0].enemies.clear();
}

// ---- sword damage, swing time and cooldown ------------------------------------------------

#[test]
fn the_sword_takes_the_damage_from_the_file_and_a_kill_still_gives_mana() {
    let mut game = game();
    game.set_tuning_field(Field::StrikeDamage, 25.0).unwrap();
    swing_at(&mut game, 100);
    assert_eq!(game.room().enemies[0].health(), 75);
    assert_eq!(game.vitals().mana(), 0.0);

    // More damage than health kills, and the health stops at zero rather than wrapping.
    let mut game = self::game();
    game.set_tuning_field(Field::StrikeDamage, 990.0).unwrap();
    swing_at(&mut game, 30);
    assert!(game.room().enemies.is_empty());
    assert_eq!(game.vitals().mana(), 10.0);
}

#[test]
fn the_shipped_damage_is_still_one_third_of_the_default_enemy() {
    let mut game = game();
    swing_at(&mut game, 30);
    assert_eq!(game.room().enemies[0].health(), 20);
}

#[test]
fn the_cooldown_from_the_file_decides_when_the_next_swing_may_start() {
    let mut game = game();
    game.set_tuning_field(Field::StrikeCooldown, 1.0).unwrap();
    game.update(swing(), 0.0);
    let first = game.player.attack().id();
    run(&mut game, 0.9);
    game.update(swing(), 0.0);
    assert_eq!(game.player.attack().id(), first, "still cooling down");
    run(&mut game, 0.2);
    game.update(swing(), 0.0);
    assert_eq!(game.player.attack().id(), first + 1);
}

#[test]
fn the_swing_time_from_the_file_decides_how_long_the_hitbox_lasts() {
    let mut game = game();
    game.set_tuning_field(Field::StrikeSwingSeconds, 0.5)
        .unwrap();
    game.update(swing(), 0.0);
    run(&mut game, 0.4);
    assert!(game.player.strike_hitbox().is_some());
    run(&mut game, 0.2);
    assert!(game.player.strike_hitbox().is_none());
}

// ---- health and immunity -------------------------------------------------------------------

#[test]
fn a_new_max_health_reaches_the_player_and_the_vitals_together() {
    let mut game = game();
    assert_eq!(game.player.health().max(), 50);
    assert_eq!(game.vitals().max_health(), 50.0);

    game.set_tuning_field(Field::MaxHealth, 80.0).unwrap();
    assert_eq!(game.player.health().max(), 80);
    assert_eq!(game.player.health().current(), 80, "full stays full");
    assert_eq!(game.vitals().max_health(), 80.0);
    assert_eq!(game.vitals().health(), 80.0);

    // Wounded: raising the cap gives the same amount of health, lowering it only cuts the excess.
    hit_player(&mut game, 20);
    assert_eq!(game.player.health().current(), 60);
    game.set_tuning_field(Field::MaxHealth, 100.0).unwrap();
    assert_eq!(game.player.health().current(), 80);
    assert_eq!(game.vitals().health(), 80.0);
    game.set_tuning_field(Field::MaxHealth, 40.0).unwrap();
    assert_eq!(game.player.health().current(), 40);
    assert_eq!(game.vitals().health(), 40.0);
    assert!(game.vitals().trailing_health() <= 40.0);
}

#[test]
fn dying_brings_the_player_back_at_the_new_max_health() {
    let mut game = game();
    game.set_tuning_field(Field::MaxHealth, 80.0).unwrap();
    hit_player(&mut game, 990);
    assert_eq!(game.player.health().current(), 80);
    assert_eq!(game.vitals().health(), 80.0);
}

#[test]
fn the_immunity_time_from_the_file_decides_when_the_next_hit_can_land() {
    let mut game = game();
    game.set_tuning_field(Field::InvincibilitySeconds, 0.5)
        .unwrap();
    hit_player(&mut game, 10);
    assert!(game.player.health().is_invincible());
    run(&mut game, 0.4);
    assert!(game.player.health().is_invincible());
    run(&mut game, 0.2);
    assert!(!game.player.health().is_invincible());

    // No immunity at all: a second hit lands at once.
    let mut game = self::game();
    game.set_tuning_field(Field::InvincibilitySeconds, 0.0)
        .unwrap();
    hit_player(&mut game, 10);
    hit_player(&mut game, 10);
    assert_eq!(game.player.health().current(), 30);
}

// ---- vitals through the draft ----------------------------------------------------------------

#[test]
fn mana_per_kill_set_through_f4_is_what_a_kill_gives() {
    let mut game = game();
    let changed = game
        .vitals_config()
        .with_field(VitalsField::ManaPerKill, 33.0)
        .unwrap();
    game.set_vitals_config(changed).unwrap();
    assert!(game.vitals_unsaved());
    assert!(game.player_draft().is_vitals_dirty());
    assert!(!game.tuning_unsaved(), "the stats are untouched");
    swing_at(&mut game, 10);
    assert_eq!(game.vitals().mana(), 33.0);
}

#[test]
fn mana_per_kill_set_in_the_editors_copy_is_in_effect_once_the_game_takes_it() {
    let mut game = game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    session
        .player
        .set_vitals_field(VitalsField::ManaPerKill, 40.0)
        .unwrap();
    assert_eq!(
        game.vitals_config().mana_per_kill,
        10.0,
        "not yet in effect"
    );
    game.sync_player_from_editor();
    assert_eq!(game.vitals_config().mana_per_kill, 40.0);
    assert!(game.vitals_unsaved());
    game.toggle_editor().unwrap();
    swing_at(&mut game, 10);
    assert_eq!(game.vitals().mana(), 40.0);
}

#[test]
fn an_invalid_vitals_copy_is_never_taken_into_effect() {
    let mut game = game();
    game.toggle_editor().unwrap();
    let mut bad = *game.player_draft();
    let mut config = *bad.edited_vitals();
    config.mana_start = config.max_mana + 1.0;
    assert!(bad.set_edited_vitals(config).is_err());
    assert_eq!(bad.edited_vitals(), game.vitals_config());
    assert!(game
        .set_vitals_config(config)
        .unwrap_err()
        .contains("mana_start"));
    assert!(!game.vitals_unsaved());
}

#[test]
fn saving_from_the_panel_writes_both_files_only_when_both_changed() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::MaxHealth, 70.0).unwrap();
    session
        .player
        .set_vitals_field(VitalsField::ManaPerKill, 22.0)
        .unwrap();
    let written = game.save_player().unwrap();
    let names: Vec<_> = written
        .iter()
        .map(|path| path.file_name().unwrap().to_owned())
        .collect();
    assert_eq!(names, ["player.ron", "vitals.ron"]);
    assert_eq!(
        crate::tuning::TuningConfig::load(&scratch.0)
            .unwrap()
            .player
            .max_health,
        70
    );
    assert_eq!(VitalsConfig::load(&scratch.0).unwrap().mana_per_kill, 22.0);
    assert!(!game.vitals_unsaved() && !game.tuning_unsaved());
    assert!(!game.editor().unwrap().player.is_dirty());

    // Only a stat changed this time: vitals.ron is left exactly as it was.
    let before = std::fs::read_to_string(scratch.0.join("vitals.ron")).unwrap();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::StrikeDamage, 15.0).unwrap();
    let written = game.save_player().unwrap();
    assert_eq!(written.len(), 1);
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("vitals.ron")).unwrap(),
        before
    );
}

#[test]
fn an_invalid_draft_writes_neither_file() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    game.toggle_editor().unwrap();
    let before = std::fs::read_to_string(scratch.0.join("vitals.ron")).unwrap();
    let session = game.editor_mut().unwrap();
    session
        .player
        .set_vitals_field(VitalsField::ManaPerKill, 22.0)
        .unwrap();
    // A held jump that never ends blocks the whole save, vitals included.
    session
        .player
        .set_field(Field::HeldJumpGravityMultiplier, 0.0)
        .unwrap();
    assert!(game.save_player().unwrap_err().contains("never end"));
    assert!(!scratch.0.join("player.ron").exists());
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("vitals.ron")).unwrap(),
        before
    );
}

#[test]
fn discarding_the_panels_edits_puts_back_the_saved_vitals_and_max_health() {
    let mut game = game();
    game.toggle_editor().unwrap();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::MaxHealth, 90.0).unwrap();
    session
        .player
        .set_vitals_field(VitalsField::ManaPerKill, 40.0)
        .unwrap();
    game.sync_player_from_editor();
    assert_eq!(game.player.health().max(), 90);
    assert_eq!(game.vitals().max_health(), 90.0);

    game.discard_player_edits();
    assert_eq!(game.player.health().max(), 50);
    assert_eq!(game.player.health().current(), 50);
    assert_eq!(game.vitals().max_health(), 50.0);
    assert_eq!(game.vitals_config().mana_per_kill, 10.0);
    assert!(!game.vitals_unsaved());
    assert!(!game.editor().unwrap().player.is_dirty());
}

#[test]
fn an_unsaved_vitals_edit_counts_as_an_unsaved_player_for_the_guard() {
    let mut game = game();
    let changed = game
        .vitals_config()
        .with_field(VitalsField::ManaPerKill, 15.0)
        .unwrap();
    game.set_vitals_config(changed).unwrap();
    game.toggle_editor().unwrap();
    let session = game.editor().unwrap();
    assert!(session.player.is_vitals_dirty());
    assert_eq!(
        crate::editor_guard::unsaved_besides_rooms(session),
        ["Player"]
    );
}

#[test]
fn f4_saves_the_vitals_and_a_reload_makes_the_file_the_saved_values() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    let changed = game
        .vitals_config()
        .with_field(VitalsField::ManaPerKill, 18.0)
        .unwrap();
    game.set_vitals_config(changed).unwrap();
    game.save_vitals().unwrap();
    assert!(!game.vitals_unsaved());
    assert_eq!(VitalsConfig::load(&scratch.0).unwrap().mana_per_kill, 18.0);

    let text = std::fs::read_to_string(scratch.0.join("vitals.ron"))
        .unwrap()
        .replace("mana_per_kill: 18.0", "mana_per_kill: 27.0");
    std::fs::write(scratch.0.join("vitals.ron"), text).unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(game.vitals_config().mana_per_kill, 27.0);
    assert!(!game.vitals_unsaved(), "what was just loaded is the saved");
}

#[test]
fn a_test_room_starts_at_the_edited_max_health() {
    let mut game = game();
    game.toggle_editor().unwrap();
    game.editor_mut()
        .unwrap()
        .player
        .set_field(Field::MaxHealth, 75.0)
        .unwrap();
    game.start_test_room().unwrap();
    assert_eq!(game.player.health().current(), 75);
    assert_eq!(game.vitals().health(), 75.0);
    assert_eq!(game.vitals().max_health(), 75.0);
}
