//! Generated runs, continued: the editor is closed while a run is generated (the same refusal as
//! a flipped run's), and a test room is not part of any of it.
use super::super::run_tests::{game_in, walk, world};
use super::fixture::generated_world;
use super::{begin, game_with, ids, index_of, run_with_seed};
use crate::dungeon_map::{MapLayout, MapRoomInput};
use crate::game::GameState;
use crate::item::Item;
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

#[test]
fn the_pause_menu_shows_the_runs_seed_only_while_a_run_is_going() {
    let (_scratch, mut game) = game_with((0, 1));
    assert_eq!(game.pause_seed_label(), None, "the hub has no seed");

    begin(&mut game);
    let seed = game.run_seed().expect("a run has a seed");
    assert_eq!(
        game.pause_seed_label(),
        Some(format!("Seed {}", crate::seed::format_seed(seed)))
    );
    // The label is the seed the result screen and F4 show.
    assert_eq!(game.run_seed(), game.last_run_seed());

    game.abandon_run();
    assert_eq!(game.pause_seed_label(), None, "the run is over");
    game.continue_after_result();
    assert_eq!(game.pause_seed_label(), None, "back in the hub");
}

// ---- a pool that cannot build ------------------------------------------------------------------

/// A scratch world whose pool has Start pieces but no Boss piece: the two Boss piece files are
/// gone, so the generator has nothing to end a dungeon with.
fn world_without_boss_pieces() -> crate::game::charm_tests::Scratch {
    let scratch = generated_world((0, 1));
    for id in super::boss_pieces() {
        let RoomId::Custom(number) = id else {
            unreachable!("pieces have custom ids")
        };
        std::fs::remove_file(scratch.0.join(format!("rooms/room_{number}.ron"))).unwrap();
    }
    scratch
}

fn assert_stays_in_the_hub_with_the_reason(game: &mut GameState, reason: &str) {
    let seed_before = game.run.seeds.next_seed().0;
    let authored = ids(game);
    game.update(walk(1.0), 10.0);

    assert!(!game.run_in_progress(), "the run did not start");
    assert!(game.in_hub(), "the player is still in the hub");
    assert_eq!(game.room().id, super::HUB);
    assert_eq!(ids(game), authored, "the authored world is untouched");
    let message = game.run_message().expect("the player is told why");
    assert!(
        message.starts_with("Could not start the run: "),
        "{message}"
    );
    assert!(message.contains(reason), "{message}");
    assert_eq!(game.last_run_seed(), None, "a failed start takes no seed");
    assert_eq!(
        game.run.seeds.next_seed().0,
        seed_before,
        "and uses none up"
    );
    assert!(game.run_plan().is_none());
}

#[test]
fn always_with_no_boss_piece_keeps_the_player_in_the_hub_with_the_reason() {
    let scratch = world_without_boss_pieces();
    let mut game = game_in(&scratch);
    game.set_generate_choice(GenerateChoice::Always);
    assert_stays_in_the_hub_with_the_reason(&mut game, "no Boss piece");
}

#[test]
fn always_with_no_pieces_at_all_fails_with_the_reason() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.set_generate_choice(GenerateChoice::Always);
    assert_stays_in_the_hub_with_the_reason(&mut game, "no Start piece");
}

#[test]
fn auto_with_no_boss_piece_plays_the_fixed_dungeon_instead_of_failing() {
    // The rules say generate, but the pool cannot end a dungeon: Auto falls back, since the
    // authored rooms are there to play; only Always insists.
    let scratch = world_without_boss_pieces();
    let mut game = game_in(&scratch);
    begin(&mut game);
    assert!(game.run_in_progress() && !game.run_generated());
    assert_eq!(game.run_message(), None);
    assert!(ids(&game).contains(&RoomId::Second), "the authored rooms");
}

#[test]
fn never_keeps_the_fixed_dungeon_even_with_a_full_pool() {
    let (_scratch, mut game) = game_with((0, 1));
    let authored = ids(&game);
    game.set_generate_choice(GenerateChoice::Never);
    begin(&mut game);
    assert!(game.run_in_progress() && !game.run_generated());
    assert!(game.run_plan().is_none());
    assert_eq!(ids(&game), authored, "the authored world, every room of it");
    assert_eq!(game.run_message(), None);
}

#[test]
fn a_rules_switch_that_is_off_plays_the_fixed_dungeon_but_always_still_generates() {
    let scratch = generated_world((0, 1));
    scratch.write(
        "variety.ron",
        "(flip_percent: 0.0, generator: (enabled: false))",
    );
    let mut game = game_in(&scratch);
    let authored = ids(&game);
    begin(&mut game);
    assert!(!game.run_generated(), "Auto obeys generator.enabled");
    assert_eq!(ids(&game), authored);
    game.abandon_run();
    game.continue_after_result();

    game.set_generate_choice(GenerateChoice::Always);
    begin(&mut game);
    assert!(game.run_generated(), "Always does not");
}

// ---- nothing carries from one run to the next --------------------------------------------------

fn lock_in_the_plan(game: &GameState) -> (RoomId, crate::room::Exit, u8) {
    game.rooms
        .iter()
        .find_map(|room| {
            let exit = room.exits.iter().find(|exit| exit.locked_by.is_some())?;
            Some((room.id, *exit, exit.locked_by?))
        })
        .expect("a dungeon with a key has a locked exit")
}

fn key_in_the_plan(game: &GameState) -> (RoomId, crate::item::WorldItem) {
    game.rooms
        .iter()
        .find_map(|room| {
            let key = room
                .items
                .iter()
                .find(|item| matches!(item.item, Item::Key(_)))?;
            Some((room.id, *key))
        })
        .expect("a dungeon with a key has a key lying in a room")
}

#[test]
fn taken_items_opened_locks_and_puzzle_state_start_over_with_each_run() {
    let (_scratch, mut game) = game_with((1, 1));
    run_with_seed(&mut game, 21);
    let (key_room, key) = key_in_the_plan(&game);
    let (lock_room, lock, number) = lock_in_the_plan(&game);

    // Take the key off the floor of its room.
    game.current = index_of(&game, key_room);
    game.player
        .place_on_floor(key.position.x, game.world_bounds());
    assert!(game.pick_up_item(), "the key is within reach");
    assert!(game.player.collected_items().contains(&key.item));
    assert!(!game.taken_items.is_empty());
    assert!(!game.rooms[game.current]
        .items
        .iter()
        .any(|i| i.item == key.item));

    // Open a lock with a key (the number the plan locked), and leave a wall broken and a switch
    // on, as play would.
    game.player.collect_item(Item::Key(number));
    game.current = index_of(&game, lock_room);
    assert!(game.pass_exit(lock));
    assert_eq!(game.locks.opened.len(), 1);
    let wall = macroquad::math::Rect::new(10.0, 10.0, 20.0, 20.0);
    game.puzzles.broken.record(lock_room, wall);
    game.puzzles.on.record(lock_room, wall);
    assert_ne!(game.puzzles, Default::default());

    // The same seed again: the same dungeon, built whole.
    game.abandon_run();
    game.continue_after_result();
    run_with_seed(&mut game, 21);

    assert!(
        game.taken_items.is_empty(),
        "nothing is remembered as taken"
    );
    assert!(
        game.locks.opened.is_empty(),
        "no door is remembered as open"
    );
    assert_eq!(
        game.puzzles,
        Default::default(),
        "no wall broken, no switch on"
    );
    let (_, lying) = key_in_the_plan(&game);
    assert_eq!(lying, key, "the key lies where it did");
    let (_, relocked, again) = lock_in_the_plan(&game);
    assert_eq!(
        (relocked, again),
        (lock, number),
        "the exit is locked again"
    );
    assert!(
        !game.player.collected_items().contains(&Item::Key(number)),
        "the player starts with no key"
    );

    // F5 builds the rooms again and takes nothing out of them.
    let before = super::layouts(&game);
    game.reload_rooms().unwrap();
    assert_eq!(super::layouts(&game), before);
}

#[test]
fn a_run_of_another_seed_does_not_inherit_what_the_last_one_took() {
    let (_scratch, mut game) = game_with((1, 1));
    run_with_seed(&mut game, 5);
    let (key_room, key) = key_in_the_plan(&game);
    game.current = index_of(&game, key_room);
    game.player
        .place_on_floor(key.position.x, game.world_bounds());
    assert!(game.pick_up_item());
    game.abandon_run();
    game.continue_after_result();

    // Whatever the next dungeon is, every key in it is there to take.
    for seed in 6..16 {
        run_with_seed(&mut game, seed);
        let lying: usize = game
            .rooms
            .iter()
            .flat_map(|room| &room.items)
            .filter(|item| matches!(item.item, Item::Key(_)))
            .count();
        assert_eq!(lying, 1, "seed {seed}: the one key lies in its room");
        assert!(game.taken_items.is_empty(), "seed {seed}");
        game.abandon_run();
        game.continue_after_result();
    }
}

// ---- the plans themselves ----------------------------------------------------------------------

#[test]
fn no_plan_has_a_map_conflict_over_a_hundred_seeds() {
    let (_scratch, mut game) = game_with((0, 2));
    for seed in 0..100 {
        let plan = run_with_seed(&mut game, seed);
        let inputs: Vec<MapRoomInput> = plan
            .catalog()
            .iter()
            .map(|(id, layout)| MapRoomInput::from_layout(*id, layout))
            .collect();
        let map = MapLayout::build(&inputs);
        assert!(map.conflicts.is_empty(), "seed {seed}: {:?}", map.conflicts);
        let mut cells: Vec<_> = map.rooms.iter().map(|room| room.cell).collect();
        let rooms = cells.len();
        cells.sort_unstable();
        cells.dedup();
        assert_eq!(cells.len(), rooms, "seed {seed}: two rooms share a cell");
        // The map the run itself shows is the one built from the rooms in play.
        let live = game.dungeon_map();
        let live_cells: Vec<_> = live.rooms.iter().map(|room| (room.id, room.cell)).collect();
        let planned: Vec<_> = map.rooms.iter().map(|room| (room.id, room.cell)).collect();
        assert_eq!(live_cells, planned, "seed {seed}");
        game.abandon_run();
        game.continue_after_result();
    }
}

#[test]
fn a_failed_start_leaves_its_reason_to_show_in_the_hub_until_the_choice_changes() {
    // The player is sent back to the hub spawn when the generator refuses; without the reason on
    // screen that looks like the door does nothing.
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.set_generate_choice(GenerateChoice::Always);
    assert_eq!(game.start_failure(), None, "nothing before an attempt");

    game.update(walk(1.0), 10.0);
    assert!(game.in_hub() && !game.run_in_progress());
    let reason = game.start_failure().expect("the hub shows why");
    assert!(reason.starts_with("Could not start the run: "), "{reason}");
    assert!(reason.contains("no Start piece"), "{reason}");

    // Another try with the same choice says it again, and a new choice clears the old reason.
    game.update(walk(1.0), 10.0);
    assert!(game.start_failure().is_some());
    game.set_generate_choice(GenerateChoice::Never);
    assert_eq!(game.start_failure(), None);

    // A start that works leaves nothing to show, and neither does a run in progress or its result.
    begin(&mut game);
    assert!(game.run_in_progress());
    assert_eq!(game.start_failure(), None);
}
