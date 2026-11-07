use hollow_steps::{game::GameState, item::Item, player::PlayerInput, room::RoomId};
use macroquad::math::Vec2;

fn walk_to_exit(game: &mut GameState, move_axis: f32) {
    let starting_room = game.room().id;
    for _ in 0..600 {
        game.update(
            PlayerInput {
                move_axis,
                ..PlayerInput::default()
            },
            1.0 / 60.0,
        );
        if game.room().id != starting_room {
            return;
        }
    }
    panic!("walking did not reach an exit");
}

#[test]
fn entering_exit_loads_target_room_and_exact_spawn() {
    let mut game = GameState::new();
    let exit = game.room().exits[0];
    let original_bounds = game.world_bounds();

    walk_to_exit(&mut game, 1.0);

    assert_eq!(game.room().id, exit.target_room);
    assert_eq!(game.player().position(), exit.spawn);
    assert_ne!(game.world_bounds(), original_bounds);
    assert_eq!(
        game.player().position().y + game.player().size().y / 2.0,
        game.room().floor_y
    );
    assert_eq!(game.player().velocity(), Vec2::ZERO);
    assert!(game.player().is_grounded());
    assert!(game.items().is_empty());
}

#[test]
fn return_exit_uses_its_own_spawn_and_arrival_does_not_bounce_back() {
    let mut game = GameState::new();
    walk_to_exit(&mut game, 1.0);
    let exit = game.room().exits[0];
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().id, RoomId::Second);

    walk_to_exit(&mut game, -1.0);

    assert_eq!(game.room().id, exit.target_room);
    assert_eq!(game.player().position(), exit.spawn);
    assert_eq!(game.player().velocity(), Vec2::ZERO);
    for _ in 0..10 {
        game.update(PlayerInput::default(), 1.0 / 60.0);
        assert_eq!(game.room().id, RoomId::First);
        assert_eq!(game.player().position(), exit.spawn);
    }
}

#[test]
fn player_away_from_exit_does_not_transition() {
    let mut game = GameState::new();
    game.update(PlayerInput::default(), 0.0);
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        0.25,
    );
    game.update(PlayerInput::default(), 10.0);

    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(game.player().position().x, 235.0);
}

#[test]
fn large_step_transitions_once_and_stops_at_destination_spawn() {
    let mut game = GameState::new();
    let exit = game.room().exits[0];

    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );

    assert_eq!(game.room().id, exit.target_room);
    assert_eq!(game.player().position(), exit.spawn);
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().id, exit.target_room);
    assert_eq!(game.player().position(), exit.spawn);
}

#[test]
fn collected_items_stay_collected_after_visiting_another_room() {
    let mut game = GameState::new();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        120.0 / 220.0,
    );
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );

    walk_to_exit(&mut game, 1.0);
    assert_eq!(game.player().collected_items(), &[Item::AmberShard]);
    walk_to_exit(&mut game, -1.0);

    assert_eq!(game.player().collected_items(), &[Item::AmberShard]);
    assert!(game.items().is_empty());
}
