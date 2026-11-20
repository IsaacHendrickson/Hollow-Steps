use macroquad::math::{vec2, Vec2};

use super::*;
use crate::upgrades::UpgradeCatalog;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);

/// Three upgrades written for the tests (never the shipped numbers, which the owner tunes).
const SAMPLE: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "More health.",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "Faster.",
     effect: MoveSpeedPercent(5.0), costs: [40, 80, 140]),
    (id: "keen_edge", name: "Keen Edge", description: "Sharper.",
     effect: SwordDamage(3), costs: [50, 100, 180]),
])"#;

fn catalog() -> UpgradeCatalog {
    UpgradeCatalog::from_ron(SAMPLE).expect("the sample catalog is valid")
}

fn rich(coins: u32) -> Profile {
    Profile {
        coins,
        ..Profile::default()
    }
}

fn open_menu() -> ShopMenu {
    let mut menu = ShopMenu::default();
    menu.open();
    menu
}

// ---- rows -------------------------------------------------------------------------------------

#[test]
fn a_row_shows_the_level_the_next_price_and_what_a_level_gives() {
    let mut profile = rich(500);
    profile.upgrades.insert("vitality".into(), 2);
    let rows = rows(&catalog(), &profile);

    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].name, "Vitality");
    assert_eq!(rows[0].level, "2/3");
    assert_eq!(rows[0].cost, "100");
    assert_eq!(rows[0].effect, "+10 max health per level");
    assert_eq!(rows[1].level, "0/3");
    assert_eq!(rows[1].cost, "40");
    assert_eq!(rows[1].effect, "+5% move speed per level");
    assert_eq!(rows[2].effect, "+3 sword damage per level");
}

#[test]
fn the_last_level_shows_max_instead_of_a_price() {
    let mut profile = rich(1000);
    profile.upgrades.insert("keen_edge".into(), 3);
    let rows = rows(&catalog(), &profile);
    assert_eq!(rows[2].level, "3/3");
    assert_eq!(rows[2].cost, "Max");
    assert!(rows[2].maxed);
    assert!(!rows[2].affordable, "there is nothing left to buy");
    assert!(!rows[0].maxed);
}

#[test]
fn a_level_above_the_maximum_in_a_file_shows_as_the_maximum() {
    let mut profile = rich(0);
    profile.upgrades.insert("vitality".into(), 99);
    let rows = rows(&catalog(), &profile);
    assert_eq!(rows[0].level, "3/3");
    assert_eq!(rows[0].cost, "Max");
}

#[test]
fn a_row_is_affordable_exactly_when_the_coins_cover_the_next_price() {
    let rows_for = |coins| rows(&catalog(), &rich(coins));
    assert!(!rows_for(29)[0].affordable);
    assert!(rows_for(30)[0].affordable);
    assert!(!rows_for(30)[1].affordable, "swift feet costs 40");
    assert!(rows_for(40)[1].affordable);
}

#[test]
fn the_banked_coins_are_shown_as_a_line() {
    assert_eq!(coins_text(&rich(0)), "Banked coins: 0");
    assert_eq!(coins_text(&rich(1234)), "Banked coins: 1234");
}

#[test]
fn an_upgrade_in_the_profile_that_the_catalog_lacks_has_no_row() {
    let mut profile = rich(0);
    profile.upgrades.insert("gone".into(), 4);
    assert_eq!(rows(&catalog(), &profile).len(), 3);
}

// ---- the cursor -------------------------------------------------------------------------------

#[test]
fn the_shop_starts_closed_and_opens_and_closes() {
    let mut menu = ShopMenu::default();
    assert!(!menu.is_open());
    menu.open();
    assert!(menu.is_open());
    menu.close();
    assert!(!menu.is_open());
}

#[test]
fn the_cursor_moves_and_wraps_around_both_ends() {
    let mut menu = open_menu();
    assert_eq!(menu.selected(3), Some(0));
    menu.step(3, 1, 3);
    assert_eq!(menu.selected(3), Some(1));
    menu.step(3, 2, 3);
    assert_eq!(
        menu.selected(3),
        Some(0),
        "down from the last wraps to the first"
    );
    menu.step(3, -1, 3);
    assert_eq!(
        menu.selected(3),
        Some(2),
        "up from the first wraps to the last"
    );
}

#[test]
fn nothing_is_selected_when_nothing_is_for_sale() {
    let mut menu = open_menu();
    assert_eq!(menu.selected(0), None);
    menu.step(0, 1, 0);
    assert_eq!(menu.first(0, 0), 0);
    assert!(!menu.select(0, 0));
    let empty = UpgradeCatalog::from_ron("(upgrades: [])").expect("an empty shop is valid");
    assert!(menu.buy(&empty, &mut rich(9)).is_none());
}

#[test]
fn a_cursor_past_the_end_counts_as_the_last_upgrade() {
    let mut menu = open_menu();
    menu.step(10, -1, 10);
    assert_eq!(menu.selected(10), Some(9));
    // The list got shorter (a reload): the cursor is pulled back, not out of range.
    assert_eq!(menu.selected(3), Some(2));
}

#[test]
fn selecting_a_row_by_the_mouse_reports_only_a_change() {
    let mut menu = open_menu();
    assert!(menu.select(2, 3));
    assert_eq!(menu.selected(3), Some(2));
    assert!(!menu.select(2, 3), "the same row again is no change");
    assert!(!menu.select(3, 3), "a row that is not there is ignored");
    assert_eq!(menu.selected(3), Some(2));
}

#[test]
fn a_long_list_scrolls_to_keep_the_cursor_in_view() {
    let mut menu = open_menu();
    let (count, visible) = (10, 4);
    assert_eq!(menu.first(count, visible), 0);
    for _ in 0..4 {
        menu.step(count, 1, visible);
    }
    assert_eq!(menu.selected(count), Some(4));
    assert_eq!(
        menu.first(count, visible),
        1,
        "one step past the window scrolls one row"
    );
    menu.step(count, -1, visible);
    assert_eq!(menu.selected(count), Some(3));
    assert_eq!(
        menu.first(count, visible),
        1,
        "moving inside the window does not scroll"
    );
    menu.step(count, -4, visible);
    assert_eq!(
        menu.selected(count),
        Some(9),
        "up past the first wraps to the last"
    );
    assert_eq!(
        menu.first(count, visible),
        6,
        "wrapping to the end shows the last rows"
    );
    menu.step(count, 1, visible);
    assert_eq!(menu.selected(count), Some(0));
    assert_eq!(
        menu.first(count, visible),
        0,
        "wrapping to the start shows the first rows"
    );
}

// ---- buying -----------------------------------------------------------------------------------

#[test]
fn buying_pays_for_the_selected_upgrade_and_says_what_was_bought() {
    let mut menu = open_menu();
    let mut profile = rich(100);
    menu.step(3, 1, 3);

    assert_eq!(menu.buy(&catalog(), &mut profile), Some(Purchase::Bought));
    assert_eq!(profile.coins, 60);
    assert_eq!(profile.upgrades.get("swift_feet"), Some(&1));
    let note = menu.note().unwrap();
    assert_eq!(note.text, "Bought the Swift Feet: level 1");
    assert!(!note.refusal);
}

#[test]
fn an_unaffordable_upgrade_is_refused_with_its_reason_and_costs_nothing() {
    let mut menu = open_menu();
    let mut profile = rich(10);

    let outcome = menu.buy(&catalog(), &mut profile);
    assert!(matches!(outcome, Some(Purchase::Refused(_))), "{outcome:?}");
    assert_eq!(profile, rich(10), "a refusal changes nothing");
    let note = menu.note().unwrap();
    assert!(note.refusal);
    assert!(
        note.text.contains("Vitality") && note.text.contains("20 more"),
        "{}",
        note.text
    );
}

#[test]
fn an_upgrade_at_its_maximum_is_refused_with_its_reason() {
    let mut menu = open_menu();
    let mut profile = rich(1000);
    profile.upgrades.insert("vitality".into(), 3);
    let before = profile.clone();

    assert!(matches!(
        menu.buy(&catalog(), &mut profile),
        Some(Purchase::Refused(_))
    ));
    assert_eq!(profile, before);
    assert!(menu.note().unwrap().text.contains("maximum"));
}

#[test]
fn buying_until_the_maximum_charges_every_level_and_then_refuses() {
    let mut menu = open_menu();
    let mut profile = rich(190);
    let catalog = catalog();
    for _ in 0..3 {
        assert_eq!(menu.buy(&catalog, &mut profile), Some(Purchase::Bought));
    }
    assert_eq!(profile.coins, 0, "30 + 60 + 100");
    assert_eq!(profile.upgrades.get("vitality"), Some(&3));
    assert!(matches!(
        menu.buy(&catalog, &mut profile),
        Some(Purchase::Refused(_))
    ));
    assert_eq!(profile.coins, 0);
}

#[test]
fn the_message_stays_until_the_cursor_moves_or_the_shop_is_opened_again() {
    let mut menu = open_menu();
    let mut profile = rich(0);
    menu.buy(&catalog(), &mut profile);
    assert!(menu.note().is_some());

    menu.step(3, 1, 3);
    assert!(
        menu.note().is_none(),
        "moving the cursor clears the message"
    );

    menu.buy(&catalog(), &mut profile);
    menu.close();
    menu.open();
    assert!(
        menu.note().is_none(),
        "an old result is not kept for the next visit"
    );

    menu.buy(&catalog(), &mut profile);
    menu.step(3, 0, 3);
    assert!(menu.note().is_some(), "a step that does not move keeps it");
}

#[test]
fn a_message_can_be_set_by_the_game() {
    let mut menu = open_menu();
    menu.set_note("The profile did not save".into(), true);
    let note = menu.note().unwrap();
    assert!(note.refusal);
    assert_eq!(note.text, "The profile did not save");
}

#[test]
fn the_selected_upgrade_is_the_one_the_cursor_is_on() {
    let mut menu = open_menu();
    let catalog = catalog();
    assert_eq!(menu.selected_upgrade(&catalog).unwrap().id, "vitality");
    menu.step(3, -1, 3);
    assert_eq!(menu.selected_upgrade(&catalog).unwrap().id, "keen_edge");
}

// ---- layout -----------------------------------------------------------------------------------

fn bounds_of(layout: &ShopLayout) -> (f32, f32, f32, f32) {
    (
        layout.panel.x,
        layout.panel.y,
        layout.panel.right(),
        layout.panel.bottom(),
    )
}

#[test]
fn the_panel_is_centered_and_inside_the_view() {
    for upgrades in [0, 1, 3, 12, 40] {
        let layout = layout(VIEW, upgrades, 0);
        let (left, top, right, bottom) = bounds_of(&layout);
        assert!(left >= 0.0 && top >= 0.0, "{upgrades}: {left}, {top}");
        assert!(
            right <= VIEW.x && bottom <= VIEW.y,
            "{upgrades}: {right}, {bottom}"
        );
        assert!((layout.panel.center().x - VIEW.x / 2.0).abs() < 1e-3);
        assert!((layout.panel.center().y - VIEW.y / 2.0).abs() < 1e-3);
        assert_eq!(layout.panel.w, PANEL_WIDTH);
    }
}

#[test]
fn every_row_and_text_line_is_inside_the_panel() {
    for upgrades in [0, 3, 40] {
        let layout = layout(VIEW, upgrades, 0);
        let panel = layout.panel;
        for row in &layout.rows {
            assert!(
                row.rect.x >= panel.x
                    && row.rect.right() <= panel.right()
                    && row.rect.y >= panel.y
                    && row.rect.bottom() <= panel.bottom(),
                "{upgrades}: {row:?} outside {panel:?}"
            );
        }
        let lines = [layout.title, layout.coins, layout.header, layout.footer]
            .into_iter()
            .chain(layout.description)
            .chain(layout.message);
        for at in lines {
            assert!(panel.contains(at), "{upgrades}: {at:?} outside {panel:?}");
        }
        assert!(layout.text_width <= panel.w);
    }
}

#[test]
fn the_rows_are_in_order_and_do_not_overlap() {
    let layout = layout(VIEW, 3, 0);
    assert_eq!(layout.rows.len(), 3);
    for (position, row) in layout.rows.iter().enumerate() {
        assert_eq!(row.index, position);
        assert_eq!(row.rect.h, ROW_HEIGHT);
    }
    for pair in layout.rows.windows(2) {
        assert!(pair[0].rect.bottom() < pair[1].rect.y);
    }
    assert!(!layout.more_above && !layout.more_below);
}

#[test]
fn the_text_below_the_list_does_not_overlap_it_or_itself() {
    let layout = layout(VIEW, 3, 0);
    let list_bottom = layout.rows.last().unwrap().rect.bottom();
    assert!(layout.description[0].y > list_bottom);
    assert!(layout.description[1].y > layout.description[0].y);
    assert!(layout.message[0].y > layout.description[1].y);
    assert!(layout.message[1].y > layout.message[0].y);
    assert!(layout.footer.y > layout.message[1].y);
    assert!(layout.header.y < layout.rows[0].rect.y);
}

#[test]
fn a_long_list_shows_a_window_that_fits_the_screen() {
    let visible = visible_rows(VIEW, 40);
    assert!((1..40).contains(&visible), "{visible}");
    let layout = layout(VIEW, 40, 0);
    assert_eq!(layout.rows.len(), visible);
    assert!(!layout.more_above && layout.more_below);

    let layout = super::layout(VIEW, 40, 40 - visible);
    assert_eq!(layout.rows.first().unwrap().index, 40 - visible);
    assert_eq!(layout.rows.last().unwrap().index, 39);
    assert!(layout.more_above && !layout.more_below);
    // A start past the end is pulled back instead of showing rows that do not exist.
    let layout = super::layout(VIEW, 40, 500);
    assert_eq!(layout.rows.last().unwrap().index, 39);
}

#[test]
fn a_short_list_is_never_cut() {
    assert_eq!(visible_rows(VIEW, 3), 3);
    assert_eq!(visible_rows(VIEW, 1), 1);
    assert_eq!(visible_rows(VIEW, 0), 0);
}

#[test]
fn a_tiny_or_broken_view_still_shows_one_row_without_panicking() {
    for view in [vec2(100.0, 50.0), vec2(0.0, 0.0), vec2(f32::NAN, f32::NAN)] {
        assert_eq!(visible_rows(view, 5), 1);
        let layout = layout(view, 5, 2);
        assert_eq!(layout.rows.len(), 1);
    }
}

#[test]
fn the_mouse_finds_the_row_under_it_and_nothing_between_or_outside() {
    let layout = layout(VIEW, 3, 0);
    for row in &layout.rows {
        assert_eq!(upgrade_at(&layout, row.rect.center()), Some(row.index));
    }
    let gap = vec2(
        layout.rows[0].rect.center().x,
        layout.rows[0].rect.bottom() + 1.0,
    );
    assert_eq!(upgrade_at(&layout, gap), None);
    assert_eq!(upgrade_at(&layout, vec2(1.0, 1.0)), None);
}

#[test]
fn the_mouse_uses_the_scrolled_window_s_indices() {
    let visible = visible_rows(VIEW, 40);
    let layout = layout(VIEW, 40, 40 - visible);
    let top = layout.rows[0];
    assert_eq!(upgrade_at(&layout, top.rect.center()), Some(40 - visible));
}

#[test]
fn the_columns_of_a_row_are_in_order_and_inside_it() {
    let layout = layout(VIEW, 3, 0);
    let rect = layout.rows[0].rect;
    let columns = row_columns(rect);
    assert!(rect.x < columns.name_x);
    assert!(columns.name_x + columns.name_width <= columns.level_x);
    assert!(columns.level_x < columns.cost_x);
    assert!(columns.cost_x < columns.effect_x);
    assert!(columns.effect_x + columns.effect_width <= rect.right());
    assert!(columns.effect_width > 100.0, "room for the effect text");
}
