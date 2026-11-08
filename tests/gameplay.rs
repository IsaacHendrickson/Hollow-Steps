mod support;

use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::RoomId;

#[test]
fn player_can_move_and_jump_in_sequence() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    let starting_x = game.player().position().x;

    game.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        0.25,
    );
    let moved_x = game.player().position().x;

    game.update(
        PlayerInput {
            move_axis: 0.0,
            jump_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );

    assert!(moved_x > starting_x);
    assert!(!game.player().is_grounded());
    assert!(game.player().velocity().y < 0.0);
}

#[test]
fn player_stays_within_the_active_room_horizontally() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();

    game.update(
        PlayerInput {
            move_axis: -1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.player().position().x, 16.0);
    assert_eq!(game.room().id, RoomId::First);

    game.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Second);
    assert!(game.player().position().x >= game.world_bounds().left + game.player().size().x / 2.0);
    assert!(game.player().position().x <= game.world_bounds().right - game.player().size().x / 2.0);

    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    // The second room's former right wall now leads into the boss arena.
    assert_eq!(game.room().id, RoomId::Boss);
    assert!(game.player().position().x >= game.world_bounds().left + game.player().size().x / 2.0);
    assert!(game.player().position().x <= game.world_bounds().right - game.player().size().x / 2.0);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Boss);
    assert_eq!(game.player().position().x, 944.0);
}
