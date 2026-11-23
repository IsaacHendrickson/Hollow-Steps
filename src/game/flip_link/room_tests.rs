//! A run that mirrors rooms marked flippable, through `GameState` (`flip_link`): which rooms the
//! run builds mirrored, where the player arrives, F5, the end of the run and the editor's
//! refusal. The pure choice is in `variety::room_flip_tests` and the mirror of documents in
//! `mirror::tests::source`; the whole-dungeon flip is in `play_tests`.
//!
//! The world is the fixture dungeon of `play_tests` with some rooms marked `flippable`.
use super::super::charm_tests::Scratch;
use super::super::run_tests::game_in;
use super::play_tests::{
    assert_rooms_are, documents, layout_in, leave_the_hub, live_layout, write_documents, ROOMS,
};
use super::*;
use crate::exit_arrival::arrival_error;
use crate::mirror::fixture::{BOSS, HUB, SECOND, START};
use crate::mirror::flip_world;
use crate::room::{RoomEnemies, RoomId};
use crate::run::RunPhase;
use crate::variety::FlipChoice;
use macroquad::math::Vec2;

/// The fixture dungeon with the rooms in `marked` flippable.
fn marked(ids: &[RoomId]) -> Vec<(RoomId, RoomEnemies)> {
    let mut documents = documents(false);
    for (id, document) in &mut documents {
        if ids.contains(id) {
            document.layout.as_mut().unwrap().flippable = true;
        }
    }
    documents
}

/// What the rooms are when exactly those of `mirrored` are mirrored.
fn mirrored_by(ids: &[RoomId], mirrored: &[RoomId]) -> Vec<(RoomId, RoomEnemies)> {
    let mut documents = marked(ids);
    flip_world(&mut documents, |id| mirrored.contains(&id));
    documents
}

/// The fixture dungeon as files with `ids` marked, whose `variety.ron` says `room_percent`.
fn world(ids: &[RoomId], room_percent: f32) -> Scratch {
    let scratch = Scratch::new();
    write_documents(&scratch, &marked(ids));
    scratch.write(
        "run.ron",
        "(hub_room: Custom(0), start_room: First, final_boss_room: Boss)",
    );
    scratch.write(
        "variety.ron",
        &format!("(flip_percent: 0.0, room_flip_percent: {room_percent:?})"),
    );
    scratch
}

/// A game on `scratch` in the hub whose next runs do `dungeon` and `rooms`.
fn game_choosing(scratch: &Scratch, dungeon: FlipChoice, rooms: FlipChoice) -> GameState {
    let mut game = game_in(scratch);
    game.set_flip_choice(dungeon);
    game.set_room_flip_choice(rooms);
    game
}

fn live_rooms_agree(game: &GameState) {
    for id in ROOMS {
        let layout = live_layout(game, id);
        for exit in &layout.exits {
            let target = live_layout(game, exit.target_room);
            let error = arrival_error(&target, Vec2::from(exit.spawn));
            assert_eq!(error, None, "{id:?} into {:?}", exit.target_room);
        }
    }
}

#[test]
fn the_choice_for_rooms_starts_on_auto_and_is_separate_from_the_dungeons() {
    let scratch = world(&[], 50.0);
    let mut game = game_in(&scratch);
    assert_eq!(game.room_flip_choice(), FlipChoice::Auto);
    game.set_room_flip_choice(FlipChoice::Always);
    assert_eq!(game.room_flip_choice(), FlipChoice::Always);
    assert_eq!(
        game.flip_choice(),
        FlipChoice::Auto,
        "the dungeon row is its own"
    );
    assert_eq!(game.variety_rules().room_flip_percent, 50.0);
}

#[test]
fn always_mirrors_exactly_the_marked_rooms_and_the_files_are_untouched() {
    let scratch = world(&[SECOND], 50.0);
    let files = scratch.0.join("rooms").join("second.ron");
    let before = std::fs::read_to_string(&files).unwrap();
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    assert_rooms_are(&game, &marked(&[SECOND]), "the hub");
    leave_the_hub(&mut game);

    assert!(game.run_flipped());
    assert!(!game.dungeon_flipped());
    assert_eq!(game.flipped_room_count(), 1);
    assert_rooms_are(&game, &mirrored_by(&[SECOND], &[SECOND]), "a run");
    assert_ne!(
        live_layout(&game, SECOND),
        layout_in(&marked(&[SECOND]), SECOND),
        "Second really is mirrored"
    );
    live_rooms_agree(&game);
    assert_eq!(std::fs::read_to_string(&files).unwrap(), before);
}

#[test]
fn never_mirrors_nothing_and_a_room_that_is_not_marked_is_never_mirrored() {
    let scratch = world(&[SECOND], 100.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Never);
    leave_the_hub(&mut game);
    assert!(!game.run_flipped());
    assert_eq!(game.flipped_room_count(), 0);
    assert_rooms_are(&game, &marked(&[SECOND]), "Never");

    // Always, with nothing marked, changes nothing either; and the run is not a flipped one.
    let scratch = world(&[], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    leave_the_hub(&mut game);
    assert!(!game.run_flipped() && game.flipped_room_count() == 0);
    assert_rooms_are(&game, &marked(&[]), "nothing marked");
}

#[test]
fn auto_follows_the_percent_in_the_file() {
    for (percent, flips) in [(0.0, false), (100.0, true)] {
        let scratch = world(&[SECOND], percent);
        let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Auto);
        for seed in [1, 2, 3, 0xABC] {
            game.set_fixed_seed(Some(seed));
            leave_the_hub(&mut game);
            assert_eq!(game.run_flipped(), flips, "{percent} percent, seed {seed}");
            game.abandon_run();
            game.continue_after_result();
        }
    }
}

#[test]
fn the_seed_decides_which_marked_rooms_a_run_mirrors() {
    // Seed 1 at 50 percent flips Boss and not First or Second; seed 3 flips Second and Boss
    // (`variety::room_flip_tests`).
    let marks = [START, SECOND, BOSS];
    let scratch = world(&marks, 50.0);
    for (seed, mirrored) in [(1, vec![BOSS]), (3, vec![SECOND, BOSS])] {
        let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Auto);
        game.set_fixed_seed(Some(seed));
        leave_the_hub(&mut game);
        assert_eq!(game.flipped_room_count(), mirrored.len(), "seed {seed}");
        assert_rooms_are(&game, &mirrored_by(&marks, &mirrored), "a seeded run");
        live_rooms_agree(&game);
        // The same seed again mirrors the same rooms.
        game.abandon_run();
        game.continue_after_result();
        leave_the_hub(&mut game);
        assert_rooms_are(&game, &mirrored_by(&marks, &mirrored), "the same seed");
    }
}

#[test]
fn a_flipped_dungeon_mirrors_a_marked_room_back_when_its_own_draw_says_so() {
    let scratch = world(&[SECOND], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Always, FlipChoice::Always);
    leave_the_hub(&mut game);
    assert!(game.dungeon_flipped() && game.run_flipped());
    let others = [START, BOSS, RoomId::Custom(1), RoomId::Custom(2)];
    assert_rooms_are(&game, &mirrored_by(&[SECOND], &others), "both flips");
    live_rooms_agree(&game);
}

#[test]
fn the_hub_is_never_mirrored_even_when_it_is_marked() {
    let scratch = world(&[HUB, SECOND], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    assert_rooms_are(&game, &marked(&[HUB, SECOND]), "the hub before a run");
    leave_the_hub(&mut game);
    assert_rooms_are(&game, &mirrored_by(&[HUB, SECOND], &[SECOND]), "a run");
}

#[test]
fn f5_in_the_middle_of_a_run_keeps_the_same_rooms_mirrored() {
    let scratch = world(&[SECOND], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    leave_the_hub(&mut game);
    for _ in 0..2 {
        game.reload_rooms().unwrap();
        assert!(game.run_flipped() && game.flipped_room_count() == 1);
        assert_rooms_are(&game, &mirrored_by(&[SECOND], &[SECOND]), "after F5");
    }
}

#[test]
fn the_end_of_a_run_returns_to_an_unmirrored_hub_and_the_next_run_asks_again() {
    let scratch = world(&[SECOND], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    leave_the_hub(&mut game);
    game.abandon_run();
    assert!(matches!(game.run_phase(), Some(RunPhase::Result(_))));
    assert!(
        game.run_flipped(),
        "the result screen shows the mirrored world"
    );
    game.continue_after_result();
    assert!(game.in_hub() && !game.run_flipped());
    assert_eq!(game.flipped_room_count(), 0);
    assert_rooms_are(&game, &marked(&[SECOND]), "the hub");

    game.set_room_flip_choice(FlipChoice::Never);
    leave_the_hub(&mut game);
    assert_rooms_are(&game, &marked(&[SECOND]), "the next run");
}

#[test]
fn the_editor_refuses_during_a_run_that_mirrored_a_room_and_not_otherwise() {
    let scratch = world(&[SECOND], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    game.toggle_editor().unwrap();
    game.toggle_editor().unwrap();
    leave_the_hub(&mut game);
    for error in [
        game.toggle_editor().unwrap_err(),
        game.edit_room(SECOND).unwrap_err(),
    ] {
        assert!(error.contains("abandon the run"), "{error}");
        assert!(error.contains("Flip rooms next run"), "{error}");
    }
    assert!(game.editor().is_none());

    // A run that mirrored nothing (nothing marked) leaves the editor available.
    let scratch = world(&[], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    leave_the_hub(&mut game);
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some());
}

#[test]
fn a_run_started_from_the_editor_in_a_mirrored_room_lands_at_its_spawn() {
    let scratch = world(&[SECOND], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    game.edit_room(SECOND).unwrap();
    game.toggle_editor().unwrap();
    game.update(super::super::run_tests::input(), 0.0);
    assert!(game.run_in_progress() && game.run_flipped());
    assert_eq!(game.room().id, SECOND);
    let spawn = layout_in(&mirrored_by(&[SECOND], &[SECOND]), SECOND).spawn;
    assert_eq!(game.player.position(), spawn.into());
}

#[test]
fn marking_a_room_in_the_editor_and_saving_makes_the_next_run_mirror_it() {
    // Nothing is marked; the editor ticks Second, Save writes it, and a run mirrors it. The
    // editor itself, and Save, still show and keep the room as authored.
    let scratch = world(&[], 50.0);
    let mut game = game_choosing(&scratch, FlipChoice::Never, FlipChoice::Always);
    game.edit_room(SECOND).unwrap();
    let authored = game.editor().unwrap().draft.layout.clone();
    game.editor_mut()
        .unwrap()
        .draft
        .set_flippable(true)
        .unwrap();
    game.save_editor().unwrap();
    let file = std::fs::read_to_string(scratch.0.join("rooms").join("second.ron")).unwrap();
    assert!(file.contains("flippable"), "{file}");
    let saved = game.editor().unwrap().draft.layout.clone();
    assert_eq!(
        saved.spawn, authored.spawn,
        "Save keeps the room as authored"
    );
    assert_eq!(saved.exits, authored.exits);

    game.toggle_editor().unwrap();
    game.current = game.rooms.iter().position(|r| r.id == HUB).unwrap();
    leave_the_hub(&mut game);
    assert_eq!(game.flipped_room_count(), 1);
    assert_ne!(live_layout(&game, SECOND).exits, authored.exits);
}
