//! A charm lying on a room's floor is out of reach at the start, can be reached by walking, and
//! picking it up makes it owned and takes it out of the room. Built on a private copy of the
//! fixtures (the editor places the charm and saves), never on the shipped rooms.
mod support;

use hollow_steps::editor::Placement;
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::FLOOR_Y;
use hollow_steps::room_charms::CHARM_SIZE;
use macroquad::math::Vec2;
use support::player_assets::Assets;

/// The player starts at x = 180, so a charm here takes a walk to reach.
const WARD_X: f32 = 400.0;

#[test]
fn a_charm_in_the_first_room_is_reached_by_walking_and_picked_up() {
    let assets = Assets::new(false);
    {
        let mut editing = assets.editing();
        editing
            .editor_mut()
            .unwrap()
            .place(
                Placement::Charm("ward".into()),
                Vec2::new(WARD_X, FLOOR_Y - CHARM_SIZE.y / 2.0),
            )
            .unwrap();
        editing.save_editor().unwrap();
    }

    let mut game = GameState::try_new(&assets.0).unwrap();
    assert_eq!(
        game.room_charms().len(),
        1,
        "one charm lies in the first room"
    );
    assert!(
        !game.charm_state().is_owned("ward"),
        "it is not owned at the start"
    );
    assert!(
        !game.room_charms()[0].in_reach,
        "it is out of reach where the player starts"
    );

    for _ in 0..900 {
        if game.room_charms()[0].in_reach {
            break;
        }
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            1.0 / 60.0,
        );
    }
    assert!(game.room_charms()[0].in_reach, "walking right reaches it");
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert!(game.charm_state().is_owned("ward"));
    assert!(game.room_charms().is_empty());
}
