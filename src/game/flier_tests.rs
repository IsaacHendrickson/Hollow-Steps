//! A flying bird in the real game loop: the player's swings and the contact damage still land
//! on an enemy hovering above the floor (a hover height like the shipped bird's).
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::combat::{StrikeDirection, SWORD_DAMAGE};
use crate::enemy::{Enemy, EnemyDefinition, Movement};

const HOVER: f32 = 56.0;

fn game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

/// A 32x40 flier that stays where it is, hovering `HOVER` px above the floor.
fn bird(x: f32, contact_damage: u16, bounds: WorldBounds) -> Enemy {
    let behavior = BehaviorDefinition::from_ron(
        r#"(initial_state: "idle", states: [
            (name: "idle", actions: [patrol(speed: 0.0)], transitions: [])])"#,
    )
    .unwrap();
    Enemy::from_definition(
        &EnemyDefinition {
            name: "bird".into(),
            health: 30,
            size: (32.0, 40.0),
            visual_size: None,
            contact_damage,
            behavior: "idle".into(),
            body: None,
            death_secs: 0.0,
            movement: Movement::Flying {
                hover_height: HOVER,
                bob_amplitude: 6.0,
                bob_speed: 0.6,
                flap: None,
            },
        },
        Arc::new(behavior),
        x,
        x,
        bounds,
    )
    .unwrap()
}

fn input(attack: bool, up: bool, down: bool) -> PlayerInput {
    PlayerInput {
        attack_pressed: attack,
        up_held: up,
        down_held: down,
        ..PlayerInput::default()
    }
}

/// A game whose only enemy is the bird, with its position on the hover line checked.
fn game_with_bird(contact_damage: u16) -> (GameState, Enemy) {
    let mut game = game();
    let bounds = game.world_bounds();
    let bird = bird(300.0, contact_damage, bounds);
    let expected_y = bounds.floor_y - bird.size().y / 2.0 - HOVER;
    assert!(
        (bird.position().y - expected_y).abs() < 1e-3,
        "starts at hover height"
    );
    game.rooms[0].enemies = vec![bird.clone()];
    (game, bird)
}

#[test]
fn an_upward_strike_from_the_floor_hits_a_bird_at_hover_height() {
    let (mut game, bird) = game_with_bird(0);
    let bounds = game.world_bounds();
    game.player.place_on_floor(bird.position().x, bounds);
    game.update(input(true, true, false), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Up);
    assert_eq!(game.room().enemies[0].health(), 30 - SWORD_DAMAGE);
    // Hit from below, it is pushed sideways and stays in the air.
    game.update(PlayerInput::default(), 0.1);
    let hit = &game.room().enemies[0];
    assert_ne!(hit.position().x, 300.0, "pushed sideways");
    let hover_y = bounds.floor_y - hit.size().y / 2.0 - HOVER;
    assert!(
        (hit.position().y - hover_y).abs() <= 6.0 + 1e-3,
        "still hovering"
    );
}

#[test]
fn a_swing_from_a_jump_hits_a_bird_at_hover_height() {
    let (mut game, bird) = game_with_bird(0);
    let bounds = game.world_bounds();
    // In the air beside the bird at its own height, facing it.
    let x = bird.position().x - 16.0 - 16.0 - 20.0;
    game.player
        .place_at(Vec2::new(x, bird.position().y + 10.0), bounds);
    assert!(!game.player.is_grounded());
    game.update(input(true, false, false), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Forward);
    assert_eq!(game.room().enemies[0].health(), 30 - SWORD_DAMAGE);
}

#[test]
fn a_downward_strike_from_above_hits_a_bird_and_bounces_the_player() {
    let (mut game, bird) = game_with_bird(0);
    let bounds = game.world_bounds();
    let feet = bird.position().y - bird.size().y / 2.0 - 10.0;
    let y = feet - game.player.size().y / 2.0;
    game.player
        .place_at(Vec2::new(bird.position().x, y), bounds);
    assert!(!game.player.is_grounded());
    game.update(input(true, false, true), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Down);
    assert_eq!(game.room().enemies[0].health(), 30 - SWORD_DAMAGE);
    assert_eq!(
        game.player.velocity().y,
        -game.tuning().player.strike.pogo_speed,
        "a hit bird is a pogo target"
    );
}

#[test]
fn a_swing_from_the_floor_passes_under_a_bird_and_walking_under_it_is_safe() {
    // The bird is a flier: a standing forward swing (a 32 px-tall box at chest height) and a
    // walk beneath it both go under a bird hovering 56 px up. Lower `hover_height` to change that.
    let (mut game, bird) = game_with_bird(10);
    let bounds = game.world_bounds();
    let x = bird.position().x - 16.0 - 16.0 - 20.0;
    game.player.place_on_floor(x, bounds);
    game.update(input(true, false, false), 0.0);
    assert_eq!(game.room().enemies[0].health(), 30, "the swing misses");
    game.player.place_on_floor(bird.position().x, bounds);
    let health = game.player.health().current();
    game.update(PlayerInput::default(), 0.05);
    assert_eq!(
        game.player.health().current(),
        health,
        "no contact on the floor"
    );
}

#[test]
fn touching_a_bird_at_hover_height_deals_its_contact_damage() {
    let (mut game, bird) = game_with_bird(10);
    let bounds = game.world_bounds();
    // Jumped into it: the player's body overlaps the bird's box.
    game.player.place_at(
        Vec2::new(bird.position().x, bird.position().y + 20.0),
        bounds,
    );
    assert!(!game.player.is_grounded());
    let before = game.player.health().current();
    game.update(PlayerInput::default(), 0.01);
    assert_eq!(game.player.health().current(), before - 10);
    assert!(
        game.player.health().is_invincible(),
        "hurt once, then immune"
    );
}
