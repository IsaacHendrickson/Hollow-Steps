//! Where `GameEvent`s say they happened: the positions effects are drawn at.
use std::path::Path;
use std::sync::Arc;

use super::*;
use crate::behavior::BehaviorDefinition;
use crate::enemy::{Enemy, EnemyDefinition};
use crate::events::LocatedEvent;

const DT: f32 = 1.0 / 120.0;

fn game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

/// A stationary enemy with `health` and `contact_damage`.
fn dummy(x: f32, health: u8, contact_damage: u8, bounds: WorldBounds) -> Enemy {
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

fn swing() -> PlayerInput {
    PlayerInput {
        attack_pressed: true,
        ..PlayerInput::default()
    }
}

/// True when `points` is exactly one position, within a pixel of `expected`.
fn is_just(points: &[Option<Vec2>], expected: Vec2) -> bool {
    matches!(points, [Some(at)] if (*at - expected).length() < 1.0)
}

fn located(events: &[LocatedEvent], event: GameEvent) -> Vec<Option<Vec2>> {
    events
        .iter()
        .filter(|located| located.event == event)
        .map(|located| located.at)
        .collect()
}

#[test]
fn player_movement_events_happen_at_the_players_feet() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.update(swing(), DT);
    let events = game.take_located_events();

    let feet = game.player.position() + Vec2::new(0.0, game.player.size().y / 2.0);
    let swings = located(&events, GameEvent::PlayerAttacked);
    assert_eq!(swings.len(), 1);
    let at = swings[0].expect("a swing has a place");
    assert!(
        (at - feet).length() < 1.0,
        "{at} should be at the feet {feet}"
    );
    // Standing on the floor, the feet are the floor line.
    assert!((at.y - bounds.floor_y).abs() < 1.0);
}

#[test]
fn enemy_hits_and_deaths_happen_at_the_enemy_and_damage_at_the_player() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(235.0, 1, 0, bounds)];
    // The one-hit enemy is gone after the swing, so note where it stands first.
    let enemy_at = game.rooms[0].enemies[0].position();
    game.update(swing(), DT);
    let events = game.take_located_events();

    assert!(is_just(&located(&events, GameEvent::EnemyHit), enemy_at));
    assert!(is_just(&located(&events, GameEvent::EnemyDied), enemy_at));

    let mut game = self::game();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(205.0, 3, 1, bounds)];
    game.update(PlayerInput::default(), DT);
    let events = game.take_located_events();
    assert!(is_just(
        &located(&events, GameEvent::PlayerDamaged),
        game.player.position()
    ));
}

#[test]
fn a_boss_hit_and_its_defeat_happen_at_the_boss() {
    let mut game = game();
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(520.0, game.world_bounds());
    let boss = game.rooms[2].boss_special.as_mut().unwrap();
    boss.damage_boss(boss.health() - 1);
    game.update(swing(), 0.0);
    let events = game.take_located_events();

    let boss_at = game.room().boss_special.as_ref().unwrap().position();
    assert!(is_just(&located(&events, GameEvent::EnemyHit), boss_at));
    assert!(is_just(&located(&events, GameEvent::BossDefeated), boss_at));
}

#[test]
fn events_with_no_place_carry_none_and_both_takers_drain_the_same_queue() {
    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(bounds.right - 20.0, bounds);
    let right = PlayerInput {
        move_axis: 1.0,
        ..PlayerInput::default()
    };
    let mut events = Vec::new();
    for _ in 0..60 {
        game.update(right, DT);
        events.extend(game.take_located_events());
    }
    assert_eq!(located(&events, GameEvent::RoomEntered), [None]);
    assert!(game.take_events().is_empty());
    assert!(game.take_located_events().is_empty());
}

#[test]
fn a_kill_spawns_sparks_and_a_burst_around_the_enemy_through_the_particle_system() {
    use crate::particles::{ParticleConfig, ParticleSystem};

    let mut game = game();
    let bounds = game.world_bounds();
    game.player.place_on_floor(200.0, bounds);
    game.rooms[0].enemies = vec![dummy(235.0, 1, 0, bounds)];
    let enemy_at = game.rooms[0].enemies[0].position();
    game.update(swing(), DT);

    let config = ParticleConfig::default();
    let expected = config.hit_spark.count + config.death_burst.count;
    let mut system = ParticleSystem::new(config);
    let mut spawned = 0;
    for located in game.take_located_events() {
        spawned += system.handle(located.event, located.at);
    }
    assert_eq!(spawned as u32, expected);
    assert_eq!(system.len() as u32, expected);
    for particle in system.particles() {
        assert!(
            (particle.position - enemy_at).length() < 40.0,
            "{} is far from the enemy at {enemy_at}",
            particle.position
        );
    }
}

#[test]
fn leaving_a_room_clears_particles_from_the_room_just_left() {
    use crate::particles::{ParticleConfig, ParticleSystem};

    let mut game = game();
    let bounds = game.world_bounds();
    let mut system = ParticleSystem::new(ParticleConfig::default());
    system.handle(GameEvent::EnemyDied, Some(Vec2::new(100.0, 100.0)));
    assert!(!system.is_empty());

    game.player.place_on_floor(bounds.right - 20.0, bounds);
    let right = PlayerInput {
        move_axis: 1.0,
        ..PlayerInput::default()
    };
    for _ in 0..60 {
        game.update(right, DT);
        for located in game.take_located_events() {
            system.handle(located.event, located.at);
        }
    }
    assert_eq!(game.room().id, RoomId::Second);
    assert!(system.is_empty());
}
