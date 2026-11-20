//! The hub-and-run loop's edges through `GameState` (`run_link`): the saved profile, a test room
//! (a sandbox that never starts or ends a run), the editor moving the player, and a game whose
//! `run.ron` names no hub. The loop itself is tested in `run_tests`, whose world and helpers
//! these tests share.
use std::path::PathBuf;

use super::run_tests::{
    assert_phase_matches_room, game_in, input, kill, profile_path, start_run, swing, walk, world,
    HUB,
};
use super::*;
use crate::profile::{self, Profile};
use crate::run::{RunOutcome, RunPhase};

// ---- the profile ------------------------------------------------------------------------------

#[test]
fn two_runs_add_up_and_the_file_on_disk_matches() {
    let scratch = world(50.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 10;
    game.abandon_run();
    game.continue_after_result();
    start_run(&mut game);
    game.coin_count = 4;
    kill(&mut game);

    assert_eq!(game.profile().coins, 7);
    let (saved, problem) = profile::load(&profile_path(&scratch));
    assert_eq!(problem, None);
    assert_eq!(saved, *game.profile());
}

#[test]
fn a_saved_profile_is_read_when_the_game_starts() {
    let scratch = world(100.0);
    let saved = Profile {
        coins: 12,
        tier: 2,
        cleared: 3,
        ..Profile::default()
    };
    profile::save(&profile_path(&scratch), &saved).unwrap();
    let game = game_in(&scratch);
    assert_eq!(*game.profile(), saved);
}

#[test]
fn a_bad_profile_starts_fresh_keeps_a_copy_and_reports_it() {
    let scratch = world(100.0);
    std::fs::write(profile_path(&scratch), "not a profile").unwrap();
    let mut game = GameState::try_new(&scratch.0).unwrap();

    let message = game.use_profile_file(profile_path(&scratch)).unwrap_err();

    assert!(message.contains("starting fresh"), "{message}");
    assert_eq!(*game.profile(), Profile::default());
    assert!(profile::backup_path(&profile_path(&scratch)).exists());
}

#[test]
fn a_profile_that_cannot_be_saved_keeps_the_coins_for_the_session_and_says_so() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    // A folder where the staged file goes makes every save fail.
    std::fs::create_dir(scratch.0.join("profile.ron.tmp")).unwrap();
    start_run(&mut game);
    game.coin_count = 5;

    game.abandon_run();

    assert_eq!(game.profile().coins, 5);
    assert!(game.run_result().is_some(), "the result still shows");
    let message = game.run_message().unwrap();
    assert!(message.contains("Profile not saved"), "{message}");
}

#[test]
fn nothing_is_saved_before_a_profile_file_is_chosen() {
    let scratch = world(100.0);
    let mut game = GameState::try_new(&scratch.0).unwrap();
    start_run(&mut game);
    game.coin_count = 5;
    game.abandon_run();
    assert_eq!(game.profile().coins, 5);
    assert!(game.run_message().is_none());
    assert!(!profile_path(&scratch).exists());
}

#[test]
fn set_profile_replaces_and_saves_it() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    let profile = Profile {
        coins: 1,
        tier: 4,
        cleared: 0,
        ..Profile::default()
    };
    game.set_profile(profile.clone()).unwrap();
    assert_eq!(*game.profile(), profile);
    assert_eq!(profile::load(&profile_path(&scratch)).0, profile);
}

// ---- test rooms, the editor and a game with no hub ------------------------------------------------

#[test]
fn dying_in_a_test_room_respawns_and_touches_nothing() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 5;
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();

    kill(&mut game);

    assert!(game.testing_room());
    assert_eq!(
        game.player().health().current(),
        game.player().health().max()
    );
    assert!(game.run_result().is_none());
    assert_eq!(*game.profile(), Profile::default());
    assert!(!profile_path(&scratch).exists());
    assert_eq!(
        game.run_phase(),
        Some(RunPhase::Running),
        "the run is parked, not ended"
    );
    game.end_test_room().unwrap();
    assert!(game.run_in_progress());
    assert_eq!(game.coin_count(), 5);
}

#[test]
fn a_boss_defeated_in_a_test_room_clears_nothing() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.edit_room(RoomId::Boss).unwrap();
    game.start_test_room().unwrap();
    let boss = game.rooms[game.current].boss_special.as_mut().unwrap();
    boss.damage_boss(boss.health() - 1);
    game.player.place_on_floor(520.0, game.world_bounds());

    game.update(swing(), 0.0);

    assert!(game.rooms[game.current]
        .boss_special
        .as_ref()
        .unwrap()
        .is_defeated());
    assert!(game.run_result().is_none());
    assert_eq!(*game.profile(), Profile::default());
}

#[test]
fn the_editor_moving_the_player_out_of_the_hub_starts_a_run_on_the_next_tick() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.edit_room(RoomId::First).unwrap();
    game.update(input(), 0.01);
    assert_eq!(
        game.run_phase(),
        Some(RunPhase::Hub),
        "frozen while editing"
    );

    game.toggle_editor().unwrap();
    game.update(input(), 0.01);

    assert!(game.run_in_progress());
    assert_phase_matches_room(&game);
}

#[test]
fn the_editor_moving_the_player_into_the_hub_abandons_the_run() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 3;
    game.edit_room(HUB).unwrap();
    game.toggle_editor().unwrap();

    game.update(input(), 0.01);

    assert_eq!(game.run_result().unwrap().outcome, RunOutcome::Abandoned);
    assert_eq!(game.profile().coins, 3);
}

#[test]
fn saving_in_the_editor_during_a_run_leaves_the_run_and_the_profile_alone() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.toggle_editor().unwrap();
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    game.update(input(), 0.01);
    assert!(game.run_in_progress());
    assert_eq!(*game.profile(), Profile::default());
}

#[test]
fn with_no_hub_the_game_plays_as_before() {
    let mut game = GameState::try_new(&fixtures_path()).expect("fixtures load");
    assert_eq!(game.run_phase(), None);
    assert!(!game.in_hub() && !game.run_in_progress());
    assert_eq!(game.room().id, RoomId::First);

    game.coin_count = 4;
    kill(&mut game);
    assert_eq!(
        game.player().health().current(),
        game.player().health().max(),
        "respawned"
    );
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.coin_count(), 4, "nothing is reset by dying");
    assert!(game.run_result().is_none());

    game.update(walk(1.0), 10.0);
    assert_eq!(game.room().id, RoomId::Second);
    game.abandon_run();
    game.continue_after_result();
    game.abandon_run_on_exit();
    assert_eq!(game.run_phase(), None);
    assert_eq!(*game.profile(), Profile::default());
    assert!(!game.is_paused());
}

pub(super) fn fixtures_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}
