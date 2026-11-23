//! Optional enemy placements through `GameState` (`tier_link::Layer`, `crate::variety`): a run's
//! seed decides which enemies a room has, the same seed always gives the same ones, every rebuild
//! in the run (F5, an editor Save) puts back the same ones, and nothing outside a run leaves a
//! placement out: not the hub, a game with no hub, a test room or the room files.
//!
//! The first room is the one of `variety::tests`: three enemies in group 1 (patrols starting at
//! 100, 220 and 340), one with chance 50 (460), one always there (560) and a chance 0 twin of the
//! first (100). An enemy is named by the left end of its patrol. The seed comes from F4's fixed
//! seed (`set_fixed_seed`), never from a clock.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, world};
use super::tier_tests::{begin, with_tier, RULES};
use super::*;
use crate::room::RoomEnemies;

const SCENE: &str = r#"(enemies: [
    (definition: "chase", patrol_left: 100.0, patrol_right: 200.0, group: Some(1)),
    (definition: "chase", patrol_left: 220.0, patrol_right: 320.0, group: Some(1)),
    (definition: "chase", patrol_left: 340.0, patrol_right: 440.0, group: Some(1)),
    (definition: "chase", patrol_left: 460.0, patrol_right: 540.0, chance: 50),
    (definition: "chase", patrol_left: 560.0, patrol_right: 800.0),
    (definition: "chase", patrol_left: 100.0, patrol_right: 200.0, chance: 0),
])"#;

fn game_with(first: &str) -> (Scratch, GameState) {
    let scratch = world(100.0);
    scratch.write("difficulty.ron", RULES);
    scratch.write("rooms/first.ron", first);
    let game = game_in(&scratch);
    (scratch, game)
}

fn lefts(game: &GameState, id: RoomId) -> Vec<u32> {
    game.rooms[id.index()]
        .enemies
        .iter()
        .map(|enemy| enemy.placement_key().1 as u32)
        .collect()
}

/// Back in the hub, with whatever run was going over.
fn to_the_hub(game: &mut GameState) {
    game.abandon_run();
    game.continue_after_result();
    assert!(game.in_hub());
}

/// A run with exactly `seed`: the enemies of the first room.
fn run_with(game: &mut GameState, seed: u64) -> Vec<u32> {
    if !game.in_hub() {
        to_the_hub(game);
    }
    game.set_fixed_seed(Some(seed));
    begin(game);
    assert_eq!(game.run_seed(), Some(seed));
    lefts(game, RoomId::First)
}

#[test]
fn a_run_has_the_enemies_its_seed_decides_and_these_are_pinned() {
    // The same numbers as `variety::tests::these_seeds_leave_exactly_these_enemies`, worked out
    // apart from the code: the game builds the room the same way the rule was written for.
    let (_scratch, mut game) = game_with(SCENE);
    assert_eq!(run_with(&mut game, 1), [340, 460, 560]);
    assert_eq!(run_with(&mut game, 2), [340, 560]);
    assert_eq!(run_with(&mut game, 3), [220, 460, 560]);
    assert_eq!(run_with(&mut game, u64::MAX), [340, 560]);
}

#[test]
fn the_same_seed_gives_the_same_enemies_in_every_run_and_other_seeds_give_other_ones() {
    let (_scratch, mut game) = game_with(SCENE);
    let first = run_with(&mut game, 7);
    for _ in 0..3 {
        assert_eq!(run_with(&mut game, 7), first);
    }
    let outcomes: std::collections::BTreeSet<_> =
        (1..=30).map(|seed| run_with(&mut game, seed)).collect();
    assert!(outcomes.len() >= 3, "30 seeds gave only {outcomes:?}");
}

#[test]
fn a_run_without_a_fixed_seed_takes_the_source_and_replays_from_its_seed() {
    let (_scratch, mut game) = game_with(SCENE);
    game.use_seed_source(crate::seed::SeedSource::new(5));
    begin(&mut game);
    let seed = game.run_seed().unwrap();
    let played = lefts(&game, RoomId::First);
    to_the_hub(&mut game);
    assert_eq!(
        run_with(&mut game, seed),
        played,
        "type the seed from the result screen"
    );
}

#[test]
fn over_many_seeds_the_group_shows_all_three_and_chance_0_and_100_hold() {
    let (_scratch, mut game) = game_with(SCENE);
    let (mut picks, mut fifty) = (std::collections::BTreeSet::new(), 0);
    for seed in 1..=60 {
        let left = run_with(&mut game, seed);
        let grouped: Vec<_> = left
            .iter()
            .filter(|x| [100, 220, 340].contains(x))
            .collect();
        assert_eq!(
            grouped.len(),
            1,
            "seed {seed}: exactly one of the group in {left:?}"
        );
        picks.insert(*grouped[0]);
        fifty += usize::from(left.contains(&460));
        assert!(
            left.contains(&560),
            "seed {seed}: chance 100 is always there"
        );
        // The only other enemy with a patrol at 100 is the group member, never the chance 0 twin.
        let twins = game.rooms[RoomId::First.index()]
            .enemies
            .iter()
            .filter(|enemy| enemy.group().is_none() && enemy.chance() == 0)
            .count();
        assert_eq!(twins, 0, "seed {seed}: chance 0 never stays");
    }
    assert_eq!(picks.len(), 3, "{picks:?}");
    assert!(
        (15..=45).contains(&fifty),
        "chance 50 was present in {fifty} of 60"
    );
}

#[test]
fn the_hub_a_game_with_no_hub_and_a_finished_run_show_every_placement() {
    let (_scratch, mut game) = game_with(SCENE);
    assert!(game.in_hub());
    assert_eq!(
        lefts(&game, RoomId::First).len(),
        6,
        "the hub builds the world whole"
    );
    run_with(&mut game, 1);
    assert_eq!(lefts(&game, RoomId::First).len(), 3);
    to_the_hub(&mut game);
    assert_eq!(
        lefts(&game, RoomId::First).len(),
        6,
        "back in the hub nothing is left out"
    );

    let scratch = Scratch::new();
    scratch.write("rooms/first.ron", SCENE);
    let mut plain = GameState::try_new(&scratch.0).unwrap();
    plain.set_fixed_seed(Some(1));
    assert_eq!(
        lefts(&plain, RoomId::First).len(),
        6,
        "no hub, no runs, no seed"
    );
    plain.reload_rooms().unwrap();
    assert_eq!(lefts(&plain, RoomId::First).len(), 6);
}

#[test]
fn a_test_room_shows_every_placement_under_any_seed() {
    let (_scratch, mut game) = game_with(SCENE);
    game.set_fixed_seed(Some(1));
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(lefts(&game, RoomId::First).len(), 6);
    game.end_test_room().unwrap();
    assert_eq!(game.editor().unwrap().draft.enemies.len(), 6);

    // Inside a run the test room still shows them all, and the run's rooms are back after it.
    game.toggle_editor().unwrap();
    let played = run_with(&mut game, 1);
    assert_eq!(played.len(), 3);
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(
        lefts(&game, RoomId::First).len(),
        6,
        "a sandbox takes no seed"
    );
    game.end_test_room().unwrap();
    assert_eq!(
        lefts(&game, RoomId::First),
        played,
        "and the run's own enemies return"
    );
}

#[test]
fn f5_in_the_middle_of_a_run_keeps_the_same_picks() {
    for seed in [1, 2, 3, 50] {
        let (_scratch, mut game) = game_with(SCENE);
        let played = run_with(&mut game, seed);
        game.reload_rooms().unwrap();
        assert_eq!(lefts(&game, RoomId::First), played, "seed {seed}");
        game.reload_rooms().unwrap();
        assert_eq!(lefts(&game, RoomId::First), played, "seed {seed}, twice");
    }
}

#[test]
fn f5_after_a_file_edit_keeps_the_picks_of_placements_that_did_not_change() {
    let (scratch, mut game) = game_with(SCENE);
    let played = run_with(&mut game, 1);
    assert_eq!(played, [340, 460, 560]);
    // A new always-there enemy at the front and a new one at the end: nobody else's identity
    // is its place in the list, so the old picks stand.
    let edited = SCENE
        .replace(
            "(enemies: [",
            "(enemies: [\n    (definition: \"chase\", patrol_left: 20.0, patrol_right: 60.0),",
        )
        .replace(
            "])",
            "    (definition: \"chase\", patrol_left: 880.0, patrol_right: 940.0),\n])",
        );
    scratch.write("rooms/first.ron", &edited);
    game.reload_rooms().unwrap();
    assert_eq!(lefts(&game, RoomId::First), [20, 340, 460, 560, 880]);
}

#[test]
fn an_editor_save_keeps_every_placement_in_the_file_under_any_seed_and_the_same_picks_in_play() {
    for seed in [1, 2, 3] {
        let (scratch, mut game) = game_with(SCENE);
        let played = run_with(&mut game, seed);
        game.edit_room(RoomId::First).unwrap();
        assert_eq!(
            game.editor().unwrap().draft.enemies.len(),
            6,
            "the editor has all of them"
        );
        game.save_editor().unwrap();
        game.toggle_editor().unwrap();

        let saved = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();
        let document: RoomEnemies = ron::from_str(&saved).unwrap();
        assert_eq!(document.enemies.len(), 6, "seed {seed}: {saved}");
        let groups: Vec<_> = document.enemies.iter().map(|e| e.group).collect();
        let chances: Vec<_> = document.enemies.iter().map(|e| e.chance).collect();
        assert_eq!(
            groups,
            [Some(1), Some(1), Some(1), None, None, None],
            "seed {seed}"
        );
        assert_eq!(chances, [100, 100, 100, 50, 100, 0], "seed {seed}");
        assert_eq!(
            lefts(&game, RoomId::First),
            played,
            "seed {seed}: rebuilt with the run's seed"
        );
    }
}

#[test]
fn a_boss_is_never_left_out_and_still_locks_its_exits() {
    let boss_room = r#"(
        enemies: [(definition: "chase", patrol_left: 300.0, patrol_right: 400.0, chance: 0)],
        boss: Some((definition: "Ash Warden", position: (560.0, 436.0))),
    )"#;
    let (scratch, _) = game_with(SCENE);
    scratch.write("rooms/boss.ron", boss_room);
    let mut game = game_in(&scratch);
    for seed in 1..=10 {
        game.set_fixed_seed(Some(seed));
        if !game.in_hub() {
            to_the_hub(&mut game);
        }
        begin(&mut game);
        let room = &game.rooms[RoomId::Boss.index()];
        assert!(room.boss_special.is_some(), "seed {seed}");
        assert!(room.exits_locked(), "seed {seed}");
        assert_eq!(
            room.enemies.len(),
            0,
            "seed {seed}: the chance 0 enemy next to it is out"
        );
    }
}

#[test]
fn the_min_tier_is_applied_first_so_a_group_never_picks_an_enemy_the_tier_left_out() {
    let grouped = r#"(enemies: [
        (definition: "chase", patrol_left: 100.0, patrol_right: 200.0, group: Some(1), min_tier: 2),
        (definition: "chase", patrol_left: 220.0, patrol_right: 320.0, group: Some(1), min_tier: 2),
        (definition: "chase", patrol_left: 340.0, patrol_right: 440.0, group: Some(1)),
    ])"#;
    let (_scratch, mut game) = game_with(grouped);
    for seed in 1..=20 {
        assert_eq!(
            run_with(&mut game, seed),
            [340],
            "at tier 0 the only member in the room stays, never an empty group (seed {seed})"
        );
    }
    to_the_hub(&mut game);
    with_tier(&mut game, 2);
    let picked: std::collections::BTreeSet<_> =
        (1..=40).map(|seed| run_with(&mut game, seed)).collect();
    assert_eq!(
        picked.len(),
        3,
        "at tier 2 any of the three can be picked: {picked:?}"
    );
    assert!(picked.iter().all(|left| left.len() == 1));
}

#[test]
fn a_room_in_which_a_group_has_a_hidden_boss_neighbour_and_nothing_else_is_unchanged() {
    // A room with no optional placements builds exactly as before, with or without a seed.
    let plain = r#"(enemies: [
        (definition: "chase", patrol_left: 560.0, patrol_right: 800.0),
        (definition: "chase", patrol_left: 300.0, patrol_right: 400.0),
    ])"#;
    let (_scratch, mut game) = game_with(plain);
    for seed in 1..=10 {
        assert_eq!(run_with(&mut game, seed), [560, 300]);
    }
}
