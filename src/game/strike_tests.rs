//! Up and down strikes through `GameState`: what they hit, who they push, and when they bounce.
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::combat::StrikeDirection;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::item::Item;

fn game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

/// An enemy that stays where it is, with no contact damage, so only the strike moves it.
fn dummy(x: f32, size: (f32, f32), bounds: WorldBounds) -> Enemy {
    let behavior = BehaviorDefinition::from_ron(
        r#"(initial_state: "idle", states: [
            (name: "idle", actions: [patrol(speed: 0.0)], transitions: [])])"#,
    )
    .unwrap();
    Enemy::from_definition(
        &EnemyDefinition {
            name: "dummy".into(),
            health: 3,
            size,
            contact_damage: 0,
            behavior: "idle".into(),
            body: None,
        },
        Arc::new(behavior),
        x,
        x,
        bounds,
    )
    .unwrap()
}

fn strike(attack: bool, up: bool, down: bool) -> PlayerInput {
    PlayerInput {
        attack_pressed: attack,
        up_held: up,
        down_held: down,
        ..PlayerInput::default()
    }
}

/// Puts the player in the air with its feet `gap` px above `enemy`'s head, centered on `x`.
fn hover_over(game: &mut GameState, enemy: &Enemy, x: f32, gap: f32) {
    let feet = enemy.position().y - enemy.size().y / 2.0 - gap;
    let y = feet - game.player.size().y / 2.0;
    game.player.place_at(Vec2::new(x, y), game.world_bounds());
    assert!(!game.player.is_grounded());
}

#[test]
fn a_downward_strike_hurts_the_enemy_pushes_it_away_and_bounces_the_player() {
    let mut game = game();
    let bounds = game.world_bounds();
    let enemy = dummy(300.0, (36.0, 36.0), bounds);
    hover_over(&mut game, &enemy, 290.0, 10.0);
    game.rooms[0].enemies = vec![enemy];
    game.update(strike(true, false, true), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Down);
    assert_eq!(game.room().enemies[0].health(), 2);
    assert_eq!(
        game.player.velocity().y,
        -game.tuning().player.strike.pogo_speed
    );
    // The enemy is right of the player, so it is pushed right.
    game.update(PlayerInput::default(), 0.1);
    assert!(game.room().enemies[0].position().x > 300.0);
}

#[test]
fn a_swing_still_hits_each_enemy_once_when_it_points_down() {
    let mut game = game();
    let bounds = game.world_bounds();
    let (a, b) = (
        dummy(290.0, (20.0, 36.0), bounds),
        dummy(305.0, (20.0, 36.0), bounds),
    );
    hover_over(&mut game, &a, 298.0, 10.0);
    game.rooms[0].enemies = vec![a, b];
    game.update(strike(true, false, true), 0.0);
    for _ in 0..3 {
        game.update(strike(false, false, true), 0.0);
    }
    let health: Vec<_> = game.room().enemies.iter().map(Enemy::health).collect();
    assert_eq!(health, [2, 2], "one hit per enemy per swing");
}

#[test]
fn an_upward_strike_hits_what_is_above_pushes_it_away_and_does_not_bounce() {
    let bounds = game().world_bounds();
    // Tall and narrow: left of the player's forward reach, but reaching over its head.
    let tall = |bounds| dummy(190.0, (20.0, 140.0), bounds);
    let setup = |bounds| {
        let mut game = game();
        game.player.place_on_floor(200.0, bounds);
        game.rooms[0].enemies = vec![tall(bounds)];
        game
    };
    // Control: a plain forward swing (facing right) misses it.
    let mut control = setup(bounds);
    control.update(strike(true, false, false), 0.0);
    assert_eq!(control.room().enemies[0].health(), 3);
    // Up + X hits it and pushes it left, away from the player.
    let mut game = setup(bounds);
    game.update(strike(true, true, false), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Up);
    assert_eq!(game.room().enemies[0].health(), 2);
    game.update(PlayerInput::default(), 0.1);
    assert!(game.room().enemies[0].position().x < 190.0);
    assert_eq!(
        game.player.velocity().y,
        0.0,
        "only a downward strike bounces"
    );
    assert!(game.player.is_grounded());
}

#[test]
fn down_on_the_floor_is_a_forward_swing_and_never_bounces() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(240.0, (36.0, 36.0), bounds)];
    game.update(strike(true, false, true), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Forward);
    assert_eq!(
        game.room().enemies[0].health(),
        2,
        "the forward swing still hits"
    );
    assert_eq!(game.player.velocity().y, 0.0);
    assert!(game.player.is_grounded());
}

#[test]
fn a_downward_strike_that_misses_does_not_bounce() {
    let mut game = game();
    let bounds = game.world_bounds();
    let enemy = dummy(600.0, (36.0, 36.0), bounds);
    hover_over(&mut game, &enemy, 200.0, 10.0);
    game.rooms[0].enemies = vec![enemy];
    game.update(strike(true, false, true), 0.0);
    assert_eq!(game.room().enemies[0].health(), 3);
    assert_eq!(game.player.velocity().y, 0.0);
}

#[test]
fn a_downward_strike_on_the_boss_bounces_but_not_once_it_is_defeated() {
    let mut game = game();
    game.current = RoomId::Boss.index();
    let bounds = game.world_bounds();
    let (x, top, health) = {
        let boss = game.room().boss_special.as_ref().unwrap();
        (
            boss.position().x,
            boss.position().y - boss.size().y / 2.0,
            boss.health(),
        )
    };
    let hover = |game: &mut GameState| {
        let y = top - 10.0 - game.player.size().y / 2.0;
        game.player.place_at(Vec2::new(x, y), bounds);
    };
    hover(&mut game);
    game.update(strike(true, false, true), 0.0);
    assert_eq!(
        game.room().boss_special.as_ref().unwrap().health(),
        health - 1
    );
    assert_eq!(
        game.player.velocity().y,
        -game.tuning().player.strike.pogo_speed
    );
    // Defeated: the swing finds nothing to bounce off.
    let index = game.current;
    game.rooms[index]
        .boss_special
        .as_mut()
        .unwrap()
        .damage_boss(10_000);
    game.update(PlayerInput::default(), 1.0);
    hover(&mut game);
    game.update(strike(false, false, false), crate::combat::ATTACK_COOLDOWN);
    game.update(strike(true, false, true), 0.0);
    assert!(game.player.velocity().y >= 0.0);
}

#[test]
fn up_with_x_is_a_strike_and_up_alone_still_picks_up() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies.clear();
    game.rooms[0].items = vec![WorldItem {
        item: Item::AmberShard,
        position: game.player.position(),
    }];
    let pick_up = |attack, up| PlayerInput {
        pickup_pressed: true,
        attack_pressed: attack,
        up_held: up,
        ..PlayerInput::default()
    };
    game.update(pick_up(true, true), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Up);
    assert_eq!(game.items().len(), 1, "an up strike does not pick up");
    assert!(game.player.collected_items().is_empty());
    game.update(pick_up(false, true), 0.0);
    assert!(game.items().is_empty(), "Up alone still picks up");
    assert_eq!(game.player.collected_items(), &[Item::AmberShard]);
}
