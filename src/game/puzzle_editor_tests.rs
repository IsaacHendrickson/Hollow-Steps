//! The puzzle pieces between the editor and play: pieces put in with the editor's own tools are
//! unsaved until Save, are played by Test room before that, and come back from the room file
//! after Save. The world is a scratch copy of `tests/fixtures`: the first room is 960 wide with
//! its floor at y = 460 and its spawn at (180, 436).
use macroquad::math::Vec2;

use super::charm_tests::Scratch;
use super::puzzle_play_tests::{rest, stand_at, swing, walk_right};
use super::*;
use crate::editor::{RoomLayout, Selection};
use crate::editor_puzzle::PieceFields;

fn game_in(scratch: &Scratch) -> GameState {
    let mut game = GameState::try_new(&scratch.0).expect("fixed test assets must load");
    for room in &mut game.rooms {
        room.enemies.clear();
        room.coins.clear();
    }
    game
}

/// The editor open on the first room with a 1-hit wall, switch 1 far to the right and a barrier
/// that switch opens: the pieces are put in with the Room tools buttons' own functions and
/// dragged where they belong, and nothing is saved.
fn author_the_pieces(game: &mut GameState) {
    game.edit_room(RoomId::First).unwrap();
    let session = game.editor_mut().unwrap();
    let draft = &mut session.draft;
    draft.add_breakable().unwrap();
    let wall = draft.piece_fields(Selection::Breakable(0)).unwrap();
    draft
        .set_piece_fields(Selection::Breakable(0), PieceFields { number: 1, ..wall })
        .unwrap();
    draft.add_switch().unwrap();
    draft.add_barrier().unwrap();
    let definitions = &session.definitions;
    for (selection, x) in [
        (Selection::Breakable(0), 400.0),
        (Selection::Switch(0), 700.0),
        (Selection::Barrier(0), 500.0),
    ] {
        session
            .draft
            .move_to(selection, Vec2::new(x, 430.0), definitions)
            .unwrap();
    }
    session.dirty = true;
}

fn first_room(game: &GameState) -> &Room {
    game.rooms.iter().find(|r| r.id == RoomId::First).unwrap()
}

#[test]
fn pieces_added_in_the_editor_are_unsaved_until_saved() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    game.edit_room(RoomId::First).unwrap();
    assert!(!game.editor_mut().unwrap().has_unsaved());

    author_the_pieces(&mut game);
    let session = game.editor_mut().unwrap();
    assert!(session.has_unsaved(), "the unsaved-changes guard sees it");
    assert_eq!(session.unsaved_rooms().len(), 1);
    assert!(
        first_room(&game).puzzles.breakables().is_empty(),
        "the live room is still the saved one"
    );

    game.save_editor().unwrap();
    assert!(!game.editor_mut().unwrap().has_unsaved());
    assert_eq!(first_room(&game).puzzles.breakables().len(), 1);
}

#[test]
fn save_and_reload_gives_back_the_same_pieces() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    author_the_pieces(&mut game);
    let saved: RoomLayout = game.editor_mut().unwrap().draft.layout.clone();
    game.save_editor().unwrap();

    // A fresh start reads the room file the editor wrote.
    let reloaded = GameState::try_new(&scratch.0).expect("the saved world loads");
    let layout = RoomLayout::from_room(first_room(&reloaded));
    assert_eq!(layout.breakables, saved.breakables);
    assert_eq!(layout.switches, saved.switches);
    assert_eq!(layout.barriers, saved.barriers);
    assert_eq!(
        (
            layout.breakables.len(),
            layout.switches.len(),
            layout.barriers.len()
        ),
        (1, 1, 1)
    );
    let room = first_room(&reloaded);
    assert_eq!(
        room.puzzles.breakables()[0].remaining,
        1,
        "whole at the start"
    );
    assert!(!room.puzzles.switches()[0].on, "off at the start");

    // Opening it again in the editor shows the same pieces, and nothing is unsaved.
    let mut again = reloaded;
    again.edit_room(RoomId::First).unwrap();
    let session = again.editor_mut().unwrap();
    assert_eq!(session.draft.layout.breakables, saved.breakables);
    assert!(!session.has_unsaved());
}

#[test]
fn a_test_room_plays_unsaved_pieces_and_ending_it_gives_back_the_draft_and_the_saved_room() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    author_the_pieces(&mut game);
    let draft = game.editor_mut().unwrap().draft.clone();

    game.start_test_room().unwrap();
    assert!(game.testing_room());
    let puzzles = &game.room().puzzles;
    assert_eq!(
        (
            puzzles.breakables().len(),
            puzzles.switches().len(),
            puzzles.barriers().len()
        ),
        (1, 1, 1),
        "the unsaved pieces are in the room that is played"
    );

    // The barrier (x 488..512) stands in the way: walking right from the spawn stops at it.
    let barrier = puzzles.barriers()[0].rect;
    stand_at(&mut game, 470.0);
    walk_right(&mut game, 200);
    let stopped = game.player.position().x;
    assert!(
        (stopped - (barrier.x - 16.0)).abs() < 0.01,
        "stopped at the barrier, at {stopped}"
    );
    // Striking the switch (x 688..712) opens it, and the player walks on.
    stand_at(&mut game, 670.0);
    swing(&mut game, false, false);
    assert!(game.room().puzzles.switches()[0].on);
    rest(&mut game);
    stand_at(&mut game, 470.0);
    walk_right(&mut game, 200);
    assert!(game.player.position().x > barrier.right(), "walked through");

    game.end_test_room().unwrap();
    assert!(!game.testing_room());
    assert_eq!(
        game.editor_mut().unwrap().draft,
        draft,
        "the draft is as it was"
    );
    assert!(
        first_room(&game).puzzles.barriers().is_empty(),
        "the saved room has none"
    );
    assert!(
        (game.puzzles.broken.len(), game.puzzles.on.len()) == (0, 0),
        "nothing remembered"
    );
}

#[test]
fn a_test_room_wall_breaks_after_its_hits_and_is_whole_again_in_the_next_test() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    author_the_pieces(&mut game);
    for _ in 0..2 {
        game.start_test_room().unwrap();
        assert_eq!(game.room().puzzles.breakables()[0].remaining, 1);
        stand_at(&mut game, 340.0);
        swing(&mut game, false, false);
        assert_eq!(
            game.room().puzzles.breakables()[0].remaining,
            0,
            "broken in one hit"
        );
        game.end_test_room().unwrap();
    }
}

#[test]
fn an_invalid_unsaved_piece_stops_test_room_and_save_and_says_so() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    author_the_pieces(&mut game);
    // A piece that no longer fits the room is a draft the editor itself refuses to build.
    game.editor_mut().unwrap().draft.layout.barriers[0].switch = 6;
    let error = game.start_test_room().unwrap_err();
    assert!(
        error.contains("barrier 0") && error.contains("switch 6"),
        "{error}"
    );
    assert!(!game.testing_room());
    let error = game.save_editor().unwrap_err();
    assert!(error.contains("barrier 0"), "{error}");
}
