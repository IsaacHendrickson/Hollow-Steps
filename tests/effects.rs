//! Gameplay events through the particle system, using only the public API.
mod support;

use hollow_steps::events::GameEvent;
use hollow_steps::game::GameState;
use hollow_steps::particles::{Particle, ParticleConfig, ParticleSystem};
use hollow_steps::player::PlayerInput;
use macroquad::math::Vec2;

const DT: f32 = 1.0 / 120.0;

/// Jumps from the start of the game and lands again, feeding every event to the particles
/// and stepping them each tick, as the front end does. Returns what is left alive after
/// the run and every event that had a position, with it.
fn jump_and_land() -> (Vec<Particle>, Vec<(GameEvent, Vec2)>) {
    let mut game = GameState::try_new(&support::fixture_assets()).expect("fixtures load");
    let mut system = ParticleSystem::new(ParticleConfig::default());
    let mut seen = Vec::new();
    for tick in 0..300 {
        game.update(
            PlayerInput {
                jump_pressed: tick == 0,
                ..PlayerInput::default()
            },
            DT,
        );
        system.update(DT);
        for located in game.take_located_events() {
            system.handle(located.event, located.at);
            if let Some(at) = located.at {
                seen.push((located.event, at));
            }
        }
    }
    (system.particles().to_vec(), seen)
}

#[test]
fn a_jump_and_landing_raise_located_events_at_the_players_feet_and_spawn_dust() {
    let (_, seen) = jump_and_land();
    let jumps: Vec<_> = seen
        .iter()
        .filter(|(event, _)| *event == GameEvent::PlayerJumped)
        .collect();
    let landings: Vec<_> = seen
        .iter()
        .filter(|(event, _)| *event == GameEvent::PlayerLanded)
        .collect();
    assert_eq!(jumps.len(), 1);
    assert_eq!(landings.len(), 1);
    // Landing is on the floor line. Takeoff is where the feet are at the end of the tick the
    // jump began in: just off the floor, never below it and never far above it.
    let floor = GameState::try_new(&support::fixture_assets())
        .expect("fixtures load")
        .world_bounds()
        .floor_y;
    assert!((landings[0].1.y - floor).abs() < 1.0);
    assert!(jumps[0].1.y <= floor + 1.0 && floor - jumps[0].1.y < 16.0);

    // The shipped dust preset reacts to both, so a system fed these events is not empty
    // right after them.
    let mut system = ParticleSystem::new(ParticleConfig::default());
    let spawned = system.handle(GameEvent::PlayerJumped, Some(jumps[0].1))
        + system.handle(GameEvent::PlayerLanded, Some(landings[0].1));
    assert!(spawned > 0);
}

#[test]
fn the_whole_pipeline_is_deterministic() {
    let (first, _) = jump_and_land();
    let (second, _) = jump_and_land();
    assert_eq!(first, second);
}
