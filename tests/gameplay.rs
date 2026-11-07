use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;

#[test]
fn player_can_move_and_jump_in_sequence() {
    let mut game = GameState::new();
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
fn player_cannot_leave_the_room_horizontally() {
    let mut game = GameState::new();

    game.update(
        PlayerInput {
            move_axis: -1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.player().position().x, 16.0);

    game.update(
        PlayerInput {
            move_axis: 1.0,
            jump_pressed: false,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.player().position().x, 944.0);
}
