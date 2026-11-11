//! The vitals through `GameState`: what a sword hit or kill gives, how a heal changes movement
//! and input, what damage does to it, and how F4 / F5 reach the values. The rules themselves
//! are tested in `crate::vitals`.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::boss_definition::BossDefinition;
use crate::boss_special::BossSpecials;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::item::Item;
use crate::vitals::{HealPhase, VitalsField};

const DT: f32 = 0.01;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A game in the first room with nothing in it, the player standing at x = 200.
fn game() -> GameState {
    let mut game = GameState::try_new(&fixtures()).expect("fixed test assets must load");
    game.rooms[0].enemies.clear();
    game.player.place_on_floor(200.0, game.world_bounds());
    game
}

/// A private copy of the fixtures, so saving and reloading never touch the repository.
struct Scratch(PathBuf);

impl Scratch {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-vitals-game-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(&fixtures(), &path);
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).expect("fixed test assets must load")
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

fn held() -> PlayerInput {
    PlayerInput {
        heal_held: true,
        ..idle()
    }
}

fn run(game: &mut GameState, input: PlayerInput, seconds: f32) {
    for _ in 0..(seconds / DT).round() as usize {
        game.update(input, DT);
    }
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.05,
        "{actual} is not close to {expected}"
    );
}

/// Takes `amount` of health the way damage does, without the immunity timer or knockback, so a
/// test can hurt the player and then hurt it again.
fn wound(game: &mut GameState, amount: f32) {
    game.vitals.damage(amount);
    game.player
        .set_health_current(game.vitals.health().ceil() as u16);
}

fn give_mana(game: &mut GameState, kills: usize) {
    for _ in 0..kills {
        game.vitals.gain_kill_mana();
    }
}

/// Real damage through the game's damage path: a `damage`-strong enemy on top of the player,
/// for one tick of `input`.
fn hit_player_with(game: &mut GameState, damage: u16, input: PlayerInput) {
    let bounds = game.world_bounds();
    let x = game.player.position().x;
    game.rooms[0].enemies = vec![dummy(x, 30, damage, bounds)];
    game.update(input, 0.0);
    game.rooms[0].enemies.clear();
}

fn hit_player(game: &mut GameState, damage: u16) {
    hit_player_with(game, damage, idle());
}

/// A wounded player with `kills` kills' worth of mana, holding the heal button until `phase`.
fn healing(phase: HealPhase, kills: usize) -> GameState {
    let mut game = game();
    wound(&mut game, 30.0);
    give_mana(&mut game, kills);
    match phase {
        HealPhase::Idle => {}
        HealPhase::Starting => run(&mut game, held(), 0.10),
        HealPhase::Channeling => run(&mut game, held(), 0.50),
        HealPhase::Stopping => {
            run(&mut game, held(), 0.50);
            game.update(idle(), DT);
        }
    }
    assert_eq!(game.vitals().heal_phase(), phase);
    game
}

// ---- sword hits and kills -----------------------------------------------------------------

#[test]
fn a_sword_kill_gives_mana_per_kill_and_a_plain_hit_gives_none() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.rooms[0].enemies = vec![dummy(250.0, 30, 0, bounds)];
    game.update(swing(), 0.0);
    assert_eq!(game.room().enemies[0].health(), 20);
    assert_eq!(game.vitals().mana(), 0.0, "a hit that does not kill");

    let mut game = game_with_kill();
    assert_eq!(game.vitals().mana(), 10.0);
    // The same swing never counts twice.
    game.update(swing(), 0.0);
    assert_eq!(game.vitals().mana(), 10.0);
}

/// One enemy killed by one sword swing.
fn game_with_kill() -> GameState {
    let mut game = game();
    let bounds = game.world_bounds();
    game.rooms[0].enemies = vec![dummy(250.0, 10, 0, bounds)];
    game.update(swing(), 0.0);
    assert!(game.room().enemies.is_empty());
    game
}

#[test]
fn several_kills_add_up_and_stop_at_the_mana_cap() {
    let mut game = game();
    let bounds = game.world_bounds();
    for kill in 1..=12 {
        game.update(idle(), crate::combat::ATTACK_COOLDOWN);
        game.rooms[0].enemies = vec![dummy(250.0, 10, 0, bounds)];
        game.update(swing(), 0.0);
        assert_eq!(game.vitals().mana(), (kill as f32 * 10.0).min(100.0));
    }
}

#[test]
fn a_kill_by_anything_but_the_sword_gives_no_mana() {
    let mut game = game();
    let bounds = game.world_bounds();
    // Dead before the swing arrives: nothing to hit, nothing to gain.
    let mut corpse = dummy(250.0, 10, 0, bounds);
    corpse.hit(1, 1.0);
    game.rooms[0].enemies = vec![corpse];
    game.update(swing(), 0.0);
    assert_eq!(game.vitals().mana(), 0.0);
}

#[test]
fn a_sword_hit_regains_part_of_the_trailing_gap_and_the_player_health_follows() {
    let mut game = game();
    hit_player(&mut game, 20);
    assert_eq!(game.player.health().current(), 30);
    assert_eq!(game.vitals().health(), 30.0);
    assert_eq!(game.vitals().trailing_health(), 50.0);

    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(250.0, 30, 0, bounds)];
    game.update(swing(), 0.0);
    // Half of the 20-health gap comes back; the trailing bar does not move.
    assert_eq!(game.vitals().health(), 40.0);
    assert_eq!(game.vitals().trailing_health(), 50.0);
    assert_eq!(game.player.health().current(), 40);
}

#[test]
fn every_enemy_a_swing_hits_regains_once_and_a_second_pass_of_the_swing_does_not() {
    let mut game = game();
    hit_player(&mut game, 40);
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![
        dummy(250.0, 30, 0, bounds),
        dummy(240.0, 30, 0, bounds),
        dummy(400.0, 30, 0, bounds),
    ];
    game.update(swing(), 0.0);
    // Two enemies in reach: 10 -> 30 -> 40.
    assert_eq!(game.vitals().health(), 40.0);
    game.update(swing(), 0.0);
    assert_eq!(game.vitals().health(), 40.0, "same swing, no more regain");
}

#[test]
fn a_hit_with_nothing_to_regain_changes_nothing() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.rooms[0].enemies = vec![dummy(250.0, 30, 0, bounds)];
    game.update(swing(), 0.0);
    assert_eq!(game.vitals().health(), 50.0);
    assert_eq!(game.player.health().current(), 50);
}

/// The arena boss with its attacks pushed out and its contact damage off, so only the sword
/// matters. The player stands in sword reach.
fn boss_game() -> GameState {
    let mut game = game();
    game.current = RoomId::Boss.index();
    let mut definition =
        BossDefinition::from_ron(include_str!("../../tests/fixtures/bosses/ground_pound.ron"))
            .unwrap();
    definition.pause_seconds = 1_000.0;
    definition.contact_damage = 0;
    game.rooms[RoomId::Boss.index()].boss_special = Some(
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(560.0, FLOOR_Y - 24.0))
            .unwrap(),
    );
    game.player.place_on_floor(520.0, game.world_bounds());
    game
}

#[test]
fn a_boss_hit_regains_health_but_even_a_boss_kill_gives_no_mana() {
    let mut game = boss_game();
    wound(&mut game, 30.0);
    game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_mut()
        .unwrap()
        .damage_boss(80);
    game.update(swing(), 0.0);
    let boss = game.room().boss_special.as_ref().unwrap();
    assert_eq!(boss.health(), 10);
    assert_eq!(game.vitals().health(), 35.0, "half of the 30 gap");
    assert_eq!(game.player.health().current(), 35);
    assert_eq!(game.vitals().mana(), 0.0);

    game.update(idle(), crate::combat::ATTACK_COOLDOWN);
    game.update(swing(), 0.0);
    assert!(game.room().boss_special.as_ref().unwrap().is_defeated());
    assert_eq!(game.vitals().mana(), 0.0, "bosses give no mana");
    assert_eq!(game.vitals().health(), 42.5);
}

// ---- healing: input and movement ----------------------------------------------------------

#[test]
fn healing_walks_at_the_configured_fraction_of_the_speed() {
    let speed = game().tuning().player.move_speed;
    let walk = PlayerInput {
        move_axis: 1.0,
        ..idle()
    };
    let step = |game: &mut GameState, input: PlayerInput| {
        let before = game.player.position().x;
        game.update(input, DT);
        game.player.position().x - before
    };

    let mut game = healing(HealPhase::Idle, 5);
    close(step(&mut game, walk), speed * DT);

    let mut game = healing(HealPhase::Starting, 5);
    let healing_walk = PlayerInput {
        heal_held: true,
        ..walk
    };
    for phase in [HealPhase::Starting, HealPhase::Channeling] {
        if phase == HealPhase::Channeling {
            run(&mut game, held(), 0.3);
        }
        assert_eq!(game.vitals().heal_phase(), phase);
        let moved = step(&mut game, healing_walk);
        assert!(
            (moved - speed * DT * 0.5).abs() < 1e-3,
            "{phase:?}: {moved} vs {}",
            speed * DT * 0.5
        );
    }
    // The stop delay slows too, and normal speed is back once the heal is over.
    game.update(idle(), DT);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Stopping);
    assert!((step(&mut game, walk) - speed * DT * 0.5).abs() < 1e-3);
    run(&mut game, idle(), 0.3);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Idle);
    close(step(&mut game, walk), speed * DT);
}

#[test]
fn healing_blocks_jump_dash_and_attack_in_every_phase_and_they_work_again_afterwards() {
    for phase in [
        HealPhase::Starting,
        HealPhase::Channeling,
        HealPhase::Stopping,
    ] {
        let mut game = healing(phase, 5);
        game.player.collect_item(Item::Dash);
        let bounds = game.world_bounds();
        game.rooms[0].enemies = vec![dummy(250.0, 30, 0, bounds)];
        let swing_id = game.player.attack().id();
        game.take_events();

        let everything = PlayerInput {
            jump_pressed: true,
            jump_held: true,
            dash_pressed: true,
            attack_pressed: true,
            heal_held: phase != HealPhase::Stopping,
            ..idle()
        };
        game.update(everything, DT);
        assert!(game.player.is_grounded(), "{phase:?}: no jump");
        assert_eq!(game.player.velocity(), Vec2::ZERO, "{phase:?}: no dash");
        assert_eq!(game.player.attack().id(), swing_id, "{phase:?}: no swing");
        assert_eq!(game.room().enemies[0].health(), 30, "{phase:?}");
        let events = game.take_events();
        for blocked in [
            GameEvent::PlayerJumped,
            GameEvent::PlayerDashed,
            GameEvent::PlayerAttacked,
        ] {
            assert!(!events.contains(&blocked), "{phase:?}: {blocked:?}");
        }
    }

    // Let go during the start delay (free), and all three work on the next tick.
    let mut game = healing(HealPhase::Starting, 5);
    game.player.collect_item(Item::Dash);
    game.update(idle(), DT);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Idle);
    game.take_events();
    game.update(
        PlayerInput {
            jump_pressed: true,
            attack_pressed: true,
            dash_pressed: true,
            ..idle()
        },
        DT,
    );
    let events = game.take_events();
    for allowed in [
        GameEvent::PlayerJumped,
        GameEvent::PlayerDashed,
        GameEvent::PlayerAttacked,
    ] {
        assert!(events.contains(&allowed), "{allowed:?}");
    }
}

#[test]
fn nothing_heals_in_the_air_and_leaving_the_ground_applies_the_pending_health() {
    let mut game = game();
    wound(&mut game, 30.0);
    give_mana(&mut game, 10);
    let high = Vec2::new(200.0, 100.0);
    game.player.place_at(high, game.world_bounds());
    run(&mut game, held(), 0.30);
    assert!(!game.player.is_grounded());
    assert_eq!(
        game.vitals().heal_phase(),
        HealPhase::Idle,
        "not from the air"
    );
    assert_eq!(game.vitals().mana(), 100.0);

    let mut game = healing(HealPhase::Channeling, 10);
    let pending = game.vitals().pending_heal();
    assert!(pending > 5.0);
    // Knocked off the ground mid-heal: the pending health is added at once.
    game.player.place_at(high, game.world_bounds());
    game.update(held(), DT);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Idle);
    close(game.vitals().health(), 20.0 + pending);
    assert_eq!(
        game.player.health().current(),
        game.vitals().health().ceil() as u16
    );
}

// ---- healing: result, damage, respawn ----------------------------------------------------

#[test]
fn a_finished_heal_raises_the_players_health_by_the_mana_spent() {
    let mut game = healing(HealPhase::Idle, 5);
    assert_eq!(game.player.health().current(), 20);
    run(&mut game, held(), 0.25 + 1.0);
    run(&mut game, idle(), 0.5);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Idle);
    // 30 mana bought the 30 health that was missing; 20 mana is left.
    close(game.vitals().health(), 50.0);
    assert_eq!(game.player.health().current(), 50);
    close(game.vitals().mana(), 20.0);
}

#[test]
fn healing_can_keep_the_player_alive_past_what_the_old_health_would_allow() {
    let mut game = healing(HealPhase::Idle, 5);
    run(&mut game, held(), 0.25 + 0.6);
    run(&mut game, idle(), 0.5);
    let healed = game.player.health().current();
    assert!(healed > 30, "{healed}");
    // 30 health left means three more hits before dying, not two.
    for _ in 0..2 {
        run(&mut game, idle(), 1.1);
        hit_player(&mut game, 10);
    }
    assert!(game.player.health().current() > 0);
    assert_ne!(game.player.position(), game.room().spawn);
}

#[test]
fn damage_while_healing_cancels_it_loses_the_pending_health_and_keeps_the_spent_mana() {
    for phase in [
        HealPhase::Starting,
        HealPhase::Channeling,
        HealPhase::Stopping,
    ] {
        let mut game = healing(phase, 5);
        let mana = game.vitals().mana();
        let health = game.vitals().health();
        // The button is down when the hit lands (after the release, in the stop delay).
        let input = if phase == HealPhase::Stopping {
            idle()
        } else {
            held()
        };
        hit_player_with(&mut game, 10, input);
        assert_eq!(game.vitals().heal_phase(), HealPhase::Idle, "{phase:?}");
        assert_eq!(game.vitals().pending_heal(), 0.0, "{phase:?}");
        assert_eq!(game.vitals().mana(), mana, "{phase:?}: no refund");
        assert_eq!(game.vitals().health(), health - 10.0, "{phase:?}");
        assert_eq!(game.player.health().current(), 10, "{phase:?}");
        assert!(game.vitals().heal_locked(), "{phase:?}");
        // The button is still down, so no heal starts again, even once the player lands...
        run(&mut game, held(), 1.5);
        assert_eq!(game.vitals().heal_phase(), HealPhase::Idle, "{phase:?}");
        assert_eq!(game.vitals().mana(), mana, "{phase:?}");
        // ...until it is let go and pressed again.
        game.update(idle(), DT);
        run(&mut game, held(), 0.05);
        assert_eq!(game.vitals().heal_phase(), HealPhase::Starting, "{phase:?}");
    }
}

#[test]
fn dying_resets_the_vitals_and_a_button_still_held_does_not_start_a_heal() {
    let mut game = game();
    wound(&mut game, 40.0);
    give_mana(&mut game, 5);
    run(&mut game, held(), 0.5);
    hit_player(&mut game, 10);
    assert_eq!(
        game.player.health().current(),
        50,
        "respawned at full health"
    );
    assert_eq!(game.player.position(), game.room().spawn);
    let vitals = game.vitals();
    assert_eq!(vitals.health(), 50.0);
    assert_eq!(vitals.trailing_health(), 50.0);
    assert_eq!(vitals.mana(), 0.0, "back to the starting mana");
    assert_eq!(vitals.pending_heal(), 0.0);
    assert_eq!(vitals.heal_phase(), HealPhase::Idle);
    run(&mut game, held(), 1.0);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Idle);
}

#[test]
fn the_trailing_bar_runs_with_the_game_clock_and_nothing_moves_while_paused() {
    let mut game = game();
    hit_player(&mut game, 20);
    run(&mut game, idle(), 0.4);
    close(game.vitals().trailing_health(), 50.0);
    game.toggle_inventory();
    run(&mut game, idle(), 5.0);
    close(game.vitals().trailing_health(), 50.0);
    game.toggle_inventory();
    run(&mut game, idle(), 0.6);
    assert!(game.vitals().trailing_health() < 50.0);
    assert!(game.vitals().trailing_health() >= game.vitals().health());

    // A heal does not progress while paused either.
    let mut game = healing(HealPhase::Channeling, 5);
    let before = game.vitals().clone();
    game.toggle_inventory();
    run(&mut game, held(), 2.0);
    assert_eq!(game.vitals(), &before);
    for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        game.toggle_inventory();
        game.update(held(), dt);
        game.toggle_inventory();
    }
    assert!(game.vitals().health().is_finite() && game.vitals().mana().is_finite());
}

// ---- tuning, reload, test room -------------------------------------------------------------

#[test]
fn the_fixture_vitals_are_the_agreed_starting_values() {
    assert_eq!(
        *game().vitals_config(),
        crate::vitals::VitalsConfig::default()
    );
}

#[test]
fn f4_edits_apply_at_once_are_checked_and_save_to_vitals_ron() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    assert!(!game.vitals_unsaved());

    let invalid = VitalsConfig {
        regain_fraction: 3.0,
        ..*game.vitals_config()
    };
    let error = game.set_vitals_config(invalid).unwrap_err();
    assert!(error.contains("vitals.regain_fraction"), "{error}");
    assert!(!game.vitals_unsaved(), "nothing changed");

    let changed = game
        .vitals_config()
        .with_field(VitalsField::ManaPerKill, 25.0)
        .unwrap();
    game.set_vitals_config(changed).unwrap();
    assert!(game.vitals_unsaved());
    // In effect on the next kill, without any reset.
    game.player.place_on_floor(200.0, game.world_bounds());
    let bounds = game.world_bounds();
    game.rooms[0].enemies = vec![dummy(250.0, 10, 0, bounds)];
    game.update(swing(), 0.0);
    assert_eq!(game.vitals().mana(), 25.0);

    let path = game.save_vitals().unwrap();
    assert_eq!(path, scratch.0.join(crate::vitals::FILE_NAME));
    assert_eq!(VitalsConfig::load(&scratch.0).unwrap(), changed);
    assert!(!game.vitals_unsaved());
}

#[test]
fn f5_rereads_vitals_ron_without_resetting_the_player_and_a_bad_file_changes_nothing() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    wound(&mut game, 30.0);
    give_mana(&mut game, 5);
    run(&mut game, held(), 0.6);
    let (position, health, mana, pending) = (
        game.player.position(),
        game.vitals().health(),
        game.vitals().mana(),
        game.vitals().pending_heal(),
    );
    assert!(pending > 0.0 && game.vitals().heal_phase() == HealPhase::Channeling);

    let text = std::fs::read_to_string(fixtures().join("vitals.ron"))
        .unwrap()
        .replace("mana_per_kill: 10.0", "mana_per_kill: 33.0");
    std::fs::write(scratch.0.join("vitals.ron"), text).unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(game.vitals_config().mana_per_kill, 33.0);
    assert!(!game.vitals_unsaved(), "matches the file just read");
    assert_eq!(game.player.position(), position);
    assert_eq!(game.vitals().health(), health);
    assert_eq!(game.vitals().mana(), mana);
    assert_eq!(game.vitals().pending_heal(), pending);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Channeling);

    std::fs::write(scratch.0.join("vitals.ron"), "(max_mana: 1.0)").unwrap();
    let error = game.reload_rooms().unwrap_err();
    assert!(error.contains("vitals.ron"), "{error}");
    assert_eq!(game.vitals_config().mana_per_kill, 33.0);
    assert_eq!(game.vitals().heal_phase(), HealPhase::Channeling);
}

#[test]
fn a_bad_vitals_file_stops_the_game_from_loading_and_names_the_file() {
    let scratch = Scratch::new();
    std::fs::write(scratch.0.join("vitals.ron"), "(trail_hold: -1.0)").unwrap();
    let error = GameState::try_new(&scratch.0).unwrap_err();
    assert!(error.contains("vitals.ron"), "{error}");
}

#[test]
fn a_test_room_runs_on_fresh_vitals_and_hands_the_games_own_back() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    wound(&mut game, 20.0);
    give_mana(&mut game, 3);
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    assert_eq!(game.vitals().health(), 50.0);
    assert_eq!(game.vitals().mana(), 0.0);
    wound(&mut game, 30.0);
    game.vitals.gain_kill_mana();
    game.end_test_room().unwrap();
    assert_eq!(game.vitals().health(), 30.0);
    assert_eq!(game.vitals().mana(), 30.0);
    assert_eq!(game.player.health().current(), 30);
}
