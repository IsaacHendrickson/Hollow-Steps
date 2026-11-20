//! Doors in play: Up in front of one travels through it, nothing else does, and the prompt
//! (`door_in_reach`) shows exactly when Up would work. The first fixture room is 960 wide with
//! its floor at y = 460; the door stands on that floor, x from 380 to 428.
use super::*;
use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::room::Exit;
use macroquad::math::vec2;

const DT: f32 = 1.0 / 128.0;
const DOOR: Rect = Rect {
    x: 380.0,
    y: 388.0,
    w: 48.0,
    h: 72.0,
};
const ARRIVAL: Vec2 = Vec2::new(200.0, 376.0);

fn up() -> PlayerInput {
    PlayerInput {
        pickup_pressed: true,
        ..PlayerInput::default()
    }
}

/// The fixture rooms with no enemies or coins, and a door in the first room to the second.
fn game_with_door() -> GameState {
    let mut game =
        GameState::try_new(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
            .expect("fixed test assets must load");
    for room in &mut game.rooms {
        room.enemies.clear();
        room.coins.clear();
    }
    game.rooms[0]
        .exits
        .push(Exit::doorway(DOOR, RoomId::Second, ARRIVAL));
    game
}

/// Stands the player on the floor at `x` and lets one tick settle.
fn stand_at(game: &mut GameState, x: f32) {
    game.player.place_on_floor(x, game.world_bounds());
    game.update(PlayerInput::default(), DT);
}

#[test]
fn up_in_front_of_a_door_goes_through_it() {
    let mut game = game_with_door();
    stand_at(&mut game, 404.0);
    assert!(game.door_in_reach().is_some());

    game.update(up(), DT);

    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.player.position(), ARRIVAL);
}

#[test]
fn walking_into_a_door_does_not_open_it() {
    let mut game = game_with_door();
    stand_at(&mut game, 404.0);
    for _ in 0..40 {
        game.update(PlayerInput::default(), DT);
    }
    assert_eq!(game.room().id, RoomId::First);
}

#[test]
fn up_beside_a_door_does_nothing() {
    let mut game = game_with_door();
    // The body touches the door at 372 and 436, but the player's center is outside its width.
    for x in [372.0, 436.0, 100.0] {
        stand_at(&mut game, x);
        assert!(game.door_in_reach().is_none(), "x = {x}");
        game.update(up(), DT);
        assert_eq!(game.room().id, RoomId::First, "x = {x}");
    }
}

#[test]
fn the_door_edges_count_as_in_front_of_it() {
    let mut game = game_with_door();
    for x in [DOOR.x, DOOR.right()] {
        stand_at(&mut game, x);
        assert!(game.door_in_reach().is_some(), "x = {x}");
    }
}

#[test]
fn a_door_cannot_be_used_in_the_air() {
    let mut game = game_with_door();
    // Hanging 16 px above the floor, inside the door's rectangle.
    game.player
        .place_at(vec2(404.0, 420.0), game.world_bounds());
    assert!(!game.player.is_grounded());
    let (position, size) = (game.player.position(), game.player.size());
    assert!(
        game.room().door_at(position, size).is_some(),
        "the body is at the door, so only being airborne can stop it"
    );
    assert!(game.door_in_reach().is_none());
    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::First);
}

#[test]
fn up_with_the_attack_key_strikes_upward_and_does_not_open_the_door() {
    let mut game = game_with_door();
    stand_at(&mut game, 404.0);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            attack_pressed: true,
            up_held: true,
            ..PlayerInput::default()
        },
        DT,
    );
    assert_eq!(game.room().id, RoomId::First);
}

#[test]
fn an_item_in_reach_takes_the_up_press_before_the_door_and_the_prompt_waits() {
    let mut game = game_with_door();
    game.rooms[0].items.push(WorldItem {
        item: Item::AmberShard,
        position: vec2(404.0, 460.0 - ITEM_SIZE.y / 2.0),
    });
    stand_at(&mut game, 404.0);
    let before = game.player.collected_items().len();
    assert!(
        game.door_in_reach().is_none(),
        "no \"Up: enter\" while Up would pick the item up"
    );

    game.update(up(), DT);
    assert_eq!(
        game.room().id,
        RoomId::First,
        "the item was picked up instead"
    );
    assert_eq!(game.player.collected_items().len(), before + 1);
    assert!(
        game.door_in_reach().is_some(),
        "the door's prompt appears once it is taken"
    );

    game.update(up(), DT);
    assert_eq!(
        game.room().id,
        RoomId::Second,
        "the next press opens the door"
    );
}

#[test]
fn a_charm_in_reach_also_makes_the_door_wait() {
    let mut game = game_with_door();
    let id = game.charms.charms()[0].id.clone();
    game.rooms[0].charms.push(crate::room_charms::WorldCharm {
        charm: id,
        position: vec2(404.0, 440.0),
    });
    stand_at(&mut game, 404.0);
    assert!(game.charm_in_reach().is_some(), "the charm is within reach");
    assert!(game.door_in_reach().is_none());
    game.update(up(), DT);
    assert_eq!(
        game.room().id,
        RoomId::First,
        "the charm was taken, not the door"
    );
    assert!(game.door_in_reach().is_some());
}

#[test]
fn doors_stay_shut_while_a_boss_holds_the_exits_and_open_when_it_falls() {
    let mut game = game_with_door();
    game.current = 2;
    game.rooms[2]
        .exits
        .push(Exit::doorway(DOOR, RoomId::First, vec2(100.0, 436.0)));
    assert!(game.rooms[2].exits_locked(), "the fixture boss is alive");
    stand_at(&mut game, 404.0);
    assert!(game.door_in_reach().is_none());
    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::Boss);

    game.rooms[2]
        .boss_special
        .as_mut()
        .unwrap()
        .damage_boss(100_000);
    assert!(game.door_in_reach().is_some());
    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::First);
}

#[test]
fn a_test_room_is_a_sandbox_so_its_doors_do_nothing() {
    let mut game = game_with_door();
    game.edit_room(RoomId::First).unwrap();
    let draft = &mut game.editor_mut().unwrap().draft;
    draft.add_door(RoomId::Second, ARRIVAL).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    let door = game.room().exits.iter().find_map(|exit| exit.door).unwrap();
    stand_at(&mut game, door.center().x);
    assert!(game.door_in_reach().is_none());
    game.update(up(), DT);
    assert!(game.testing_room());
    assert_eq!(game.room().id, RoomId::First);
}

#[test]
fn the_prompt_shows_exactly_where_up_would_work() {
    let mut game = game_with_door();
    let mut x = 300.0;
    while x <= 520.0 {
        stand_at(&mut game, x);
        let prompt = game.door_in_reach().is_some();
        game.update(up(), DT);
        assert_eq!(
            prompt,
            game.room().id == RoomId::Second,
            "x = {x}: prompt and travel disagree"
        );
        game.current = 0;
        x += 4.0;
    }
}

#[test]
fn edge_exits_still_work_by_walking_into_them() {
    let mut game = game_with_door();
    let right_edge = game.world_bounds().right;
    stand_at(&mut game, right_edge - 20.0);
    game.update(PlayerInput::default(), DT);
    assert_eq!(game.room().id, RoomId::Second);
}

#[test]
fn the_prompt_and_travel_agree_with_an_item_lying_in_front_of_the_door() {
    let mut x = 300.0;
    while x <= 520.0 {
        // A fresh room each time: the press at the item would otherwise take it for good.
        let mut game = game_with_door();
        game.rooms[0].items.push(WorldItem {
            item: Item::AmberShard,
            position: vec2(404.0, 460.0 - ITEM_SIZE.y / 2.0),
        });
        stand_at(&mut game, x);
        let prompt = game.door_in_reach().is_some();
        game.update(up(), DT);
        assert_eq!(
            prompt,
            game.room().id == RoomId::Second,
            "x = {x}: prompt and travel disagree"
        );
        x += 4.0;
    }
}
