use hollow_steps::{game::GameState, item::Item, player::PlayerInput};

fn near_item() -> GameState {
    let mut game = GameState::new();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        120.0 / 220.0,
    );
    game
}

fn pickup(game: &mut GameState) {
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
}

#[test]
fn nearby_item_requires_up_then_moves_into_inventory_once() {
    let mut game = near_item();
    assert!(game.items()[0].can_pick_up(game.player()));
    assert!(game.player().collected_items().is_empty());
    assert_eq!(game.items().len(), 1);
    pickup(&mut game);
    assert!(game.items().is_empty());
    assert_eq!(game.player().collected_items(), &[Item::AmberShard]);
    pickup(&mut game);
    assert_eq!(game.player().collected_items(), &[Item::AmberShard]);
}

#[test]
fn distant_item_cannot_be_collected() {
    let mut game = GameState::new();
    assert!(!game.items()[0].can_pick_up(game.player()));
    pickup(&mut game);
    assert_eq!(game.items().len(), 1);
    assert!(game.player().collected_items().is_empty());
}

#[test]
fn airborne_player_cannot_collect_even_when_close() {
    let mut game = near_item();
    game.update(
        PlayerInput {
            jump_pressed: true,
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert!(!game.items()[0].can_pick_up(game.player()));
    assert_eq!(game.items().len(), 1);
    assert!(game.player().collected_items().is_empty());
}

#[test]
fn large_step_uses_final_position_for_pickup() {
    let mut game = near_item();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.items().len(), 1);
    assert!(game.player().collected_items().is_empty());
}
