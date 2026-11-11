//! Player-side `GameEvent`s: jump, landing, dash and swing start.
use super::*;
use crate::item::Item;
use crate::terrain::Terrain;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;

fn input() -> PlayerInput {
    PlayerInput::default()
}

fn grounded_player() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(400.0, BOUNDS);
    player
}

/// Steps with `first` on the first tick, collecting events from every tick.
fn run(player: &mut Player, terrain: &Terrain, first: PlayerInput, ticks: usize) -> Vec<GameEvent> {
    let mut events = Vec::new();
    for tick in 0..ticks {
        let step = if tick == 0 { first } else { input() };
        player.update_with_terrain(step, DT, BOUNDS, terrain);
        events.extend(player.take_events());
    }
    events
}

/// Steps with the same `held` input on every tick, collecting events from every tick.
fn run_held(
    player: &mut Player,
    terrain: &Terrain,
    held: PlayerInput,
    ticks: usize,
) -> Vec<GameEvent> {
    let mut events = Vec::new();
    for _ in 0..ticks {
        player.update_with_terrain(held, DT, BOUNDS, terrain);
        events.extend(player.take_events());
    }
    events
}

/// Center height of a player standing on the platform.
fn platform_top_y() -> f32 {
    380.0 - PlayerConfig::default().size.y / 2.0
}

fn platform() -> Terrain {
    Terrain::from_tuples(&[(300.0, 380.0, 200.0, 20.0)])
}

#[test]
fn a_jump_raises_one_jump_and_one_landing() {
    let mut player = grounded_player();
    let jump = PlayerInput {
        jump_pressed: true,
        ..input()
    };
    let events = run(&mut player, &Terrain::default(), jump, 300);
    assert_eq!(events, [GameEvent::PlayerJumped, GameEvent::PlayerLanded]);
}

#[test]
fn standing_still_or_walking_on_the_floor_raises_nothing() {
    let mut player = grounded_player();
    let walk = PlayerInput {
        move_axis: 1.0,
        ..input()
    };
    assert!(run_held(&mut player, &Terrain::default(), walk, 120).is_empty());
}

#[test]
fn a_jump_ignored_while_airborne_raises_no_event() {
    let mut player = grounded_player();
    let jump = PlayerInput {
        jump_pressed: true,
        ..input()
    };
    let mut events = run(&mut player, &Terrain::default(), jump, 10);
    // Pressing jump again mid-air (past the buffer) neither jumps nor lands.
    events.extend(run(&mut player, &Terrain::default(), jump, 1));
    assert_eq!(events, [GameEvent::PlayerJumped]);
}

#[test]
fn a_buffered_jump_pressed_just_before_landing_raises_jump_after_landing() {
    let mut player = grounded_player();
    let terrain = Terrain::default();
    let jump = PlayerInput {
        jump_pressed: true,
        ..input()
    };
    let mut events = run(&mut player, &terrain, jump, 1);
    // Fall until just above the floor, press jump, then land into the buffered jump.
    while player.position().y + player.size().y / 2.0 < BOUNDS.floor_y - 6.0
        || player.velocity().y < 0.0
    {
        player.update_with_terrain(input(), DT, BOUNDS, &terrain);
        events.extend(player.take_events());
    }
    events.extend(run(&mut player, &terrain, jump, 6));
    assert_eq!(
        events,
        [
            GameEvent::PlayerJumped,
            GameEvent::PlayerLanded,
            GameEvent::PlayerJumped
        ]
    );
}

#[test]
fn walking_off_a_ledge_and_falling_raises_a_landing_but_no_jump() {
    let terrain = platform();
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_at(Vec2::new(480.0, platform_top_y()), BOUNDS);
    let settle = run(&mut player, &terrain, input(), 10);
    assert!(settle.is_empty(), "standing on a platform is not a landing");
    let right = PlayerInput {
        move_axis: 1.0,
        ..input()
    };
    let events = run_held(&mut player, &terrain, right, 240);
    assert!(player.is_grounded());
    assert_eq!(events, [GameEvent::PlayerLanded]);
}

#[test]
fn placing_the_player_on_a_platform_is_not_a_landing() {
    let terrain = platform();
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    // Placement ignores solids, so the first update finds the platform under the player.
    player.place_at(Vec2::new(400.0, platform_top_y()), BOUNDS);
    let events = run(&mut player, &terrain, input(), 30);
    assert!(player.is_grounded());
    assert!(events.is_empty());
}

#[test]
fn a_spawn_in_the_air_raises_a_landing_when_the_player_touches_down() {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_at(Vec2::new(400.0, 200.0), BOUNDS);
    let events = run(&mut player, &Terrain::default(), input(), 300);
    assert_eq!(events, [GameEvent::PlayerLanded]);
}

#[test]
fn a_ground_dash_raises_one_dash_and_a_second_press_during_it_none() {
    let mut player = grounded_player();
    player.collect_item(Item::Dash);
    let dash = PlayerInput {
        dash_pressed: true,
        ..input()
    };
    let mut events = run(&mut player, &Terrain::default(), dash, 1);
    events.extend(run(&mut player, &Terrain::default(), dash, 5));
    assert_eq!(events, [GameEvent::PlayerDashed]);
}

#[test]
fn dashing_without_the_item_raises_nothing() {
    let mut player = grounded_player();
    let dash = PlayerInput {
        dash_pressed: true,
        ..input()
    };
    assert!(run(&mut player, &Terrain::default(), dash, 10).is_empty());
}

#[test]
fn a_swing_raises_one_attack_and_the_cooldown_blocks_repeats() {
    let mut player = grounded_player();
    let swing = PlayerInput {
        attack_pressed: true,
        ..input()
    };
    let mut events = run(&mut player, &Terrain::default(), swing, 1);
    // Pressing again right away is still on cooldown.
    events.extend(run(&mut player, &Terrain::default(), swing, 5));
    assert_eq!(events, [GameEvent::PlayerAttacked]);
    // After the cooldown a new swing is a new event.
    run(&mut player, &Terrain::default(), input(), 60);
    events.extend(run(&mut player, &Terrain::default(), swing, 1));
    assert_eq!(
        events,
        [GameEvent::PlayerAttacked, GameEvent::PlayerAttacked]
    );
}

#[test]
fn zero_and_non_finite_steps_raise_nothing() {
    let mut player = grounded_player();
    for dt in [0.0, f32::NAN, f32::INFINITY, -1.0] {
        player.update_with_terrain(input(), dt, BOUNDS, &Terrain::default());
    }
    assert!(player.take_events().is_empty());
}
