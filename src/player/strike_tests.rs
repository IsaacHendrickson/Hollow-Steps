//! Pogo rules that need the player's private air-dash state.
use super::*;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;

fn input(attack: bool, down: bool, dash: bool) -> PlayerInput {
    PlayerInput {
        attack_pressed: attack,
        down_held: down,
        dash_pressed: dash,
        ..PlayerInput::default()
    }
}

/// High above the floor with Dash, so a whole dash and its cooldown fit before landing.
fn high_dasher() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.collect_item(Item::Dash);
    player.place_at(Vec2::new(400.0, -3000.0), BOUNDS);
    player
}

fn run(player: &mut Player, seconds: f32) {
    for _ in 0..(seconds / DT).ceil() as usize {
        player.update(PlayerInput::default(), DT, BOUNDS);
    }
}

#[test]
fn a_downward_hit_restores_the_air_dash() {
    let mut player = high_dasher();
    player.update(input(false, false, true), DT, BOUNDS);
    assert!(player.air_dash_used);
    run(&mut player, 0.6);
    // The dash is over and its cooldown has run out, but the air dash is still spent.
    player.update(input(false, false, true), DT, BOUNDS);
    assert_eq!(player.velocity.x, 0.0, "no second air dash before a pogo");
    run(&mut player, 0.6);
    player.update(input(true, true, false), 0.0, BOUNDS);
    assert!(player.pogo());
    assert!(!player.air_dash_used);
    assert_eq!(player.velocity.y, -player.config.strike.pogo_speed);
    run(&mut player, 0.6);
    player.update(input(false, false, true), DT, BOUNDS);
    assert_eq!(player.velocity.x, player.facing * player.config.dash_speed);
    assert!(player.air_dash_used);
}

#[test]
fn a_swing_that_cannot_pogo_leaves_the_air_dash_spent() {
    let mut player = high_dasher();
    player.update(input(false, false, true), DT, BOUNDS);
    assert!(player.air_dash_used);
    run(&mut player, 0.6);
    // Forward swing: pogo() is refused and changes nothing.
    player.update(input(true, false, false), 0.0, BOUNDS);
    let velocity = player.velocity;
    assert!(!player.pogo());
    assert!(player.air_dash_used);
    assert_eq!(player.velocity, velocity);
}
