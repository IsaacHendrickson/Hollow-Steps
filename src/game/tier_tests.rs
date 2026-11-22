//! Difficulty tiers through `GameState` (`tier_link`): a run at the profile's tier builds its rooms
//! scaled, the scale is held for the run, every rebuild keeps it, the editor never sees it, a test
//! room plays at the profile's tier, and a run's coins are scaled when it ends.
//!
//! The world is the one of `run_tests` (the fixtures plus a hub) with its own `difficulty.ron`,
//! so no number here comes from the shipped placeholders: enemy health +10% per tier, enemy
//! damage +20%, boss health +20%, boss damage +50%, coins +10%, tiers above 20 count as 20.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, input, profile_path, swing, walk, world};
use super::*;
use crate::difficulty::Difficulty;
use crate::hazard::{Hazard, HazardPlacement};
use crate::profile::Profile;

const RULES: &str = "(enemy_health_percent: 10.0, enemy_damage_percent: 20.0, \
    boss_health_percent: 20.0, boss_damage_percent: 50.0, coin_percent: 10.0, max_effective_tier: 20)";

fn rules() -> Difficulty {
    Difficulty::from_ron(RULES).unwrap()
}

fn with_tier(game: &mut GameState, tier: u32) {
    let profile = Profile {
        tier,
        ..game.profile().clone()
    };
    game.set_profile(profile).unwrap();
}

/// A world with the test rules, keeping `keep_percent` of a run's coins, and a game in it whose
/// profile is at `tier`.
fn tiered_with(tier: u32, keep_percent: f32, rules: &str) -> (Scratch, GameState) {
    let scratch = world(keep_percent);
    scratch.write("difficulty.ron", rules);
    let mut game = game_in(&scratch);
    with_tier(&mut game, tier);
    (scratch, game)
}

fn tiered(tier: u32) -> (Scratch, GameState) {
    tiered_with(tier, 100.0, RULES)
}

/// Walks out of the hub: a run starts. Unlike `run_tests::start_run` the enemies stay.
fn begin(game: &mut GameState) {
    game.update(walk(1.0), 10.0);
    assert!(
        game.run_in_progress(),
        "walking out of the hub starts a run"
    );
}

fn first_enemy(game: &GameState) -> (u16, u16) {
    let enemy = &game.rooms[RoomId::First.index()].enemies[0];
    (enemy.health(), enemy.max_health())
}

fn boss(game: &GameState) -> (u32, u32) {
    let boss = game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_ref()
        .expect("the boss room has its boss");
    (boss.health(), boss.max_health())
}

/// Ends the arrival invincibility, then stands the player on the first room's enemy for one tick
/// and returns the health that cost.
fn touch_loss(game: &mut GameState) -> u16 {
    let bounds = game.world_bounds();
    let terrain = &game.rooms[game.current].terrain;
    game.player
        .update_with_terrain(input(), 10.0, bounds, terrain);
    let before = game.player().health().current();
    let at = game.rooms[RoomId::First.index()].enemies[0].position();
    game.player.place_on_floor(at.x, bounds);
    game.update(input(), 0.0);
    before - game.player().health().current()
}

// ---- health -------------------------------------------------------------------------------------

#[test]
fn a_run_at_tier_3_builds_enemies_and_the_boss_with_scaled_health() {
    let (_scratch, mut game) = tiered(3);
    // Before the run, in the hub, the rooms are as authored.
    assert_eq!(first_enemy(&game), (30, 30));
    assert_eq!(boss(&game), (1000, 1000));
    assert!(game.tier_scale().is_neutral());
    assert_eq!(game.world_tier(), 0);

    begin(&mut game);

    assert_eq!(first_enemy(&game), (39, 39));
    assert_eq!(boss(&game), (1600, 1600));
    assert_eq!(game.tier_scale(), rules().scale(3));
    assert_eq!(game.world_tier(), 3);
    assert_eq!(game.difficulty(), &rules());
}

#[test]
fn a_run_at_tier_0_has_exactly_the_authored_numbers() {
    let (_scratch, mut game) = tiered(0);
    begin(&mut game);
    assert_eq!(first_enemy(&game), (30, 30));
    assert_eq!(boss(&game), (1000, 1000));
    assert!(game.tier_scale().is_neutral());
}

#[test]
fn a_tier_above_the_cap_plays_as_the_cap_and_says_the_real_tier() {
    let rules = RULES.replace("max_effective_tier: 20", "max_effective_tier: 5");
    let (_scratch, mut game) = tiered_with(9, 100.0, &rules);
    begin(&mut game);
    assert_eq!(first_enemy(&game), (45, 45), "x1.5, the cap's multiplier");
    assert_eq!(game.world_tier(), 5);
    game.coin_count = 10;
    game.abandon_run();
    assert_eq!(game.run_result().unwrap().coins_earned, 15);
    assert_eq!(
        game.result_bonus().unwrap().line(),
        "Tier 9 bonus: +50%",
        "the player's tier, the cap's bonus"
    );
}

#[test]
fn the_hub_and_a_game_with_no_hub_play_with_no_tier_on_the_rooms() {
    let (_scratch, mut game) = tiered(7);
    assert!(game.in_hub());
    assert!(game.tier_scale().is_neutral());
    assert_eq!(first_enemy(&game), (30, 30));
    // F5 in the hub rebuilds the rooms with no tier on them.
    game.reload_rooms().unwrap();
    assert_eq!(first_enemy(&game), (30, 30));
    assert!(game.tier_scale().is_neutral());

    let scratch = Scratch::new();
    scratch.write("difficulty.ron", RULES);
    let mut plain = GameState::try_new(&scratch.0).unwrap();
    plain
        .set_profile(Profile {
            tier: 7,
            ..Profile::default()
        })
        .unwrap();
    assert_eq!(plain.run_phase(), None);
    assert!(plain.tier_scale().is_neutral());
    plain.reload_rooms().unwrap();
    assert_eq!(first_enemy(&plain), (30, 30));
}

#[test]
fn leaving_the_result_screen_takes_the_tier_off_the_rooms() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    assert_eq!(first_enemy(&game), (39, 39));
    game.abandon_run();
    game.continue_after_result();
    assert!(game.in_hub());
    assert_eq!(first_enemy(&game), (30, 30));
    assert!(game.tier_scale().is_neutral());
}

#[test]
fn enemies_a_boss_summons_during_the_run_have_scaled_health_too() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(200.0, game.world_bounds());
    let (player, bounds) = (game.player().position(), game.world_bounds());
    game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_mut()
        .unwrap()
        .start_attack("Summon", player, bounds);
    for _ in 0..40 {
        game.update(input(), 0.1);
        if !game.rooms[RoomId::Boss.index()].enemies.is_empty() {
            break;
        }
    }
    let summoned = &game.rooms[RoomId::Boss.index()].enemies;
    assert!(!summoned.is_empty(), "the boss summoned enemies");
    let base = game.tuning().enemy.default_health;
    for enemy in summoned {
        assert_eq!(enemy.max_health(), (f64::from(base) * 1.3).round() as u16);
        assert_eq!(enemy.health(), enemy.max_health());
    }
}

// ---- damage -------------------------------------------------------------------------------------

#[test]
fn an_enemy_touch_at_tier_3_takes_the_scaled_damage_and_at_tier_0_the_authored() {
    let (_scratch, mut plain) = tiered(0);
    begin(&mut plain);
    assert_eq!(touch_loss(&mut plain), 10);

    let (_scratch, mut hard) = tiered(3);
    begin(&mut hard);
    assert_eq!(touch_loss(&mut hard), 16, "10 x 1.6");
}

#[test]
fn a_hazard_hurts_the_same_at_every_tier() {
    let loss_at = |tier| {
        let (_scratch, mut game) = tiered(tier);
        begin(&mut game);
        game.rooms[RoomId::First.index()].enemies.clear();
        game.rooms[RoomId::First.index()].hazards =
            vec![Hazard::from(&HazardPlacement::standard(400.0, 444.0))];
        let bounds = game.world_bounds();
        let terrain = &game.rooms[game.current].terrain;
        game.player
            .update_with_terrain(input(), 10.0, bounds, terrain);
        let before = game.player().health().current();
        game.player.place_on_floor(440.0, bounds);
        game.update(input(), 0.0);
        before - game.player().health().current()
    };
    let authored = loss_at(0);
    assert!(authored > 0, "the spikes hurt");
    assert_eq!(loss_at(3), authored);
    assert_eq!(loss_at(20), authored);
}

// ---- the tier is held for the run ----------------------------------------------------------------

#[test]
fn the_f4_tier_takes_effect_at_the_next_run_and_not_in_the_current_one() {
    let (_scratch, mut game) = tiered(2);
    begin(&mut game);
    assert_eq!(first_enemy(&game), (36, 36));
    let held = game.tier_scale();

    // F4 changes the tier mid-run: nothing in this run changes, not even after a reload.
    with_tier(&mut game, 5);
    assert_eq!(game.tier_scale(), held);
    assert_eq!(first_enemy(&game), (36, 36));
    game.reload_rooms().unwrap();
    assert_eq!(first_enemy(&game), (36, 36));
    assert_eq!(game.tier_scale(), held);

    // Back in the hub nothing has a tier on it; the next run reads the profile again.
    game.abandon_run();
    game.continue_after_result();
    assert_eq!(first_enemy(&game), (30, 30));
    with_tier(&mut game, 6);
    assert_eq!(
        first_enemy(&game),
        (30, 30),
        "the hub does not change either"
    );
    begin(&mut game);
    assert_eq!(first_enemy(&game), (48, 48));
    assert_eq!(game.tier_scale(), rules().scale(6));
}

// ---- the editor never sees a tier ---------------------------------------------------------------------

#[test]
fn the_editor_session_and_the_loaders_below_the_world_are_never_scaled() {
    let (scratch, mut game) = tiered(3);
    begin(&mut game);
    assert_eq!(first_enemy(&game), (39, 39));

    game.edit_room(RoomId::First).unwrap();
    let session = game.editor().unwrap();
    let built = session
        .draft
        .build(RoomId::First, &session.definitions)
        .unwrap();
    assert_eq!(
        built.enemies[0].health(),
        30,
        "the session builds what the file says"
    );
    assert_eq!(built.enemies[0].max_health(), 30);
    let spawned = session
        .definitions
        .spawn("chase", 560.0, 560.0, game.world_bounds())
        .unwrap();
    assert_eq!(spawned.health(), 30);

    let [first, _, boss_room] = crate::room::load_rooms(&scratch.0).unwrap();
    assert_eq!(first.enemies[0].health(), 30);
    assert_eq!(boss_room.boss_special.unwrap().max_health(), 1000);
    let extra = crate::editor::load_extra_rooms(&scratch.0, &session.definitions).unwrap();
    assert!(extra.iter().all(|room| room.enemies.is_empty()));
}

// ---- a rebuild in the middle of a run keeps the scale -----------------------------------------------

#[test]
fn an_f5_reload_in_a_run_rebuilds_enemies_and_bosses_at_the_runs_tier() {
    let (scratch, mut game) = tiered(3);
    begin(&mut game);
    let rewrite = |file: &str, from: &str, to: &str| {
        let path = scratch.0.join(file);
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains(from), "{file} has {from}");
        std::fs::write(path, text.replace(from, to)).unwrap();
    };
    rewrite("entities/chase.ron", "health: 30", "health: 50");
    rewrite("bosses/ash_warden.ron", "health: 1000", "health: 2000");

    game.reload_rooms().unwrap();

    assert_eq!(first_enemy(&game), (65, 65), "50 x 1.3");
    assert_eq!(boss(&game), (3200, 3200), "2000 x 1.6");
    assert_eq!(game.tier_scale(), rules().scale(3));
}

#[test]
fn an_f5_reload_keeps_a_boss_that_is_unchanged_with_the_health_it_has_left() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_mut()
        .unwrap()
        .damage_boss(100);
    game.reload_rooms().unwrap();
    assert_eq!(boss(&game), (1500, 1600), "wounded, and still scaled");
}

#[test]
fn an_editor_save_in_a_run_rebuilds_the_saved_room_at_the_runs_tier() {
    let (scratch, mut game) = tiered(3);
    begin(&mut game);
    let file = scratch.0.join("rooms").join("first.ron");
    game.rooms[RoomId::First.index()].enemies.clear();

    game.edit_room(RoomId::First).unwrap();
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();

    let room = game
        .rooms
        .iter()
        .find(|room| room.id == RoomId::First)
        .unwrap();
    assert_eq!(
        room.enemies.len(),
        1,
        "the saved room is rebuilt from its file"
    );
    assert_eq!(
        (room.enemies[0].health(), room.enemies[0].max_health()),
        (39, 39)
    );
    assert_eq!(boss(&game), (1600, 1600), "other rooms keep their scale");
    assert_eq!(game.tier_scale(), rules().scale(3));
    // The file still holds the placement as authored: a tier is never written back.
    let saved = std::fs::read_to_string(&file).unwrap();
    assert!(saved.contains("chase") && !saved.contains("39"), "{saved}");
}

#[test]
fn an_editor_save_with_a_changed_boss_rebuilds_it_at_the_runs_tier() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    game.edit_room(RoomId::Boss).unwrap();
    // Moving the boss in the draft counts as a changed boss placement.
    game.editor_mut()
        .unwrap()
        .draft
        .boss
        .as_mut()
        .unwrap()
        .position = (600.0, 436.0);
    game.save_editor().unwrap();
    game.toggle_editor().unwrap();
    assert_eq!(boss(&game), (1600, 1600));
}

// ---- a test room --------------------------------------------------------------------------------------

#[test]
fn a_test_room_plays_at_the_profiles_tier_and_writes_nothing() {
    let (scratch, mut game) = tiered(0);
    begin(&mut game);
    with_tier(&mut game, 5); // F4 set it; the test room reads it, a running run does not
    let files = ["rooms/first.ron", "entities/chase.ron", "run.ron"]
        .map(|name| (name, std::fs::read_to_string(scratch.0.join(name)).unwrap()));
    let profile_file = std::fs::read_to_string(profile_path(&scratch)).unwrap();
    let run_scale = game.tier_scale();

    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();

    assert_eq!(first_enemy(&game), (45, 45), "30 x 1.5");
    assert_eq!(game.tier_scale(), rules().scale(5));
    game.end_test_room().unwrap();

    assert_eq!(game.tier_scale(), run_scale, "the run's layer is put back");
    assert_eq!(
        first_enemy(&game),
        (30, 30),
        "the saved room was never touched"
    );
    for (name, text) in files {
        assert_eq!(
            std::fs::read_to_string(scratch.0.join(name)).unwrap(),
            text,
            "{name}"
        );
    }
    assert_eq!(
        std::fs::read_to_string(profile_path(&scratch)).unwrap(),
        profile_file
    );
}

#[test]
fn a_test_room_in_a_game_with_no_hub_uses_the_profile_tier_too() {
    let scratch = Scratch::new();
    scratch.write("difficulty.ron", RULES);
    let mut game = GameState::try_new(&scratch.0).unwrap();
    with_tier(&mut game, 3);
    assert_eq!(first_enemy(&game), (30, 30));

    game.toggle_editor().unwrap();
    game.start_test_room().unwrap();
    assert_eq!(first_enemy(&game), (39, 39));
    game.end_test_room().unwrap();

    assert_eq!(first_enemy(&game), (30, 30));
    assert!(game.tier_scale().is_neutral());
}

#[test]
fn a_test_room_at_tier_0_is_the_room_as_drawn() {
    let (_scratch, mut game) = tiered(0);
    game.edit_room(RoomId::First).unwrap();
    game.start_test_room().unwrap();
    assert_eq!(first_enemy(&game), (30, 30));
    assert!(game.tier_scale().is_neutral());
}

// ---- coins ------------------------------------------------------------------------------------------------

#[test]
fn a_runs_coins_are_scaled_at_the_end_before_the_share_that_is_kept() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    game.coin_count = 10;
    assert_eq!(
        game.coin_count(),
        10,
        "the counter in the run is what was collected"
    );

    game.abandon_run();

    let result = game.run_result().unwrap();
    assert_eq!((result.coins_earned, result.coins_banked), (13, 13));
    assert_eq!(game.profile().coins, 13);
    assert_eq!(game.result_bonus().unwrap().line(), "Tier 3 bonus: +30%");
    game.continue_after_result();
    assert!(game.result_bonus().is_none(), "only while the result shows");
}

#[test]
fn the_kept_share_is_taken_from_the_scaled_coins() {
    let (_scratch, mut game) = tiered_with(3, 50.0, RULES);
    begin(&mut game);
    game.coin_count = 10;
    game.abandon_run();
    let result = game.run_result().unwrap();
    assert_eq!((result.coins_earned, result.coins_banked), (13, 6));
}

#[test]
fn scaled_coins_are_rounded_to_the_nearest_whole_coin() {
    for (collected, banked) in [(0, 0), (1, 1), (2, 3), (5, 7), (7, 9), (100, 130)] {
        let (_scratch, mut game) = tiered(3);
        begin(&mut game);
        game.coin_count = collected;
        game.abandon_run();
        assert_eq!(game.profile().coins, banked, "{collected} coins at x1.3");
    }
}

#[test]
fn tier_0_pays_no_bonus_and_says_nothing() {
    let (_scratch, mut game) = tiered(0);
    begin(&mut game);
    game.coin_count = 10;
    game.abandon_run();
    assert_eq!(game.run_result().unwrap().coins_earned, 10);
    assert!(game.result_bonus().is_none());
}

#[test]
fn a_coin_percent_of_zero_pays_no_bonus_at_any_tier() {
    let rules = RULES.replace("coin_percent: 10.0", "coin_percent: 0.0");
    let (_scratch, mut game) = tiered_with(9, 100.0, &rules);
    begin(&mut game);
    game.coin_count = 10;
    game.abandon_run();
    assert_eq!(game.run_result().unwrap().coins_earned, 10);
    assert!(game.result_bonus().is_none());
}

#[test]
fn a_clear_pays_at_the_tier_it_was_played_at_and_then_raises_the_tier() {
    let (_scratch, mut game) = tiered(3);
    begin(&mut game);
    game.coin_count = 10;
    game.current = RoomId::Boss.index();
    game.player.place_on_floor(520.0, game.world_bounds());
    let boss = game.rooms[RoomId::Boss.index()]
        .boss_special
        .as_mut()
        .unwrap();
    boss.damage_boss(boss.health() - 1);

    game.update(swing(), 0.0);

    let result = game.run_result().expect("the boss fell");
    assert_eq!((result.tier_before, result.tier_after), (3, 4));
    assert_eq!(result.coins_earned, 13, "paid at tier 3, not 4");
    assert_eq!(game.result_bonus().unwrap().line(), "Tier 3 bonus: +30%");
    let lines = crate::run::result_lines_with_bonus(&result, game.profile(), game.result_bonus());
    assert_eq!(lines[0], "Coins earned: 13");
    assert_eq!(lines[1], "Tier 3 bonus: +30%");
    assert_eq!(lines.last().unwrap(), "Difficulty tier 3 -> 4");
}
