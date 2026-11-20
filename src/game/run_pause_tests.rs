//! The pause menu through `GameState` during the hub-and-run loop: Abandon run only in a run,
//! its confirm screen, and what each button does to the run. The menu's own screens are tested
//! in `crate::pause_menu`; the world is `run_tests`'.
use super::run_tests::{game_in, input, start_run, world};
use super::*;
use crate::pause_menu::{PauseItem, PauseOutcome, PauseScreen};
use crate::run::{RunOutcome, RunPhase};

fn pause_screen(game: &mut GameState) -> PauseScreen {
    game.press_escape();
    let screen = game.pause_menu().screen().expect("the menu opened");
    game.press_escape();
    screen
}

#[test]
fn the_pause_menu_lists_abandon_run_only_during_a_run() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    assert_eq!(pause_screen(&mut game), PauseScreen::Main, "in the hub");
    start_run(&mut game);
    assert_eq!(pause_screen(&mut game), PauseScreen::RunMain, "in a run");
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert_eq!(pause_screen(&mut game), PauseScreen::Main, "in a test room");
}

#[test]
fn with_no_hub_the_pause_menu_is_the_one_it_always_was() {
    let mut game = GameState::try_new(&super::run_profile_tests::fixtures_path()).unwrap();
    assert_eq!(pause_screen(&mut game), PauseScreen::Main);
}

#[test]
fn abandoning_from_the_pause_menu_needs_the_confirmation() {
    let scratch = world(50.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 8;
    game.press_escape();

    assert_eq!(
        game.pause_activate(PauseItem::AbandonRun),
        PauseOutcome::Continue
    );
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Confirm));
    assert!(game.run_in_progress(), "nothing has happened yet");
    assert_eq!(game.profile().coins, 0);

    // The front end gets Continue, not AbandonRun: the game has done it.
    assert_eq!(
        game.pause_activate(PauseItem::ConfirmAbandon),
        PauseOutcome::Continue
    );
    let result = game.run_result().expect("the result shows");
    assert_eq!(result.outcome, RunOutcome::Abandoned);
    assert_eq!((result.coins_earned, result.coins_banked), (8, 4));
    assert!(!game.pause_menu().is_open());
    assert!(game.is_paused(), "the result screen holds the world");
}

#[test]
fn keep_playing_and_escape_leave_the_run_alone() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 3;
    game.press_escape();
    game.pause_activate(PauseItem::AbandonRun);
    game.pause_activate(PauseItem::KeepPlaying);
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::RunMain));
    game.pause_activate(PauseItem::AbandonRun);
    game.press_escape();
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::RunMain));
    game.press_escape();

    assert!(!game.pause_menu().is_open());
    assert!(game.run_in_progress());
    assert_eq!((game.coin_count(), game.profile().coins), (3, 0));
    game.update(input(), 0.01);
    assert_eq!(game.run_phase(), Some(RunPhase::Running));
}

#[test]
fn exit_from_the_pause_menu_is_left_to_the_front_end_which_banks_the_run_first() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 5;
    game.press_escape();
    assert_eq!(game.pause_activate(PauseItem::Exit), PauseOutcome::Exit);
    assert!(game.run_in_progress(), "the game itself did nothing");

    game.abandon_run_on_exit();

    assert_eq!(game.profile().coins, 5);
}
