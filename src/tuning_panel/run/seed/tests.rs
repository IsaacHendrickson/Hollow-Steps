//! `set_fixed_seed` and `seed_label`: the logic of the F4 Seed row, with no window.
use super::super::tests::Scratch;
use super::*;
use hollow_steps::player::PlayerInput;
use hollow_steps::seed::SeedSource;

/// Leaves the hub through its exit: a run starts.
fn start_run(game: &mut GameState) {
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert!(
        game.run_in_progress(),
        "walking out of the hub starts a run"
    );
}

#[test]
fn turning_it_on_fixes_the_seed_and_the_next_run_takes_it() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    set_fixed_seed(&mut game, true, "1f3a").unwrap();
    assert_eq!(game.fixed_seed(), Some(0x1F3A));
    start_run(&mut game);
    assert_eq!(game.run_seed(), Some(0x1F3A));
}

#[test]
fn the_text_may_have_a_prefix_any_case_and_spaces_around_it() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    for (text, seed) in [("0xAbC", 0xABC), ("  ff  ", 0xFF), ("0X10", 0x10), ("0", 0)] {
        set_fixed_seed(&mut game, true, text).unwrap();
        assert_eq!(game.fixed_seed(), Some(seed), "{text:?}");
    }
}

#[test]
fn a_bad_text_is_refused_with_the_reason_and_a_seed_that_was_fixed_stays() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    set_fixed_seed(&mut game, true, "abc").unwrap();
    for (text, why) in [
        ("", "empty"),
        ("   ", "empty"),
        ("12g4", "'g'"),
        ("0x", "after 0x"),
        ("12345678901234567", "at most 16"),
    ] {
        let error = set_fixed_seed(&mut game, true, text).unwrap_err();
        assert!(error.contains(why), "{text:?}: {error}");
        assert_eq!(
            game.fixed_seed(),
            Some(0xABC),
            "{text:?} keeps the old seed"
        );
    }
}

#[test]
fn a_bad_text_does_not_turn_the_fixed_seed_on() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    assert!(set_fixed_seed(&mut game, true, "nope").is_err());
    assert_eq!(game.fixed_seed(), None);
    start_run(&mut game);
    assert!(game.run_seed().is_some());
    assert_ne!(
        game.run_seed(),
        Some(0),
        "the run took a seed from the source"
    );
}

#[test]
fn turning_it_off_goes_back_to_the_source_and_never_reads_the_text() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.use_seed_source(SeedSource::new(5));
    set_fixed_seed(&mut game, true, "abc").unwrap();
    set_fixed_seed(&mut game, false, "not even hex").unwrap();
    assert_eq!(game.fixed_seed(), None);
    start_run(&mut game);
    assert_eq!(game.run_seed(), Some(SeedSource::new(5).next()));
}

#[test]
fn the_label_follows_the_run_in_progress_the_result_and_the_last_run() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    assert!(
        seed_label(&game).contains("No run has started"),
        "{}",
        seed_label(&game)
    );

    set_fixed_seed(&mut game, true, "ab").unwrap();
    start_run(&mut game);
    assert_eq!(seed_label(&game), "Seed of this run: 00000000000000ab");

    game.abandon_run();
    assert_eq!(
        seed_label(&game),
        "Seed of this run: 00000000000000ab",
        "the result screen is still that run"
    );

    game.continue_after_result();
    assert_eq!(seed_label(&game), "Seed of the last run: 00000000000000ab");

    set_fixed_seed(&mut game, true, "cd").unwrap();
    assert_eq!(
        seed_label(&game),
        "Seed of the last run: 00000000000000ab",
        "fixing a seed does not rewrite the label until a run takes it"
    );
}

#[test]
fn the_fixed_seed_is_not_written_to_the_profile() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    set_fixed_seed(&mut game, true, "abc").unwrap();
    start_run(&mut game);
    game.abandon_run();
    let saved = std::fs::read_to_string(scratch.profile_file()).unwrap();
    assert!(!saved.to_lowercase().contains("seed"), "{saved}");
    assert!(!saved.contains("abc"), "{saved}");
}
