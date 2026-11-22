//! `min_tier` through `GameState`: the game's world loading leaves out a placement whose tier is
//! above the run's, the editor and Save never do, and a boss left out locks no exit.
//!
//! The first room holds one enemy shown at every tier and one shown from tier 2; the boss room's
//! boss is shown from tier 2 as well (`HIDDEN_BOSS`) where a test asks for it.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, world};
use super::tier_tests::{begin, with_tier, RULES};
use super::*;
use crate::room::RoomEnemies;

const FIRST: &str = r#"(enemies: [
    (definition: "chase", patrol_left: 560.0, patrol_right: 800.0),
    (definition: "chase", patrol_left: 300.0, patrol_right: 400.0, min_tier: 2),
])"#;

const HIDDEN_BOSS: &str = r#"(enemies: [], boss: Some((definition: "Ash Warden", position: (560.0, 436.0), min_tier: 2)))"#;

fn game_with(tier: u32, first: &str, boss: &str) -> (Scratch, GameState) {
    let scratch = world(100.0);
    scratch.write("difficulty.ron", RULES);
    scratch.write("rooms/first.ron", first);
    scratch.write("rooms/boss.ron", boss);
    let mut game = game_in(&scratch);
    with_tier(&mut game, tier);
    (scratch, game)
}

fn enemies_in(game: &GameState, id: RoomId) -> usize {
    game.rooms[id.index()].enemies.len()
}

fn boss_room_text() -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rooms/boss.ron"),
    )
    .unwrap()
}

#[test]
fn a_placement_with_min_tier_2_is_absent_at_tiers_0_and_1_and_present_from_tier_2() {
    for (tier, shown) in [(0, 1), (1, 1), (2, 2), (3, 2), (9, 2)] {
        let (_scratch, mut game) = game_with(tier, FIRST, &boss_room_text());
        begin(&mut game);
        assert_eq!(enemies_in(&game, RoomId::First), shown, "at tier {tier}");
    }
}

#[test]
fn the_hidden_placement_is_the_one_with_the_min_tier_and_the_rest_keep_their_order() {
    let (_scratch, mut game) = game_with(1, FIRST, &boss_room_text());
    begin(&mut game);
    let enemy = &game.rooms[RoomId::First.index()].enemies[0];
    assert_eq!(enemy.position().x, 560.0, "the every-tier enemy stays");

    let (_scratch, mut game) = game_with(2, FIRST, &boss_room_text());
    begin(&mut game);
    let xs: Vec<f32> = game.rooms[RoomId::First.index()]
        .enemies
        .iter()
        .map(|enemy| enemy.position().x)
        .collect();
    assert_eq!(xs, [560.0, 300.0]);
}

#[test]
fn the_hub_and_a_game_with_no_hub_show_tier_0() {
    let (_scratch, game) = game_with(5, FIRST, &boss_room_text());
    assert!(game.in_hub());
    assert_eq!(
        enemies_in(&game, RoomId::First),
        1,
        "the hub builds at tier 0"
    );

    let scratch = Scratch::new();
    scratch.write("rooms/first.ron", FIRST);
    let mut plain = GameState::try_new(&scratch.0).unwrap();
    with_tier(&mut plain, 5);
    assert_eq!(enemies_in(&plain, RoomId::First), 1);
}

#[test]
fn a_hidden_placement_is_still_checked_when_the_world_loads() {
    let broken = FIRST.replace(
        "patrol_left: 300.0, patrol_right: 400.0",
        "patrol_left: 900.0, patrol_right: 300.0",
    );
    let scratch = world(100.0);
    scratch.write("rooms/first.ron", &broken);
    let error = GameState::try_new(&scratch.0).map(|_| ()).unwrap_err();
    assert!(error.contains("first.ron"), "{error}");
}

#[test]
fn a_boss_left_out_does_not_lock_the_exits_and_one_that_is_shown_does() {
    let (_scratch, mut low) = game_with(1, FIRST, HIDDEN_BOSS);
    begin(&mut low);
    let room = &low.rooms[RoomId::Boss.index()];
    assert!(room.boss_special.is_none());
    assert!(!room.exits_locked(), "nothing to defeat, nothing locked");

    // The player can stand in the room and play on without the boss.
    low.current = RoomId::Boss.index();
    low.update(PlayerInput::default(), 0.1);
    assert!(!low.rooms[RoomId::Boss.index()].exits_locked());

    let (_scratch, mut high) = game_with(2, FIRST, HIDDEN_BOSS);
    begin(&mut high);
    let room = &high.rooms[RoomId::Boss.index()];
    assert!(room.boss_special.is_some());
    assert!(room.exits_locked());
}

#[test]
fn a_room_with_a_hidden_enemy_and_a_shown_boss_loses_only_the_enemy() {
    let boss = r#"(
        enemies: [(definition: "chase", patrol_left: 300.0, patrol_right: 400.0, min_tier: 4)],
        boss: Some((definition: "Ash Warden", position: (560.0, 436.0))),
    )"#;
    let (_scratch, mut game) = game_with(0, FIRST, boss);
    begin(&mut game);
    assert_eq!(enemies_in(&game, RoomId::Boss), 0);
    assert!(game.rooms[RoomId::Boss.index()].boss_special.is_some());
}

#[test]
fn the_rebuild_of_an_f5_reload_and_an_editor_save_keep_leaving_the_placement_out() {
    let (_scratch, mut game) = game_with(1, FIRST, &boss_room_text());
    begin(&mut game);
    game.reload_rooms().unwrap();
    assert_eq!(enemies_in(&game, RoomId::First), 1);

    game.edit_room(RoomId::First).unwrap();
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    assert_eq!(
        enemies_in(&game, RoomId::First),
        1,
        "rebuilt at the run's tier"
    );
}

#[test]
fn save_at_tier_0_keeps_the_hidden_placement_in_the_file_and_a_later_run_shows_it() {
    let (scratch, mut game) = game_with(0, FIRST, &boss_room_text());
    begin(&mut game);
    assert_eq!(
        enemies_in(&game, RoomId::First),
        1,
        "tier 0: the min tier 2 enemy is out"
    );

    game.edit_room(RoomId::First).unwrap();
    assert_eq!(
        game.editor().unwrap().draft.enemies.len(),
        2,
        "the editor has both"
    );
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();

    let saved = std::fs::read_to_string(scratch.0.join("rooms").join("first.ron")).unwrap();
    let document: RoomEnemies = ron::from_str(&saved).unwrap();
    assert_eq!(document.enemies.len(), 2, "{saved}");
    assert_eq!(document.enemies[0].min_tier, 0);
    assert_eq!(document.enemies[1].min_tier, 2);
    assert_eq!(
        enemies_in(&game, RoomId::First),
        1,
        "and in play it is still out"
    );

    // Back to the hub, F4 sets tier 3, and the next run has both.
    game.abandon_run();
    game.continue_after_result();
    with_tier(&mut game, 3);
    begin(&mut game);
    assert_eq!(enemies_in(&game, RoomId::First), 2);
}

#[test]
fn save_keeps_a_hidden_boss_placement_too() {
    let (scratch, mut game) = game_with(0, FIRST, HIDDEN_BOSS);
    begin(&mut game);
    assert!(game.rooms[RoomId::Boss.index()].boss_special.is_none());

    game.edit_room(RoomId::Boss).unwrap();
    assert_eq!(
        game.editor().unwrap().draft.boss.as_ref().unwrap().min_tier,
        2
    );
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();

    let saved = std::fs::read_to_string(scratch.0.join("rooms").join("boss.ron")).unwrap();
    let document: RoomEnemies = ron::from_str(&saved).unwrap();
    assert_eq!(document.boss.unwrap().min_tier, 2, "{saved}");
    assert!(game.rooms[RoomId::Boss.index()].boss_special.is_none());
}

#[test]
fn a_test_room_plays_at_the_profile_tier_and_leaves_the_draft_whole() {
    let (_scratch, mut game) = game_with(0, FIRST, &boss_room_text());
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(enemies_in(&game, RoomId::First), 1, "profile tier 0");
    game.end_test_room().unwrap();
    assert_eq!(game.editor().unwrap().draft.enemies.len(), 2);

    with_tier(&mut game, 2);
    game.start_test_room().unwrap();
    assert_eq!(enemies_in(&game, RoomId::First), 2, "profile tier 2");
    game.end_test_room().unwrap();
    assert_eq!(game.editor().unwrap().draft.enemies.len(), 2);
}

#[test]
fn a_tier_above_the_cap_counts_as_the_cap_for_min_tier_too() {
    let rules = RULES.replace("max_effective_tier: 20", "max_effective_tier: 1");
    let scratch = world(100.0);
    scratch.write("difficulty.ron", &rules);
    scratch.write("rooms/first.ron", FIRST);
    let mut game = game_in(&scratch);
    with_tier(&mut game, 9);
    begin(&mut game);
    assert_eq!(
        enemies_in(&game, RoomId::First),
        1,
        "tier 9 plays as tier 1, so a min tier of 2 never shows"
    );
}
