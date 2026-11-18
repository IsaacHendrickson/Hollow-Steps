//! Charms through `GameState`: what wearing one does to the player and the vitals, that the base
//! numbers (`player.ron`, `vitals.ron`, F4, the Player panel) never see it, and that it survives
//! reloads, edits, death and room changes. The rules themselves are tested in `crate::charms`.
//! The charms come from the fixed `tests/fixtures/charms.ron`: heart (+10 max health, 15 mana),
//! swift (+15 % speed, 10), sharp (+5 sword damage, 20) and ward (+20 max health, 5, not owned).
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::charms::Toggle;
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

/// A private copy of the fixtures, so saving and reloading never touch the repository.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-charms-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(&fixtures(), &path);
        Self(path)
    }

    fn write(&self, name: &str, text: &str) {
        std::fs::write(self.0.join(name), text).unwrap();
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
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-2, "{a} is not close to {b}");
}

/// An enemy that stays where it is and cannot hurt the player, with 30 health.
fn dummy(x: f32, bounds: WorldBounds) -> Enemy {
    let behavior = BehaviorDefinition::from_ron(
        r#"(initial_state: "idle", states: [
            (name: "idle", actions: [patrol(speed: 0.0)], transitions: [])])"#,
    )
    .unwrap();
    Enemy::from_definition(
        &EnemyDefinition {
            name: "dummy".into(),
            health: 30,
            size: (36.0, 36.0),
            visual_size: None,
            contact_damage: 0,
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

fn walk_right() -> PlayerInput {
    PlayerInput {
        move_axis: 1.0,
        ..PlayerInput::default()
    }
}

/// Fills the mana bar: a sword kill gives `mana_per_kill` (10 in the fixtures).
fn fill_mana(game: &mut GameState) {
    for _ in 0..20 {
        game.vitals.gain_kill_mana();
    }
}

fn set_base_max_mana(game: &mut GameState, max_mana: f32) {
    let config = game
        .vitals_config()
        .with_field(VitalsField::MaxMana, max_mana)
        .unwrap();
    game.set_vitals_config(config).unwrap();
}

// ---- a new run -----------------------------------------------------------------------------

#[test]
fn a_new_game_owns_the_starting_charms_and_wears_none() {
    let game = game();
    assert_eq!(game.charm_state().owned(), ["heart", "swift", "sharp"]);
    assert!(game.charm_state().equipped().is_empty());
    assert_eq!(game.charm_catalog().charms().len(), 4);
    // Nothing worn: the player and the vitals run on the base numbers.
    assert_eq!(*game.player().config(), game.tuning().player);
    assert_eq!(game.vitals().config(), game.vitals_config());
}

// ---- health --------------------------------------------------------------------------------

#[test]
fn the_heart_charm_raises_max_health_by_its_amount_and_unequipping_lowers_it() {
    let mut game = game();
    let base = game.tuning().player.max_health;
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped);
    assert_eq!(game.player().health().max(), base + 10);
    assert_eq!(game.vitals().max_health(), f32::from(base + 10));
    // A raised maximum adds the same health: a full player stays full.
    assert_eq!(game.player().health().current(), base + 10);
    assert_eq!(game.vitals().health(), f32::from(base + 10));
    assert_eq!(game.toggle_charm("heart"), Toggle::Unequipped);
    assert_eq!(game.player().health().max(), base);
    assert_eq!(game.player().health().current(), base);
    assert_eq!(game.vitals().max_health(), f32::from(base));
    assert_eq!(game.vitals().health(), f32::from(base));
}

#[test]
fn a_hurt_player_gains_the_same_health_and_unequipping_refills_nothing() {
    let mut game = game();
    game.player.take_damage_amount(20);
    game.vitals.damage(20.0);
    assert_eq!(game.player().health().current(), 30);
    game.toggle_charm("heart");
    assert_eq!(game.player().health().current(), 40);
    assert_eq!(game.vitals().health(), 40.0);
    game.toggle_charm("heart");
    assert_eq!(game.player().health().current(), 40);
    assert_eq!(game.player().health().max(), 50);
    assert_eq!(game.vitals().health(), 40.0);
}

#[test]
fn unequipping_cuts_health_that_is_above_the_lowered_maximum() {
    let mut game = game();
    game.toggle_charm("heart");
    assert_eq!(game.player().health().current(), 60);
    game.player.take_damage_amount(5);
    game.vitals.damage(5.0);
    game.toggle_charm("heart");
    assert_eq!(game.player().health().current(), 50);
    assert_eq!(game.vitals().health(), 50.0);
}

// ---- mana ----------------------------------------------------------------------------------

#[test]
fn equipping_lowers_max_mana_by_the_cost_and_clamps_the_mana_in_it() {
    let mut game = game();
    fill_mana(&mut game);
    assert_eq!(
        (game.vitals().mana(), game.vitals().max_mana()),
        (100.0, 100.0)
    );
    game.toggle_charm("heart");
    assert_eq!(
        (game.vitals().mana(), game.vitals().max_mana()),
        (85.0, 85.0)
    );
    game.toggle_charm("swift");
    assert_eq!(
        (game.vitals().mana(), game.vitals().max_mana()),
        (75.0, 75.0)
    );
    // Unequipping raises the maximum and refills nothing.
    game.toggle_charm("heart");
    game.toggle_charm("swift");
    assert_eq!(
        (game.vitals().mana(), game.vitals().max_mana()),
        (75.0, 100.0)
    );
}

#[test]
fn mana_below_the_new_maximum_is_left_alone() {
    let mut game = game();
    game.vitals.gain_kill_mana();
    game.vitals.gain_kill_mana();
    game.toggle_charm("sharp");
    assert_eq!(
        (game.vitals().mana(), game.vitals().max_mana()),
        (20.0, 80.0)
    );
}

// ---- refusals ------------------------------------------------------------------------------

#[test]
fn a_charm_over_the_budget_is_refused_and_nothing_changes() {
    let mut game = game();
    set_base_max_mana(&mut game, 30.0);
    assert_eq!(game.toggle_charm("heart"), Toggle::Equipped); // 15
    assert_eq!(game.toggle_charm("swift"), Toggle::Equipped); // 25
    let (state, player, vitals) = (
        game.charm_state().clone(),
        *game.player().config(),
        game.vitals().clone(),
    );
    let Toggle::Refused(message) = game.toggle_charm("sharp") else {
        panic!("25 + 20 mana is more than 30");
    };
    assert!(message.contains("15 more"), "{message}");
    assert_eq!(*game.charm_state(), state);
    assert_eq!(*game.player().config(), player);
    assert_eq!(*game.vitals(), vitals);
}

#[test]
fn a_charm_that_is_not_owned_cannot_be_worn() {
    let mut game = game();
    let Toggle::Refused(message) = game.toggle_charm("ward") else {
        panic!("the ward is not owned");
    };
    assert!(message.contains("do not own"), "{message}");
    assert!(game.charm_state().equipped().is_empty());
    assert!(matches!(game.toggle_charm("no-such"), Toggle::Refused(_)));
}

#[test]
fn unequipping_always_works() {
    let mut game = game();
    set_base_max_mana(&mut game, 30.0);
    game.toggle_charm("heart");
    game.toggle_charm("swift");
    // The base maximum drops below what is worn: the cap bottoms out at zero, not below.
    set_base_max_mana(&mut game, 1.0);
    assert_eq!(game.vitals().max_mana(), 0.0);
    assert_eq!(game.toggle_charm("heart"), Toggle::Unequipped);
    assert_eq!(game.toggle_charm("swift"), Toggle::Unequipped);
    assert_eq!(game.vitals().max_mana(), 1.0);
}

// ---- speed, damage and combinations ----------------------------------------------------------

#[test]
fn the_swift_charm_makes_the_player_walk_faster() {
    let distance = |game: &mut GameState| {
        let start = game.player().position().x;
        for _ in 0..20 {
            game.update(walk_right(), DT);
        }
        game.player().position().x - start
    };
    let mut plain = game();
    let walked = distance(&mut plain);
    let mut swift = game();
    swift.toggle_charm("swift");
    close(
        swift.player().config().move_speed,
        swift.tuning().player.move_speed * 1.15,
    );
    close(distance(&mut swift), walked * 1.15);
}

#[test]
fn the_sharp_charm_adds_to_sword_damage() {
    let hit = |worn: bool| {
        let mut game = game();
        let bounds = game.world_bounds();
        game.rooms[0].enemies = vec![dummy(235.0, bounds)];
        if worn {
            game.toggle_charm("sharp");
        }
        game.update(
            PlayerInput {
                attack_pressed: true,
                ..PlayerInput::default()
            },
            DT,
        );
        30 - game.room().enemies[0].health()
    };
    assert_eq!(hit(false), 10);
    assert_eq!(hit(true), 15);
}

#[test]
fn two_charms_add_up() {
    let mut game = game();
    let base = *game.tuning();
    game.toggle_charm("heart");
    game.toggle_charm("sharp");
    assert_eq!(
        game.player().config().max_health,
        base.player.max_health + 10
    );
    assert_eq!(
        game.player().config().melee.damage,
        base.player.melee.damage + 5
    );
    assert_eq!(game.vitals().max_mana(), 100.0 - 15.0 - 20.0);
    close(game.player().config().move_speed, base.player.move_speed);
}

// ---- the base numbers ------------------------------------------------------------------------

#[test]
fn the_base_numbers_never_see_the_charms_and_saves_write_the_base() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    let (tuning, vitals) = (*game.tuning(), *game.vitals_config());
    for id in ["heart", "swift", "sharp"] {
        assert_eq!(game.toggle_charm(id), Toggle::Equipped);
    }
    // What F4 and the Player panel read is still the base, and nothing looks edited.
    assert_eq!(*game.tuning(), tuning);
    assert_eq!(*game.vitals_config(), vitals);
    assert_eq!(*game.player_draft().edited(), tuning);
    assert!(!game.tuning_unsaved() && !game.vitals_unsaved());
    assert_ne!(game.player().config().max_health, tuning.player.max_health);
    // Saving writes the base values, not the ones in effect.
    game.save_tuning().unwrap();
    game.save_vitals().unwrap();
    assert_eq!(TuningConfig::load(&scratch.0).unwrap(), tuning);
    assert_eq!(VitalsConfig::load(&scratch.0).unwrap(), vitals);
    assert_eq!(*game.player_draft().saved(), tuning);
}

#[test]
fn an_edited_but_unsaved_base_keeps_the_charms_on_top() {
    let mut game = game();
    game.toggle_charm("heart");
    game.toggle_charm("swift");
    game.set_tuning_field(Field::MaxHealth, 70.0).unwrap();
    game.set_tuning_field(Field::MoveSpeed, 300.0).unwrap();
    set_base_max_mana(&mut game, 80.0);
    assert_eq!(game.tuning().player.max_health, 70);
    assert_eq!(game.player().config().max_health, 80);
    close(game.player().config().move_speed, 300.0 * 1.15);
    assert_eq!(game.vitals().max_mana(), 80.0 - 25.0);
    assert!(game.tuning_unsaved() && game.vitals_unsaved());
    // The charms' cost is held against the edited base: 25 worn, 55 left of 80.
    assert_eq!(game.toggle_charm("sharp"), Toggle::Equipped);
    assert_eq!(game.vitals().max_mana(), 80.0 - 45.0);
}

#[test]
fn the_editor_panel_edits_and_discards_keep_the_charms_on_top() {
    let mut game = game();
    game.toggle_charm("heart");
    game.toggle_editor().unwrap();
    // The panel's copy is the base numbers, charms not included.
    assert_eq!(game.editor().unwrap().player.edited().player.max_health, 50);
    game.editor_mut()
        .unwrap()
        .player
        .set_field(Field::MaxHealth, 70.0)
        .unwrap();
    game.sync_player_from_editor();
    assert_eq!(game.player().config().max_health, 80);
    game.discard_player_edits();
    assert_eq!(game.player().config().max_health, 60);
    assert_eq!(game.editor().unwrap().player.edited().player.max_health, 50);
    assert!(game.charm_state().is_equipped("heart"));
}

#[test]
fn f5_reload_rereads_the_base_and_keeps_the_charms_on_top() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    game.toggle_charm("heart");
    game.toggle_charm("swift");
    // An unsaved edit is replaced by the file, as before; the charms stay on the new numbers.
    game.set_tuning_field(Field::MaxHealth, 70.0).unwrap();
    let mut tuning = TuningConfig::default();
    tuning.player.max_health = 80;
    tuning.player.move_speed = 300.0;
    scratch.write("player.ron", &tuning.to_ron().unwrap());
    let mut vitals = VitalsConfig::load(&scratch.0).unwrap();
    vitals.max_mana = 60.0;
    scratch.write("vitals.ron", &vitals.to_ron().unwrap());
    game.reload_rooms().unwrap();
    assert_eq!(game.tuning().player.max_health, 80);
    assert_eq!(game.player().config().max_health, 90);
    close(game.player().config().move_speed, 300.0 * 1.15);
    assert_eq!(game.vitals_config().max_mana, 60.0);
    assert_eq!(game.vitals().max_mana(), 60.0 - 25.0);
    assert!(!game.tuning_unsaved());
    assert_eq!(game.charm_state().equipped(), ["heart", "swift"]);
}

#[test]
fn a_reload_drops_charms_the_file_no_longer_lists() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    game.toggle_charm("heart");
    game.toggle_charm("sharp");
    let only_heart = std::fs::read_to_string(fixtures().join("charms.ron"))
        .unwrap()
        .replace("id: \"sharp\"", "id: \"blade\"");
    scratch.write("charms.ron", &only_heart);
    game.reload_rooms().unwrap();
    assert_eq!(game.charm_state().equipped(), ["heart"]);
    assert_eq!(game.charm_state().owned(), ["heart", "swift"]);
    assert_eq!(game.vitals().max_mana(), 85.0);
    assert_eq!(game.player().config().melee.damage, 10);
}

#[test]
fn a_bad_charms_file_stops_the_game_and_a_reload_changes_nothing() {
    let scratch = Scratch::new();
    let mut game = game_at(&scratch.0);
    game.toggle_charm("heart");
    let before = (game.charm_state().clone(), *game.player().config());
    // A cost above the base max mana (100) is refused, and the error names the file.
    scratch.write(
        "charms.ron",
        &std::fs::read_to_string(fixtures().join("charms.ron"))
            .unwrap()
            .replace("mana_cost: 20.0", "mana_cost: 120.0"),
    );
    let error = game.reload_rooms().unwrap_err();
    assert!(
        error.contains("charms.ron") && error.contains("mana_cost"),
        "{error}"
    );
    assert_eq!(
        (game.charm_state().clone(), *game.player().config()),
        before
    );
    let error = GameState::try_new(&scratch.0).unwrap_err();
    assert!(error.contains("charms.ron"), "{error}");
}

// ---- death, rooms and the test room ----------------------------------------------------------

#[test]
fn respawning_keeps_the_charms_and_starts_at_the_raised_maximum() {
    let mut game = game();
    game.toggle_charm("heart");
    game.toggle_charm("swift");
    game.player.take_damage_amount(u16::MAX);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.charm_state().equipped(), ["heart", "swift"]);
    assert_eq!(game.player().health().current(), 60);
    assert_eq!(game.vitals().health(), 60.0);
    assert_eq!(game.vitals().max_health(), 60.0);
    assert_eq!(game.vitals().max_mana(), 75.0);
    close(
        game.player().config().move_speed,
        game.tuning().player.move_speed * 1.15,
    );
}

#[test]
fn travelling_between_rooms_keeps_the_charms() {
    let mut game = game();
    game.toggle_charm("heart");
    game.toggle_charm("sharp");
    game.update(walk_right(), 10.0);
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.charm_state().equipped(), ["heart", "sharp"]);
    assert_eq!(game.player().config().max_health, 60);
    assert_eq!(game.vitals().max_mana(), 65.0);
    game.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.charm_state().equipped(), ["heart", "sharp"]);
    assert_eq!(game.player().config().melee.damage, 15);
}

#[test]
fn a_test_room_plays_with_the_charms_and_undoes_changes_made_in_it() {
    let mut game = game();
    game.toggle_charm("heart");
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    // The run starts with the charms on: raised health, lowered mana.
    assert_eq!(game.vitals().max_health(), 60.0);
    assert_eq!(game.vitals().max_mana(), 85.0);
    game.toggle_charm("heart");
    game.toggle_charm("sharp");
    assert_eq!(game.vitals().max_health(), 50.0);
    game.end_test_room().unwrap();
    assert_eq!(game.charm_state().equipped(), ["heart"]);
    assert_eq!(game.player().config().max_health, 60);
    assert_eq!(game.vitals().max_mana(), 85.0);
    assert_eq!(game.player().config().melee.damage, 10);
}
