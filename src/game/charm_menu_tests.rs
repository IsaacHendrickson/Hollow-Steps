//! The inventory's Charms section through `GameState`: the cursor, equipping and the refusal
//! only work while the inventory is open, the preview says what equipping then does, and the
//! layout the drawing uses is the one the mouse hits. The pure rules are tested in
//! `crate::charm_menu`. The charms come from `tests/fixtures/charms.ron`: heart (+10 max
//! health, 15 mana), swift (+15 % speed, 10) and sharp (+5 sword damage, 20) are owned, ward
//! (not owned) is not in the list.
use std::path::{Path, PathBuf};

use super::charm_tests::Scratch;
use super::*;
use crate::charm_menu::charm_at;
use crate::charms::Toggle;
use crate::vitals::VitalsField;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

/// A game with the inventory open and the cursor on the first charm.
fn open_game() -> GameState {
    let mut game = GameState::try_new(&fixtures()).expect("fixed test assets must load");
    game.toggle_inventory();
    assert!(game.inventory_open());
    game
}

fn set_base_max_mana(game: &mut GameState, max_mana: f32) {
    let config = game
        .vitals_config()
        .with_field(VitalsField::MaxMana, max_mana)
        .unwrap();
    game.set_vitals_config(config).unwrap();
}

fn selected_id(game: &GameState) -> String {
    game.selected_charm()
        .expect("a charm is selected")
        .id
        .clone()
}

#[test]
fn the_list_is_the_owned_charms_with_the_first_selected() {
    let game = open_game();
    let ids: Vec<&str> = game.owned_charms().iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids, ["heart", "swift", "sharp"]);
    assert_eq!(selected_id(&game), "heart");
}

#[test]
fn nothing_works_while_the_inventory_is_closed() {
    let mut game = open_game();
    game.toggle_inventory();
    assert!(!game.inventory_open());
    let before = (game.charm_state().clone(), game.charm_menu().clone());
    game.charm_step(1, VIEW);
    game.charm_hover(2);
    assert_eq!(game.charm_press(), None);
    assert_eq!(game.charm_click(1), None);
    assert_eq!(
        (game.charm_state().clone(), game.charm_menu().clone()),
        before
    );
}

#[test]
fn up_and_down_move_the_cursor_and_wrap() {
    let mut game = open_game();
    game.charm_step(1, VIEW);
    assert_eq!(selected_id(&game), "swift");
    game.charm_step(1, VIEW);
    game.charm_step(1, VIEW);
    assert_eq!(
        selected_id(&game),
        "heart",
        "down from the last wraps to the first"
    );
    game.charm_step(-1, VIEW);
    assert_eq!(
        selected_id(&game),
        "sharp",
        "up from the first wraps to the last"
    );
}

#[test]
fn pressing_enter_equips_the_selected_charm_at_once_and_again_takes_it_off() {
    let mut game = open_game();
    let base_health = game.vitals().max_health();
    assert_eq!(game.charm_press(), Some(Toggle::Equipped));
    assert_eq!(game.charm_state().equipped(), ["heart"]);
    assert_eq!(game.vitals().max_health(), base_health + 10.0);
    assert_eq!(game.vitals().max_mana(), 85.0);
    assert_eq!(game.charm_press(), Some(Toggle::Unequipped));
    assert!(game.charm_state().equipped().is_empty());
    assert_eq!(game.vitals().max_health(), base_health);
    assert_eq!(game.vitals().max_mana(), 100.0);
}

#[test]
fn the_mana_preview_is_what_equipping_then_does() {
    let mut game = open_game();
    for step in [0, 1, 1, 0] {
        game.charm_step(step, VIEW);
        let (now, after) = game.charm_mana_preview().unwrap();
        assert_eq!(now, game.vitals().max_mana());
        let outcome = game.charm_press().unwrap();
        assert!(matches!(outcome, Toggle::Equipped | Toggle::Unequipped));
        assert_eq!(after, game.vitals().max_mana(), "after {step}");
        assert_eq!(
            after,
            game.charm_state().max_mana(game.charm_catalog(), 100.0)
        );
    }
}

#[test]
fn the_preview_of_a_charm_that_would_be_refused_is_the_mana_now() {
    let mut game = open_game();
    set_base_max_mana(&mut game, 30.0);
    game.charm_press(); // heart, 15
    game.charm_step(1, VIEW);
    game.charm_press(); // swift, 10
    game.charm_step(1, VIEW);
    assert_eq!(selected_id(&game), "sharp");
    let (now, after) = game.charm_mana_preview().unwrap();
    assert_eq!((now, after), (5.0, 5.0));
}

#[test]
fn a_refusal_changes_nothing_and_its_message_shows_until_the_cursor_moves() {
    let mut game = open_game();
    set_base_max_mana(&mut game, 30.0);
    game.charm_press();
    game.charm_step(1, VIEW);
    game.charm_press();
    game.charm_step(1, VIEW);
    let (state, player, vitals) = (
        game.charm_state().clone(),
        *game.player().config(),
        game.vitals().clone(),
    );
    let Some(Toggle::Refused(message)) = game.charm_press() else {
        panic!("25 + 20 mana is more than 30");
    };
    assert_eq!(game.charm_menu().message(), Some(message.as_str()));
    assert!(message.contains("15 more"), "{message}");
    assert_eq!(*game.charm_state(), state);
    assert_eq!(*game.player().config(), player);
    assert_eq!(*game.vitals(), vitals);
    game.charm_step(1, VIEW);
    assert_eq!(game.charm_menu().message(), None);
}

#[test]
fn a_refusal_is_not_kept_for_the_next_time_the_inventory_opens() {
    let mut game = open_game();
    set_base_max_mana(&mut game, 5.0);
    assert!(matches!(game.charm_press(), Some(Toggle::Refused(_))));
    assert!(game.charm_menu().message().is_some());
    game.toggle_inventory();
    game.toggle_inventory();
    assert_eq!(game.charm_menu().message(), None);
}

#[test]
fn resting_the_mouse_on_a_row_selects_it_without_equipping() {
    let mut game = open_game();
    game.charm_hover(2);
    assert_eq!(selected_id(&game), "sharp");
    assert!(game.charm_state().equipped().is_empty());
    game.charm_hover(9);
    assert_eq!(
        selected_id(&game),
        "sharp",
        "a row the list does not have is ignored"
    );
}

#[test]
fn a_click_selects_the_row_and_equips_or_takes_off_its_charm() {
    let mut game = open_game();
    assert_eq!(game.charm_click(2), Some(Toggle::Equipped));
    assert_eq!(game.charm_state().equipped(), ["sharp"]);
    assert_eq!(selected_id(&game), "sharp");
    assert_eq!(game.player().config().melee.damage, 15);
    assert_eq!(game.charm_click(2), Some(Toggle::Unequipped));
    assert_eq!(game.player().config().melee.damage, 10);
    assert_eq!(
        game.charm_click(7),
        None,
        "a click past the list does nothing"
    );
    assert!(game.charm_state().equipped().is_empty());
}

#[test]
fn the_drawn_rows_are_the_rows_the_mouse_hits() {
    let mut game = open_game();
    let layout = game.inventory_layout(VIEW);
    assert_eq!(layout.rows.len(), 3);
    for row in &layout.rows {
        assert_eq!(charm_at(&layout, row.rect.center()), Some(row.index));
    }
    let middle = layout.rows[1].rect.center();
    let index = charm_at(&layout, middle).unwrap();
    assert_eq!(game.charm_click(index), Some(Toggle::Equipped));
    assert_eq!(game.charm_state().equipped(), ["swift"]);
    // The panel stays the same size and place whatever is equipped.
    assert_eq!(game.inventory_layout(VIEW).panel, layout.panel);
}

#[test]
fn the_panel_grows_when_an_item_is_collected() {
    let mut game = open_game();
    let before = game.inventory_layout(VIEW).panel.h;
    game.player.collect_item(crate::item::Item::Dash);
    game.player.collect_item(crate::item::Item::Dash);
    let after = game.inventory_layout(VIEW);
    assert!(after.panel.h > before);
    assert!(after.panel.bottom() <= VIEW.y);
}

#[test]
fn charms_stay_equipped_when_the_inventory_is_closed_and_opened_again() {
    let mut game = open_game();
    game.charm_press();
    game.toggle_inventory();
    game.toggle_inventory();
    assert_eq!(game.charm_state().equipped(), ["heart"]);
    assert_eq!(selected_id(&game), "heart");
}

#[test]
fn a_reload_that_shortens_the_list_leaves_the_cursor_on_a_charm() {
    let scratch = Scratch::new();
    let mut game = GameState::try_new(&scratch.0).unwrap();
    game.toggle_inventory();
    game.charm_hover(2);
    assert_eq!(selected_id(&game), "sharp");
    // The file now calls the third charm something else: sharp is forgotten, and the cursor,
    // which was past the end of the shorter list, is on its last charm.
    let renamed = std::fs::read_to_string(fixtures().join("charms.ron"))
        .unwrap()
        .replace("id: \"sharp\"", "id: \"blade\"");
    scratch.write("charms.ron", &renamed);
    game.reload_rooms().unwrap();
    assert_eq!(game.owned_charms().len(), 2);
    assert_eq!(game.charm_menu().selected(2), Some(1));
    assert_eq!(selected_id(&game), "swift");
    assert_eq!(game.inventory_layout(VIEW).rows.len(), 2);
    assert_eq!(game.charm_press(), Some(Toggle::Equipped));
    assert_eq!(game.charm_state().equipped(), ["swift"]);
}
