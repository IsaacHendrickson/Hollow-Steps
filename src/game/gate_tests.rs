//! Abilities and keys through `GameState`: what a run keeps and loses. The world is the scratch
//! copy of `tests/fixtures` with a hub that `run_tests` builds; the first room holds an Amber
//! Shard at x = 300 and the Dash at x = 500.
use super::run_tests::{game_in, input, kill, start_run, world};
use super::*;
use crate::ability::Ability;
use crate::item::Item;

const DT: f32 = 1.0 / 128.0;
const DASH_X: f32 = 500.0;

/// Stands the player on the floor at `x`, settles, and presses the pickup key.
fn pick_up_at(game: &mut GameState, x: f32) {
    game.player.place_on_floor(x, game.world_bounds());
    game.update(input(), DT);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..input()
        },
        0.0,
    );
}

#[test]
fn picking_up_the_dash_in_a_room_grants_the_ability() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    assert!(!game.player().abilities().has(Ability::Dash));

    pick_up_at(&mut game, DASH_X);

    assert!(game.player().abilities().has(Ability::Dash));
}

#[test]
fn the_abilities_are_lost_when_a_run_ends_and_a_new_run_starts_without_them() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    start_run(&mut game);
    pick_up_at(&mut game, DASH_X);
    assert!(game.player().abilities().has(Ability::Dash));

    kill(&mut game);
    assert!(game.run_result().is_some());
    game.continue_after_result();
    assert!(
        game.player().abilities().is_empty(),
        "back in the hub with nothing"
    );

    start_run(&mut game);
    assert!(game.player().abilities().is_empty());
    assert!(game.player().collected_items().is_empty());
    assert!(
        game.items().iter().any(|item| item.item == Item::Dash),
        "the Dash lies in its room again"
    );
}

#[test]
fn dying_with_no_hub_keeps_the_abilities_as_it_keeps_the_items() {
    let scratch = super::charm_tests::Scratch::new();
    let mut game = GameState::try_new(&scratch.0).expect("fixed test assets must load");
    for room in &mut game.rooms {
        room.enemies.clear();
    }
    pick_up_at(&mut game, DASH_X);
    assert!(game.player().abilities().has(Ability::Dash));

    kill(&mut game);

    assert!(game.run_phase().is_none(), "no hub: no run");
    assert!(game.player().abilities().has(Ability::Dash));
}
