//! What a run has broken and switched on, through `GameState`. The world is a scratch copy of
//! `tests/fixtures`: the first room is 960 wide with its floor at y = 460 and its spawn at
//! (180, 436). The pieces go into the room's file through the editor, the way an author puts them
//! there, and are struck with `Puzzles::strike` (the swing itself is tested with `GameState::step`
//! in `puzzle_play_tests`).
use macroquad::math::Rect;

use super::super::charm_tests::Scratch;
use super::super::run_tests;
use super::super::*;
use super::Puzzles;
use crate::editor::RoomLayout;
use crate::puzzle::{BarrierPlacement, BreakablePlacement, SwitchPlacement};

const DT: f32 = 1.0 / 128.0;
const WALL: Rect = Rect {
    x: 400.0,
    y: 396.0,
    w: 32.0,
    h: 64.0,
};
const PLATE: Rect = Rect {
    x: 500.0,
    y: 436.0,
    w: 24.0,
    h: 24.0,
};
const BARS: Rect = Rect {
    x: 600.0,
    y: 364.0,
    w: 24.0,
    h: 96.0,
};
/// A swing box over the wall, and one over the switch.
const OVER_WALL: Rect = Rect {
    x: 380.0,
    y: 410.0,
    w: 38.0,
    h: 32.0,
};
const OVER_PLATE: Rect = Rect {
    x: 490.0,
    y: 436.0,
    w: 38.0,
    h: 32.0,
};

fn clear_rooms(game: &mut GameState) {
    for room in &mut game.rooms {
        room.enemies.clear();
        room.coins.clear();
    }
}

fn game_in(scratch: &Scratch) -> GameState {
    let mut game = GameState::try_new(&scratch.0).expect("fixed test assets must load");
    clear_rooms(&mut game);
    game
}

fn room_of(game: &GameState, id: RoomId) -> &Room {
    game.rooms.iter().find(|room| room.id == id).unwrap()
}

/// A 3-hit wall, switch 2 and a barrier opened by switch 2: the three pieces the tests use.
fn put_pieces(layout: &mut RoomLayout) {
    layout.breakables = vec![BreakablePlacement {
        rect: (WALL.x, WALL.y, WALL.w, WALL.h),
        hits: 3,
    }];
    layout.switches = vec![SwitchPlacement {
        rect: (PLATE.x, PLATE.y, PLATE.w, PLATE.h),
        id: 2,
    }];
    layout.barriers = vec![BarrierPlacement {
        rect: (BARS.x, BARS.y, BARS.w, BARS.h),
        switch: 2,
    }];
}

/// F2 on `id`, `edit` the draft's layout, Save, F2: the way an author changes a room's file.
fn edit_and_save(game: &mut GameState, id: RoomId, edit: impl FnOnce(&mut RoomLayout)) {
    game.edit_room(id).unwrap();
    edit(&mut game.editor_mut().unwrap().draft.layout);
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
}

fn save_unchanged(game: &mut GameState, id: RoomId) {
    edit_and_save(game, id, |_| {});
}

/// One swing with `hitbox` in room `id`, the way play does it (remembered unless testing).
fn strike(game: &mut GameState, id: RoomId, hitbox: Rect, swing: u64) -> Option<String> {
    let remember = !game.testing_room();
    let room = game.rooms.iter_mut().find(|room| room.id == id).unwrap();
    game.puzzles.strike(room, hitbox, swing, remember)
}

/// Three swings over the wall, which break it, and one over the switch.
fn break_and_switch(game: &mut GameState, id: RoomId) {
    for swing in 1..=3 {
        strike(game, id, OVER_WALL, swing);
    }
    strike(game, id, OVER_PLATE, 4);
}

fn wall_remaining(game: &GameState, id: RoomId) -> u8 {
    room_of(game, id).puzzles.breakables()[0].remaining
}

fn switch_on(game: &GameState, id: RoomId) -> bool {
    room_of(game, id).puzzles.switches()[0].on
}

fn barrier_open(game: &GameState, id: RoomId) -> bool {
    let puzzles = &room_of(game, id).puzzles;
    puzzles.is_open(&puzzles.barriers()[0])
}

/// A world whose first room has the three pieces in its file.
fn world_with_pieces() -> (Scratch, GameState) {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    edit_and_save(&mut game, RoomId::First, put_pieces);
    clear_rooms(&mut game);
    (scratch, game)
}

#[test]
fn a_struck_wall_and_switch_are_remembered_and_the_notice_names_the_switch() {
    let (_scratch, mut game) = world_with_pieces();
    assert_eq!(wall_remaining(&game, RoomId::First), 3);
    assert!(!switch_on(&game, RoomId::First));

    for swing in 1..=2 {
        assert_eq!(strike(&mut game, RoomId::First, OVER_WALL, swing), None);
    }
    assert_eq!(game.puzzles.broken.len(), 0, "cracked is not broken");
    assert_eq!(strike(&mut game, RoomId::First, OVER_WALL, 3), None);
    assert_eq!(game.puzzles.broken.len(), 1);

    let said = strike(&mut game, RoomId::First, OVER_PLATE, 4);
    assert_eq!(said.as_deref(), Some("Switch 2 on"));
    assert_eq!(game.puzzles.on.len(), 1);
    assert_eq!(
        strike(&mut game, RoomId::First, OVER_PLATE, 5),
        None,
        "a switch already on says nothing and is not remembered twice"
    );
    assert_eq!(game.puzzles.on.len(), 1);
}

#[test]
fn a_barrier_stays_open_when_the_player_leaves_the_room_and_returns() {
    let (_scratch, mut game) = world_with_pieces();
    strike(&mut game, RoomId::First, OVER_PLATE, 1);
    assert!(barrier_open(&game, RoomId::First));

    game.current = RoomId::Second.index();
    game.update(PlayerInput::default(), DT);
    game.current = RoomId::First.index();
    game.update(PlayerInput::default(), DT);

    assert!(barrier_open(&game, RoomId::First));
    assert!(!room_of(&game, RoomId::First)
        .puzzles
        .standing_solids()
        .any(|rect| rect == BARS));
}

// ---- the editor's Save ----

#[test]
fn an_editor_save_keeps_broken_walls_and_switches_on_and_writes_nothing_about_them() {
    let (scratch, mut game) = world_with_pieces();
    break_and_switch(&mut game, RoomId::First);
    assert_eq!(wall_remaining(&game, RoomId::First), 0);
    assert!(switch_on(&game, RoomId::First));
    let before = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();

    save_unchanged(&mut game, RoomId::First);

    assert_eq!(
        wall_remaining(&game, RoomId::First),
        0,
        "the rebuilt wall is broken again"
    );
    assert!(switch_on(&game, RoomId::First));
    assert!(barrier_open(&game, RoomId::First));
    let layout = RoomLayout::from_room(room_of(&game, RoomId::First));
    assert_eq!(layout.breakables.len(), 1, "it is still in the room's file");
    assert_eq!(layout.breakables[0].hits, 3);
    let after = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();
    assert_eq!(before, after, "breaking things never changes the file");
    assert_eq!((game.puzzles.broken.len(), game.puzzles.on.len()), (1, 1));
}

#[test]
fn a_wall_that_was_cracked_but_not_broken_is_whole_after_a_save() {
    let (_scratch, mut game) = world_with_pieces();
    strike(&mut game, RoomId::First, OVER_WALL, 1);
    assert_eq!(wall_remaining(&game, RoomId::First), 2);
    save_unchanged(&mut game, RoomId::First);
    assert_eq!(wall_remaining(&game, RoomId::First), 3);
}

#[test]
fn a_save_that_moves_resizes_or_deletes_a_piece_forgets_it() {
    let (_scratch, mut game) = world_with_pieces();
    break_and_switch(&mut game, RoomId::First);

    edit_and_save(&mut game, RoomId::First, |layout| {
        layout.breakables[0].rect.0 += 8.0; // moved
        layout.switches[0].rect.2 += 8.0; // resized
    });
    assert_eq!(
        wall_remaining(&game, RoomId::First),
        3,
        "a moved wall is new"
    );
    assert!(!switch_on(&game, RoomId::First), "a resized switch is new");
    assert!(!barrier_open(&game, RoomId::First));
    assert_eq!((game.puzzles.broken.len(), game.puzzles.on.len()), (0, 0));

    break_and_switch(&mut game, RoomId::First);
    edit_and_save(&mut game, RoomId::First, |layout| {
        layout.barriers.clear();
        layout.breakables.clear();
        layout.switches.clear();
    });
    assert_eq!((game.puzzles.broken.len(), game.puzzles.on.len()), (0, 0));
    edit_and_save(&mut game, RoomId::First, put_pieces);
    assert_eq!(
        wall_remaining(&game, RoomId::First),
        3,
        "put back: new again"
    );
    assert!(!switch_on(&game, RoomId::First));
}

#[test]
fn a_save_changes_only_the_number_not_the_place_and_the_piece_is_still_the_same_one() {
    let (_scratch, mut game) = world_with_pieces();
    break_and_switch(&mut game, RoomId::First);
    edit_and_save(&mut game, RoomId::First, |layout| {
        layout.breakables[0].hits = 5;
    });
    assert_eq!(wall_remaining(&game, RoomId::First), 0, "same rectangle");
}

#[test]
fn saving_another_room_leaves_this_rooms_walls_and_switches_alone() {
    let (_scratch, mut game) = world_with_pieces();
    break_and_switch(&mut game, RoomId::First);
    strike(&mut game, RoomId::First, OVER_WALL, 99);
    save_unchanged(&mut game, RoomId::Second);
    assert_eq!(wall_remaining(&game, RoomId::First), 0);
    assert!(switch_on(&game, RoomId::First));
    assert_eq!((game.puzzles.broken.len(), game.puzzles.on.len()), (1, 1));
}

// ---- runs and test rooms ----

#[test]
fn a_run_reset_makes_every_wall_whole_and_every_switch_off_again() {
    let scratch = run_tests::world(100.0);
    let mut game = run_tests::game_in(&scratch);
    edit_and_save(&mut game, RoomId::First, put_pieces);
    // The editor put the player outside the hub; the next tick counts that as walking out.
    game.update(PlayerInput::default(), DT);
    assert!(game.run_in_progress());
    clear_rooms(&mut game);
    break_and_switch(&mut game, RoomId::First);
    assert!(barrier_open(&game, RoomId::First));

    run_tests::kill(&mut game);
    assert!(game.run_result().is_some());
    game.continue_after_result();
    run_tests::start_run(&mut game);

    assert_eq!(wall_remaining(&game, RoomId::First), 3);
    assert!(!switch_on(&game, RoomId::First));
    assert!(!barrier_open(&game, RoomId::First));
    assert_eq!(game.puzzles, Puzzles::default());
}

#[test]
fn a_test_room_is_a_sandbox_and_ending_it_restores_what_the_saved_room_had() {
    let (_scratch, mut game) = world_with_pieces();
    // The run broke the wall before the test.
    for swing in 1..=3 {
        strike(&mut game, RoomId::First, OVER_WALL, swing);
    }
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert_eq!(
        wall_remaining(&game, RoomId::First),
        3,
        "the test room starts whole: nothing the run did is carried in"
    );

    break_and_switch(&mut game, RoomId::First);
    assert_eq!(wall_remaining(&game, RoomId::First), 0);
    assert!(barrier_open(&game, RoomId::First));
    assert_eq!(
        (game.puzzles.broken.len(), game.puzzles.on.len()),
        (1, 0),
        "nothing from the test room is remembered"
    );

    game.end_test_room().unwrap();
    assert_eq!(
        wall_remaining(&game, RoomId::First),
        0,
        "the run's own break"
    );
    assert!(
        !switch_on(&game, RoomId::First),
        "the test's switch is gone"
    );
    assert!(!barrier_open(&game, RoomId::First));
    assert_eq!((game.puzzles.broken.len(), game.puzzles.on.len()), (1, 0));
}

#[test]
fn a_test_room_plays_the_unsaved_pieces() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    game.edit_room(RoomId::First).unwrap();
    put_pieces(&mut game.editor_mut().unwrap().draft.layout);
    assert!(room_of(&game, RoomId::First)
        .puzzles
        .breakables()
        .is_empty());

    game.start_test_room().unwrap();
    assert_eq!(room_of(&game, RoomId::First).puzzles.breakables().len(), 1);
    assert_eq!(room_of(&game, RoomId::First).puzzles.barriers().len(), 1);

    game.end_test_room().unwrap();
    assert!(room_of(&game, RoomId::First)
        .puzzles
        .breakables()
        .is_empty());
}
