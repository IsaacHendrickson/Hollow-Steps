//! The map screen through `GameState` (`map_link`): it opens and closes by its key and by
//! Escape, freezes the world like the inventory, stays shut where it must (the editor, a test
//! room, the pause menu, the inventory, the shop, the result screen), follows a rebound key, and
//! shows what the player has seen. The layout is tested in `dungeon_map::grid_tests`.
use macroquad::input::KeyCode;

use super::run_profile_tests::fixtures_path;
use super::run_tests::{game_in, input, kill, start_run, walk, world};
use super::*;
use crate::keybinds::{Action, Slot};
use crate::pause_menu::PauseItem;

/// A game with no hub: it starts in the first room, playing.
fn plain_game() -> GameState {
    GameState::try_new(&fixtures_path()).expect("fixtures load")
}

#[test]
fn the_map_key_opens_and_closes_the_map() {
    let mut game = plain_game();
    assert!(!game.map_open() && !game.is_paused());
    game.toggle_map();
    assert!(game.map_open() && game.is_paused());
    game.toggle_map();
    assert!(!game.map_open() && !game.is_paused());
}

#[test]
fn escape_closes_the_map_and_does_not_open_the_pause_menu() {
    let mut game = plain_game();
    game.toggle_map();
    game.press_escape();
    assert!(!game.map_open());
    assert!(!game.pause_menu().is_open(), "that Escape was the map's");
    game.press_escape();
    assert!(
        game.pause_menu().is_open(),
        "the next one is the pause menu's"
    );
}

#[test]
fn the_world_stands_still_while_the_map_is_open() {
    let mut game = plain_game();
    game.update(walk(1.0), 0.1);
    let here = game.player().position();
    assert!(here.x > game.room().spawn.x, "the player walks when it can");

    game.toggle_map();
    for _ in 0..20 {
        game.update(walk(1.0), 0.1);
    }
    assert_eq!(game.player().position(), here);
    assert_eq!(game.shake_offset(), Vec2::ZERO);

    game.toggle_map();
    game.update(walk(1.0), 0.1);
    assert!(game.player().position().x > here.x, "and carries on after");
}

#[test]
fn the_map_does_not_open_in_the_editor_and_opening_the_editor_closes_it() {
    let mut game = plain_game();
    game.toggle_editor().unwrap();
    game.toggle_map();
    assert!(!game.map_open());
    game.toggle_editor().unwrap();
    game.toggle_map();
    assert!(game.map_open());
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some() && !game.map_open());
}

#[test]
fn the_map_does_not_open_in_a_test_room() {
    let mut game = plain_game();
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    game.toggle_map();
    assert!(game.testing_room() && !game.map_open());
}

#[test]
fn the_map_does_not_open_over_the_pause_menu_the_inventory_or_the_shop() {
    let mut game = plain_game();
    game.press_escape();
    game.toggle_map();
    assert!(!game.map_open(), "pause menu");
    game.press_escape();

    game.toggle_inventory();
    game.toggle_map();
    assert!(!game.map_open(), "inventory");
    game.toggle_inventory();

    game.shop.open();
    game.toggle_map();
    assert!(!game.map_open(), "shop");
    game.shop.close();

    game.toggle_map();
    assert!(game.map_open(), "and it opens once they are gone");
}

#[test]
fn the_inventory_does_not_open_over_the_map() {
    let mut game = plain_game();
    game.toggle_map();
    game.toggle_inventory();
    assert!(game.map_open() && !game.inventory_open());
}

#[test]
fn the_map_does_not_open_on_the_result_screen_and_a_run_ending_closes_it() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    kill(&mut game);
    assert!(game.run_result().is_some());
    game.toggle_map();
    assert!(!game.map_open());

    game.continue_after_result();
    start_run(&mut game);
    game.toggle_map();
    assert!(game.map_open());
    game.abandon_run();
    assert!(!game.map_open(), "the run is over");
}

#[test]
fn a_new_run_starts_with_the_map_shut() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    game.toggle_map();
    assert!(game.map_open());
    game.map.open = false;
    start_run(&mut game);
    assert!(!game.map_open());
}

#[test]
fn a_rebound_key_opens_the_map_and_the_old_one_does_not() {
    let mut game = plain_game();
    game.press_escape();
    game.pause_activate(PauseItem::Settings);
    game.pause_activate_slot(Action::Map, Slot::Primary);
    game.pause_bind_key(KeyCode::N);
    game.press_escape();
    game.press_escape();
    assert!(!game.pause_menu().is_open());

    // What the frame loop does with this frame's keys.
    let press = |game: &mut GameState, key: KeyCode| {
        if game.keybinds().triggered(Action::Map, |bound| bound == key) {
            game.toggle_map();
        }
    };
    press(&mut game, KeyCode::M);
    assert!(!game.map_open(), "M is no longer the map");
    press(&mut game, KeyCode::N);
    assert!(game.map_open());
    press(&mut game, KeyCode::N);
    assert!(!game.map_open());
}

#[test]
fn the_screen_shows_the_first_room_and_the_next_one_and_grows_as_the_player_walks() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);

    let map = game.play_map();
    let rows: Vec<_> = map
        .rooms
        .iter()
        .map(|room| (room.id, room.visited, room.current))
        .collect();
    assert_eq!(
        rows,
        [(RoomId::First, true, true), (RoomId::Second, false, false)],
        "only the first room, and the empty box of the one beyond"
    );

    game.update(walk(1.0), 10.0);
    assert_eq!(game.room().id, RoomId::Second);
    let rows: Vec<_> = game
        .play_map()
        .rooms
        .iter()
        .map(|room| (room.id, room.visited, room.current))
        .collect();
    assert_eq!(
        rows,
        [
            (RoomId::First, true, false),
            (RoomId::Second, true, true),
            (RoomId::Boss, false, false)
        ]
    );
}

#[test]
fn after_a_death_the_next_run_starts_with_an_empty_looking_map() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    game.update(walk(1.0), 10.0);
    kill(&mut game);
    game.continue_after_result();
    start_run(&mut game);
    let seen: Vec<_> = game
        .play_map()
        .rooms
        .iter()
        .filter(|room| room.visited)
        .map(|room| room.id)
        .collect();
    assert_eq!(seen, [RoomId::First]);
    let _ = input();
}
