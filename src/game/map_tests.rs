//! Which rooms the dungeon map has seen (`map_link`): entering records, a run starting or ending
//! clears, an editor Save keeps, a test room records nothing, and a game with no hub never
//! clears. The world is the scratch one of `run_tests`: a hub, then First, Second and Boss.
use super::run_profile_tests::fixtures_path;
use super::run_tests::{game_in, input, kill, start_run, walk, world, HUB};
use super::*;
use crate::dungeon_map::CellSource;

/// The rooms seen so far, in room list order.
fn seen(game: &GameState) -> Vec<RoomId> {
    game.rooms
        .iter()
        .map(|room| room.id)
        .filter(|id| game.map_discovery().contains(*id))
        .collect()
}

/// Through the first room's exit into the second, in a run.
fn walk_to_second(game: &mut GameState) {
    game.update(walk(1.0), 10.0);
    assert_eq!(game.room().id, RoomId::Second);
}

#[test]
fn the_game_starts_having_seen_only_the_room_it_starts_in() {
    let scratch = world(100.0);
    let game = game_in(&scratch);
    assert_eq!(seen(&game), [HUB]);
}

#[test]
fn leaving_the_hub_starts_a_run_that_has_seen_only_the_first_room() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    assert_eq!(
        seen(&game),
        [RoomId::First],
        "the hub is not part of the run"
    );
}

#[test]
fn walking_through_an_exit_adds_the_next_room() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    walk_to_second(&mut game);
    assert_eq!(seen(&game), [RoomId::First, RoomId::Second]);
    assert!(!game.map_discovery().contains(RoomId::Boss));
}

#[test]
fn a_room_is_recorded_once_however_often_it_is_entered() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    walk_to_second(&mut game);
    game.update(walk(-1.0), 10.0);
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.map_discovery().len(), 2);
}

#[test]
fn dying_empties_the_map_and_the_next_run_starts_over() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    walk_to_second(&mut game);

    kill(&mut game);

    assert!(game.run_result().is_some());
    assert!(game.map_discovery().is_empty(), "the run ended");
    game.continue_after_result();
    assert_eq!(seen(&game), [HUB], "back in the hub, the only room seen");
    start_run(&mut game);
    assert_eq!(seen(&game), [RoomId::First]);
}

#[test]
fn abandoning_a_run_empties_the_map_too() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    walk_to_second(&mut game);
    game.abandon_run();
    assert!(game.map_discovery().is_empty());
}

#[test]
fn an_editor_save_keeps_what_was_seen() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    walk_to_second(&mut game);

    game.toggle_editor().unwrap();
    game.save_editor().unwrap();
    assert_eq!(
        seen(&game),
        [RoomId::First, RoomId::Second],
        "the rooms were rebuilt"
    );
    game.toggle_editor().unwrap();
    game.update(input(), 0.01);

    assert_eq!(seen(&game), [RoomId::First, RoomId::Second]);
    assert!(game.run_in_progress());
}

#[test]
fn a_reload_keeps_what_was_seen() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    walk_to_second(&mut game);
    game.reload_rooms().unwrap();
    assert_eq!(seen(&game), [RoomId::First, RoomId::Second]);
}

#[test]
fn a_test_room_records_nothing() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.edit_room(RoomId::Boss).unwrap();
    game.start_test_room().unwrap();

    for _ in 0..3 {
        game.update(input(), 0.01);
    }

    assert!(game.testing_room());
    assert_eq!(game.room().id, RoomId::Boss);
    assert_eq!(
        seen(&game),
        [RoomId::First],
        "the test room is not part of the run"
    );
    game.end_test_room().unwrap();
    assert_eq!(seen(&game), [RoomId::First]);
}

#[test]
fn the_editor_moving_the_player_counts_as_walking_there() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.edit_room(RoomId::Second).unwrap();
    assert_eq!(
        seen(&game),
        [RoomId::First],
        "nothing changes while editing"
    );

    game.toggle_editor().unwrap();
    game.update(input(), 0.01);

    assert_eq!(seen(&game), [RoomId::First, RoomId::Second]);
}

#[test]
fn the_editor_starting_a_run_from_the_hub_leaves_the_hub_unseen() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.edit_room(RoomId::Second).unwrap();
    game.toggle_editor().unwrap();
    game.update(input(), 0.01);
    assert!(game.run_in_progress());
    assert_eq!(seen(&game), [RoomId::Second]);
}

#[test]
fn with_no_hub_the_map_starts_with_the_start_room_and_is_never_cleared() {
    let mut game = GameState::try_new(&fixtures_path()).expect("fixtures load");
    assert_eq!(game.run_phase(), None);
    assert_eq!(seen(&game), [RoomId::First]);

    game.update(walk(1.0), 10.0);
    assert_eq!(game.room().id, RoomId::Second);
    kill(&mut game);

    assert_eq!(game.run_result(), None, "death only respawns without a hub");
    assert_eq!(seen(&game), [RoomId::First, RoomId::Second]);
}

#[test]
fn the_dungeon_map_covers_every_room_seen_or_not() {
    let scratch = world(100.0);
    let game = game_in(&scratch);
    let map = game.dungeon_map();
    assert_eq!(map.rooms.len(), game.rooms.len());
    let cell = |id| map.room(id).expect("on the map").cell;
    // First, Second and Boss are chained left to right by their exits.
    assert_eq!(
        [
            cell(RoomId::First),
            cell(RoomId::Second),
            cell(RoomId::Boss)
        ],
        [(0, 0), (1, 0), (2, 0)]
    );
    // Nothing leads into the hub, so no walk reaches it: it sits in the row below.
    assert_eq!(map.room(HUB).unwrap().source, CellSource::Row);
    assert!(!map.has_entrance(HUB));
}
