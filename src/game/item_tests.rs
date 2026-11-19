//! Items picked up during a run stay gone when a room is rebuilt from its file (the editor's
//! Save), like coins and charms do. The first fixture room holds an Amber Shard at x = 300 and
//! the Dash at x = 500; both rooms are rebuilt from files that place them again.
use super::charm_tests::Scratch;
use super::*;
use crate::editor::ItemPlacement;
use crate::item::{Item, ITEM_SIZE};

const DT: f32 = 1.0 / 128.0;
const SHARD_X: f32 = 300.0;
const DASH_X: f32 = 500.0;

fn game_in(scratch: &Scratch) -> GameState {
    let mut game = GameState::try_new(&scratch.0).expect("fixed test assets must load");
    for room in &mut game.rooms {
        room.enemies.clear();
        room.coins.clear();
    }
    game
}

/// The kinds of the items lying in the room being played, in order.
fn kinds(game: &GameState) -> Vec<Item> {
    game.items().iter().map(|item| item.item).collect()
}

/// Stands the player on the floor at `x`, settles, and presses the pickup key.
fn pick_up_at(game: &mut GameState, x: f32) {
    game.player.place_on_floor(x, game.world_bounds());
    game.update(PlayerInput::default(), DT);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
}

fn collected(game: &GameState) -> Vec<Item> {
    game.player.collected_items().to_vec()
}

/// F2, change the draft with `edit`, Save, F2: the way an author edits a room mid-run.
fn edit_and_save(game: &mut GameState, id: RoomId, edit: impl FnOnce(&mut Vec<ItemPlacement>)) {
    game.edit_room(id).unwrap();
    edit(&mut game.editor_mut().unwrap().draft.layout.items);
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
}

/// The room with `id`; a Save rebuilds the room list in file order, so an id is not an index.
fn room_of(game: &GameState, id: RoomId) -> &Room {
    game.rooms.iter().find(|room| room.id == id).unwrap()
}

fn floor_y(game: &GameState) -> f32 {
    game.world_bounds().floor_y - ITEM_SIZE.y / 2.0
}

#[test]
fn a_taken_item_stays_gone_after_an_editor_save_and_cannot_be_taken_again() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    pick_up_at(&mut game, SHARD_X);
    assert_eq!(kinds(&game), [Item::Dash]);
    assert_eq!(collected(&game), [Item::AmberShard]);

    game.toggle_editor().unwrap();
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();

    assert_eq!(kinds(&game), [Item::Dash], "the shard is still taken");
    pick_up_at(&mut game, SHARD_X);
    assert_eq!(
        collected(&game),
        [Item::AmberShard],
        "the Items count is unchanged"
    );
}

#[test]
fn dash_stays_collected_and_both_items_stay_gone() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    pick_up_at(&mut game, SHARD_X);
    pick_up_at(&mut game, DASH_X);
    assert_eq!(collected(&game), [Item::AmberShard, Item::Dash]);

    game.toggle_editor().unwrap();
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();

    assert!(game.items().is_empty());
    pick_up_at(&mut game, DASH_X);
    pick_up_at(&mut game, SHARD_X);
    assert_eq!(collected(&game), [Item::AmberShard, Item::Dash]);
}

#[test]
fn a_moved_item_is_new_and_comes_back_once_at_its_new_position() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    pick_up_at(&mut game, SHARD_X);
    let y = floor_y(&game);

    edit_and_save(&mut game, RoomId::First, |items| {
        let shard = items
            .iter_mut()
            .find(|placed| placed.item == Item::AmberShard)
            .unwrap();
        shard.position = (340.0, y);
    });

    assert_eq!(kinds(&game), [Item::AmberShard, Item::Dash]);
    assert_eq!(game.items()[0].position.x, 340.0, "once, at the new spot");
    assert_eq!(
        collected(&game),
        [Item::AmberShard],
        "nothing was given back"
    );
    pick_up_at(&mut game, 340.0);
    assert_eq!(collected(&game), [Item::AmberShard, Item::AmberShard]);
    assert_eq!(kinds(&game), [Item::Dash]);

    // Put back where the first one lay, it is another new item, not the old one.
    edit_and_save(&mut game, RoomId::First, |items| {
        items[0].position = (SHARD_X, y);
    });
    assert_eq!(kinds(&game), [Item::AmberShard, Item::Dash]);
    assert_eq!(game.items()[0].position.x, SHARD_X);
}

#[test]
fn an_item_added_in_the_editor_appears_next_to_the_taken_ones() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    pick_up_at(&mut game, SHARD_X);
    let y = floor_y(&game);

    edit_and_save(&mut game, RoomId::First, |items| {
        items.push(ItemPlacement {
            item: Item::AmberShard,
            position: (380.0, y),
        });
    });

    assert_eq!(kinds(&game), [Item::Dash, Item::AmberShard]);
    assert_eq!(game.items()[1].position.x, 380.0);
}

#[test]
fn identical_items_are_taken_one_at_a_time() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    let y = floor_y(&game);
    // Two shards on the very same spot: taking one leaves the other, also after a Save.
    edit_and_save(&mut game, RoomId::First, |items| {
        items.push(ItemPlacement {
            item: Item::AmberShard,
            position: (SHARD_X, y),
        });
    });
    assert_eq!(
        kinds(&game),
        [Item::AmberShard, Item::Dash, Item::AmberShard]
    );
    pick_up_at(&mut game, SHARD_X);
    assert_eq!(kinds(&game), [Item::Dash, Item::AmberShard]);

    game.toggle_editor().unwrap();
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();

    assert_eq!(
        kinds(&game),
        [Item::Dash, Item::AmberShard],
        "one left, not two"
    );
    assert_eq!(game.items()[1].position.x, SHARD_X);
}

#[test]
fn saving_one_room_leaves_the_items_of_every_other_room_alone() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    let second = RoomId::Second;
    pick_up_at(&mut game, SHARD_X);

    // The second room gets two shards of its own through the editor; the player takes one.
    game.edit_room(second).unwrap();
    let y = floor_y(&game);
    game.editor_mut().unwrap().draft.layout.items = vec![
        ItemPlacement {
            item: Item::AmberShard,
            position: (300.0, y),
        },
        ItemPlacement {
            item: Item::AmberShard,
            position: (360.0, y),
        },
    ];
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    assert_eq!(game.room().id, second);
    pick_up_at(&mut game, 300.0);
    assert_eq!(game.items().len(), 1);
    assert_eq!(game.items()[0].position.x, 360.0);

    // Saving the first room: its shard stays taken, the Dash is there, the second room is as it was.
    edit_and_save(&mut game, RoomId::First, |_| {});
    let first = room_of(&game, RoomId::First);
    assert_eq!(
        first.items.iter().map(|i| i.item).collect::<Vec<_>>(),
        [Item::Dash]
    );
    let other = room_of(&game, second);
    assert_eq!(other.items.len(), 1);
    assert_eq!(other.items[0].position.x, 360.0);

    // And the other way round: saving the second room keeps its taken shard gone and the first
    // room's items as they were.
    edit_and_save(&mut game, second, |_| {});
    let other = room_of(&game, second);
    assert_eq!(other.items.len(), 1);
    assert_eq!(other.items[0].position.x, 360.0);
    assert_eq!(room_of(&game, RoomId::First).items.len(), 1);
}

#[test]
fn an_item_taken_in_a_test_room_is_back_after_it_ends_and_after_a_save() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    pick_up_at(&mut game, SHARD_X);
    assert_eq!(collected(&game), [Item::AmberShard]);
    assert_eq!(kinds(&game), [Item::Dash], "gone inside the test room");

    game.end_test_room().unwrap();

    assert!(collected(&game).is_empty(), "the run's inventory is back");
    assert_eq!(
        kinds(&game),
        [Item::AmberShard, Item::Dash],
        "the saved room kept it"
    );
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    assert_eq!(
        kinds(&game),
        [Item::AmberShard, Item::Dash],
        "nothing was remembered"
    );
}

#[test]
fn room_travel_respawn_and_reload_keep_taken_items_gone() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    pick_up_at(&mut game, SHARD_X);

    // Through the exit to the second room and back.
    for _ in 0..2 {
        let (zone, bounds) = {
            let room = game.room();
            (room.exits[0].zone(room.bounds), room.world_bounds())
        };
        game.player.place_at(zone.center(), bounds);
        game.update(PlayerInput::default(), DT);
    }
    assert_eq!(game.room().id, RoomId::First);
    assert_eq!(kinds(&game), [Item::Dash]);

    // Death sends the player back to the spawn; the world is not rebuilt.
    game.player.set_health_current(0);
    game.update(PlayerInput::default(), DT);
    assert_eq!(kinds(&game), [Item::Dash]);

    // F5 re-reads the files and keeps what the player picked up.
    game.reload_rooms().unwrap();
    assert_eq!(kinds(&game), [Item::Dash]);
    pick_up_at(&mut game, SHARD_X);
    assert_eq!(collected(&game), [Item::AmberShard]);
}
