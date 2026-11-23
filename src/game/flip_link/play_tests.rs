//! A flipped run through `GameState`: the world a run builds, where the player arrives, every exit
//! of the flipped dungeon, F5, the editor's refusal and the map. Whether a run flips is in
//! `flip_tests`; the mirror itself is in `crate::mirror`.
//!
//! The world is the fixture dungeon of `mirror::fixture` (hub, start room, second room, boss room,
//! tower and closet) written into a private copy of `tests/fixtures`, so the rooms have layouts
//! and can be flipped. Seeds and the F4 choice are fixed by each test.
use super::super::charm_tests::Scratch;
use super::super::run_tests::{game_in, input, kill, walk, HUB};
use super::*;
use crate::editor::RoomLayout;
use crate::editor_files::room_filename;
use crate::exit_arrival::{arrival_error, arrival_next_to};
use crate::mirror::fixture::{self, BOSS, CLOSET, SECOND, START, TOWER};
use crate::mirror::flip_world;
use crate::room::{EnemyPlacement, RoomEnemies, RoomId};
use crate::run::RunPhase;
use crate::variety::FlipChoice;

const ROOMS: [RoomId; 6] = [HUB, START, SECOND, BOSS, TOWER, CLOSET];

/// A chaser patrolling the right half of the start room, so a flip puts it in the left half.
const CHASER: &str = r#"(definition: "chase", patrol_left: 560.0, patrol_right: 800.0)"#;

fn documents(with_enemy: bool) -> Vec<(RoomId, RoomEnemies)> {
    let mut documents = fixture::dungeon(true);
    if with_enemy {
        let (_, start) = documents.iter_mut().find(|(id, _)| *id == START).unwrap();
        start.enemies = vec![ron::from_str::<EnemyPlacement>(CHASER).unwrap()];
    }
    documents
}

fn write_documents(scratch: &Scratch, documents: &[(RoomId, RoomEnemies)]) {
    for (id, document) in documents {
        let name = format!("rooms/{}", room_filename(*id));
        scratch.write(&name, &ron::to_string(document).unwrap());
    }
}

/// The fixture dungeon as the game's files, with `variety.ron` saying `percent`.
fn dungeon_world(percent: f32) -> Scratch {
    let scratch = Scratch::new();
    write_documents(&scratch, &documents(false));
    scratch.write(
        "run.ron",
        "(hub_room: Custom(0), start_room: First, final_boss_room: Boss)",
    );
    scratch.write("variety.ron", &format!("(flip_percent: {percent:?})"));
    scratch
}

/// A game on `dungeon_world` in the hub, whose next runs do `choice`.
fn game_with(scratch: &Scratch, choice: FlipChoice) -> GameState {
    let mut game = game_in(scratch);
    game.set_flip_choice(choice);
    game
}

/// Walks out of the hub: a run starts and the player is in the start room.
fn leave_the_hub(game: &mut GameState) {
    game.update(walk(1.0), 10.0);
    assert!(game.run_in_progress(), "{:?}", game.run_message());
    assert_eq!(game.room().id, START);
}

fn flip_of(documents: &[(RoomId, RoomEnemies)]) -> Vec<(RoomId, RoomEnemies)> {
    let mut flipped = documents.to_vec();
    flip_world(&mut flipped, |id| id != HUB);
    flipped
}

fn layout_in(documents: &[(RoomId, RoomEnemies)], id: RoomId) -> RoomLayout {
    let (_, document) = documents.iter().find(|(room, _)| *room == id).unwrap();
    document.layout.clone().unwrap()
}

fn live_layout(game: &GameState, id: RoomId) -> RoomLayout {
    let room = game.rooms.iter().find(|room| room.id == id).unwrap();
    RoomLayout::from_room(room)
}

fn assert_rooms_are(game: &GameState, expected: &[(RoomId, RoomEnemies)], what: &str) {
    for id in ROOMS {
        assert_eq!(
            live_layout(game, id),
            layout_in(expected, id),
            "{what}: {id:?}"
        );
    }
}

fn room_files(scratch: &Scratch) -> Vec<String> {
    ROOMS
        .iter()
        .map(|id| scratch.0.join("rooms").join(room_filename(*id)))
        .map(|path| std::fs::read_to_string(path).unwrap())
        .collect()
}

#[test]
fn the_rooms_are_built_from_the_files_as_they_are_when_nothing_flips() {
    // The comparison itself: a room built from a document gives that document's layout back.
    let scratch = dungeon_world(100.0);
    let mut game = game_with(&scratch, FlipChoice::Never);
    assert_rooms_are(&game, &documents(false), "the hub");
    leave_the_hub(&mut game);
    assert!(!game.run_flipped());
    assert!(!game.tiers.layer.flip);
    assert_rooms_are(&game, &documents(false), "an unflipped run");
}

#[test]
fn a_flipped_runs_rooms_are_flip_world_of_the_files_and_the_files_are_untouched() {
    let scratch = dungeon_world(0.0);
    let files = room_files(&scratch);
    let mut game = game_with(&scratch, FlipChoice::Always);
    leave_the_hub(&mut game);
    assert!(game.run_flipped());
    assert!(game.tiers.layer.flip);

    let expected = flip_of(&documents(false));
    assert_rooms_are(&game, &expected, "a flipped run");
    for id in [START, SECOND, BOSS, TOWER, CLOSET] {
        assert_ne!(
            live_layout(&game, id),
            layout_in(&documents(false), id),
            "{id:?} really is mirrored"
        );
    }
    assert_eq!(room_files(&scratch), files, "nothing is ever written");
}

#[test]
fn enemies_stand_where_the_flip_puts_their_patrol() {
    let scratch = dungeon_world(0.0);
    write_documents(&scratch, &documents(true));
    let mut plain = game_with(&scratch, FlipChoice::Never);
    leave_the_hub(&mut plain);
    let mut flipped = game_with(&scratch, FlipChoice::Always);
    leave_the_hub(&mut flipped);

    let x_of = |game: &GameState| game.rooms[START.index()].enemies[0].position().x;
    assert!((560.0..=800.0).contains(&x_of(&plain)), "{}", x_of(&plain));
    // The patrol 560 to 800 in a room 960 wide becomes 160 to 400.
    assert!(
        (160.0..=400.0).contains(&x_of(&flipped)),
        "{}",
        x_of(&flipped)
    );
}

#[test]
fn the_hub_is_never_flipped_whatever_the_choice() {
    let (file, flipped) = (
        layout_in(&documents(false), HUB),
        flip_of(&documents(false)),
    );
    let scratch = dungeon_world(100.0);
    let mut game = game_with(&scratch, FlipChoice::Always);
    assert!(!game.run_flipped());
    assert_eq!(live_layout(&game, HUB), file);
    leave_the_hub(&mut game);
    // Its walls, spawn and the side its door is on stay; only the exit's arrival point, which
    // is a spot in the start room, moved with that room.
    let mut expected = file.clone();
    expected.exits[0].spawn = layout_in(&flipped, HUB).exits[0].spawn;
    assert_ne!(expected.exits[0].spawn, file.exits[0].spawn);
    assert_eq!(live_layout(&game, HUB), expected);
    assert_eq!(expected.exits[0].edge, file.exits[0].edge);
    game.abandon_run();
    game.continue_after_result();
    assert_eq!(live_layout(&game, HUB), file);
}

#[test]
fn leaving_the_hub_into_a_flipped_start_puts_the_player_at_the_mirrored_arrival() {
    let scratch = dungeon_world(0.0);
    let mut plain = game_with(&scratch, FlipChoice::Never);
    leave_the_hub(&mut plain);
    let mut flipped = game_with(&scratch, FlipChoice::Always);
    leave_the_hub(&mut flipped);

    // The hub's exit in the files, and in the flipped dungeon: the same exit, another arrival.
    let before = layout_in(&documents(false), HUB).exits[0].spawn;
    let after = layout_in(&flip_of(&documents(false)), HUB).exits[0].spawn;
    assert_ne!(before, after, "the arrival moved with the start room");
    assert_eq!(plain.player.position(), before.into());
    assert_eq!(flipped.player.position(), after.into());
    // And it is a spot of the flipped start room, beside its exit back to the hub.
    let layout = live_layout(&flipped, START);
    assert_eq!(arrival_error(&layout, flipped.player.position()), None);
    let back = layout.exits.iter().position(|e| e.target_room == HUB);
    let beside = arrival_next_to(&layout, back.unwrap()).unwrap();
    assert_eq!(flipped.player.position(), beside);
}

#[test]
fn every_exit_of_the_dungeon_leads_to_the_right_room_and_lands_on_a_valid_spot() {
    for choice in [FlipChoice::Never, FlipChoice::Always] {
        let scratch = dungeon_world(0.0);
        let mut game = game_with(&scratch, choice);
        leave_the_hub(&mut game);
        let catalog: Vec<_> = ROOMS
            .iter()
            .map(|id| (*id, live_layout(&game, *id)))
            .collect();
        let layout_of = |id: RoomId| &catalog.iter().find(|(room, _)| *room == id).unwrap().1;
        let mut walked = 0;
        for source in [START, SECOND, BOSS, TOWER, CLOSET] {
            for index in 0..game.rooms[source.index()].exits.len() {
                // Back in the run, in the source room, with the key for the locked exit.
                game.current = game.rooms.iter().position(|r| r.id == source).unwrap();
                let exit = &mut game.rooms[game.current].exits[index];
                exit.locked_by = None;
                let exit = *exit;
                game.use_exit(exit);
                walked += 1;
                let what = format!("{choice:?}: {source:?} exit {index}");
                if exit.target_room == HUB {
                    assert!(game.run_result().is_some(), "{what}: the hub ends the run");
                    game.continue_after_result();
                    leave_the_hub(&mut game);
                    continue;
                }
                assert_eq!(game.room().id, exit.target_room, "{what}");
                assert_eq!(game.player.position(), exit.spawn, "{what}");
                let target = layout_of(exit.target_room);
                assert_eq!(arrival_error(target, exit.spawn), None, "{what}");
                // The spot beside the exit of that room that comes back here.
                let back = target.exits.iter().position(|e| e.target_room == source);
                let beside = arrival_next_to(target, back.unwrap()).unwrap();
                assert_eq!(exit.spawn, beside, "{what}: beside the way back");
            }
        }
        assert_eq!(walked, 9, "{choice:?}: every exit but the hub's was used");
    }
}

#[test]
fn the_end_of_a_run_returns_to_an_unflipped_hub_and_world() {
    for end in ["abandon", "death"] {
        let scratch = dungeon_world(0.0);
        let mut game = game_with(&scratch, FlipChoice::Always);
        leave_the_hub(&mut game);
        assert!(game.run_flipped());
        if end == "abandon" {
            game.abandon_run();
        } else {
            kill(&mut game);
        }
        assert!(
            matches!(game.run_phase(), Some(RunPhase::Result(_))),
            "{end}"
        );
        game.continue_after_result();
        assert!(game.in_hub() && !game.run_flipped(), "{end}");
        assert_eq!(game.room().id, HUB);
        assert!(!game.tiers.layer.flip);
        assert_rooms_are(&game, &documents(false), end);
        // The next run starts from the choice again.
        game.set_flip_choice(FlipChoice::Never);
        leave_the_hub(&mut game);
        assert_rooms_are(&game, &documents(false), "the next run");
    }
}

#[test]
fn f5_in_the_middle_of_a_flipped_run_keeps_the_flip() {
    let scratch = dungeon_world(0.0);
    let mut game = game_with(&scratch, FlipChoice::Always);
    leave_the_hub(&mut game);
    // The start room gets a chaser in its file; F5 brings it in, mirrored like the rest.
    write_documents(&scratch, &documents(true));
    game.reload_rooms().unwrap();
    assert!(game.run_flipped() && game.tiers.layer.flip);
    let enemies = &game.rooms[START.index()].enemies;
    assert_eq!(enemies.len(), 1, "the reload brought it in");
    assert!((160.0..=400.0).contains(&enemies[0].position().x));
    assert_rooms_are(&game, &flip_of(&documents(false)), "after F5");
    // A second F5 changes nothing either.
    game.reload_rooms().unwrap();
    assert!((160.0..=400.0).contains(&game.rooms[START.index()].enemies[0].position().x));
}

#[test]
fn f5_in_the_hub_and_in_an_unflipped_run_does_not_flip() {
    let scratch = dungeon_world(0.0);
    let mut game = game_with(&scratch, FlipChoice::Always);
    write_documents(&scratch, &documents(true));
    game.reload_rooms().unwrap();
    let x = game.rooms[START.index()].enemies[0].position().x;
    assert!(
        (560.0..=800.0).contains(&x),
        "the hub is not a flipped run: {x}"
    );

    game.set_flip_choice(FlipChoice::Never);
    leave_the_hub(&mut game);
    game.reload_rooms().unwrap();
    let x = game.rooms[START.index()].enemies[0].position().x;
    assert!((560.0..=800.0).contains(&x), "{x}");
}

#[test]
fn a_run_that_starts_from_the_editor_lands_in_its_flipped_room_at_the_spawn() {
    let scratch = dungeon_world(0.0);
    let mut game = game_with(&scratch, FlipChoice::Always);
    // In the hub the editor opens and can send the player to another room; standing outside
    // the hub when it closes starts the run, as it always did.
    game.edit_room(SECOND).unwrap();
    game.toggle_editor().unwrap();
    assert!(game.editor().is_none());
    game.update(input(), 0.0);
    assert!(game.run_in_progress() && game.run_flipped());
    assert_eq!(game.room().id, SECOND);
    let spawn = layout_in(&flip_of(&documents(false)), SECOND).spawn;
    assert_ne!(spawn, layout_in(&documents(false), SECOND).spawn);
    assert_eq!(game.player.position(), spawn.into());
    assert_eq!(
        arrival_error(&live_layout(&game, SECOND), game.player.position()),
        None
    );
}

#[test]
fn the_editor_refuses_during_a_flipped_run_and_works_everywhere_else() {
    let scratch = dungeon_world(0.0);
    let mut game = game_with(&scratch, FlipChoice::Always);
    // The hub, even with Always chosen.
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some());
    game.toggle_editor().unwrap();
    game.edit_room(START).unwrap();
    game.toggle_editor().unwrap();
    game.current = game.rooms.iter().position(|r| r.id == HUB).unwrap();

    leave_the_hub(&mut game);
    let (room, position) = (game.room().id, game.player.position());
    for error in [
        game.toggle_editor().unwrap_err(),
        game.edit_room(SECOND).unwrap_err(),
    ] {
        assert!(error.contains("abandon the run"), "{error}");
        assert!(error.contains("pause menu"), "{error}");
        assert!(error.contains("Flip next run"), "{error}");
    }
    assert!(game.editor().is_none(), "no editor opened");
    assert_eq!((game.room().id, game.player.position()), (room, position));
    assert!(game.run_in_progress(), "and the run goes on");

    // The frozen world of the result screen is still the flipped one.
    game.abandon_run();
    assert!(game.toggle_editor().is_err() && game.edit_room(START).is_err());
    game.continue_after_result();
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some(), "back in the hub it opens");
}

#[test]
fn the_editor_works_during_an_unflipped_run() {
    let scratch = dungeon_world(0.0);
    let mut game = game_with(&scratch, FlipChoice::Never);
    leave_the_hub(&mut game);
    game.toggle_editor().unwrap();
    assert!(game.editor().is_some());
    game.toggle_editor().unwrap();
    game.edit_room(SECOND).unwrap();
    assert!(game.editor().is_some());
}

#[test]
fn a_test_room_is_not_a_flipped_run_and_plays_the_file_as_it_is() {
    let scratch = dungeon_world(0.0);
    let mut game = game_with(&scratch, FlipChoice::Always);
    game.edit_room(START).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room() && !game.run_flipped());
    assert_eq!(
        live_layout(&game, START),
        layout_in(&documents(false), START)
    );
    game.end_test_room().unwrap();
    assert!(!game.run_flipped());
    // And nothing in the hub depended on it: the run that follows still flips.
    game.toggle_editor().unwrap();
    game.current = game.rooms.iter().position(|r| r.id == HUB).unwrap();
    leave_the_hub(&mut game);
    assert!(game.run_flipped());
}

#[test]
fn the_map_of_a_flipped_run_is_the_mirror_image_with_the_hub_where_it_was() {
    let cells = [
        (HUB, (0, 0)),
        (START, (1, 0)),
        (SECOND, (2, 0)),
        (BOSS, (3, 0)),
        (TOWER, (2, -1)),
        (CLOSET, (1, 1)),
    ];
    let scratch = dungeon_world(0.0);
    let mut docs = documents(false);
    for (id, cell) in cells {
        let (_, doc) = docs.iter_mut().find(|(room, _)| *room == id).unwrap();
        doc.layout.as_mut().unwrap().map_cell = Some(cell);
    }
    write_documents(&scratch, &docs);
    let cell_of = |game: &GameState, id: RoomId| {
        let map = game.dungeon_map();
        let room = map.rooms.iter().find(|room| room.id == id).unwrap();
        room.cell
    };
    let mut plain = game_with(&scratch, FlipChoice::Never);
    leave_the_hub(&mut plain);
    let mut flipped = game_with(&scratch, FlipChoice::Always);
    leave_the_hub(&mut flipped);
    for (id, (x, y)) in cells {
        assert_eq!(cell_of(&plain, id), (x, y), "{id:?} as authored");
        let expected = if id == HUB { (x, y) } else { (-x, y) };
        assert_eq!(cell_of(&flipped, id), expected, "{id:?} mirrored");
    }
    // The links follow: each one joins the same two rooms in the same rows.
    let (a, b) = (plain.dungeon_map(), flipped.dungeon_map());
    assert_eq!(a.links.len(), b.links.len());
}
