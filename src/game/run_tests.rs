//! The hub-and-run loop through `GameState` (`run_link`): starting a run by leaving the hub, the
//! three ways a run ends, the result screen and what a run resets. The saved profile, test rooms,
//! the editor and a game with no hub are in `run_profile_tests`; the rules themselves are in
//! `crate::run` and `crate::profile`.
//!
//! The world is a private copy of `tests/fixtures` with a hub added as `room_0.ron` (its exit
//! leads to the first room) and a `run.ron` naming it.
use std::path::PathBuf;

use super::charm_tests::Scratch;
use super::*;
use crate::item::Item;
use crate::profile::Profile;
use crate::room::{Exit, RoomEdge};
use crate::run::{RunOutcome, RunPhase};

pub(super) const HUB: RoomId = RoomId::Custom(0);
const HUB_FILE: &str = r#"(enemies: [], layout: Some((
    bounds: (0.0, 0.0, 960.0, 460.0),
    floor_y: 460.0,
    spawn: (180.0, 436.0),
    items: [],
    exits: [(edge: Right, offset: 364.0, length: 96.0, target_room: First, spawn: (180.0, 436.0))],
)))"#;

pub(super) fn run_file(keep_percent: f32, final_boss: &str) -> String {
    format!(
        "(hub_room: Custom(0), start_room: First, final_boss_room: {final_boss}, \
         coin_keep_percent: {keep_percent:?})"
    )
}

/// A scratch world with a hub, where a run keeps `keep_percent` of its coins.
pub(super) fn world(keep_percent: f32) -> Scratch {
    let scratch = Scratch::new();
    std::fs::write(scratch.0.join("rooms").join("room_0.ron"), HUB_FILE).unwrap();
    scratch.write("run.ron", &run_file(keep_percent, "Boss"));
    scratch
}

pub(super) fn profile_path(scratch: &Scratch) -> PathBuf {
    scratch.0.join("profile.ron")
}

/// The game of `scratch`, starting in the hub, saving its profile inside `scratch`.
pub(super) fn game_in(scratch: &Scratch) -> GameState {
    let mut game = GameState::try_new(&scratch.0).expect("the scratch world must load");
    game.use_profile_file(profile_path(scratch)).unwrap();
    game
}

pub(super) fn input() -> PlayerInput {
    PlayerInput::default()
}

pub(super) fn walk(axis: f32) -> PlayerInput {
    PlayerInput {
        move_axis: axis,
        ..input()
    }
}

pub(super) fn swing() -> PlayerInput {
    PlayerInput {
        attack_pressed: true,
        ..input()
    }
}

/// Leaves the hub through its exit: a run starts and the player is in the first room.
pub(super) fn start_run(game: &mut GameState) {
    game.update(walk(1.0), 10.0);
    assert!(
        game.run_in_progress(),
        "walking out of the hub starts a run"
    );
    game.rooms[RoomId::First.index()].enemies.clear();
}

/// The player dies, once the arrival invincibility is over, and the tick notices.
pub(super) fn kill(game: &mut GameState) {
    let bounds = game.world_bounds();
    let terrain = &game.rooms[game.current].terrain;
    game.player
        .update_with_terrain(input(), 10.0, bounds, terrain);
    assert!(game.player.take_damage_amount(u16::MAX));
    game.update(input(), 0.0);
}

/// Touches the three coins of the first room.
fn collect_first_room_coins(game: &mut GameState) {
    for x in [360.0, 410.0, 460.0] {
        game.player.place_on_floor(x, game.world_bounds());
        game.update(input(), 0.0);
    }
}

/// The boss of the boss room one hit from defeat, with the player in reach of it.
fn stand_before_the_weakened_boss(game: &mut GameState) {
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(520.0, game.world_bounds());
    let boss = game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_mut()
        .unwrap();
    boss.damage_boss(boss.health() - 1);
}

/// The player is in the hub exactly when the loop is in its Hub phase, outside a result.
pub(super) fn assert_phase_matches_room(game: &GameState) {
    if game.run_result().is_none() {
        assert_eq!(
            game.run_phase() == Some(RunPhase::Hub),
            game.room().id == HUB,
            "phase {:?} in room {:?}",
            game.run_phase(),
            game.room().id
        );
    }
}

// ---- starting a run ---------------------------------------------------------------------------

#[test]
fn the_game_starts_in_the_hub_with_a_fresh_profile() {
    let scratch = world(100.0);
    let game = game_in(&scratch);
    assert_eq!(game.room().id, HUB);
    assert_eq!(game.run_phase(), Some(RunPhase::Hub));
    assert!(game.in_hub() && !game.run_in_progress());
    assert_eq!(*game.profile(), Profile::default());
    assert_phase_matches_room(&game);
}

#[test]
fn leaving_the_hub_starts_a_run_in_the_first_room() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.update(walk(1.0), 10.0);
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.run_phase(), Some(RunPhase::Running));
    assert!(game.run_in_progress() && !game.in_hub());
    assert_phase_matches_room(&game);
    assert_eq!(game.take_events().last(), Some(&GameEvent::RoomEntered));
}

#[test]
fn walking_around_the_hub_does_not_start_a_run() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.update(walk(-1.0), 10.0);
    game.update(walk(1.0), 0.05);
    assert_eq!(game.room().id, HUB);
    assert!(game.in_hub());
}

#[test]
fn a_run_starts_with_every_room_rebuilt_and_everything_the_last_one_touched_reset() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    // Leftovers of earlier play: coins, items, a taken item, a dead enemy's room emptied, an
    // owned and an equipped charm, and a wounded player with no mana.
    let first = RoomId::First.index();
    game.coin_count = 7;
    game.rooms[first].coins.clear();
    game.rooms[first].items.clear();
    game.rooms[first].enemies.clear();
    game.player.collect_item(Item::Dash);
    game.charm_state.grant("ward");
    game.toggle_charm("heart");
    game.toggle_charm("sharp");
    game.apply_stats();
    game.player.take_damage_amount(20);
    game.vitals.respawn();

    game.update(walk(1.0), 10.0);

    assert!(game.run_in_progress());
    assert_eq!(game.coin_count(), 0);
    assert_eq!(game.rooms[first].coins.len(), 3, "coins are back");
    assert_eq!(game.rooms[first].items.len(), 2, "items are back");
    assert_eq!(game.rooms[first].enemies.len(), 1, "enemies are back");
    assert!(game.player().collected_items().is_empty());
    assert!(!game.charm_state().is_owned("ward"));
    assert!(game.charm_state().equipped().is_empty());
    assert_eq!(game.charm_state().owned(), ["heart", "swift", "sharp"]);
    assert_eq!(
        game.player().health().current(),
        game.player().health().max()
    );
    assert_eq!(
        game.player().config().melee.damage,
        game.tuning().player.melee.damage
    );
    assert_eq!(game.vitals().mana(), game.vitals().max_mana());
}

#[test]
fn items_taken_and_coins_collected_in_a_run_are_back_in_the_next() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    collect_first_room_coins(&mut game);
    assert_eq!(game.coin_count(), 3);
    game.taken_items.record(
        RoomId::First,
        game.rooms[RoomId::First.index()].items.remove(0),
    );
    game.abandon_run();
    game.continue_after_result();

    start_run(&mut game);
    assert_eq!(game.coin_count(), 0);
    assert_eq!(game.rooms[RoomId::First.index()].coins.len(), 3);
    assert_eq!(game.rooms[RoomId::First.index()].items.len(), 2);
}

#[test]
fn a_run_that_cannot_start_leaves_the_player_in_the_hub_and_says_why() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    let first_file = scratch.0.join("rooms").join("first.ron");
    let good = std::fs::read_to_string(&first_file).unwrap();
    std::fs::write(&first_file, "this is not a room").unwrap();

    game.update(walk(1.0), 10.0);

    assert_eq!(game.room().id, HUB);
    assert!(game.in_hub());
    assert!(game
        .run_message()
        .unwrap()
        .contains("Could not start the run"));
    assert_phase_matches_room(&game);

    std::fs::write(&first_file, good).unwrap();
    game.update(walk(1.0), 10.0);
    assert!(game.run_in_progress());
    assert!(game.run_message().is_none());
}

// ---- death --------------------------------------------------------------------------------------

#[test]
fn dying_in_a_run_banks_the_kept_coins_and_shows_the_result() {
    let scratch = world(50.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 7;

    kill(&mut game);

    let result = game.run_result().expect("the result screen shows");
    assert_eq!(result.outcome, RunOutcome::Died);
    assert_eq!((result.coins_earned, result.coins_banked), (7, 3));
    assert_eq!((result.tier_before, result.tier_after), (0, 0));
    assert_eq!(game.profile().coins, 3);
    assert_eq!((game.profile().tier, game.profile().cleared), (0, 0));
    assert!(!game.run_in_progress());
}

#[test]
fn nothing_is_reset_until_the_result_screen_is_left() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 4;
    kill(&mut game);
    assert_eq!(game.coin_count(), 4, "still the run's coins");
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.player().health().current(), 0);

    game.continue_after_result();

    assert_eq!(game.coin_count(), 0);
    assert_eq!(game.room().id, HUB);
    assert_eq!(game.run_phase(), Some(RunPhase::Hub));
    assert_eq!(
        game.player().health().current(),
        game.player().health().max()
    );
    assert_eq!(game.profile().coins, 4, "the bank is kept");
    assert_phase_matches_room(&game);
}

#[test]
fn continuing_puts_the_player_in_the_hub_at_full_health_and_mana_without_the_run_items() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.player.collect_item(Item::Dash);
    game.charm_state.grant("ward");
    game.toggle_charm("heart");
    game.vitals.gain_kill_mana();
    kill(&mut game);

    game.continue_after_result();

    assert!(game.player().collected_items().is_empty());
    assert!(!game.charm_state().is_owned("ward"));
    assert!(game.charm_state().equipped().is_empty());
    assert_eq!(game.vitals().mana(), game.vitals().max_mana());
    assert_eq!(game.vitals().health(), game.vitals().max_health());
    assert_eq!(game.player().position().x, game.room().spawn.x);
}

// ---- the result screen --------------------------------------------------------------------------

#[test]
fn the_world_is_frozen_on_the_result_screen_and_escape_does_nothing() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    kill(&mut game);
    game.take_events();
    let (place, coins) = (game.player().position(), game.coin_count());
    game.rooms[RoomId::First.index()]
        .coins
        .push(crate::coin::Coin { position: place });

    for _ in 0..30 {
        game.update(walk(1.0), 0.1);
    }
    game.press_escape();
    game.toggle_inventory();

    assert!(game.is_paused());
    assert!(!game.pause_menu().is_open() && !game.inventory_open);
    assert_eq!(game.player().position(), place);
    assert_eq!(game.coin_count(), coins);
    assert!(game.take_events().is_empty());
    assert!(game.run_result().is_some());
    assert_eq!(game.shake_offset(), Vec2::ZERO);
}

#[test]
fn continue_does_nothing_unless_a_result_is_showing() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.continue_after_result();
    assert!(game.in_hub());
    start_run(&mut game);
    game.coin_count = 2;
    game.continue_after_result();
    assert!(game.run_in_progress());
    assert_eq!(game.coin_count(), 2);
}

#[test]
fn a_finished_run_cannot_end_a_second_time() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 5;
    game.abandon_run();
    game.abandon_run();
    game.abandon_run_on_exit();
    assert_eq!(game.profile().coins, 5, "banked once");
}

// ---- the other ways a run ends --------------------------------------------------------------------

#[test]
fn defeating_the_final_boss_clears_the_run() {
    let scratch = world(50.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 4;
    stand_before_the_weakened_boss(&mut game);

    game.update(swing(), 0.0);

    let result = game.run_result().expect("the run is over");
    assert_eq!(result.outcome, RunOutcome::Cleared);
    assert_eq!((result.coins_earned, result.coins_banked), (4, 2));
    assert_eq!((result.tier_before, result.tier_after), (0, 1));
    assert_eq!((game.profile().tier, game.profile().cleared), (1, 1));
    assert_eq!(game.profile().coins, 2);
}

#[test]
fn a_boss_that_is_not_the_final_boss_clears_nothing() {
    let scratch = world(100.0);
    scratch.write("run.ron", &run_file(100.0, "Second"));
    let mut game = game_in(&scratch);
    start_run(&mut game);
    stand_before_the_weakened_boss(&mut game);

    game.update(swing(), 0.0);

    assert!(game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_ref()
        .unwrap()
        .is_defeated());
    assert!(game.run_in_progress(), "the run goes on");
    assert_eq!((game.profile().tier, game.profile().cleared), (0, 0));
}

#[test]
fn abandoning_banks_the_kept_coins_and_shows_the_result() {
    let scratch = world(50.0);
    let mut game = game_in(&scratch);
    game.abandon_run();
    assert!(game.run_result().is_none(), "nothing to abandon in the hub");
    start_run(&mut game);
    game.coin_count = 9;

    game.abandon_run();

    let result = game.run_result().unwrap();
    assert_eq!(result.outcome, RunOutcome::Abandoned);
    assert_eq!((result.coins_earned, result.coins_banked), (9, 4));
    assert_eq!((game.profile().tier, game.profile().coins), (0, 4));
}

#[test]
fn entering_the_hub_during_a_run_abandons_it() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.coin_count = 6;
    let first = RoomId::First.index();
    game.rooms[first].exits.push(Exit {
        edge: RoomEdge::Left,
        offset: 364.0,
        length: 96.0,
        target_room: HUB,
        spawn: Vec2::new(180.0, 436.0),
        door: None,
        locked_by: None,
    });
    game.player.place_on_floor(60.0, game.world_bounds());

    game.update(walk(-1.0), 10.0);

    assert_eq!(game.room().id, HUB);
    let result = game.run_result().expect("the result shows in the hub");
    assert_eq!(result.outcome, RunOutcome::Abandoned);
    assert_eq!(game.profile().coins, 6);
    game.continue_after_result();
    assert_eq!(game.run_phase(), Some(RunPhase::Hub));
    assert_phase_matches_room(&game);
}

/// A door in the room being played, on the floor from x = 380 to 428, leading to `target`.
fn put_door_in_current_room(game: &mut GameState, target: RoomId) {
    let door = Rect::new(380.0, 388.0, 48.0, 72.0);
    let index = game.current;
    game.rooms[index]
        .exits
        .push(Exit::doorway(door, target, Vec2::new(180.0, 436.0)));
    game.player.place_on_floor(404.0, game.world_bounds());
    game.update(input(), 0.01);
    assert!(
        game.door_in_reach().is_some(),
        "the player stands in front of the door"
    );
}

fn up() -> PlayerInput {
    PlayerInput {
        pickup_pressed: true,
        ..input()
    }
}

#[test]
fn a_door_out_of_the_hub_starts_a_run_like_an_edge_exit() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    put_door_in_current_room(&mut game, RoomId::First);

    game.update(up(), 0.01);

    assert!(game.run_in_progress());
    assert_eq!(game.room().id, RoomId::First);
    assert_phase_matches_room(&game);
}

#[test]
fn a_door_into_the_hub_during_a_run_abandons_it_like_an_edge_exit() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.rooms[RoomId::First.index()].coins.clear();
    game.coin_count = 6;
    put_door_in_current_room(&mut game, HUB);

    game.update(up(), 0.01);

    assert_eq!(game.room().id, HUB);
    let result = game.run_result().expect("the result shows");
    assert_eq!(result.outcome, RunOutcome::Abandoned);
    assert_eq!(game.profile().coins, 6);
}

#[test]
fn closing_the_game_banks_a_run_in_progress_and_ends_a_test_room_first() {
    let scratch = world(50.0);
    let mut game = game_in(&scratch);
    game.abandon_run_on_exit();
    assert_eq!(*game.profile(), Profile::default(), "nothing in the hub");

    start_run(&mut game);
    game.coin_count = 6;
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    game.abandon_run_on_exit();

    assert!(!game.testing_room());
    assert_eq!(
        game.profile().coins,
        3,
        "the run's 6 coins, not the test room's"
    );
    assert_eq!(game.run_result().unwrap().outcome, RunOutcome::Abandoned);
}
