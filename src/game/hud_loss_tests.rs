//! The health a `PlayerDamaged` event reports, through `GameState`'s real damage path: the
//! health bar's shake and particles are sized and placed from it. The HUD rules themselves are
//! tested in `crate::hud_damage`.
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::events::HealthLoss;
use crate::health::MAX_HEALTH;

const MAX: f32 = MAX_HEALTH as f32;

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

/// An enemy that stays where it is.
fn dummy(x: f32, contact_damage: u16, bounds: WorldBounds) -> Enemy {
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

/// Real damage through the game's damage path: a `damage`-strong enemy on top of the player
/// for one tick that lets no time pass, so the player's immunity timer does not run down.
fn hit_player(game: &mut GameState, damage: u16) {
    let bounds = game.world_bounds();
    let x = game.player.position().x;
    game.rooms[0].enemies = vec![dummy(x, damage, bounds)];
    game.update(PlayerInput::default(), 0.0);
    game.rooms[0].enemies.clear();
}

/// Leaves the player with `health`, the way `vitals_tests` wounds it.
fn wound_to(game: &mut GameState, health: f32) {
    game.vitals.damage(MAX - health);
    game.player
        .set_health_current(game.vitals.health().ceil() as u16);
}

fn close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-3,
        "{actual} is not close to {expected}"
    );
}

fn damage_events(game: &mut GameState) -> Vec<LocatedEvent> {
    game.take_located_events()
        .into_iter()
        .filter(|located| located.event == GameEvent::PlayerDamaged)
        .collect()
}

#[test]
fn a_hit_reports_the_health_it_took_and_the_health_left() {
    let mut game = game();
    hit_player(&mut game, 10);
    let events = damage_events(&mut game);
    assert_eq!(events.len(), 1, "one event per hit");
    let loss = events[0].loss.expect("the vitals lost health");
    close(loss.lost, 10.0);
    close(loss.remaining, MAX - 10.0);
    close(game.vitals().health(), MAX - 10.0);
    // The event still says where the player was.
    assert_eq!(events[0].at, Some(game.player().position()));
}

#[test]
fn a_hit_on_the_last_scraps_reports_only_what_was_left_and_survives_the_respawn() {
    let mut game = game();
    wound_to(&mut game, 7.5);
    game.take_located_events();
    hit_player(&mut game, 10);
    let events = damage_events(&mut game);
    assert_eq!(events.len(), 1);
    let loss = events[0].loss.expect("the vitals lost health");
    close(loss.lost, 7.5); // not the 10 of the hit
    close(loss.remaining, 0.0);
    // The player died and was put back at full health in the same tick, but the event still
    // says where on the bar the loss was.
    close(game.vitals().health(), MAX);
    assert_eq!(game.player().health().current(), MAX_HEALTH);
}

#[test]
fn a_hit_during_immunity_makes_no_event() {
    let mut game = game();
    hit_player(&mut game, 10);
    assert_eq!(damage_events(&mut game).len(), 1);
    hit_player(&mut game, 10); // still immune: no time has passed
    assert!(damage_events(&mut game).is_empty());
    close(game.vitals().health(), MAX - 10.0);
}

#[test]
fn only_the_damage_event_carries_a_loss() {
    let mut game = game();
    hit_player(&mut game, 10);
    for located in game.take_located_events() {
        match located.event {
            GameEvent::PlayerDamaged => assert!(located.loss.is_some()),
            other => assert_eq!(located.loss, None, "{other:?}"),
        }
    }
}

#[test]
fn losses_are_what_the_hud_needs_to_find_the_lost_segment() {
    let mut game = game();
    hit_player(&mut game, 20);
    let loss: HealthLoss = damage_events(&mut game)[0].loss.unwrap();
    // The lost part of the bar runs from 30 to 50 of 50: the right-hand 40% of it.
    close(loss.remaining / MAX, 0.6);
    close((loss.remaining + loss.lost) / MAX, 1.0);
}
