//! Keys and locked exits through `GameState`. The world is a scratch copy of `tests/fixtures`:
//! the first room is 960 wide with its floor at y = 460 and a right-edge exit (x from 928, to the
//! second room); the second room's left exit (x up to 112) leads back. Locks are put on the live
//! exits, or, where a rebuilt room has to have one, into the room's file through the editor.
use macroquad::math::{Rect, Vec2};

use super::super::charm_tests::Scratch;
use super::super::run_tests::{self, HUB};
use super::super::*;
use crate::editor::{ItemPlacement, RoomLayout};
use crate::item::Item;
use crate::room::Exit;

const DT: f32 = 1.0 / 128.0;
const KEY: u8 = 3;
/// Inside the first room's right-edge exit.
const EXIT_X: f32 = 940.0;
/// Inside the second room's left-edge exit, which leads back to the first room.
const BACK_X: f32 = 100.0;
const DOOR: Rect = Rect {
    x: 380.0,
    y: 388.0,
    w: 48.0,
    h: 72.0,
};
const ARRIVAL: Vec2 = Vec2::new(200.0, 376.0);

fn input() -> PlayerInput {
    PlayerInput::default()
}

fn up() -> PlayerInput {
    PlayerInput {
        pickup_pressed: true,
        ..input()
    }
}

fn clear_rooms(game: &mut GameState) {
    for room in &mut game.rooms {
        room.enemies.clear();
        room.coins.clear();
    }
}

fn game_in(scratch: &Scratch) -> GameState {
    let mut game = GameState::try_new(&scratch.0).expect("fixed test assets must load");
    clear_rooms(&mut game);
    game
}

fn room_of(game: &GameState, id: RoomId) -> &Room {
    game.rooms.iter().find(|room| room.id == id).unwrap()
}

/// Plays in the room with `id`. A Save rebuilds the room list in file order, so an id is not an
/// index, and the position is looked up each time.
fn go_to(game: &mut GameState, id: RoomId) {
    game.current = game.rooms.iter().position(|room| room.id == id).unwrap();
}

fn has_key(game: &GameState) -> bool {
    game.player.collected_items().contains(&Item::Key(KEY))
}

/// Locks the first room's right-edge exit with `KEY` (the live room only).
fn lock_first_exit(game: &mut GameState) {
    game.rooms[RoomId::First.index()].exits[0].locked_by = Some(KEY);
}

/// Stands the player on the floor at `x` and lets one tick pass.
fn stand_at(game: &mut GameState, x: f32) {
    game.player.place_on_floor(x, game.world_bounds());
    game.update(input(), DT);
}

/// F2 on `id`, `edit` the draft's layout, Save, F2: the way an author changes a room's file.
fn edit_and_save(game: &mut GameState, id: RoomId, edit: impl FnOnce(&mut RoomLayout)) {
    game.edit_room(id).unwrap();
    edit(&mut game.editor_mut().unwrap().draft.layout);
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
}

/// F2 on `id` and Save without a change, F2: the room is rebuilt from its file.
fn save_unchanged(game: &mut GameState, id: RoomId) {
    edit_and_save(game, id, |_| {});
}

fn lock_in_file(layout: &mut RoomLayout) {
    layout.exits[0].locked_by = Some(KEY);
}

fn key_at(x: f32) -> ItemPlacement {
    ItemPlacement {
        item: Item::Key(KEY),
        position: (x, 450.0),
    }
}

// ---- in play ----

#[test]
fn a_locked_exit_blocks_without_its_key_and_says_what_is_needed() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    assert_eq!(game.notice(), None);

    stand_at(&mut game, EXIT_X);

    assert_eq!(game.room().id, RoomId::First, "the exit did not open");
    assert_eq!(game.notice(), Some("Locked: needs Key 3"));
    assert_eq!(game.room().exits[0].locked_by, Some(KEY));
}

#[test]
fn the_notice_lasts_about_two_seconds_after_the_player_walks_away() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    stand_at(&mut game, EXIT_X);
    stand_at(&mut game, 400.0);
    assert!(game.notice().is_some(), "still showing right after");

    game.update(input(), 1.9);
    assert!(game.notice().is_some());
    game.update(input(), 0.2);
    assert_eq!(game.notice(), None);
}

#[test]
fn the_notice_does_not_run_down_while_the_game_is_paused() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    stand_at(&mut game, EXIT_X);
    stand_at(&mut game, 400.0);

    game.toggle_editor().unwrap();
    game.update(input(), 5.0);
    game.toggle_editor().unwrap();

    assert!(
        game.notice().is_some(),
        "nothing advances behind the editor"
    );
}

#[test]
fn the_key_opens_the_exit_is_used_up_and_the_player_goes_through() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    game.player.collect_item(Item::AmberShard);
    game.player.collect_item(Item::Key(KEY));
    game.player.collect_item(Item::Key(KEY + 1));

    stand_at(&mut game, EXIT_X);

    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(
        game.player.collected_items(),
        &[Item::AmberShard, Item::Key(KEY + 1)],
        "exactly the one key that fits was used up"
    );
    assert_eq!(room_of(&game, RoomId::First).exits[0].locked_by, None);
    assert_eq!(game.notice(), None);
}

#[test]
fn the_door_stays_open_after_leaving_and_coming_back() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    game.player.collect_item(Item::Key(KEY));
    stand_at(&mut game, EXIT_X);
    assert_eq!(game.room().id, RoomId::Second);

    stand_at(&mut game, BACK_X);
    assert_eq!(game.room().id, RoomId::First, "back in the first room");
    stand_at(&mut game, EXIT_X);

    assert_eq!(game.room().id, RoomId::Second, "through again with no key");
    assert!(!has_key(&game));
    assert_eq!(game.notice(), None);
}

#[test]
fn a_door_exit_needs_its_key_too_and_opens_with_the_up_key() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    let mut door = Exit::doorway(DOOR, RoomId::Second, ARRIVAL);
    door.locked_by = Some(KEY);
    game.rooms[0].exits.push(door);

    stand_at(&mut game, 404.0);
    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::First, "no key, no entry");
    assert_eq!(game.notice(), Some("Locked: needs Key 3"));

    game.player.collect_item(Item::Key(KEY));
    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(game.player.position(), ARRIVAL);
    assert!(!has_key(&game));
    assert!(room_of(&game, RoomId::First)
        .exits
        .iter()
        .all(|exit| exit.locked_by.is_none()));
}

#[test]
fn a_second_door_with_the_same_key_id_stays_locked() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    let mut door = Exit::doorway(DOOR, RoomId::Second, ARRIVAL);
    door.locked_by = Some(KEY);
    game.rooms[0].exits.push(door);
    game.player.collect_item(Item::Key(KEY));

    stand_at(&mut game, EXIT_X);
    assert_eq!(game.room().id, RoomId::Second, "the edge exit took the key");
    stand_at(&mut game, BACK_X);
    assert_eq!(game.room().id, RoomId::First);

    stand_at(&mut game, 404.0);
    game.update(up(), DT);
    assert_eq!(game.room().id, RoomId::First, "one key, one door");
    assert_eq!(game.notice(), Some("Locked: needs Key 3"));
    assert_eq!(game.room().exits[1].locked_by, Some(KEY));
}

#[test]
fn a_key_of_another_number_does_not_open_the_exit() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    lock_first_exit(&mut game);
    game.player.collect_item(Item::Key(KEY + 1));

    stand_at(&mut game, EXIT_X);

    assert_eq!(game.room().id, RoomId::First);
    assert!(game.player.collected_items().contains(&Item::Key(KEY + 1)));
    assert_eq!(game.notice(), Some("Locked: needs Key 3"));
}

#[test]
fn the_boss_lock_still_wins_and_spends_no_key() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    let boss_room = RoomId::Boss.index();
    game.rooms[boss_room].exits[0].locked_by = Some(KEY);
    game.current = boss_room;
    game.player.collect_item(Item::Key(KEY));
    assert!(game.room().exits_locked(), "the fixture boss is alive");

    stand_at(&mut game, 16.0);
    assert_eq!(game.room().id, RoomId::Boss);
    assert!(
        has_key(&game),
        "the boss lock held, so the key was not touched"
    );
    assert_eq!(game.notice(), None);
    assert_eq!(game.room().exits[0].locked_by, Some(KEY));

    let boss = game.rooms[boss_room].boss_special.as_mut().unwrap();
    boss.damage_boss(boss.health());
    stand_at(&mut game, 16.0);
    assert_eq!(
        game.room().id,
        RoomId::Second,
        "now the key lock applies and opens"
    );
    assert!(!has_key(&game));
}

#[test]
fn an_exit_to_a_room_that_does_not_exist_does_not_spend_the_key() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    game.rooms[0].exits[0].locked_by = Some(KEY);
    game.rooms[0].exits[0].target_room = RoomId::Custom(9);
    game.player.collect_item(Item::Key(KEY));

    stand_at(&mut game, EXIT_X);

    assert_eq!(game.room().id, RoomId::First);
    assert!(has_key(&game));
    assert_eq!(game.room().exits[0].locked_by, Some(KEY));
}

#[test]
fn a_key_is_picked_up_like_any_item_and_stays_gone_after_a_save() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    edit_and_save(&mut game, RoomId::First, |layout| {
        layout.items.push(key_at(200.0))
    });
    clear_rooms(&mut game);
    go_to(&mut game, RoomId::First);
    assert!(game.items().iter().any(|item| item.item == Item::Key(KEY)));

    stand_at(&mut game, 200.0);
    game.update(up(), DT);
    assert!(has_key(&game));
    assert_eq!(game.taken_items.len(), 1);
    assert!(!game.items().iter().any(|item| item.item == Item::Key(KEY)));

    save_unchanged(&mut game, RoomId::First);
    assert!(
        !room_of(&game, RoomId::First)
            .items
            .iter()
            .any(|item| item.item == Item::Key(KEY)),
        "the taken key does not come back with the rebuilt room"
    );
}

// ---- the editor's Save ----

#[test]
fn an_editor_save_keeps_an_opened_door_open_and_does_not_write_it_to_the_file() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    edit_and_save(&mut game, RoomId::First, lock_in_file);
    clear_rooms(&mut game);
    assert_eq!(
        room_of(&game, RoomId::First).exits[0].locked_by,
        Some(KEY),
        "the lock is in the room file now"
    );
    game.player.collect_item(Item::Key(KEY));
    go_to(&mut game, RoomId::First);
    stand_at(&mut game, EXIT_X);
    assert_eq!(game.room().id, RoomId::Second);
    assert_eq!(room_of(&game, RoomId::First).exits[0].locked_by, None);

    save_unchanged(&mut game, RoomId::First);

    assert_eq!(
        room_of(&game, RoomId::First).exits[0].locked_by,
        None,
        "the rebuilt room has its opened door open again"
    );
    let file = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();
    assert!(
        file.contains("locked_by"),
        "opening a door in play never changes the room's file: {file}"
    );
}

#[test]
fn a_save_that_changes_the_lock_forgets_that_the_door_was_opened() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    edit_and_save(&mut game, RoomId::First, lock_in_file);
    clear_rooms(&mut game);
    game.player.collect_item(Item::Key(KEY));
    go_to(&mut game, RoomId::First);
    stand_at(&mut game, EXIT_X);
    assert_eq!(room_of(&game, RoomId::First).exits[0].locked_by, None);

    edit_and_save(&mut game, RoomId::First, |layout| {
        layout.exits[0].locked_by = Some(KEY + 1);
    });
    assert_eq!(
        room_of(&game, RoomId::First).exits[0].locked_by,
        Some(KEY + 1),
        "a new lock is a new lock"
    );
    assert!(game.locks.opened.is_empty());
}

// ---- test rooms and runs ----

#[test]
fn a_test_room_is_a_sandbox_for_keys_and_doors() {
    let scratch = Scratch::new();
    let mut game = game_in(&scratch);
    game.edit_room(RoomId::First).unwrap();
    {
        let layout = &mut game.editor_mut().unwrap().draft.layout;
        lock_in_file(layout);
        layout.items.push(key_at(200.0));
    }
    game.start_test_room().unwrap();
    assert!(game.testing_room());

    // A key can be picked up there, but the exits do nothing in a test room.
    stand_at(&mut game, 200.0);
    game.update(up(), DT);
    assert!(has_key(&game));
    stand_at(&mut game, EXIT_X);
    assert!(game.testing_room());
    assert_eq!(game.notice(), None);
    assert!(has_key(&game), "no key is used in a test room");
    assert!(game.taken_items.is_empty());
    assert!(game.locks.opened.is_empty());

    game.end_test_room().unwrap();

    assert!(!has_key(&game), "the key found there is not kept");
    assert!(game.locks.opened.is_empty());
    assert!(game.taken_items.is_empty());
    assert_eq!(room_of(&game, RoomId::First).exits[0].locked_by, None);
}

#[test]
fn a_run_reset_relocks_every_door_and_puts_the_key_back_in_its_room() {
    let scratch = run_tests::world(100.0);
    let mut game = run_tests::game_in(&scratch);
    edit_and_save(&mut game, RoomId::First, |layout| {
        lock_in_file(layout);
        layout.items.push(key_at(200.0));
    });
    // The editor put the player outside the hub; the next tick counts that as walking out.
    game.update(input(), DT);
    assert!(game.run_in_progress());
    clear_rooms(&mut game);
    go_to(&mut game, RoomId::First);

    stand_at(&mut game, 200.0);
    game.update(up(), DT);
    assert!(has_key(&game));
    stand_at(&mut game, EXIT_X);
    assert_eq!(game.room().id, RoomId::Second, "the key opened the door");
    assert_eq!(room_of(&game, RoomId::First).exits[0].locked_by, None);

    run_tests::kill(&mut game);
    assert!(game.run_result().is_some());
    game.continue_after_result();
    assert_eq!(game.room().id, HUB);
    assert_eq!(game.notice(), None);
    run_tests::start_run(&mut game);

    assert_eq!(
        room_of(&game, RoomId::First).exits[0].locked_by,
        Some(KEY),
        "the door is locked again"
    );
    assert!(game.locks.opened.is_empty());
    assert!(!has_key(&game));
    assert!(
        game.items().iter().any(|item| item.item == Item::Key(KEY)),
        "the key lies in its room again"
    );
}
