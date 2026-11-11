//! A defeated enemy through `GameState`: it lingers, harmless, then goes; `EnemyDied` is
//! raised once, at the killing blow.
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::feel::{FeelConfig, HitStopConfig, ShakeBurst, ShakeConfig};

/// A binary fraction, so death timers count down exactly.
const DT: f32 = 1.0 / 128.0;
const PATROL: &str = r#"(initial_state: "walk", states: [
    (name: "walk", actions: [patrol(speed: 64.0)], transitions: [])])"#;

fn game() -> GameState {
    let mut game =
        GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
            .expect("fixed test assets must load");
    game.rooms[0].coins.clear();
    game
}

fn enemy(
    behavior: &str,
    x: f32,
    health: u16,
    contact_damage: u16,
    death_secs: f32,
    bounds: WorldBounds,
) -> Enemy {
    Enemy::from_definition(
        &EnemyDefinition {
            name: "victim".into(),
            health,
            size: (36.0, 36.0),
            contact_damage,
            behavior: "walk".into(),
            body: None,
            death_secs,
            movement: Default::default(),
        },
        Arc::new(BehaviorDefinition::from_ron(behavior).unwrap()),
        x,
        x + 400.0,
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

fn ticks(game: &mut GameState, count: usize) {
    for _ in 0..count {
        game.update(input(), DT);
    }
}

/// The player at x = 200 and one enemy in forward swing reach, dying over `death_secs`.
fn setup(behavior: &str, health: u16, contact_damage: u16, death_secs: f32) -> GameState {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![enemy(
        behavior,
        235.0,
        health,
        contact_damage,
        death_secs,
        bounds,
    )];
    game
}

#[test]
fn a_killed_enemy_lingers_fading_and_is_removed_after_exactly_death_secs() {
    // 0.5 s is 64 ticks.
    let mut game = setup(PATROL, 1, 0, 64.0 * DT);
    game.update(swing(), DT);
    assert_eq!(
        game.room().enemies.len(),
        1,
        "still here on the killing tick"
    );
    assert!(game.room().enemies[0].is_dying());
    assert_eq!(game.room().enemies[0].health(), 0);

    let mut last_opacity = game.room().enemies[0].opacity();
    for tick in 1..64 {
        game.update(input(), DT);
        assert_eq!(game.room().enemies.len(), 1, "tick {tick}");
        let opacity = game.room().enemies[0].opacity();
        assert!(opacity < last_opacity, "it keeps fading");
        last_opacity = opacity;
    }
    assert!(last_opacity > 0.0);
    game.update(input(), DT);
    assert!(game.room().enemies.is_empty(), "gone on the 64th tick");
}

#[test]
fn enemy_died_is_raised_once_at_the_killing_blow_and_never_again() {
    let mut game = setup(PATROL, 1, 0, 64.0 * DT);
    game.update(swing(), DT);
    let killing_tick = game.take_events();
    assert_eq!(
        killing_tick
            .iter()
            .filter(|e| **e == GameEvent::EnemyDied)
            .count(),
        1
    );
    assert_eq!(
        killing_tick,
        [
            GameEvent::PlayerAttacked,
            GameEvent::EnemyHit,
            GameEvent::EnemyDied
        ]
    );
    // The whole death and the removal raise nothing more.
    let mut later = Vec::new();
    for _ in 0..80 {
        game.update(input(), DT);
        later.extend(game.take_events());
    }
    assert!(game.room().enemies.is_empty());
    assert!(later.is_empty(), "{later:?}");
}

#[test]
fn another_swing_through_a_dying_enemy_hits_nothing_and_raises_only_the_swing() {
    let mut game = setup(PATROL, 1, 0, 128.0 * DT);
    game.update(swing(), DT);
    game.take_events();
    // The swing cooldown (0.35 s) is over after 48 ticks; the death (1 s) is not.
    ticks(&mut game, 48);
    game.take_events();
    let x = game.room().enemies[0].position().x;
    game.player.place_on_floor(x - 35.0, game.world_bounds());
    game.update(swing(), DT);
    assert_eq!(game.take_events(), [GameEvent::PlayerAttacked]);
    assert_eq!(game.room().enemies[0].health(), 0);
    assert!(game.room().enemies[0].is_dying());
}

#[test]
fn a_dying_enemy_does_not_hurt_the_player_on_contact() {
    // Control: a living enemy of the same kind does hurt on contact.
    let mut control = game();
    let bounds = control.world_bounds();
    control.player.place_on_floor(200.0, bounds);
    control.rooms[0].enemies = vec![enemy(PATROL, 205.0, 5, 30, 1.0, bounds)];
    ticks(&mut control, 2);
    assert_eq!(control.player.health().current(), 20);

    // Kill one out of reach, then walk into its body while it fades.
    let mut game = setup(PATROL, 1, 30, 1.0);
    game.update(swing(), DT);
    assert!(game.room().enemies[0].is_dying());
    let walk = PlayerInput {
        move_axis: 1.0,
        ..input()
    };
    for _ in 0..40 {
        game.update(walk, DT);
    }
    let enemy = &game.room().enemies[0];
    let overlap = (game.player.position() - enemy.position()).abs();
    assert!(
        overlap.x <= (enemy.size().x + game.player.size().x) / 2.0,
        "the player walked into the dying enemy"
    );
    assert_eq!(game.player.health().current(), 50);
}

#[test]
fn a_dying_enemy_is_not_a_pogo_target() {
    let mut game = setup(PATROL, 1, 0, 1.0);
    game.update(swing(), DT);
    assert!(game.room().enemies[0].is_dying());
    ticks(&mut game, 48);
    game.take_events();
    // Hover just above the dying enemy and strike down.
    let enemy = &game.room().enemies[0];
    let feet = enemy.position().y - enemy.size().y / 2.0 - 10.0;
    let at = Vec2::new(enemy.position().x, feet - game.player.size().y / 2.0);
    game.player.place_at(at, game.world_bounds());
    assert!(!game.player.is_grounded());
    game.update(
        PlayerInput {
            attack_pressed: true,
            down_held: true,
            ..input()
        },
        0.0,
    );
    assert_eq!(game.take_events(), [GameEvent::PlayerAttacked]);
    assert!(
        game.player.velocity().y >= 0.0,
        "no bounce off something already beaten"
    );
}

#[test]
fn a_blast_cannot_finish_the_job_after_its_enemy_is_defeated() {
    let blast = r#"(initial_state: "fire", states: [
        (name: "fire", actions: [area_blast(radius: 2000.0, damage: 30, knockback: (100.0, -100.0), active_secs: 2.0, telegraph_secs: 0.5)], transitions: [])])"#;
    // Control: left alone, the blast lands once its telegraph runs out.
    let mut control = setup(blast, 5, 0, 1.0);
    ticks(&mut control, 90);
    assert!(control.player.health().current() < 50);

    // Defeated during the telegraph, it never goes off.
    let mut game = setup(blast, 1, 0, 2.0);
    game.update(swing(), DT);
    assert!(game.room().enemies[0].is_dying());
    ticks(&mut game, 120);
    assert!(game.room().enemies[0].is_dying(), "still fading");
    assert_eq!(game.player.health().current(), 50);
}

#[test]
fn zero_death_secs_removes_the_enemy_on_the_killing_tick_as_before() {
    let mut game = setup(PATROL, 1, 0, 0.0);
    game.update(swing(), DT);
    assert!(game.room().enemies.is_empty());
    assert_eq!(
        game.take_events(),
        [
            GameEvent::PlayerAttacked,
            GameEvent::EnemyHit,
            GameEvent::EnemyDied
        ]
    );
}

#[test]
fn a_non_lethal_hit_leaves_a_live_enemy_not_dying() {
    let mut game = setup(PATROL, 30, 0, 1.0);
    game.update(swing(), DT);
    let enemy = &game.room().enemies[0];
    assert_eq!(enemy.health(), 20);
    assert!(!enemy.is_dead() && !enemy.is_dying());
    assert_eq!(enemy.opacity(), 1.0);
}

#[test]
fn hit_stop_holds_the_fade_for_its_duration_then_the_death_plays_out() {
    let mut game = setup(PATROL, 1, 0, 16.0 * DT);
    game.feel = Feel::new(FeelConfig {
        hit_stop: HitStopConfig {
            enemy_hit_secs: 8.0 * DT,
            player_damaged_secs: 0.0,
        },
        shake: ShakeConfig {
            enemy_hit: ShakeBurst {
                amplitude: 0.0,
                secs: 0.0,
            },
            player_damaged: ShakeBurst {
                amplitude: 0.0,
                secs: 0.0,
            },
            frequency_hz: 16.0,
        },
    });
    game.update(swing(), DT);
    assert!(game.room().enemies[0].is_dying());
    // The killing blow freezes the scene: the enemy hangs there, fully visible and flashing.
    for _ in 0..8 {
        game.update(input(), DT);
        assert_eq!(game.room().enemies[0].opacity(), 1.0);
    }
    // Then the 16 tick death plays out.
    ticks(&mut game, 15);
    assert_eq!(game.room().enemies.len(), 1);
    game.update(input(), DT);
    assert!(game.room().enemies.is_empty());
}

#[test]
fn an_enemy_built_without_a_definition_has_no_death_time_and_vanishes_at_once() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    let mut summoned = Enemy::new(235.0, 235.0, 0.0, 0.0, bounds).spawned_with(game.tuning().enemy);
    for id in 1..=u64::from(summoned.max_health()) {
        // Knocked around by earlier swings, then finished off by one in reach.
        summoned.hit(id, 1.0);
    }
    assert!(summoned.is_removable());
    game.rooms[0].enemies = vec![summoned];
    game.update(input(), DT);
    assert!(game.room().enemies.is_empty());
}
