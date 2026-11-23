//! Whether a run flips, through `GameState` (`flip_link`): the rules file, F4's choice, the
//! decision made when a run starts and kept for its life. What a flipped run does to the rooms
//! is in `flip_play_tests`.
//!
//! The world is the one of `run_tests` (the fixtures plus a hub) with a `variety.ron` written
//! into it. Seeds are fixed by the test, never taken from a clock.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, input, kill, start_run, world};
use super::*;
use crate::run::RunPhase;
use crate::seed::SeedSource;
use crate::variety::FlipChoice;

/// A hub world whose `variety.ron` flips `percent` of the runs.
fn flipping(percent: f32) -> (Scratch, GameState) {
    let scratch = world(100.0);
    scratch.write("variety.ron", &format!("(flip_percent: {percent:?})"));
    let game = game_in(&scratch);
    (scratch, game)
}

fn back_to_the_hub(game: &mut GameState) {
    game.abandon_run();
    game.continue_after_result();
    assert!(game.in_hub());
}

/// Starts a run with `seed` and says whether it flipped.
fn run_with(game: &mut GameState, seed: u64) -> bool {
    game.set_fixed_seed(Some(seed));
    start_run(game);
    let flipped = game.run_flipped();
    back_to_the_hub(game);
    flipped
}

#[test]
fn without_a_file_the_game_uses_the_default_and_never_flips() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    assert_eq!(game.variety_rules().flip_percent, 0.0);
    assert_eq!(game.flip_choice(), FlipChoice::Auto);
    for seed in 0..40 {
        assert!(!run_with(&mut game, seed), "seed {seed}");
    }
}

#[test]
fn the_rules_come_from_the_file_in_assets() {
    let (_scratch, game) = flipping(37.5);
    assert_eq!(game.variety_rules().flip_percent, 37.5);
}

#[test]
fn a_bad_variety_file_stops_the_game_loading_and_names_the_file() {
    let scratch = world(100.0);
    for text in [
        "(flip_percent: 101.0)",
        "(flip_percent: 5.0, other: 1)",
        "nonsense",
    ] {
        scratch.write("variety.ron", text);
        let error = GameState::try_new(&scratch.0).unwrap_err();
        assert!(error.contains("variety.ron"), "{text}: {error}");
    }
}

#[test]
fn zero_percent_never_flips_and_one_hundred_always_does() {
    let (_scratch, mut game) = flipping(0.0);
    for seed in 0..30 {
        assert!(!run_with(&mut game, seed), "0 percent, seed {seed}");
    }
    let (_scratch, mut game) = flipping(100.0);
    for seed in 0..30 {
        assert!(run_with(&mut game, seed), "100 percent, seed {seed}");
    }
}

#[test]
fn auto_is_pinned_for_one_seed() {
    // Seed 0 draws 14.11 percent and seed 1 draws 54.50 percent (`variety::flip` tests).
    let (_scratch, mut game) = flipping(50.0);
    assert!(run_with(&mut game, 0));
    assert!(!run_with(&mut game, 1));
    // The same seed again gives the same answer.
    assert!(run_with(&mut game, 0));
    assert!(!run_with(&mut game, 1));
}

#[test]
fn at_fifty_percent_runs_taken_from_the_source_both_flip_and_do_not() {
    let (_scratch, mut game) = flipping(50.0);
    game.use_seed_source(SeedSource::new(123));
    let mut flips = 0;
    for _ in 0..40 {
        start_run(&mut game);
        flips += usize::from(game.run_flipped());
        back_to_the_hub(&mut game);
    }
    assert!((10..=30).contains(&flips), "{flips} of 40");
}

#[test]
fn never_and_always_override_the_file() {
    let (_scratch, mut game) = flipping(100.0);
    game.set_flip_choice(FlipChoice::Never);
    for seed in 0..10 {
        assert!(!run_with(&mut game, seed), "Never over 100 percent");
    }
    let (_scratch, mut game) = flipping(0.0);
    game.set_flip_choice(FlipChoice::Always);
    for seed in 0..10 {
        assert!(run_with(&mut game, seed), "Always over 0 percent");
    }
}

#[test]
fn the_decision_is_kept_for_the_run_and_the_next_run_follows_the_new_choice() {
    let (_scratch, mut game) = flipping(0.0);
    game.set_flip_choice(FlipChoice::Always);
    start_run(&mut game);
    assert!(game.run_flipped());
    game.set_flip_choice(FlipChoice::Never);
    game.update(input(), 0.1);
    assert!(game.run_flipped(), "a run already going keeps its decision");
    back_to_the_hub(&mut game);
    start_run(&mut game);
    assert!(!game.run_flipped(), "the next run follows Never");

    let (_scratch, mut game) = flipping(100.0);
    game.set_flip_choice(FlipChoice::Never);
    start_run(&mut game);
    game.set_flip_choice(FlipChoice::Always);
    assert!(!game.run_flipped(), "and the other way round");
}

#[test]
fn a_flipped_run_is_flipped_on_its_result_screen_and_never_in_the_hub() {
    for end in ["abandon", "death"] {
        let (_scratch, mut game) = flipping(100.0);
        assert!(!game.run_flipped(), "the hub");
        start_run(&mut game);
        assert!(game.run_flipped());
        if end == "abandon" {
            game.abandon_run();
        } else {
            kill(&mut game);
        }
        assert!(
            matches!(game.run_phase(), Some(RunPhase::Result(_))),
            "{end}"
        );
        assert!(
            game.run_flipped(),
            "{end}: the result screen is of that run"
        );
        game.continue_after_result();
        assert!(game.in_hub());
        assert!(!game.run_flipped(), "{end}: back in the hub");
    }
}

#[test]
fn a_game_with_no_hub_and_a_test_room_are_never_flipped() {
    let scratch = Scratch::new();
    scratch.write("variety.ron", "(flip_percent: 100.0)");
    let mut game = GameState::try_new(&scratch.0).unwrap();
    game.set_flip_choice(FlipChoice::Always);
    assert_eq!(game.run_phase(), None);
    assert!(!game.run_flipped());
    game.update(input(), 0.1);
    assert!(!game.run_flipped());

    let (_scratch, mut game) = flipping(100.0);
    game.set_flip_choice(FlipChoice::Always);
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert!(!game.run_flipped(), "a sandbox is not a run");
    game.end_test_room().unwrap();
    assert!(!game.run_flipped());
}

#[test]
fn a_run_that_cannot_start_is_not_flipped() {
    let (scratch, mut game) = flipping(100.0);
    scratch.write("rooms/first.ron", "this is not a room");
    game.update(super::run_tests::walk(1.0), 10.0);
    assert!(!game.run_in_progress());
    assert!(!game.run_flipped());
}
