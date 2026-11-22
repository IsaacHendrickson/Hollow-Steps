//! Puzzle pieces in play, through `GameState::update`: what blocks the player, what a swing
//! hits, and what the notice says. The first sample room is 960 wide with its floor at y = 460;
//! the player is 32 by 48. Pieces are put into the live room through its layout, the way the
//! editor's Test room builds them, and enemies and coins are cleared so only the pieces act.
use std::path::Path;

use super::*;
use crate::combat::StrikeDirection;
use crate::editor::RoomLayout;
use crate::puzzle::{BarrierPlacement, BreakablePlacement, SwitchPlacement};

pub(super) const DT: f32 = 1.0 / 120.0;
const HALF_WIDTH: f32 = 16.0;
const HALF_HEIGHT: f32 = 24.0;
const WALL: Rect = Rect {
    x: 400.0,
    y: 396.0,
    w: 32.0,
    h: 64.0,
};
/// The switch that opens `BARS`, far from everything else.
const PLATE: Rect = Rect {
    x: 700.0,
    y: 436.0,
    w: 24.0,
    h: 24.0,
};
const BARS: Rect = Rect {
    x: 400.0,
    y: 364.0,
    w: 24.0,
    h: 96.0,
};

fn game() -> GameState {
    let mut game =
        GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
            .expect("fixed test assets must load");
    game.rooms[0].enemies.clear();
    game.rooms[0].coins.clear();
    game
}

fn numbers(rect: Rect) -> (f32, f32, f32, f32) {
    (rect.x, rect.y, rect.w, rect.h)
}

fn wall(rect: Rect, hits: u8) -> BreakablePlacement {
    BreakablePlacement {
        rect: numbers(rect),
        hits,
    }
}

fn plate(rect: Rect, id: u8) -> SwitchPlacement {
    SwitchPlacement {
        rect: numbers(rect),
        id,
    }
}

fn bars(rect: Rect, switch: u8) -> BarrierPlacement {
    BarrierPlacement {
        rect: numbers(rect),
        switch,
    }
}

/// The first room with `edit` applied to its layout, as the Test room would build it.
fn with_pieces(game: &mut GameState, edit: impl FnOnce(&mut RoomLayout)) {
    let mut layout = RoomLayout::from_room(&game.rooms[0]);
    edit(&mut layout);
    layout.apply(&mut game.rooms[0]).unwrap();
}

fn walls(game: &mut GameState, list: &[(Rect, u8)]) {
    with_pieces(game, |layout| {
        layout.breakables = list.iter().map(|&(rect, hits)| wall(rect, hits)).collect();
    });
}

/// `BARS` closed, with its switch `PLATE` (number 2) far to the right.
fn barrier_and_switch(game: &mut GameState) {
    with_pieces(game, |layout| {
        layout.switches = vec![plate(PLATE, 2)];
        layout.barriers = vec![bars(BARS, 2)];
    });
}

pub(super) fn walk(axis: f32) -> PlayerInput {
    PlayerInput {
        move_axis: axis,
        ..PlayerInput::default()
    }
}

pub(super) fn strike(attack: bool, up: bool, down: bool) -> PlayerInput {
    PlayerInput {
        attack_pressed: attack,
        up_held: up,
        down_held: down,
        ..PlayerInput::default()
    }
}

pub(super) fn stand_at(game: &mut GameState, x: f32) {
    let bounds = game.world_bounds();
    game.player.place_on_floor(x, bounds);
}

fn hang_at(game: &mut GameState, x: f32, y: f32) {
    let bounds = game.world_bounds();
    game.player.place_at(Vec2::new(x, y), bounds);
}

/// Walks right for `ticks` ticks.
pub(super) fn walk_right(game: &mut GameState, ticks: u32) {
    for _ in 0..ticks {
        game.update(walk(1.0), DT);
    }
}

fn remaining(game: &GameState) -> u8 {
    game.rooms[0].puzzles.breakables()[0].remaining
}

/// One swing, then the rest of its active time, so that every tick of it is played.
pub(super) fn swing(game: &mut GameState, up: bool, down: bool) {
    game.update(strike(true, up, down), DT);
    for _ in 0..20 {
        game.update(strike(false, up, down), DT);
    }
}

/// Waits out the swing's cooldown so that the next one can start.
pub(super) fn rest(game: &mut GameState) {
    game.update(PlayerInput::default(), 1.0);
}

// ---- what blocks the player ----

#[test]
fn the_player_cannot_walk_through_a_standing_wall_but_can_through_a_broken_one() {
    let mut game = game();
    walls(&mut game, &[(WALL, 2)]);
    stand_at(&mut game, 300.0);
    walk_right(&mut game, 300);
    assert!(
        (game.player.position().x - (WALL.x - HALF_WIDTH)).abs() < 0.01,
        "stopped at the wall: {}",
        game.player.position().x
    );

    assert!(game.rooms[0].puzzles.break_at(WALL));
    walk_right(&mut game, 120);
    assert!(game.player.position().x > WALL.right(), "walked through");
}

#[test]
fn the_player_cannot_walk_through_a_closed_barrier_but_can_through_an_open_one() {
    let mut game = game();
    barrier_and_switch(&mut game);
    stand_at(&mut game, 300.0);
    walk_right(&mut game, 300);
    assert!((game.player.position().x - (BARS.x - HALF_WIDTH)).abs() < 0.01);

    assert!(game.rooms[0].puzzles.switch_on_at(PLATE));
    walk_right(&mut game, 120);
    assert!(game.player.position().x > BARS.right());
}

#[test]
fn a_dash_stops_at_a_standing_wall_too() {
    let mut game = game();
    walls(&mut game, &[(WALL, 1)]);
    game.player.collect_item(crate::item::Item::Dash);
    stand_at(&mut game, 340.0);
    let dash = PlayerInput {
        dash_pressed: true,
        ..PlayerInput::default()
    };
    game.update(dash, DT);
    for _ in 0..60 {
        game.update(PlayerInput::default(), DT);
    }
    assert!(game.player.position().x <= WALL.x - HALF_WIDTH + 0.01);
}

#[test]
fn landing_on_a_barrier_works_and_the_player_falls_when_it_opens_under_them() {
    let mut game = game();
    let ledge = Rect::new(300.0, 400.0, 64.0, 16.0);
    with_pieces(&mut game, |layout| {
        layout.switches = vec![plate(PLATE, 2)];
        layout.barriers = vec![bars(ledge, 2)];
    });
    hang_at(&mut game, 330.0, 340.0);
    for _ in 0..120 {
        game.update(PlayerInput::default(), DT);
    }
    assert!(game.player.is_grounded());
    assert!((game.player.position().y - (ledge.y - HALF_HEIGHT)).abs() < 0.01);

    assert!(game.rooms[0].puzzles.switch_on_at(PLATE));
    game.update(PlayerInput::default(), DT);
    assert!(!game.player.is_grounded(), "nothing holds them up now");
    for _ in 0..120 {
        game.update(PlayerInput::default(), DT);
    }
    assert!(game.player.is_grounded());
    assert!((game.player.position().y - (460.0 - HALF_HEIGHT)).abs() < 0.01);
}

#[test]
fn a_standing_wall_is_a_floor_to_land_on() {
    let mut game = game();
    walls(&mut game, &[(WALL, 3)]);
    hang_at(&mut game, 416.0, 300.0);
    for _ in 0..120 {
        game.update(PlayerInput::default(), DT);
    }
    assert!(game.player.is_grounded());
    assert!((game.player.position().y - (WALL.y - HALF_HEIGHT)).abs() < 0.01);
}

#[test]
fn a_head_bump_on_the_underside_of_a_barrier_still_works() {
    let mut game = game();
    let roof = Rect::new(200.0, 380.0, 64.0, 16.0);
    with_pieces(&mut game, |layout| {
        layout.switches = vec![plate(PLATE, 2)];
        layout.barriers = vec![bars(roof, 2)];
    });
    stand_at(&mut game, 232.0);
    let jump = PlayerInput {
        jump_pressed: true,
        ..PlayerInput::default()
    };
    game.update(jump, DT);
    let mut highest = f32::MAX;
    for _ in 0..90 {
        game.update(PlayerInput::default(), DT);
        highest = highest.min(game.player.position().y - HALF_HEIGHT);
    }
    assert!(
        (highest - roof.bottom()).abs() < 0.01,
        "the head stopped at the barrier, not {highest}"
    );
    assert!(game.player.is_grounded(), "and came back down");
}

#[test]
fn a_wall_broken_while_the_player_is_in_the_air_changes_nothing_else() {
    let jump = PlayerInput {
        jump_pressed: true,
        ..PlayerInput::default()
    };
    let play = |break_it: bool| {
        let mut game = game();
        walls(&mut game, &[(WALL, 1)]);
        stand_at(&mut game, 250.0);
        game.update(jump, DT);
        for tick in 0..60 {
            game.update(walk(1.0), DT);
            if break_it && tick == 10 {
                assert!(game.rooms[0].puzzles.break_at(WALL));
            }
        }
        (game.player.position(), game.player.velocity())
    };
    assert_eq!(play(false), play(true));
}

// ---- what a swing hits ----

#[test]
fn a_forward_swing_breaks_a_wall_in_exactly_as_many_swings_as_it_has_hits() {
    let mut game = game();
    walls(&mut game, &[(WALL, 3)]);
    stand_at(&mut game, WALL.x - HALF_WIDTH);
    for expected in [2, 1] {
        swing(&mut game, false, false);
        assert_eq!(remaining(&game), expected, "one hit for the whole swing");
        rest(&mut game);
    }
    swing(&mut game, false, false);
    assert_eq!(remaining(&game), 0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Forward);
    assert_eq!(game.puzzles.broken.len(), 1, "remembered for the run");
}

#[test]
fn a_swing_facing_away_from_the_wall_does_not_touch_it() {
    let mut game = game();
    walls(&mut game, &[(WALL, 3)]);
    stand_at(&mut game, WALL.x - HALF_WIDTH);
    game.update(walk(-1.0), DT);
    swing(&mut game, false, false);
    assert_eq!(remaining(&game), 3);
}

#[test]
fn an_upward_swing_hits_what_is_above_the_player() {
    let mut game = game();
    let reach = game.tuning().player.strike.up_height;
    // A wall whose underside is 2 px above the head: inside the up box, out of the forward one.
    let top = 460.0 - 2.0 * HALF_HEIGHT;
    let above = Rect::new(300.0, top - 2.0 - 8.0, 32.0, 8.0);
    assert!(reach > 2.0);
    walls(&mut game, &[(above, 2)]);
    stand_at(&mut game, 316.0);
    swing(&mut game, false, false);
    assert_eq!(remaining(&game), 2, "a forward swing misses it");
    rest(&mut game);
    swing(&mut game, true, false);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Up);
    assert_eq!(remaining(&game), 1);
    assert_eq!(
        game.player.velocity().y,
        0.0,
        "only a downward swing bounces"
    );
}

#[test]
fn a_downward_swing_hits_a_wall_below_and_gives_no_pogo() {
    let mut game = game();
    let below = Rect::new(300.0, 400.0, 64.0, 32.0);
    walls(&mut game, &[(below, 2)]);
    // Feet 4 px above the wall's top, in the air.
    hang_at(&mut game, 332.0, below.y - 4.0 - HALF_HEIGHT);
    assert!(!game.player.is_grounded());
    game.update(strike(true, false, true), 0.0);
    assert_eq!(game.player.attack().strike(), StrikeDirection::Down);
    assert_eq!(remaining(&game), 1);
    assert!(
        game.player.velocity().y >= 0.0,
        "no upward bounce off a wall: {}",
        game.player.velocity().y
    );
}

#[test]
fn a_swing_hits_only_the_piece_it_overlaps() {
    let mut game = game();
    let near = WALL;
    let far = Rect::new(600.0, 396.0, 32.0, 64.0);
    let next_to_reach = Rect::new(near.x + 80.0, 396.0, 32.0, 64.0);
    walls(&mut game, &[(near, 2), (far, 2), (next_to_reach, 2)]);
    stand_at(&mut game, near.x - HALF_WIDTH);
    swing(&mut game, false, false);
    let left: Vec<u8> = game.rooms[0]
        .puzzles
        .breakables()
        .iter()
        .map(|wall| wall.remaining)
        .collect();
    assert_eq!(left, [1, 2, 2]);
}

#[test]
fn a_swing_can_break_a_wall_and_turn_on_a_switch_at_once_but_hits_each_once() {
    let mut game = game();
    let wall = Rect::new(400.0, 436.0, 16.0, 24.0);
    let plate_rect = Rect::new(416.0, 436.0, 16.0, 24.0);
    with_pieces(&mut game, |layout| {
        layout.breakables = vec![self::wall(wall, 2)];
        layout.switches = vec![plate(plate_rect, 4)];
    });
    stand_at(&mut game, 384.0);
    swing(&mut game, false, false);
    assert_eq!(remaining(&game), 1);
    assert!(game.rooms[0].puzzles.switches()[0].on);
}

// ---- switches ----

#[test]
fn striking_a_switch_opens_its_barrier_says_so_and_it_stays_open() {
    let mut game = game();
    barrier_and_switch(&mut game);
    assert_eq!(game.notice(), None);
    stand_at(&mut game, PLATE.x - HALF_WIDTH);
    swing(&mut game, false, false);

    assert!(game.rooms[0].puzzles.switches()[0].on);
    assert_eq!(game.notice(), Some("Switch 2 on"));
    stand_at(&mut game, 300.0);
    walk_right(&mut game, 300);
    assert!(game.player.position().x > BARS.right(), "walked through");

    // Out of the room and back: still open.
    game.current = RoomId::Second.index();
    game.update(PlayerInput::default(), DT);
    game.current = RoomId::First.index();
    game.update(PlayerInput::default(), DT);
    assert!(game.rooms[0].puzzles.standing_solids().next().is_none());
}

#[test]
fn striking_a_switch_that_is_already_on_says_nothing() {
    let mut game = game();
    barrier_and_switch(&mut game);
    stand_at(&mut game, PLATE.x - HALF_WIDTH);
    swing(&mut game, false, false);
    game.update(PlayerInput::default(), 2.5);
    assert_eq!(game.notice(), None, "the notice ran out");
    rest(&mut game);
    swing(&mut game, false, false);
    assert_eq!(game.notice(), None);
}

#[test]
fn a_swing_that_misses_the_switch_leaves_it_off_and_a_downward_swing_on_one_gives_no_pogo() {
    let mut game = game();
    barrier_and_switch(&mut game);
    stand_at(&mut game, 500.0);
    swing(&mut game, false, false);
    assert!(!game.rooms[0].puzzles.switches()[0].on);

    // Downward onto the plate from just above it.
    hang_at(&mut game, PLATE.x + 12.0, PLATE.y - 4.0 - HALF_HEIGHT);
    game.update(strike(true, false, true), 0.0);
    assert!(game.rooms[0].puzzles.switches()[0].on);
    assert!(game.player.velocity().y >= 0.0, "no pogo off a switch");
}

#[test]
fn nothing_a_test_room_does_is_remembered() {
    let mut game = game();
    game.edit_room(RoomId::First).unwrap();
    {
        let layout = &mut game.editor_mut().unwrap().draft.layout;
        layout.switches = vec![plate(PLATE, 2)];
        layout.breakables = vec![wall(WALL, 1)];
    }
    game.start_test_room().unwrap();
    game.rooms[game.current].enemies.clear();
    stand_at(&mut game, PLATE.x - HALF_WIDTH);
    swing(&mut game, false, false);
    assert!(game.room().puzzles.switches()[0].on);
    assert_eq!(
        game.notice(),
        Some("Switch 2 on"),
        "the author still sees it"
    );
    stand_at(&mut game, WALL.x - HALF_WIDTH);
    rest(&mut game);
    swing(&mut game, false, false);
    assert_eq!(game.room().puzzles.breakables()[0].remaining, 0);
    assert_eq!(game.puzzles, super::world_puzzles::Puzzles::default());
}
