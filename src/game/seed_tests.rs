//! A run's seed through `GameState` (`seed_link`): a run takes one when it starts, a fixed seed
//! repeats, the seed stays through the result screen and is gone in the hub, and a game with no
//! hub and a test room take none.
//!
//! The world is the one of `run_tests` (the fixtures plus a hub). Every seed here comes from a
//! `SeedSource` the test chose, never from a clock.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, input, kill, start_run, walk, world};
use super::*;
use crate::run::RunPhase;
use crate::seed::SeedSource;

fn game() -> (Scratch, GameState) {
    let scratch = world(100.0);
    let game = game_in(&scratch);
    (scratch, game)
}

/// The seeds a source started at `start` hands out, in order.
fn seeds_of(start: u64, count: usize) -> Vec<u64> {
    let mut source = SeedSource::new(start);
    (0..count).map(|_| source.next()).collect()
}

/// Ends the run in progress and goes back to the hub.
fn back_to_the_hub(game: &mut GameState) {
    game.abandon_run();
    game.continue_after_result();
    assert!(game.in_hub());
}

#[test]
fn a_run_takes_a_seed_at_start_and_the_hub_has_none() {
    let (_scratch, mut game) = game();
    assert_eq!(game.run_seed(), None, "the hub");
    assert_eq!(game.last_run_seed(), None, "no run yet");
    game.use_seed_source(SeedSource::new(77));
    start_run(&mut game);
    assert_eq!(game.run_seed(), Some(seeds_of(77, 1)[0]));
    assert_eq!(game.last_run_seed(), game.run_seed());
}

#[test]
fn a_game_never_given_a_source_starts_from_the_fixed_default() {
    let (_scratch, mut game) = game();
    start_run(&mut game);
    assert_eq!(game.run_seed(), Some(SeedSource::default().next()));
}

#[test]
fn each_run_takes_the_next_seed_of_the_source() {
    let (_scratch, mut game) = game();
    game.use_seed_source(SeedSource::new(5));
    let expected = seeds_of(5, 3);
    for seed in &expected {
        start_run(&mut game);
        assert_eq!(game.run_seed(), Some(*seed));
        back_to_the_hub(&mut game);
    }
    assert_eq!(
        expected
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        3
    );
}

#[test]
fn a_fixed_seed_repeats_across_runs_and_does_not_use_up_the_source() {
    let (_scratch, mut game) = game();
    game.use_seed_source(SeedSource::new(5));
    game.set_fixed_seed(Some(0xABC));
    assert_eq!(game.fixed_seed(), Some(0xABC));
    for _ in 0..3 {
        start_run(&mut game);
        assert_eq!(game.run_seed(), Some(0xABC));
        back_to_the_hub(&mut game);
    }
    game.set_fixed_seed(None);
    start_run(&mut game);
    assert_eq!(
        game.run_seed(),
        Some(seeds_of(5, 1)[0]),
        "the source is where it was before the fixed runs"
    );
}

#[test]
fn fixing_the_seed_does_not_change_a_run_already_going() {
    let (_scratch, mut game) = game();
    game.use_seed_source(SeedSource::new(5));
    start_run(&mut game);
    let seed = game.run_seed();
    game.set_fixed_seed(Some(1));
    assert_eq!(game.run_seed(), seed, "this run keeps its seed");
    back_to_the_hub(&mut game);
    start_run(&mut game);
    assert_eq!(
        game.run_seed(),
        Some(1),
        "the next one takes the fixed seed"
    );
}

#[test]
fn the_seed_is_kept_through_the_result_screen_and_gone_in_the_hub() {
    for end in ["abandon", "death"] {
        let (_scratch, mut game) = game();
        game.use_seed_source(SeedSource::new(9));
        start_run(&mut game);
        let seed = game.run_seed();
        assert!(seed.is_some());
        if end == "abandon" {
            game.abandon_run();
        } else {
            kill(&mut game);
        }
        assert!(
            matches!(game.run_phase(), Some(RunPhase::Result(_))),
            "{end}"
        );
        assert_eq!(
            game.run_seed(),
            seed,
            "{end}: the result screen still has it"
        );
        game.update(input(), 0.1);
        assert_eq!(game.run_seed(), seed, "{end}: and it stays while it shows");

        game.continue_after_result();
        assert!(game.in_hub());
        assert_eq!(game.run_seed(), None, "{end}: the hub has none");
        assert_eq!(
            game.last_run_seed(),
            seed,
            "{end}: the last run is remembered"
        );
    }
}

#[test]
fn walking_back_into_the_hub_keeps_the_seed_on_the_result_screen() {
    let (_scratch, mut game) = game();
    start_run(&mut game);
    let seed = game.run_seed();
    // The editor can move the player; standing in the hub during a run abandons it.
    game.current = game
        .rooms
        .iter()
        .position(|room| room.id == run_tests::HUB)
        .unwrap();
    game.update(input(), 0.1);
    assert!(matches!(game.run_phase(), Some(RunPhase::Result(_))));
    assert_eq!(game.run_seed(), seed);
}

#[test]
fn a_run_that_cannot_start_takes_no_seed_and_uses_none_up() {
    let (scratch, mut game) = game();
    game.use_seed_source(SeedSource::new(3));
    let good = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();
    scratch.write("rooms/first.ron", "this is not a room");
    game.update(walk(1.0), 10.0);
    assert!(!game.run_in_progress(), "{:?}", game.run_message());
    assert_eq!(game.run_seed(), None);
    assert_eq!(game.last_run_seed(), None);

    scratch.write("rooms/first.ron", &good);
    start_run(&mut game);
    assert_eq!(
        game.run_seed(),
        Some(seeds_of(3, 1)[0]),
        "the first seed is still the next"
    );
}

#[test]
fn a_game_with_no_hub_has_no_runs_and_no_seed() {
    let scratch = Scratch::new();
    let mut game = GameState::try_new(&scratch.0).unwrap();
    game.set_fixed_seed(Some(5));
    game.update(walk(1.0), 10.0);
    assert_eq!(game.run_phase(), None);
    assert_eq!(game.run_seed(), None);
    assert_eq!(game.last_run_seed(), None);
}

#[test]
fn a_test_room_takes_no_seed_and_uses_none_up() {
    let (_scratch, mut game) = game();
    game.use_seed_source(SeedSource::new(11));
    game.set_fixed_seed(Some(0xF1));
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert_eq!(game.run_seed(), None, "from the hub");
    assert_eq!(game.last_run_seed(), None);
    game.update(input(), 0.1);
    assert_eq!(game.run_seed(), None);
    game.end_test_room().unwrap();
    assert_eq!(game.last_run_seed(), None, "no run was started by it");

    // Inside a run the test room parks the run, and the run's seed is not the test room's.
    game.toggle_editor().unwrap();
    game.set_fixed_seed(None);
    back_to_the_hub_if_running(&mut game);
    start_run(&mut game);
    let seed = game.run_seed();
    assert_eq!(seed, Some(seeds_of(11, 1)[0]), "the test room used no seed");
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(game.run_seed(), None, "parked while the test room plays");
    game.end_test_room().unwrap();
    assert_eq!(game.run_seed(), seed, "and back when it ends");
}

fn back_to_the_hub_if_running(game: &mut GameState) {
    if game.run_in_progress() {
        back_to_the_hub(game);
    }
}

#[test]
fn the_seed_is_not_saved_in_the_profile() {
    let (scratch, mut game) = game();
    start_run(&mut game);
    game.abandon_run();
    let saved = std::fs::read_to_string(scratch.0.join("profile.ron")).unwrap();
    assert!(!saved.to_lowercase().contains("seed"), "{saved}");
    assert!(!saved.contains(&crate::seed::format_seed(game.run_seed().unwrap())));
}
