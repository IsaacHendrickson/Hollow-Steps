//! Generated runs through `GameState` (`generate_link`, `world_load::generated`): the world of a
//! run built from a plan, leaving the hub into it, walking it, clearing it, rebuilding it with F5
//! and getting the authored rooms back. The world is `fixture::generated_world`, a private copy
//! of `tests/fixtures` with a pool of pieces and the generator on; nothing here reads `assets/`.
//! The editor's refusal, F4's choice and the rest are in `more_tests`.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, input, swing, walk, HUB};
use super::*;
use crate::dungeon_gen::DungeonPlan;
use crate::editor::RoomLayout;
use crate::run::{RunOutcome, RunPhase};

mod fixture;
mod more_tests;

use fixture::{boss_pieces, generated_world, piece_ids, start_pieces};

/// The game of a scratch world whose runs ask for between `keys.0` and `keys.1` keys. The scratch
/// is returned too: it holds the files F5 reads again.
fn game_with(keys: (u8, u8)) -> (Scratch, GameState) {
    let scratch = generated_world(keys);
    let game = game_in(&scratch);
    (scratch, game)
}

/// Leaves the hub by walking out, so a run starts, and sends the enemies away so that nothing
/// kills the player on the way.
fn begin(game: &mut GameState) {
    game.update(walk(1.0), 10.0);
    assert!(
        game.run_in_progress(),
        "walking out of the hub starts a run"
    );
    clear_enemies(game);
}

fn clear_enemies(game: &mut GameState) {
    for room in &mut game.rooms {
        room.enemies.clear();
    }
}

/// Starts a run with `seed` and returns its plan.
fn run_with_seed(game: &mut GameState, seed: u64) -> DungeonPlan {
    game.set_fixed_seed(Some(seed));
    begin(game);
    game.run_plan().expect("the run is generated").clone()
}

/// What makes a plan the plan it is: which piece is where, and how it faces.
fn shape(plan: &DungeonPlan) -> Vec<(RoomId, RoomId, bool)> {
    let rooms = plan.rooms.iter();
    rooms
        .map(|room| (room.instance, room.piece, room.mirrored))
        .collect()
}

fn ids(game: &GameState) -> Vec<RoomId> {
    game.rooms.iter().map(|room| room.id).collect()
}

fn layouts(game: &GameState) -> Vec<(RoomId, RoomLayout)> {
    let rooms = game.rooms.iter();
    rooms
        .map(|room| (room.id, RoomLayout::from_room(room)))
        .collect()
}

fn index_of(game: &GameState, id: RoomId) -> usize {
    game.rooms
        .iter()
        .position(|room| room.id == id)
        .unwrap_or_else(|| panic!("{id:?} is not in the world"))
}

/// The way to `to` from `from` along the exits of the rooms in play, as the rooms to enter one
/// after another (breadth first).
fn route(game: &GameState, from: RoomId, to: RoomId) -> Option<Vec<RoomId>> {
    let mut paths = vec![vec![from]];
    let mut seen = vec![from];
    while !paths.is_empty() {
        let mut next = Vec::new();
        for path in &paths {
            let here = *path.last().unwrap();
            for exit in &game.rooms[index_of(game, here)].exits {
                if seen.contains(&exit.target_room) {
                    continue;
                }
                seen.push(exit.target_room);
                let mut longer = path.clone();
                longer.push(exit.target_room);
                if exit.target_room == to {
                    return Some(longer);
                }
                next.push(longer);
            }
        }
        paths = next;
    }
    None
}

// ---- the world of a run ------------------------------------------------------------------------

#[test]
fn a_generated_run_is_the_rewired_hub_plus_the_plans_rooms_and_nothing_else() {
    let (_scratch, mut game) = game_with((0, 1));
    let authored = ids(&game);
    assert!(
        authored.contains(&RoomId::Second),
        "the authored world has it"
    );
    begin(&mut game);

    assert!(game.run_generated());
    let plan = game.run_plan().unwrap().clone();
    let mut expected = vec![HUB];
    expected.extend(plan.rooms.iter().map(|room| room.instance));
    assert_eq!(
        ids(&game),
        expected,
        "the hub, then the plan's rooms in its order"
    );
    assert!(
        !ids(&game).contains(&RoomId::Second),
        "an authored room that is no piece is absent for the run"
    );
    assert!(ids(&game).len() < authored.len());
    for planned in &plan.rooms {
        let id = if start_pieces().contains(&planned.piece) {
            RoomId::First
        } else if boss_pieces().contains(&planned.piece) {
            RoomId::Boss
        } else {
            planned.piece
        };
        assert_eq!(planned.instance, id, "{:?}", planned.piece);
        assert!(piece_ids().contains(&planned.piece));
    }
}

#[test]
fn every_planned_room_is_built_from_the_plans_layout_and_the_pieces_own_placements() {
    let (_scratch, mut game) = game_with((0, 1));
    for seed in 0..6 {
        let plan = run_with_seed(&mut game, seed);
        for planned in &plan.rooms {
            let room = &game.rooms[index_of(&game, planned.instance)];
            assert_eq!(
                room.spawn.to_array(),
                [planned.layout.spawn.0, planned.layout.spawn.1]
            );
            let exits: Vec<_> = room.exits.iter().map(|e| (e.edge, e.target_room)).collect();
            let planned_exits: Vec<_> = planned
                .layout
                .exits
                .iter()
                .map(|e| (e.edge, e.target_room))
                .collect();
            assert_eq!(exits, planned_exits, "seed {seed} {:?}", planned.instance);
            // The Boss piece's own boss is in the boss room; a mirrored one stands mirrored.
            let is_boss_room = planned.instance == RoomId::Boss;
            assert_eq!(room.boss_special.is_some(), is_boss_room, "seed {seed}");
            if is_boss_room && planned.mirrored {
                let boss = room.boss_special.as_ref().unwrap();
                assert!((boss.position().x - (960.0 - 560.0)).abs() < 1e-3);
            }
        }
        game.abandon_run();
        game.continue_after_result();
    }
}

#[test]
fn leaving_the_hub_lands_at_the_start_rooms_spawn() {
    let (_scratch, mut game) = game_with((0, 1));
    game.set_fixed_seed(Some(11));
    let exit = game.room().exits[0];
    assert!(
        (exit.spawn.x - 180.0).abs() < 1e-3,
        "the authored hub's arrival"
    );

    game.use_exit(exit);

    assert!(game.run_in_progress());
    assert_eq!(game.room().id, RoomId::First);
    let plan = game.run_plan().unwrap();
    let start = plan.room(RoomId::First).unwrap();
    let spawn_x = start.layout.spawn.0;
    assert!(
        (game.player().position().x - spawn_x).abs() < 1.0,
        "the player arrives at {spawn_x}, not at the authored 180: {:?}",
        game.player().position()
    );
    // Mirrored or not, the spawn is the piece's (200 or its mirror).
    assert!([200.0, 760.0].iter().any(|x| (spawn_x - x).abs() < 1e-3));
}

#[test]
fn walking_the_plans_exits_from_the_start_reaches_the_boss() {
    let (_scratch, mut game) = game_with((0, 0));
    for seed in 0..8 {
        run_with_seed(&mut game, seed);
        assert_eq!(game.room().id, RoomId::First);
        let path = route(&game, RoomId::First, RoomId::Boss)
            .unwrap_or_else(|| panic!("seed {seed}: no way to the boss"));
        for next in &path[1..] {
            let exit = *game
                .room()
                .exits
                .iter()
                .find(|exit| exit.target_room == *next)
                .unwrap();
            game.use_exit(exit);
            assert_eq!(game.room().id, *next, "seed {seed}");
            let (at, bounds) = (game.player().position(), game.room().bounds);
            assert!(bounds.contains(at), "seed {seed}: arrived outside {next:?}");
        }
        assert_eq!(game.room().id, RoomId::Boss);
        assert!(game.run_in_progress(), "walking never ended the run");
        game.abandon_run();
        game.continue_after_result();
    }
}

// ---- playing it --------------------------------------------------------------------------------

fn stand_before_the_weakened_boss(game: &mut GameState) {
    game.current = index_of(game, RoomId::Boss);
    let boss = game.rooms[game.current].boss_special.as_mut().unwrap();
    boss.damage_boss(boss.health() - 1);
    let x = boss.position().x;
    let bounds = game.world_bounds();
    game.player.place_on_floor(x - 40.0, bounds);
}

#[test]
fn killing_the_boss_of_the_generated_boss_room_clears_the_run() {
    let (_scratch, mut game) = game_with((0, 1));
    run_with_seed(&mut game, 5);
    stand_before_the_weakened_boss(&mut game);

    game.update(swing(), 0.0);

    let result = game.run_result().expect("the dungeon is cleared");
    assert_eq!(result.outcome, RunOutcome::Cleared);
    assert_eq!((game.profile().tier, game.profile().cleared), (1, 1));
    assert!(
        game.run_generated(),
        "the result screen still belongs to the run"
    );
}

#[test]
fn dying_in_a_generated_run_ends_it() {
    let (_scratch, mut game) = game_with((0, 1));
    run_with_seed(&mut game, 5);
    let bounds = game.world_bounds();
    let terrain = &game.rooms[game.current].terrain;
    game.player
        .update_with_terrain(input(), 10.0, bounds, terrain);
    assert!(game.player.take_damage_amount(u16::MAX));
    game.update(input(), 0.0);

    let result = game.run_result().expect("the run is over");
    assert_eq!(result.outcome, RunOutcome::Died);
}

// ---- rebuilding and leaving ----------------------------------------------------------------------

#[test]
fn f5_mid_run_rebuilds_the_same_rooms_from_the_same_plan() {
    let (_scratch, mut game) = game_with((0, 1));
    let plan = run_with_seed(&mut game, 7);
    let (before_ids, before_layouts) = (ids(&game), layouts(&game));
    let room = game.room().id;

    game.reload_rooms().expect("the files are fine");

    assert_eq!(ids(&game), before_ids);
    assert_eq!(layouts(&game), before_layouts);
    assert_eq!(game.room().id, room);
    assert_eq!(
        game.run_plan(),
        Some(&plan),
        "the plan is the one the run started with"
    );
    assert!(game.run_in_progress());
}

#[test]
fn abandoning_a_generated_run_returns_to_the_authored_hub_and_world() {
    let (_scratch, mut game) = game_with((0, 1));
    let authored = ids(&game);
    run_with_seed(&mut game, 3);
    assert_ne!(ids(&game), authored);

    game.abandon_run();
    assert!(
        game.run_generated(),
        "the result screen keeps the run's world"
    );
    assert!(matches!(game.run_phase(), Some(RunPhase::Result(_))));
    game.continue_after_result();

    assert_eq!(game.room().id, HUB);
    assert!(game.in_hub());
    assert!(!game.run_generated() && game.run_plan().is_none());
    assert_eq!(
        ids(&game),
        authored,
        "every authored room is back with the hub"
    );
    assert!(
        (game.room().exits[0].spawn.x - 180.0).abs() < 1e-3,
        "and the hub is the authored one"
    );
}

// ---- seeds ---------------------------------------------------------------------------------------

#[test]
fn a_new_seed_builds_a_different_dungeon_and_a_fixed_seed_builds_the_same_one() {
    let (_scratch, mut game) = game_with((0, 1));
    let mut shapes = Vec::new();
    for seed in 0..12 {
        let plan = run_with_seed(&mut game, seed);
        shapes.push(shape(&plan));
        game.abandon_run();
        game.continue_after_result();
    }
    let distinct = shapes
        .iter()
        .enumerate()
        .filter(|(i, shape)| !shapes[..*i].contains(shape));
    assert!(
        distinct.count() > 6,
        "twelve seeds gave too few dungeons: {shapes:?}"
    );

    for seed in [4, 9] {
        let first = run_with_seed(&mut game, seed);
        game.abandon_run();
        game.continue_after_result();
        let again = run_with_seed(&mut game, seed);
        assert_eq!(first, again, "seed {seed} twice");
        game.abandon_run();
        game.continue_after_result();
    }

    // Without a fixed seed each run takes the next one from the source.
    game.set_fixed_seed(None);
    begin(&mut game);
    let first = shape(game.run_plan().unwrap());
    game.abandon_run();
    game.continue_after_result();
    begin(&mut game);
    let second = shape(game.run_plan().unwrap());
    assert_ne!(first, second, "the next run is a different dungeon");
}
