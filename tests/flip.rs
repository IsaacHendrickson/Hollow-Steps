//! The player is left-right symmetric: the assumption the dungeon flip stands on
//! (`docs/variety.md`). The real `Player` is stepped through a scripted sequence on a small test
//! world, and again on its mirror image with every move input mirrored. At every tick the mirror
//! player must be where the first one is, seen in the mirror: x negated, y the same, horizontal
//! velocity negated, vertical velocity the same, facing negated. If this fails, the flip must not
//! ship: a mirrored room would then play differently from the original.
use hollow_steps::events::GameEvent;
use hollow_steps::item::Item;
use hollow_steps::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use hollow_steps::terrain::Terrain;
use macroquad::math::{Rect, Vec2};

/// The step the game runs at (`timestep.rs`).
const DT: f32 = 1.0 / 120.0;
const TOLERANCE: f32 = 1e-3;
const FLOOR_Y: f32 = 460.0;

/// The mirror axis is x = 0, so a room around it maps onto itself.
fn bounds() -> WorldBounds {
    WorldBounds::new(-600.0, 600.0, FLOOR_Y).with_top(0.0)
}

/// A world that is not itself symmetric, so the mirror is a different world: a floating platform
/// to land on and bump, a tall wall to run into, and a low ledge beyond it.
fn solids() -> Vec<Rect> {
    vec![
        Rect::new(-100.0, 380.0, 160.0, 16.0),
        Rect::new(200.0, 300.0, 40.0, 160.0),
        Rect::new(320.0, 410.0, 90.0, 50.0),
        Rect::new(-420.0, 330.0, 70.0, 130.0),
    ]
}

fn mirror_rect(rect: Rect) -> Rect {
    Rect::new(-rect.x - rect.w, rect.y, rect.w, rect.h)
}

fn world(mirrored: bool) -> Terrain {
    Terrain::new(
        solids()
            .into_iter()
            .map(|rect| if mirrored { mirror_rect(rect) } else { rect })
            .collect(),
    )
}

#[derive(Clone, Copy)]
struct Keys {
    axis: f32,
    jump: bool,
    hold: bool,
    dash: bool,
    attack: bool,
}

const IDLE: Keys = Keys {
    axis: 0.0,
    jump: false,
    hold: false,
    dash: false,
    attack: false,
};

fn run(ticks: usize, axis: f32) -> Vec<Keys> {
    vec![Keys { axis, ..IDLE }; ticks]
}

/// The jump key goes down on the first tick and is held for `held` ticks while moving.
fn jump(held: usize, axis: f32) -> Vec<Keys> {
    (0..held)
        .map(|tick| Keys {
            axis,
            jump: tick == 0,
            hold: true,
            ..IDLE
        })
        .collect()
}

/// A second press of the jump key in the air (the air jump), held a while.
fn second_press(held: usize, axis: f32) -> Vec<Keys> {
    jump(held, axis)
}

fn dash(axis: f32) -> Vec<Keys> {
    vec![Keys {
        axis,
        dash: true,
        ..IDLE
    }]
}

fn swing(axis: f32) -> Vec<Keys> {
    vec![Keys {
        axis,
        attack: true,
        ..IDLE
    }]
}

fn mirrored(keys: Keys) -> Keys {
    Keys {
        axis: -keys.axis,
        ..keys
    }
}

fn input(keys: Keys) -> PlayerInput {
    PlayerInput {
        move_axis: keys.axis,
        jump_pressed: keys.jump,
        jump_held: keys.hold,
        dash_pressed: keys.dash,
        attack_pressed: keys.attack,
        ..PlayerInput::default()
    }
}

/// Run, jump, dash in the air, land, run again, jump twice, run into the wall and hold against
/// it, turn round and do it all to the left, jump under the platform, dash on the ground, swing.
fn script() -> Vec<Keys> {
    [
        run(60, 1.0),
        jump(50, 1.0),
        run(10, 1.0),
        jump(12, 1.0),
        dash(1.0),
        run(80, 1.0),
        jump(25, 1.0),
        second_press(40, 1.0),
        run(100, 1.0),
        run(40, 0.0),
        run(70, -1.0),
        jump(45, -1.0),
        dash(-1.0),
        run(60, -1.0),
        jump(20, -1.0),
        second_press(30, -1.0),
        run(120, -1.0),
        jump(60, 0.0),
        run(30, 1.0),
        dash(1.0),
        run(50, 1.0),
        swing(1.0),
        run(40, 1.0),
        jump(30, 1.0),
        swing(-1.0),
        run(60, -1.0),
        run(20, 0.0),
    ]
    .concat()
}

/// A player standing at `x` on the floor, holding Dash and Double Jump.
fn player_at(x: f32, bounds: WorldBounds) -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.collect_item(Item::Dash);
    player.collect_item(Item::DoubleJump);
    player.place_on_floor(x, bounds);
    player
}

fn assert_mirrored(tick: usize, a: &Player, b: &Player) {
    let (pa, pb) = (a.position(), b.position());
    let (va, vb) = (a.velocity(), b.velocity());
    let context = format!("tick {tick}: original {pa:?} {va:?}, mirror {pb:?} {vb:?}");
    assert!((pa.x + pb.x).abs() <= TOLERANCE, "x: {context}");
    assert!((pa.y - pb.y).abs() <= TOLERANCE, "y: {context}");
    assert!((va.x + vb.x).abs() <= TOLERANCE, "vx: {context}");
    assert!((va.y - vb.y).abs() <= TOLERANCE, "vy: {context}");
    assert_eq!(a.is_grounded(), b.is_grounded(), "grounded: {context}");
    assert_eq!(a.facing(), -b.facing(), "facing: {context}");
}

#[test]
fn the_player_moves_the_same_in_a_mirror() {
    let (world_a, world_b) = (world(false), world(true));
    let (bounds_a, bounds_b) = (bounds(), bounds());
    let start = -480.0;
    let mut a = player_at(start, bounds_a);
    let mut b = player_at(-start, bounds_b);
    let (mut jumps, mut dashes, mut landings) = (0, 0, 0);
    let mut walls = 0;
    let mut airborne_after_dash = false;
    for (tick, keys) in script().into_iter().enumerate() {
        a.update_with_terrain(input(keys), DT, bounds_a, &world_a);
        b.update_with_terrain(input(mirrored(keys)), DT, bounds_b, &world_b);
        assert_mirrored(tick, &a, &b);
        // The sequence must really exercise what it claims to.
        if keys.axis != 0.0 && a.velocity().x == 0.0 && a.position().x.abs() < 599.0 {
            walls += 1;
        }
        for (events_a, events_b) in a.take_events().into_iter().zip(b.take_events()) {
            assert_eq!(events_a, events_b, "tick {tick}: the same events");
            match events_a {
                GameEvent::PlayerJumped => jumps += 1,
                GameEvent::PlayerDashed => {
                    dashes += 1;
                    airborne_after_dash |= !a.is_grounded();
                }
                GameEvent::PlayerLanded => landings += 1,
                _ => {}
            }
        }
    }
    assert!(jumps >= 7, "jumps and air jumps happened: {jumps}");
    assert!(dashes >= 3, "dashes happened: {dashes}");
    assert!(airborne_after_dash, "one dash was in the air");
    assert!(landings >= 4, "landings happened: {landings}");
    assert!(walls >= 20, "the player pushed against solids: {walls}");
}

#[test]
fn a_swing_has_a_mirrored_hitbox() {
    let (world_a, world_b) = (world(false), world(true));
    let (bounds_a, bounds_b) = (bounds(), bounds());
    let mut a = player_at(-480.0, bounds_a);
    let mut b = player_at(480.0, bounds_b);
    let mut checked = 0;
    for axis in [1.0, -1.0] {
        for keys in [run(20, axis), swing(axis), run(40, axis)].concat() {
            a.update_with_terrain(input(keys), DT, bounds_a, &world_a);
            b.update_with_terrain(input(mirrored(keys)), DT, bounds_b, &world_b);
            match (a.strike_hitbox(), b.strike_hitbox()) {
                (Some(ra), Some(rb)) => {
                    checked += 1;
                    let mirror = mirror_rect(ra);
                    for (left, right) in [
                        (mirror.x, rb.x),
                        (mirror.y, rb.y),
                        (mirror.w, rb.w),
                        (mirror.h, rb.h),
                    ] {
                        assert!((left - right).abs() <= TOLERANCE, "{ra:?} vs {rb:?}");
                    }
                }
                (None, None) => {}
                (ra, rb) => panic!("one swing is live, the other not: {ra:?} / {rb:?}"),
            }
        }
    }
    assert!(checked > 0, "a swing was live at some tick");
}

#[test]
fn a_mirror_that_is_not_mirrored_would_be_caught() {
    // The same inputs on the mirrored world (not mirrored) must NOT stay mirrored, or the test
    // above proves nothing.
    let (world_a, world_b) = (world(false), world(true));
    let (bounds_a, bounds_b) = (bounds(), bounds());
    let mut a = player_at(-480.0, bounds_a);
    let mut b = player_at(480.0, bounds_b);
    let mut apart = false;
    for keys in script() {
        a.update_with_terrain(input(keys), DT, bounds_a, &world_a);
        b.update_with_terrain(input(keys), DT, bounds_b, &world_b);
        apart |= (a.position().x + b.position().x).abs() > 1.0;
    }
    assert!(apart, "the unmirrored inputs gave a different walk");
}
