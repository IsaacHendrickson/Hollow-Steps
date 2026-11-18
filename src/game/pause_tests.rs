//! The pause menu through `GameState`: the world stands still while it is open, Escape and the
//! inventory do not fight over the screen, and every screen of the menu keeps the world stopped.
use std::path::Path;

use super::*;
use crate::pause_menu::{PauseItem, PauseOutcome, PauseScreen};

const TICK: f32 = 1.0 / 120.0;

fn test_game() -> GameState {
    GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

/// Every key that does something in play, held at once.
fn busy_input() -> PlayerInput {
    PlayerInput {
        move_axis: 1.0,
        jump_pressed: true,
        pickup_pressed: true,
        attack_pressed: true,
        jump_held: true,
        dash_pressed: true,
        ..PlayerInput::default()
    }
}

#[test]
fn escape_opens_the_menu_and_the_world_stands_still_until_it_closes() {
    let mut game = test_game();
    game.rooms[0].coins = vec![Coin {
        position: game.player.position(),
    }];
    let player = game.player.clone();
    let enemy_position = game.room().enemies[0].position();
    assert!(!game.is_paused());

    game.press_escape();
    assert!(game.pause_menu().is_open());
    assert!(game.is_paused());
    assert!(!game.inventory_open());
    // Large, zero and non-finite delta times all leave the world exactly as it was.
    for delta in [10.0, 0.0, TICK, f32::NAN, f32::INFINITY] {
        game.update(busy_input(), delta);
    }
    assert_eq!(game.player, player);
    assert_eq!(game.room().enemies[0].position(), enemy_position);
    assert_eq!(game.coin_count(), 0);
    assert_eq!(game.room().id, RoomId::First);
    assert!(
        game.take_events().is_empty(),
        "nothing happens while paused"
    );
    assert_eq!(game.shake_offset(), Vec2::ZERO);

    game.press_escape();
    assert!(!game.pause_menu().is_open());
    assert!(!game.is_paused());
    game.update(PlayerInput::default(), TICK);
    assert_eq!(game.coin_count(), 1, "the world runs again after unpausing");
}

#[test]
fn the_player_moves_again_after_unpausing() {
    let mut game = test_game();
    let start = game.player.position();
    game.press_escape();
    for _ in 0..30 {
        game.update(busy_input(), TICK);
    }
    assert_eq!(game.player.position(), start);
    assert_eq!(
        game.pause_activate_selected(),
        PauseOutcome::Continue,
        "Unpause is selected first"
    );
    assert!(!game.is_paused());
    for _ in 0..30 {
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            TICK,
        );
    }
    assert_ne!(game.player.position(), start);
}

#[test]
fn escape_closes_an_open_inventory_before_it_opens_the_menu() {
    let mut game = test_game();
    game.toggle_inventory();
    assert!(game.inventory_open());
    game.press_escape();
    assert!(!game.inventory_open());
    assert!(
        !game.pause_menu().is_open(),
        "that Escape only closed the inventory"
    );
    assert!(!game.is_paused());
    game.press_escape();
    assert!(game.pause_menu().is_open());
}

#[test]
fn the_inventory_cannot_open_over_the_menu() {
    let mut game = test_game();
    game.press_escape();
    game.toggle_inventory();
    assert!(!game.inventory_open());
    assert!(game.pause_menu().is_open());
    // Closing the menu leaves the inventory shut, and the key works again.
    game.press_escape();
    assert!(!game.is_paused());
    game.toggle_inventory();
    assert!(game.inventory_open());
    assert!(game.is_paused());
}

#[test]
fn settings_and_back_keep_the_world_stopped_and_escape_steps_back_one_screen() {
    let mut game = test_game();
    let player = game.player.clone();
    game.press_escape();
    game.pause_step(1);
    assert_eq!(game.pause_menu().selected_item(), Some(PauseItem::Settings));
    assert_eq!(game.pause_activate_selected(), PauseOutcome::Continue);
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Settings));
    game.update(busy_input(), 1.0);
    assert_eq!(game.player, player);

    game.press_escape();
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Main));
    assert!(game.is_paused());
    assert_eq!(
        game.pause_activate(PauseItem::Settings),
        PauseOutcome::Continue
    );
    assert_eq!(game.pause_activate(PauseItem::Back), PauseOutcome::Continue);
    assert_eq!(game.pause_menu().screen(), Some(PauseScreen::Main));
    game.update(busy_input(), 1.0);
    assert_eq!(game.player, player);

    game.press_escape();
    assert!(!game.is_paused());
}

#[test]
fn exit_is_reported_to_the_front_end_and_the_hover_picks_the_button() {
    let mut game = test_game();
    game.press_escape();
    game.pause_hover(PauseItem::Exit);
    assert_eq!(game.pause_menu().selected_item(), Some(PauseItem::Exit));
    assert_eq!(game.pause_activate_selected(), PauseOutcome::Exit);
    // The game itself does not end; the front end decides what Exit means.
    assert!(game.is_paused());
    // Presses on a closed menu do nothing.
    let mut closed = test_game();
    assert_eq!(
        closed.pause_activate(PauseItem::Exit),
        PauseOutcome::Continue
    );
    assert_eq!(closed.pause_activate_selected(), PauseOutcome::Continue);
    closed.pause_step(1);
    assert!(!closed.is_paused());
}
