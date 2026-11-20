//! The hub's upgrade shop through `GameState` (`shop_link`): where and how it opens, that the
//! world stands still while it is open, buying (paid, saved and in effect at once, nothing
//! refilled), refusals, and that it never opens in a run, on the result screen or in a test room.
//! The menu and the rules are tested in `crate::shop_menu` and `crate::upgrades`.
//!
//! The world is a private copy of `tests/fixtures` with a hub (`run_tests::world`) whose
//! `run.ron` puts the shop at x = 480 on the hub floor, and an `upgrades.ron` written here.
use super::charm_tests::Scratch;
use super::run_tests::{game_in, input, kill, profile_path, start_run, swing, walk, world, HUB};
use super::*;
use crate::hud_bars::{layout_bars, HudConfig};
use crate::profile::{self, Profile};
use crate::shop_menu::SHOP_SIZE;
use crate::upgrades::Purchase;
use macroquad::math::vec2;

const DT: f32 = 1.0 / 128.0;
const VIEW: Vec2 = Vec2::new(960.0, 540.0);
const SHOP_X: f32 = 480.0;

const UPGRADES: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "More health.",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "Faster.",
     effect: MoveSpeedPercent(5.0), costs: [40, 80, 140]),
    (id: "keen_edge", name: "Keen Edge", description: "Sharper.",
     effect: SwordDamage(3), costs: [50, 100, 180]),
])"#;

fn up() -> PlayerInput {
    PlayerInput {
        pickup_pressed: true,
        ..input()
    }
}

/// A hub world with a shop and the test upgrades.
fn shop_world() -> Scratch {
    let scratch = world(100.0);
    scratch.write(
        "run.ron",
        "(hub_room: Custom(0), start_room: First, final_boss_room: Boss, \
         coin_keep_percent: 100.0, shop: (480.0, 436.0))",
    );
    scratch.write("upgrades.ron", UPGRADES);
    scratch
}

/// The game of `scratch` with `coins` banked.
fn game_with_coins(scratch: &Scratch, coins: u32) -> GameState {
    let mut game = game_in(scratch);
    game.set_profile(Profile {
        coins,
        ..Profile::default()
    })
    .unwrap();
    game
}

fn stand_at_the_shop(game: &mut GameState) {
    game.player.place_on_floor(SHOP_X, game.world_bounds());
    game.update(input(), DT);
}

/// The shop is open, with the player standing at it.
fn open_shop(game: &mut GameState) {
    stand_at_the_shop(game);
    game.update(up(), DT);
    assert!(game.shop_open(), "the shop must open for the test");
}

fn saved_profile(scratch: &Scratch) -> Profile {
    let (profile, problem) = profile::load(&profile_path(scratch));
    assert_eq!(problem, None);
    profile
}

// ---- opening and closing ----------------------------------------------------------------------

#[test]
fn the_shop_stands_where_run_ron_puts_it_and_only_there() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    assert_eq!(game.shop_position(), Some(vec2(SHOP_X, 436.0)));

    game.player.place_on_floor(180.0, game.world_bounds());
    game.update(input(), DT);
    assert!(!game.shop_in_reach(), "the spawn is far from the shop");
    stand_at_the_shop(&mut game);
    assert!(game.shop_in_reach());
    // Standing a little off still counts; across the room does not.
    game.player
        .place_on_floor(SHOP_X + SHOP_SIZE.x, game.world_bounds());
    game.update(input(), DT);
    assert!(game.shop_in_reach());
    game.player
        .place_on_floor(SHOP_X + 300.0, game.world_bounds());
    game.update(input(), DT);
    assert!(!game.shop_in_reach());
}

#[test]
fn up_in_reach_opens_the_shop_and_the_world_stands_still() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    stand_at_the_shop(&mut game);
    assert!(!game.shop_open() && !game.is_paused());

    game.update(up(), DT);
    assert!(game.shop_open());
    assert!(game.is_paused());

    let position = game.player().position();
    for _ in 0..30 {
        game.update(walk(1.0), 0.1);
    }
    assert_eq!(game.player().position(), position, "the world is frozen");
    assert_eq!(game.room().id, HUB, "and walking does not leave the hub");
}

#[test]
fn up_away_from_the_shop_opens_nothing() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    game.update(up(), DT);
    assert!(!game.shop_open());
}

#[test]
fn up_with_x_is_an_upward_strike_and_not_the_shop() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    stand_at_the_shop(&mut game);
    game.update(
        PlayerInput {
            pickup_pressed: true,
            up_held: true,
            ..swing()
        },
        DT,
    );
    assert!(!game.shop_open());
}

#[test]
fn escape_closes_the_shop_first_and_only_then_opens_the_pause_menu() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    open_shop(&mut game);

    game.press_escape();
    assert!(!game.shop_open());
    assert!(!game.pause_menu.is_open(), "the same press does not pause");
    assert!(!game.is_paused());

    game.press_escape();
    assert!(game.pause_menu.is_open());
}

#[test]
fn the_inventory_key_does_nothing_while_the_shop_is_open() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    open_shop(&mut game);
    game.toggle_inventory();
    assert!(!game.inventory_open());
    assert!(game.shop_open());
}

#[test]
fn the_shop_cannot_open_over_the_inventory_or_the_pause_menu() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    stand_at_the_shop(&mut game);

    game.toggle_inventory();
    game.update(up(), DT);
    assert!(!game.shop_open(), "a frozen world takes no input");
    game.toggle_inventory();

    game.press_escape();
    game.update(up(), DT);
    assert!(!game.shop_open());
}

#[test]
fn opening_the_editor_closes_the_shop() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    open_shop(&mut game);
    game.toggle_editor().unwrap();
    assert!(game.editor.is_some());
    assert!(!game.shop_open(), "no stale open flag behind the editor");
}

#[test]
fn an_item_in_reach_takes_the_press_before_the_shop_and_a_door_waits_for_it() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    // A doorway right where the shop stands: Up opens the shop, it does not use the door.
    let door = Rect::new(SHOP_X - 24.0, 388.0, 48.0, 72.0);
    game.rooms[game.current]
        .exits
        .push(crate::room::Exit::doorway(
            door,
            RoomId::First,
            vec2(180.0, 436.0),
        ));
    stand_at_the_shop(&mut game);
    assert!(
        game.door_in_reach().is_none(),
        "no prompt for the door either"
    );

    game.update(up(), DT);

    assert!(game.shop_open());
    assert_eq!(game.room().id, HUB);
    assert!(!game.run_in_progress());
}

// ---- where it cannot open ---------------------------------------------------------------------

#[test]
fn a_game_whose_run_ron_has_no_shop_has_none() {
    let scratch = world(100.0);
    let mut game = game_in(&scratch);
    assert_eq!(game.shop_position(), None);
    stand_at_the_shop(&mut game);
    game.update(up(), DT);
    assert!(!game.shop_in_reach() && !game.shop_open());
}

#[test]
fn a_shop_outside_the_hub_room_stops_the_game_from_loading_and_names_the_file() {
    let scratch = world(100.0);
    scratch.write(
        "run.ron",
        "(hub_room: Custom(0), start_room: First, final_boss_room: Boss, \
         coin_keep_percent: 100.0, shop: (2000.0, 436.0))",
    );
    let Err(error) = GameState::try_new(&scratch.0) else {
        panic!("a shop outside the hub must not load");
    };
    assert!(
        error.contains("run.ron") && error.contains("shop") && error.contains("hub room"),
        "{error}"
    );
}

#[test]
fn the_shop_cannot_open_in_a_run() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    start_run(&mut game);
    // Right where the shop would be in the hub.
    game.player.place_on_floor(SHOP_X, game.world_bounds());
    game.update(input(), DT);

    assert_eq!(game.shop_position(), None);
    assert!(!game.shop_in_reach());
    game.update(up(), DT);
    assert!(!game.shop_open());
    assert_eq!(game.shop_press(), None);
}

#[test]
fn the_shop_is_closed_on_the_result_screen_and_back_in_the_hub_afterwards() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    start_run(&mut game);
    kill(&mut game);
    assert!(game.run_result().is_some());
    assert_eq!(game.shop_position(), None, "no shop under a result screen");

    game.continue_after_result();
    assert_eq!(game.room().id, HUB);
    assert!(game.shop_position().is_some());
    open_shop(&mut game);
}

#[test]
fn the_shop_cannot_open_in_a_test_room_of_the_hub() {
    let scratch = shop_world();
    let mut game = game_in(&scratch);
    game.edit_room(HUB).unwrap();
    game.start_test_room().unwrap();
    assert!(game.testing_room());
    game.player.place_on_floor(SHOP_X, game.world_bounds());
    game.update(input(), DT);

    assert_eq!(game.shop_position(), None);
    game.update(up(), DT);
    assert!(!game.shop_open());
}

// ---- buying -----------------------------------------------------------------------------------

#[test]
fn a_purchase_is_paid_saved_and_in_effect_at_once_and_nothing_is_refilled() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    let base = game.tuning().player.max_health;
    let (position, health) = (game.player().position(), game.vitals().health());

    assert_eq!(game.shop_press(), Some(Purchase::Bought));

    assert_eq!(game.profile().coins, 70);
    assert_eq!(game.profile().upgrades.get("vitality"), Some(&1));
    assert_eq!(game.player().config().max_health, base + 10);
    assert_eq!(game.player().health().max(), base + 10);
    // The existing max-health rule: the same amount is added, the rest is not refilled.
    assert!((game.vitals().health() - (health + 10.0)).abs() < 1e-3);
    assert_eq!(game.player().position(), position, "not moved");
    assert!(game.shop_open(), "the shop stays open to buy more");
    assert_eq!(saved_profile(&scratch), *game.profile(), "saved at once");
    assert_eq!(
        game.shop_menu().note().map(|note| note.text.as_str()),
        Some("Bought the Vitality: level 1")
    );
}

#[test]
fn a_hurt_player_stays_hurt_after_buying() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    let max = game.vitals().max_health();
    game.vitals.damage(max / 2.0);
    let hurt = game.vitals().health();
    assert!(hurt > 0.0 && hurt < max, "hurt but alive: {hurt} of {max}");

    game.shop_press();

    // The same 10 is added to the health as to the maximum; the missing half is still missing.
    assert!((game.vitals().max_health() - (max + 10.0)).abs() < 1e-3);
    assert!((game.vitals().health() - (hurt + 10.0)).abs() < 1e-3);
    assert!(game.vitals().health() < game.vitals().max_health() - 1.0);
}

#[test]
fn a_refused_purchase_changes_nothing_and_says_why() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 10);
    open_shop(&mut game);
    let (profile, config) = (game.profile().clone(), *game.player().config());
    let saved = std::fs::read_to_string(profile_path(&scratch)).ok();

    let outcome = game.shop_press();

    assert!(matches!(outcome, Some(Purchase::Refused(_))), "{outcome:?}");
    assert_eq!(*game.profile(), profile);
    assert_eq!(*game.player().config(), config);
    assert_eq!(std::fs::read_to_string(profile_path(&scratch)).ok(), saved);
    let note = game.shop_menu().note().unwrap();
    assert!(
        note.refusal && note.text.contains("Not enough coins"),
        "{note:?}"
    );
}

#[test]
fn buying_to_the_maximum_charges_every_level_and_then_refuses() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 200);
    open_shop(&mut game);
    let base = f32::from(game.tuning().player.max_health);
    for _ in 0..3 {
        assert_eq!(game.shop_press(), Some(Purchase::Bought));
    }
    assert_eq!(game.profile().coins, 10, "30 + 60 + 100 out of 200");

    assert!(matches!(game.shop_press(), Some(Purchase::Refused(_))));
    assert_eq!(game.profile().coins, 10);
    assert!(game.shop_menu().note().unwrap().text.contains("maximum"));
    // The bought health is above the baseline, so the bar shows it as added capacity.
    let values = game.bar_values();
    assert!((values.max_health - (base + 30.0)).abs() < 1e-3);
    assert!((values.base_max_health - base).abs() < 1e-3);
}

#[test]
fn every_purchase_makes_the_original_part_of_the_bar_shorter_and_the_added_part_longer() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 200);
    open_shop(&mut game);
    let anchor = Rect::new(20.0, 20.0, 200.0, 20.0);
    let parts = |game: &GameState| {
        let layout = layout_bars(&HudConfig::default(), game.bar_values(), anchor);
        (layout.health_base.w, layout.health_extension.w)
    };
    let (mut base_part, mut added_part) = parts(&game);
    assert!(
        (base_part - anchor.w).abs() < 1e-3 && added_part == 0.0,
        "nothing bought"
    );

    for level in 1..=3 {
        assert_eq!(game.shop_press(), Some(Purchase::Bought));
        let (now_base, now_added) = parts(&game);
        assert!(
            now_base < base_part,
            "level {level}: {now_base} !< {base_part}"
        );
        assert!(
            now_added > added_part,
            "level {level}: {now_added} !> {added_part}"
        );
        assert!(
            (now_base + now_added - anchor.w).abs() < 1e-3,
            "the bar keeps its length"
        );
        (base_part, added_part) = (now_base, now_added);
    }
}

#[test]
fn the_shop_sells_what_the_cursor_is_on_and_a_click_buys_that_row() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    let base = game.tuning().player.move_speed;

    game.shop_step(1, VIEW);
    assert_eq!(game.selected_upgrade().unwrap().id, "swift_feet");
    game.shop_step(-1, VIEW);
    game.shop_step(-1, VIEW);
    assert_eq!(game.selected_upgrade().unwrap().id, "keen_edge", "wraps");

    assert_eq!(game.shop_click(1), Some(Purchase::Bought));
    assert_eq!(game.profile().upgrades.get("swift_feet"), Some(&1));
    assert_eq!(game.selected_upgrade().unwrap().id, "swift_feet");
    assert!((game.player().config().move_speed - base * 1.05).abs() < 1e-2);
    assert_eq!(game.profile().coins, 60);
}

#[test]
fn hovering_a_row_selects_it_without_buying() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 500);
    open_shop(&mut game);
    game.shop_hover(2);
    assert_eq!(game.selected_upgrade().unwrap().id, "keen_edge");
    game.shop_hover(9);
    assert_eq!(
        game.selected_upgrade().unwrap().id,
        "keen_edge",
        "no such row"
    );
    assert_eq!(game.profile().coins, 500);
    assert!(game.profile().upgrades.is_empty());
}

#[test]
fn a_click_on_a_row_that_is_not_there_buys_nothing() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 500);
    open_shop(&mut game);
    assert_eq!(game.shop_click(7), None);
    assert_eq!(game.profile().coins, 500);
}

#[test]
fn a_closed_shop_ignores_every_call() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 500);
    game.shop_step(1, VIEW);
    game.shop_hover(1);
    assert_eq!(game.shop_press(), None);
    assert_eq!(game.shop_click(0), None);
    assert_eq!(game.profile().coins, 500);
    assert_eq!(game.selected_upgrade().unwrap().id, "vitality");
}

#[test]
fn the_rows_follow_the_profile_even_when_it_changes_under_the_open_shop() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    game.shop_press();
    assert_eq!(game.shop_rows()[0].level, "1/3");

    // F4's Reset profile while the shop is open: the levels are gone, the rows say so.
    game.set_profile(Profile::default()).unwrap();
    assert_eq!(game.shop_rows()[0].level, "0/3");
    assert_eq!(game.upgrade_catalog().upgrades().len(), 3);
}

#[test]
fn a_purchase_that_cannot_be_saved_stands_for_the_session_and_says_so() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    // A folder where the staged file goes makes every save fail.
    std::fs::create_dir(scratch.0.join("profile.ron.tmp")).unwrap();

    assert_eq!(game.shop_press(), Some(Purchase::Bought));

    assert_eq!(game.profile().upgrades.get("vitality"), Some(&1));
    let note = game.shop_menu().note().unwrap();
    assert!(
        note.refusal && note.text.contains("did not save"),
        "{note:?}"
    );
}

// ---- keeping what was bought ------------------------------------------------------------------

#[test]
fn bought_levels_survive_a_run_a_death_and_a_restart_of_the_game() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    game.shop_press();
    game.close_shop();
    let base = game.tuning().player.max_health;

    start_run(&mut game);
    assert_eq!(game.player().config().max_health, base + 10, "in the run");
    kill(&mut game);
    assert_eq!(
        game.profile().upgrades.get("vitality"),
        Some(&1),
        "after dying"
    );
    game.continue_after_result();
    assert_eq!(
        game.player().config().max_health,
        base + 10,
        "back in the hub"
    );

    // A new game on the same folder reads the level from the saved profile.
    let restarted = game_in(&scratch);
    assert_eq!(restarted.profile().upgrades.get("vitality"), Some(&1));
    assert_eq!(restarted.player().config().max_health, base + 10);
}

#[test]
fn a_bought_upgrade_and_an_equipped_charm_add_up() {
    let scratch = shop_world();
    let mut game = game_with_coins(&scratch, 100);
    open_shop(&mut game);
    game.shop_press();
    let base = game.tuning().player.max_health;

    assert_eq!(game.toggle_charm("heart"), crate::charms::Toggle::Equipped);
    assert_eq!(
        game.player().config().max_health,
        base + 20,
        "10 bought + 10 heart"
    );
}

// ---- the list on screen -----------------------------------------------------------------------

#[test]
fn the_layout_follows_the_cursor_when_the_list_is_longer_than_the_screen() {
    let scratch = shop_world();
    let lines: Vec<String> = (0..40)
        .map(|n| {
            format!(
                r#"(id: "u{n}", name: "Upgrade {n}", description: "", effect: MaxHealth(1), costs: [1]),"#
            )
        })
        .collect();
    scratch.write(
        "upgrades.ron",
        &format!("(upgrades: [{}])", lines.join("\n")),
    );
    let mut game = game_with_coins(&scratch, 0);
    open_shop(&mut game);

    let layout = game.shop_layout(VIEW);
    assert!(layout.rows.len() < 40 && layout.more_below && !layout.more_above);
    assert_eq!(layout.rows[0].index, 0);

    game.shop_step(-1, VIEW);
    assert_eq!(game.selected_upgrade().unwrap().id, "u39");
    let layout = game.shop_layout(VIEW);
    assert_eq!(
        layout.rows.last().unwrap().index,
        39,
        "the last row is in view"
    );
    assert!(layout.more_above && !layout.more_below);
}
