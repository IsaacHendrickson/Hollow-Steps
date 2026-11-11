//! `GameEvent`s through `GameState`: what is raised, in what order, and exactly once.
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::coin::Coin;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::item::{Item, WorldItem};

const DT: f32 = 1.0 / 120.0;

fn game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

/// A stationary enemy with `health` and `contact_damage`.
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

fn input() -> PlayerInput {
    PlayerInput::default()
}

fn swing() -> PlayerInput {
    PlayerInput {
        attack_pressed: true,
        ..input()
    }
}

/// Steps `ticks` fixed ticks with `input` on the first one only, returning every event.
fn run(game: &mut GameState, first: PlayerInput, ticks: usize) -> Vec<GameEvent> {
    let mut events = Vec::new();
    game.update(first, DT);
    events.extend(game.take_events());
    for _ in 1..ticks {
        game.update(input(), DT);
        events.extend(game.take_events());
    }
    events
}

#[test]
fn a_scripted_run_raises_each_event_once_in_the_order_it_happens() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.player.collect_item(Item::Dash);
    game.rooms[0].coins.clear();
    game.rooms[0].items.clear();
    // One hit kills; it stands in the forward reach of a swing made facing right at x = 200.
    game.rooms[0].enemies = vec![dummy(235.0, 1, 0, bounds)];
    let mut events = Vec::new();

    // Jump, rise, fall and land.
    events.extend(run(
        &mut game,
        PlayerInput {
            jump_pressed: true,
            ..input()
        },
        240,
    ));
    assert!(game.player.is_grounded());
    assert_eq!(events, [GameEvent::PlayerJumped, GameEvent::PlayerLanded]);

    // Dash, and let it finish.
    let dash = PlayerInput {
        dash_pressed: true,
        ..input()
    };
    events.extend(run(&mut game, dash, 120));
    // Back to the start, facing the enemy, then swing.
    game.player.place_on_floor(200.0, bounds);
    events.extend(run(&mut game, swing(), 40));

    assert_eq!(
        events,
        [
            GameEvent::PlayerJumped,
            GameEvent::PlayerLanded,
            GameEvent::PlayerDashed,
            GameEvent::PlayerAttacked,
            GameEvent::EnemyHit,
            GameEvent::EnemyDied,
        ]
    );
    assert!(game.room().enemies.is_empty());
    assert!(game.take_events().is_empty(), "nothing is raised twice");
}

#[test]
fn a_swing_that_hits_nothing_still_raises_only_player_attacked() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    let events = run(&mut game, swing(), 60);
    assert_eq!(events, [GameEvent::PlayerAttacked]);
}

#[test]
fn a_swing_hits_an_enemy_once_and_only_the_killing_hit_raises_enemy_died() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(235.0, 20, 0, bounds)];

    // The swing stays active for several ticks but registers one hit.
    let first = run(&mut game, swing(), 30);
    assert_eq!(first, [GameEvent::PlayerAttacked, GameEvent::EnemyHit]);
    assert_eq!(game.room().enemies[0].health(), 10);

    // The hit knocked it back. After the cooldown, step up to it and swing again.
    run(&mut game, input(), 60);
    let enemy_x = game.room().enemies[0].position().x;
    game.player.place_on_floor(enemy_x - 35.0, bounds);
    let second = run(&mut game, swing(), 30);
    assert_eq!(
        second,
        [
            GameEvent::PlayerAttacked,
            GameEvent::EnemyHit,
            GameEvent::EnemyDied
        ]
    );
}

#[test]
fn damage_raises_player_damaged_once_and_invincibility_blocks_a_repeat() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(205.0, 3, 10, bounds)];
    let events = run(&mut game, input(), 30);
    assert_eq!(events, [GameEvent::PlayerDamaged]);
    assert_eq!(game.player.health().current(), 40);
}

#[test]
fn coins_and_items_each_raise_one_event_when_collected() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    let at = game.player.position();
    game.rooms[0].enemies.clear();
    game.rooms[0].coins = vec![Coin { position: at }];
    game.rooms[0].items = vec![WorldItem {
        item: Item::AmberShard,
        position: at,
    }];
    let pickup = PlayerInput {
        pickup_pressed: true,
        ..input()
    };
    let events = run(&mut game, pickup, 30);
    assert_eq!(
        events,
        [GameEvent::ItemPickedUp, GameEvent::CoinCollected],
        "the pickup key and the coin touch each raise their event once, items first"
    );
    assert_eq!(game.coin_count(), 1);
}

#[test]
fn walking_through_an_exit_raises_room_entered_once() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(bounds.right - 20.0, bounds);
    let right = PlayerInput {
        move_axis: 1.0,
        ..input()
    };
    let mut events = run(&mut game, right, 1);
    for _ in 0..60 {
        game.update(right, DT);
        events.extend(game.take_events());
    }
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(events, [GameEvent::RoomEntered]);
}

#[test]
fn the_killing_boss_hit_raises_enemy_hit_then_boss_defeated_and_later_swings_nothing() {
    let mut game = game();
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(520.0, game.world_bounds());
    let boss = game.rooms[2].boss_special.as_mut().unwrap();
    boss.damage_boss(boss.health() - 1);
    assert!(!game.room().boss_special.as_ref().unwrap().is_defeated());

    game.update(swing(), 0.0);
    assert_eq!(
        game.take_events(),
        [
            GameEvent::PlayerAttacked,
            GameEvent::EnemyHit,
            GameEvent::BossDefeated
        ]
    );
    assert!(game.room().boss_special.as_ref().unwrap().is_defeated());

    // A defeated boss is not hit again, even by a fresh swing.
    run(&mut game, input(), 60);
    game.update(swing(), 0.0);
    assert_eq!(game.take_events(), [GameEvent::PlayerAttacked]);
}

#[test]
fn a_paused_game_raises_nothing_and_take_events_empties_the_queue() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.toggle_inventory();
    let jump = PlayerInput {
        jump_pressed: true,
        ..input()
    };
    game.update(jump, DT);
    assert!(game.take_events().is_empty());

    game.toggle_inventory();
    game.update(jump, DT);
    assert_eq!(game.take_events(), [GameEvent::PlayerJumped]);
    assert!(game.take_events().is_empty());
}

#[test]
fn zero_and_non_finite_steps_raise_no_spurious_events() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    for dt in [0.0, f32::NAN, f32::INFINITY, -1.0] {
        game.update(input(), dt);
    }
    assert!(game.take_events().is_empty());
}
