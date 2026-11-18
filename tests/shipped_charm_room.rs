//! The one smoke test for the charm placed in the shipped first room (a manual-check aid): it is
//! there, it can be reached by walking, and picking it up makes it owned. Everything else about
//! charms in rooms is tested on fixtures.
use std::path::Path;

use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;

#[test]
fn the_shipped_first_room_holds_a_charm_to_find() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    let mut game = GameState::try_new(&assets).unwrap();
    let lying = game.room_charms();
    assert_eq!(lying.len(), 1, "one charm lies in the first room");
    assert!(
        !game.charm_state().is_owned("ward"),
        "it is not owned at the start"
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
