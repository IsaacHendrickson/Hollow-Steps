//! Generated runs, continued: the editor is closed while a run is generated (the same refusal as
//! a flipped run's), and a test room is not part of any of it.
use super::super::run_tests::walk;
use super::{begin, game_with, index_of};
use crate::room::RoomId;
use crate::variety::GenerateChoice;

const START_PIECE: RoomId = RoomId::Custom(1);
const SECOND_PIECE: RoomId = RoomId::Custom(2);

#[test]
fn the_editor_refuses_during_a_generated_run_and_works_in_the_hub() {
    let (_scratch, mut game) = game_with((0, 1));
    // The hub, even with generation on.
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some());
    game.toggle_editor().unwrap();
    game.edit_room(START_PIECE).unwrap();
    game.toggle_editor().unwrap();
    game.current = index_of(&game, super::HUB);

    begin(&mut game);
    let (room, position) = (game.room().id, game.player.position());
    for error in [
        game.toggle_editor().unwrap_err(),
        game.edit_room(RoomId::First).unwrap_err(),
    ] {
        assert!(error.contains("abandon the run"), "{error}");
        assert!(error.contains("pause menu"), "{error}");
        assert!(error.contains("Generate next run"), "{error}");
    }
    assert!(game.editor().is_none(), "no editor opened");
    assert_eq!((game.room().id, game.player.position()), (room, position));
    assert!(game.run_in_progress(), "and the run goes on");

    // The frozen world of the result screen is still the generated one.
    game.abandon_run();
    assert!(game.toggle_editor().is_err() && game.edit_room(RoomId::First).is_err());
    game.continue_after_result();
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some(), "back in the hub it opens");
}

#[test]
fn the_editor_works_during_a_fixed_run_when_generation_is_never() {
    let (_scratch, mut game) = game_with((0, 1));
    game.set_generate_choice(GenerateChoice::Never);
    begin(&mut game);
    assert!(game.run_in_progress() && !game.run_generated());
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some());
    game.toggle_editor().unwrap();
    game.edit_room(SECOND_PIECE).unwrap();
    assert!(game.editor().is_some());
}

#[test]
fn a_test_room_is_not_a_generated_run() {
    let (_scratch, mut game) = game_with((0, 1));
    game.set_generate_choice(GenerateChoice::Always);
    game.edit_room(START_PIECE).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room() && !game.run_generated());
    assert!(game.run_plan().is_none(), "a test room has no plan");
    game.end_test_room().unwrap();
    assert!(!game.run_generated());

    // And nothing in the hub depended on it: the run that follows is generated.
    game.toggle_editor().unwrap();
    game.current = index_of(&game, super::HUB);
    game.update(walk(1.0), 10.0);
    assert!(game.run_in_progress() && game.run_generated());
}

#[test]
fn closing_the_editor_in_a_room_the_plan_lacks_starts_the_run_at_its_start_room() {
    // The editor can leave the player in a room the plan does not have (the second Start piece is
    // never in a dungeon, since every Start piece becomes the one start room); the run that starts
    // when it closes puts him at the start room's spawn, as a flipped run does.
    let (_scratch, mut game) = game_with((0, 1));
    game.edit_room(SECOND_PIECE).unwrap();
    game.toggle_editor().unwrap();
    assert_eq!(game.room().id, SECOND_PIECE);

    game.update(super::input(), 0.0);

    assert!(game.run_in_progress() && game.run_generated());
    assert_eq!(game.room().id, RoomId::First);
    let spawn = game.room().spawn;
    assert!((game.player.position().x - spawn.x).abs() < 1.0);
    assert!(game.room().bounds.contains(game.player.position()));
}
