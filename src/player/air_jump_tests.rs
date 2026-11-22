//! The air jump of `Ability::DoubleJump` (`air_jump.rs`): when it is taken, when it comes back,
//! and what must never happen. Every number comes from a config written here, not the shipped one.
use super::*;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;
/// The air jump's speed in these tests: unlike every other jump number, easy to tell apart.
const AIR_SPEED: f32 = 300.0;

fn config() -> PlayerConfig {
    PlayerConfig {
        double_jump_speed: AIR_SPEED,
        ..PlayerConfig::default()
    }
}

fn press() -> PlayerInput {
    PlayerInput {
        jump_pressed: true,
        ..PlayerInput::default()
    }
}

fn held() -> PlayerInput {
    PlayerInput {
        jump_held: true,
        ..PlayerInput::default()
    }
}

fn idle() -> PlayerInput {
    PlayerInput::default()
}

fn dash() -> PlayerInput {
    PlayerInput {
        dash_pressed: true,
        ..PlayerInput::default()
    }
}

/// A player standing on the room floor holding `items`.
fn standing(items: &[Item]) -> Player {
    let mut player = Player::new(Vec2::ZERO, config());
    player.place_on_floor(400.0, BOUNDS);
    for item in items {
        player.collect_item(*item);
    }
    player
}

fn step(player: &mut Player, input: PlayerInput) {
    player.update(input, DT, BOUNDS);
}

fn step_in(player: &mut Player, terrain: &Terrain, input: PlayerInput) {
    player.update_with_terrain(input, DT, BOUNDS, terrain);
}

fn wait(player: &mut Player, ticks: usize) {
    for _ in 0..ticks {
        step(player, idle());
    }
}

/// Steps until the player stands on something.
fn land(player: &mut Player, terrain: &Terrain) {
    for _ in 0..600 {
        if player.is_grounded() {
            return;
        }
        step_in(player, terrain, idle());
    }
    panic!("never landed");
}

/// The upward speed right after a tick in which a jump of `speed` was taken: gravity acts in
/// the same tick.
fn after_launch(speed: f32) -> f32 {
    -speed + config().gravity * DT
}

fn near(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 1e-2,
        "{actual} is not {expected}"
    );
}

fn jumps(player: &mut Player) -> usize {
    player
        .take_events()
        .iter()
        .filter(|event| **event == GameEvent::PlayerJumped)
        .count()
}

fn head(player: &Player) -> f32 {
    player.position.y - player.config.size.y / 2.0
}

#[test]
fn there_is_no_air_jump_without_the_ability() {
    let mut player = standing(&[]);
    step(&mut player, press());
    wait(&mut player, 10);
    let before = player.velocity().y;

    step(&mut player, press());

    near(player.velocity().y, before + config().gravity * DT);
    assert_eq!(jumps(&mut player), 1, "only the jump from the ground");
}

#[test]
fn with_the_ability_there_is_one_air_jump_and_not_a_second() {
    let mut player = standing(&[Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 10);

    step(&mut player, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
    assert!(player.air_jump_used);

    wait(&mut player, 10);
    let before = player.velocity().y;
    step(&mut player, press());
    near(player.velocity().y, before + config().gravity * DT);
    assert_eq!(
        jumps(&mut player),
        2,
        "the ground jump and the one air jump"
    );
}

#[test]
fn the_air_jump_sets_the_speed_whatever_it_was_before() {
    let mut player = standing(&[Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 60); // past the top (a held-off jump peaks near 0.34 s) and falling
    assert!(player.velocity().y > 0.0 && !player.is_grounded());

    step(&mut player, press());

    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn the_air_jump_comes_back_on_landing_on_the_floor() {
    let mut player = standing(&[Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 10);
    step(&mut player, press());
    assert!(player.air_jump_used);

    land(&mut player, &Terrain::default());
    assert!(!player.air_jump_used);

    step(&mut player, press());
    wait(&mut player, 10);
    step(&mut player, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn the_air_jump_comes_back_on_landing_on_a_solid_platform() {
    // A platform (top y=380) well above the floor (y=460).
    let platform = Terrain::from_tuples(&[(200.0, 380.0, 300.0, 20.0)]);
    let mut player = standing(&[Item::DoubleJump]);
    player.place_at(Vec2::new(300.0, 380.0 - 24.0), BOUNDS);
    step_in(&mut player, &platform, idle());
    assert!(player.is_grounded());
    let standing_y = player.position().y;

    step_in(&mut player, &platform, press());
    wait_in(&mut player, &platform, 10);
    step_in(&mut player, &platform, press());
    assert!(player.air_jump_used);

    land(&mut player, &platform);
    assert_eq!(
        player.position().y,
        standing_y,
        "on the platform, not the floor"
    );
    assert!(!player.air_jump_used);
}

fn wait_in(player: &mut Player, terrain: &Terrain, ticks: usize) {
    for _ in 0..ticks {
        step_in(player, terrain, idle());
    }
}

#[test]
fn placing_the_player_gives_the_air_jump_back() {
    let mut player = standing(&[Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 10);
    step(&mut player, press());
    assert!(player.air_jump_used);

    player.place_at(Vec2::new(300.0, 200.0), BOUNDS);

    assert!(!player.air_jump_used, "a room change starts a fresh flight");
    step(&mut player, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn on_the_ground_a_press_is_the_normal_jump_and_the_air_jump_is_still_there() {
    let mut player = standing(&[Item::DoubleJump]);

    step(&mut player, press());

    near(player.velocity().y, after_launch(config().jump_speed));
    assert!(!player.air_jump_used, "one press never makes both jumps");
    assert_eq!(jumps(&mut player), 1);
    wait(&mut player, 10);
    step(&mut player, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn a_held_key_does_not_take_the_air_jump() {
    let mut player = standing(&[Item::DoubleJump]);
    step(
        &mut player,
        PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..idle()
        },
    );
    let mut previous = player.velocity().y;
    for tick in 0..90 {
        step(&mut player, held());
        assert!(
            player.velocity().y >= previous,
            "tick {tick}: the held key made a jump ({} after {previous})",
            player.velocity().y
        );
        previous = player.velocity().y;
    }
    assert!(!player.air_jump_used);
    assert_eq!(jumps(&mut player), 1);
}

/// A platform from x=100 to 300 (top y=400) over the room floor (y=460), and a player walking
/// off its right end. Returns the player on the first tick it is in the air.
fn just_off_the_ledge(items: &[Item]) -> (Player, Terrain) {
    let ledge = Terrain::from_tuples(&[(100.0, 400.0, 200.0, 20.0)]);
    let mut player = standing(items);
    player.place_at(Vec2::new(280.0, 400.0 - 24.0), BOUNDS);
    step_in(&mut player, &ledge, idle());
    assert!(player.is_grounded());
    let walk = PlayerInput {
        move_axis: 1.0,
        ..idle()
    };
    for _ in 0..200 {
        step_in(&mut player, &ledge, walk);
        if !player.is_grounded() {
            return (player, ledge);
        }
    }
    panic!("never left the ledge");
}

#[test]
fn falling_off_a_ledge_without_jumping_still_allows_the_air_jump_after_coyote_expiry() {
    let (mut player, ledge) = just_off_the_ledge(&[Item::DoubleJump]);
    // The coyote window is 0.1 s (12 ticks); wait it out without pressing anything.
    wait_in(&mut player, &ledge, 14);
    assert_eq!(player.coyote_remaining, 0.0);
    assert!(!player.is_grounded() && !player.air_jump_used);

    step_in(&mut player, &ledge, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
    assert!(player.air_jump_used);

    // Once: a second press in the same flight does nothing.
    wait_in(&mut player, &ledge, 5);
    let before = player.velocity().y;
    step_in(&mut player, &ledge, press());
    near(player.velocity().y, before + config().gravity * DT);
}

#[test]
fn a_press_inside_the_coyote_window_is_the_normal_jump_and_the_air_jump_is_left() {
    let (mut player, ledge) = just_off_the_ledge(&[Item::DoubleJump]);
    wait_in(&mut player, &ledge, 2);
    assert!(player.coyote_remaining > 0.0);

    step_in(&mut player, &ledge, press());

    near(player.velocity().y, after_launch(config().jump_speed));
    assert!(!player.air_jump_used);
    wait_in(&mut player, &ledge, 10);
    step_in(&mut player, &ledge, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn the_air_jump_itself_refuses_while_the_coyote_window_is_open() {
    // The caller offers a press to the ground jump first, so this is the guard's own test.
    let (mut player, ledge) = just_off_the_ledge(&[Item::DoubleJump]);
    wait_in(&mut player, &ledge, 2);
    assert!(player.coyote_remaining > 0.0);
    player.jump_buffer_remaining = 0.1;

    assert!(!player.try_air_jump());

    assert!(!player.air_jump_used);
    assert_eq!(
        player.jump_buffer_remaining, 0.1,
        "the press is still waiting"
    );
}

#[test]
fn a_ceiling_bump_stops_the_air_jump_too() {
    // The block's underside is y=360, as in the dash ceiling tests.
    let ceiling = Terrain::from_tuples(&[(100.0, 340.0, 400.0, 20.0)]);
    let mut player = standing(&[Item::DoubleJump]);
    player.place_on_floor(150.0, BOUNDS);
    step_in(
        &mut player,
        &ceiling,
        PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..idle()
        },
    );
    for _ in 0..120 {
        if player.velocity().y == 0.0 && (head(&player) - 360.0).abs() < 0.01 {
            break;
        }
        step_in(&mut player, &ceiling, held());
    }
    assert!(
        (head(&player) - 360.0).abs() < 0.01,
        "never met the ceiling"
    );
    assert!(!player.is_grounded());

    step_in(
        &mut player,
        &ceiling,
        PlayerInput {
            jump_pressed: true,
            jump_held: true,
            ..idle()
        },
    );

    assert!(player.air_jump_used, "the press was taken");
    assert!(head(&player) >= 360.0 - 0.01, "went through the ceiling");
    assert!(!ceiling.overlaps_body(player.position(), player.size()));
    assert_eq!(player.velocity().y, 0.0, "the ceiling ends the rise");
}

#[test]
fn a_press_during_a_dash_is_ignored_and_the_air_jump_works_once_the_dash_ends() {
    let mut player = standing(&[Item::Dash, Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 5);
    step(&mut player, dash());
    assert!(player.dash_remaining > 0.0);
    wait(&mut player, 2);

    // Early in the dash: ignored, and the press is gone before the dash is over.
    step(&mut player, press());
    while player.dash_remaining > 0.0 {
        assert_eq!(player.velocity().y, 0.0, "nothing rises during the dash");
        step(&mut player, idle());
    }
    step(&mut player, idle());
    assert!(
        !player.air_jump_used,
        "the early press did not wait for the end"
    );
    assert!(player.velocity().y > 0.0);

    // A fresh press after the dash is the air jump.
    step(&mut player, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn a_press_in_the_last_moments_of_a_dash_is_kept_by_the_buffer_and_taken_when_it_ends() {
    let mut player = standing(&[Item::Dash, Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 5);
    step(&mut player, dash());
    while player.dash_remaining > 0.03 {
        step(&mut player, idle());
    }

    step(&mut player, press());
    while player.dash_remaining > 0.0 {
        assert!(!player.air_jump_used, "taken during the dash");
        step(&mut player, idle());
    }
    step(&mut player, idle());

    assert!(player.air_jump_used);
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn picking_up_the_item_grants_the_air_jump_and_only_it() {
    let mut player = standing(&[]);
    assert!(!player.abilities().has(Ability::DoubleJump));

    player.collect_item(Item::DoubleJump);

    assert!(player.abilities().has(Ability::DoubleJump));
    assert!(!player.abilities().has(Ability::Dash), "not the dash");
    assert_eq!(
        player.collected_items(),
        &[Item::DoubleJump],
        "still an item"
    );
    // Granted once however many are taken.
    player.collect_item(Item::DoubleJump);
    assert_eq!(player.collected_items().len(), 2);
    step(&mut player, press());
    wait(&mut player, 10);
    step(&mut player, press());
    near(player.velocity().y, after_launch(AIR_SPEED));
}

#[test]
fn the_dash_item_alone_gives_no_air_jump() {
    let mut player = standing(&[Item::Dash]);
    step(&mut player, press());
    wait(&mut player, 10);
    let before = player.velocity().y;

    step(&mut player, press());

    near(player.velocity().y, before + config().gravity * DT);
}

#[test]
fn a_run_reset_takes_the_air_jump_away() {
    let mut player = standing(&[Item::DoubleJump]);
    player.reset_for_run();
    assert!(player.abilities().is_empty());

    player.place_on_floor(400.0, BOUNDS);
    step(&mut player, press());
    wait(&mut player, 10);
    let before = player.velocity().y;
    step(&mut player, press());

    near(player.velocity().y, before + config().gravity * DT);
}

#[test]
fn a_zero_or_a_very_long_step_takes_the_air_jump_once() {
    let mut player = standing(&[Item::DoubleJump]);
    step(&mut player, press());
    wait(&mut player, 10);

    player.update(press(), 0.0, BOUNDS);
    assert_eq!(
        player.velocity().y,
        -AIR_SPEED,
        "no time passed, no gravity"
    );
    assert!(player.air_jump_used);

    // A long step lands, refills, and does not spend the refill on the same press.
    let mut other = standing(&[Item::DoubleJump]);
    step(&mut other, press());
    wait(&mut other, 10);
    other.update(press(), 0.5, BOUNDS);
    assert_eq!(jumps(&mut other), 2, "the ground jump and one air jump");
}
