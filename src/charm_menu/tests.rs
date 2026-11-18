use macroquad::math::{vec2, Rect, Vec2};

use super::*;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);
/// The base maximum mana the tests budget against.
const MANA: f32 = 100.0;

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} is not close to {b}");
}

/// A charm line; the effect is a +1 max health, which these tests never look at.
fn line(id: &str, name: &str, cost: f32, owned: bool) -> String {
    format!(
        r#"(id: "{id}", name: "{name}", description: "About the {name}.", mana_cost: {cost:?},
        effect: MaxHealth(1), color: (10, 20, 30, 255), owned_at_start: {owned}),"#
    )
}

fn catalog_of(lines: &[String]) -> CharmCatalog {
    let text = format!("(charms: [{}])", lines.join("\n"));
    CharmCatalog::from_ron(&text, MANA).expect("the test charms must be valid")
}

/// Heart (15), swift (10) and sharp (20) are owned; ward (5) is not.
fn catalog() -> CharmCatalog {
    catalog_of(&[
        line("heart", "Heart Charm", 15.0, true),
        line("swift", "Swift Charm", 10.0, true),
        line("sharp", "Sharp Charm", 20.0, true),
        line("ward", "Ward Charm", 5.0, false),
    ])
}

/// `count` owned charms of cost 1 each.
fn many(count: usize) -> CharmCatalog {
    let lines: Vec<String> = (0..count)
        .map(|n| line(&format!("c{n}"), &format!("Charm {n}"), 1.0, true))
        .collect();
    catalog_of(&lines)
}

fn state(catalog: &CharmCatalog) -> CharmState {
    CharmState::starting(catalog)
}

fn names(catalog: &CharmCatalog, state: &CharmState) -> Vec<String> {
    owned_charms(catalog, state)
        .iter()
        .map(|charm| charm.name.clone())
        .collect()
}

// ---- the list and the cursor ------------------------------------------------------------

#[test]
fn the_list_is_the_owned_charms_in_order_and_leaves_out_the_rest() {
    let catalog = catalog();
    assert_eq!(
        names(&catalog, &state(&catalog)),
        ["Heart Charm", "Swift Charm", "Sharp Charm"]
    );
}

#[test]
fn the_cursor_starts_on_the_first_charm_and_steps_down_and_up() {
    let mut menu = CharmMenu::default();
    assert_eq!(menu.selected(3), Some(0));
    menu.step(3, 1, 3);
    assert_eq!(menu.selected(3), Some(1));
    menu.step(3, 1, 3);
    assert_eq!(menu.selected(3), Some(2));
    menu.step(3, -1, 3);
    assert_eq!(menu.selected(3), Some(1));
    menu.step(3, -1, 3);
    menu.step(3, 0, 3);
    assert_eq!(menu.selected(3), Some(0));
}

#[test]
fn the_cursor_wraps_around_both_ends() {
    let mut menu = CharmMenu::default();
    menu.step(3, -1, 3);
    assert_eq!(
        menu.selected(3),
        Some(2),
        "up from the first goes to the last"
    );
    menu.step(3, 1, 3);
    assert_eq!(
        menu.selected(3),
        Some(0),
        "down from the last goes to the first"
    );
    menu.step(3, 4, 3);
    assert_eq!(
        menu.selected(3),
        Some(1),
        "a long step wraps as often as it needs"
    );
    menu.step(3, -7, 3);
    assert_eq!(menu.selected(3), Some(0));
}

#[test]
fn a_cursor_past_the_end_counts_as_the_last_charm() {
    let mut menu = CharmMenu::default();
    assert!(menu.select(4, 5));
    assert_eq!(menu.selected(5), Some(4));
    // The list got shorter (a reload dropped charms): the cursor is on the last one left.
    assert_eq!(menu.selected(2), Some(1));
    menu.step(2, 1, 2);
    assert_eq!(menu.selected(2), Some(0));
}

#[test]
fn selecting_an_index_the_list_does_not_have_is_ignored() {
    let mut menu = CharmMenu::default();
    assert!(!menu.select(3, 3));
    assert!(!menu.select(0, 3), "already selected is no change");
    assert!(menu.select(2, 3));
    assert_eq!(menu.selected(3), Some(2));
}

#[test]
fn an_empty_list_has_no_selection_and_nothing_to_toggle() {
    let catalog = catalog_of(&[line("ward", "Ward Charm", 5.0, false)]);
    let mut owned = state(&catalog);
    assert!(owned_charms(&catalog, &owned).is_empty());
    let mut menu = CharmMenu::default();
    assert_eq!(menu.selected(0), None);
    assert_eq!(menu.first(0, 0), 0);
    menu.step(0, 1, 0);
    menu.step(0, -1, 0);
    assert!(!menu.select(0, 0));
    assert_eq!(menu.toggle(&catalog, &mut owned, MANA), None);
    assert!(owned.equipped().is_empty());
    assert_eq!(menu, CharmMenu::default());
}

// ---- equipping --------------------------------------------------------------------------

#[test]
fn toggling_equips_the_selected_charm_and_toggling_again_takes_it_off() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    let mut menu = CharmMenu::default();
    menu.step(3, 1, 3);
    assert_eq!(
        menu.toggle(&catalog, &mut owned, MANA),
        Some(Toggle::Equipped)
    );
    assert_eq!(owned.equipped(), ["swift"]);
    assert_eq!(
        menu.toggle(&catalog, &mut owned, MANA),
        Some(Toggle::Unequipped)
    );
    assert!(owned.equipped().is_empty());
    assert_eq!(menu.message(), None);
}

#[test]
fn charms_are_equipped_in_the_order_they_are_chosen() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    let mut menu = CharmMenu::default();
    menu.step(3, 2, 3);
    menu.toggle(&catalog, &mut owned, MANA);
    menu.step(3, -2, 3);
    menu.toggle(&catalog, &mut owned, MANA);
    assert_eq!(owned.equipped(), ["sharp", "heart"]);
}

#[test]
fn a_refusal_keeps_the_state_and_shows_the_message() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    let mut menu = CharmMenu::default();
    // Heart 15 and swift 10 fit in 30 mana; sharp, which costs 20, does not.
    let budget = 30.0;
    assert_eq!(
        menu.toggle(&catalog, &mut owned, budget),
        Some(Toggle::Equipped)
    );
    menu.step(3, 1, 3);
    assert_eq!(
        menu.toggle(&catalog, &mut owned, budget),
        Some(Toggle::Equipped)
    );
    menu.step(3, 1, 3);
    let before = owned.clone();
    let refused = menu.toggle(&catalog, &mut owned, budget);
    let Some(Toggle::Refused(why)) = refused else {
        panic!("equipping over the budget must be refused, not {refused:?}");
    };
    assert_eq!(owned, before, "a refusal changes nothing");
    assert!(
        why.contains("Not enough mana") && why.contains("Sharp Charm"),
        "{why}"
    );
    assert_eq!(menu.message(), Some(why.as_str()));
}

#[test]
fn the_refusal_goes_when_the_cursor_moves_or_something_is_toggled() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    let mut menu = CharmMenu::default();
    // No mana at all: everything is refused.
    assert!(matches!(
        menu.toggle(&catalog, &mut owned, 0.0),
        Some(Toggle::Refused(_))
    ));
    assert!(menu.message().is_some());
    // Resting on the same row keeps it; moving to another clears it.
    assert!(!menu.select(0, 3));
    assert!(menu.message().is_some());
    menu.step(3, 1, 3);
    assert_eq!(menu.message(), None);
    // A refusal, then a successful toggle with enough mana, clears it too.
    assert!(matches!(
        menu.toggle(&catalog, &mut owned, 0.0),
        Some(Toggle::Refused(_))
    ));
    assert_eq!(
        menu.toggle(&catalog, &mut owned, MANA),
        Some(Toggle::Equipped)
    );
    assert_eq!(menu.message(), None);
    assert!(matches!(
        menu.toggle(&catalog, &mut owned, 0.0),
        Some(Toggle::Unequipped)
    ));
    // Clearing by hand.
    assert!(matches!(
        menu.toggle(&catalog, &mut owned, 0.0),
        Some(Toggle::Refused(_))
    ));
    menu.clear_message();
    assert_eq!(menu.message(), None);
}

#[test]
fn taking_a_charm_off_is_never_refused() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    let mut menu = CharmMenu::default();
    menu.toggle(&catalog, &mut owned, MANA);
    // The budget shrank under the charm (F4 lowered the base): it still comes off.
    assert_eq!(
        menu.toggle(&catalog, &mut owned, 0.0),
        Some(Toggle::Unequipped)
    );
}

#[test]
fn toggling_deep_in_a_long_list_equips_the_charm_the_cursor_is_on() {
    let catalog = many(12);
    let mut owned = state(&catalog);
    let mut menu = CharmMenu::default();
    for _ in 0..7 {
        menu.step(12, 1, 4);
    }
    assert_eq!(
        menu.toggle(&catalog, &mut owned, MANA),
        Some(Toggle::Equipped)
    );
    assert_eq!(owned.equipped(), ["c7"]);
    assert_eq!(
        owned_charms(&catalog, &owned)[menu.selected(12).unwrap()].id,
        "c7"
    );
}

// ---- the mana preview -------------------------------------------------------------------

#[test]
fn the_mana_after_matches_what_toggling_does_for_every_charm_and_every_state() {
    let catalog = catalog();
    let owned_ids = ["heart", "swift", "sharp"];
    for budget in [100.0, 30.0, 25.0, 10.0, 0.0] {
        for mask in 0..8u32 {
            let mut owned = state(&catalog);
            for (bit, id) in owned_ids.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    let _ = owned.equip(&catalog, id, budget);
                }
            }
            for id in owned_ids {
                let preview = max_mana_after(&catalog, &owned, id, budget);
                let mut toggled = owned.clone();
                toggled.toggle(&catalog, id, budget);
                close(preview, toggled.max_mana(&catalog, budget));
            }
        }
    }
}

#[test]
fn the_mana_after_of_a_toggle_that_would_be_refused_is_the_mana_now() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    owned.equip(&catalog, "heart", 30.0).unwrap();
    owned.equip(&catalog, "swift", 30.0).unwrap();
    let now = owned.max_mana(&catalog, 30.0);
    close(now, 5.0);
    close(max_mana_after(&catalog, &owned, "sharp", 30.0), now);
    // An id that is not in the catalog is refused too.
    close(max_mana_after(&catalog, &owned, "nope", 30.0), now);
}

#[test]
fn equipping_lowers_the_mana_by_the_cost_and_taking_off_gives_it_back() {
    let catalog = catalog();
    let mut owned = state(&catalog);
    close(max_mana_after(&catalog, &owned, "heart", MANA), 85.0);
    owned.equip(&catalog, "heart", MANA).unwrap();
    close(max_mana_after(&catalog, &owned, "heart", MANA), 100.0);
    close(max_mana_after(&catalog, &owned, "sharp", MANA), 65.0);
}

#[test]
fn the_mana_line_says_both_numbers() {
    assert_eq!(mana_text(100.0, 85.0), "Max mana: 100 now, 85 after");
    assert_eq!(mana_text(7.5, 0.0), "Max mana: 7.5 now, 0 after");
}

// ---- scrolling --------------------------------------------------------------------------

#[test]
fn a_list_that_fits_never_scrolls() {
    let mut menu = CharmMenu::default();
    for _ in 0..8 {
        menu.step(4, 1, 4);
        assert_eq!(menu.first(4, 4), 0);
    }
}

#[test]
fn the_window_follows_the_cursor_only_as_far_as_it_must() {
    let mut menu = CharmMenu::default();
    let (count, visible) = (10, 4);
    for expected in [0, 0, 0, 0, 1, 2, 3, 4, 5, 6] {
        assert_eq!(menu.first(count, visible), expected);
        menu.step(count, 1, visible);
    }
    // Back up: the window stays until the cursor reaches its top row.
    assert_eq!(menu.selected(count), Some(0), "wrapped to the top");
    assert_eq!(menu.first(count, visible), 0);
    menu.step(count, -1, visible);
    assert_eq!(menu.selected(count), Some(9));
    assert_eq!(
        menu.first(count, visible),
        6,
        "wrapping to the end shows the end"
    );
    for _ in 0..3 {
        menu.step(count, -1, visible);
        assert_eq!(menu.first(count, visible), 6);
    }
    menu.step(count, -1, visible);
    assert_eq!(menu.selected(count), Some(5));
    assert_eq!(menu.first(count, visible), 5);
}

#[test]
fn the_selected_charm_is_always_in_the_window() {
    let (count, visible) = (9, 3);
    let mut menu = CharmMenu::default();
    for step in [1, 1, 5, -2, -7, 3, 1, 1, 1, -1, -1, 4] {
        menu.step(count, step, visible);
        let selected = menu.selected(count).unwrap();
        let first = menu.first(count, visible);
        assert!(
            first <= selected && selected < first + visible,
            "{first} {selected}"
        );
        assert!(first + visible <= count);
    }
}

#[test]
fn resting_the_mouse_on_a_visible_row_never_moves_the_window() {
    let (count, visible) = (10, 4);
    let mut menu = CharmMenu::default();
    for _ in 0..7 {
        menu.step(count, 1, visible);
    }
    let first = menu.first(count, visible);
    assert_eq!(first, 4);
    for row in first..first + visible {
        menu.select(row, count);
        assert_eq!(menu.first(count, visible), first);
    }
}

#[test]
fn a_window_that_shrank_or_a_list_that_did_is_pulled_back_to_the_cursor() {
    let mut menu = CharmMenu::default();
    for _ in 0..9 {
        menu.step(10, 1, 5);
    }
    assert_eq!(menu.first(10, 5), 5);
    // Fewer rows fit (a smaller view): the cursor is still in view.
    let first = menu.first(10, 2);
    assert!(first <= 9 && 9 < first + 2);
    // Fewer charms (a reload): the window cannot start past the last full page.
    assert_eq!(menu.first(6, 5), 1);
    assert_eq!(menu.first(3, 5), 0);
}

// ---- the layout and the mouse -----------------------------------------------------------

fn inside(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x - 1e-3
        && inner.y >= outer.y - 1e-3
        && inner.right() <= outer.right() + 1e-3
        && inner.bottom() <= outer.bottom() + 1e-3
}

#[test]
fn the_panel_is_centered_on_the_view() {
    for view in [VIEW, vec2(1280.0, 720.0), vec2(800.0, 600.0)] {
        let layout = layout(view, 1, 3, 0);
        close(layout.panel.center().x, view.x / 2.0);
        close(layout.panel.center().y, view.y / 2.0);
        close(layout.panel.w, PANEL_WIDTH);
    }
}

#[test]
fn the_panel_grows_with_its_content_and_every_part_is_inside_it() {
    let small = layout(VIEW, 0, 1, 0);
    let more_charms = layout(VIEW, 0, 3, 0);
    let more_items = layout(VIEW, 2, 1, 0);
    assert!(more_charms.panel.h > small.panel.h);
    assert!(more_items.panel.h > small.panel.h);
    for layout in [&small, &more_charms, &more_items] {
        for row in &layout.rows {
            assert!(inside(layout.panel, row.rect), "{:?}", row.rect);
        }
        for at in [
            layout.title,
            layout.coins,
            layout.items_header,
            layout.items_first,
            layout.charms_header,
            layout.mana,
            layout.message[0],
            layout.message[MESSAGE_LINES - 1],
            layout.footer,
        ] {
            assert!(
                layout.panel.contains(at),
                "{at:?} is outside {:?}",
                layout.panel
            );
        }
    }
}

#[test]
fn the_sections_follow_each_other_without_overlapping() {
    let layout = layout(VIEW, 2, 3, 0);
    assert!(layout.title.y < layout.coins.y);
    assert!(layout.coins.y < layout.items_header.y);
    assert!(layout.items_header.y < layout.items_first.y);
    assert!(layout.item(1).y < layout.charms_header.y);
    assert!(layout.charms_header.y < layout.rows[0].rect.y);
    for pair in layout.rows.windows(2) {
        assert!(pair[0].rect.bottom() <= pair[1].rect.y);
    }
    let last = layout.rows.last().unwrap().rect;
    assert!(last.bottom() < layout.description[0].y);
    assert!(layout.description[0].y < layout.description[1].y);
    assert!(layout.description[1].y < layout.mana.y);
    assert!(layout.mana.y < layout.message[0].y);
    assert!(layout.message[0].y < layout.message[1].y);
    assert!(layout.message[1].y < layout.footer.y);
}

#[test]
fn the_panel_fits_the_view_for_any_number_of_charms_and_items() {
    for items in [0, 1, 3, 5] {
        for charms in [0, 1, 3, 8, 40] {
            let layout = layout(VIEW, items, charms, 0);
            let case = format!("{items} items, {charms} charms: {:?}", layout.panel);
            if items <= 3 {
                assert!(
                    layout.panel.y >= 0.0 && layout.panel.bottom() <= VIEW.y,
                    "{case}"
                );
            }
            assert_eq!(
                layout.rows.len(),
                visible_rows(VIEW, items, charms),
                "{case}"
            );
            assert!(layout.rows.len() <= charms, "{case}");
            assert_eq!(layout.rows.is_empty(), charms == 0, "{case}");
        }
    }
}

#[test]
fn shipped_sized_inventories_show_every_charm() {
    // Three charms and up to two collected items fit on the 960 x 540 screen whole.
    for items in 0..=2 {
        assert_eq!(visible_rows(VIEW, items, 3), 3, "{items} items");
    }
    assert_eq!(visible_rows(VIEW, 0, 0), 0);
}

#[test]
fn a_long_list_shows_a_window_and_says_where_there_is_more() {
    let top = layout(VIEW, 0, 20, 0);
    let rows = top.rows.len();
    assert!((1..20).contains(&rows), "{rows}");
    assert!(!top.more_above && top.more_below);
    assert_eq!(top.rows[0].index, 0);
    let middle = layout(VIEW, 0, 20, 5);
    assert!(middle.more_above && middle.more_below);
    assert_eq!(middle.rows[0].index, 5);
    assert_eq!(middle.rows.last().unwrap().index, 5 + rows - 1);
    let end = layout(VIEW, 0, 20, 99);
    assert!(end.more_above && !end.more_below);
    assert_eq!(end.rows.last().unwrap().index, 19);
}

#[test]
fn a_bad_view_height_still_gives_one_row_and_no_nan() {
    for height in [0.0, -10.0, f32::NAN, 1.0] {
        let view = vec2(960.0, height);
        assert_eq!(visible_rows(view, 0, 5), 1);
        let layout = layout(view, 0, 5, 0);
        assert_eq!(layout.rows.len(), 1);
    }
}

#[test]
fn the_mouse_finds_the_row_it_is_on() {
    let layout = layout(VIEW, 1, 3, 0);
    for row in &layout.rows {
        assert_eq!(charm_at(&layout, row.rect.center()), Some(row.index));
        assert_eq!(charm_at(&layout, row.rect.point()), Some(row.index));
    }
    assert_eq!(
        layout.rows.iter().map(|row| row.index).collect::<Vec<_>>(),
        [0, 1, 2]
    );
    // The gap between two rows, the heading, the description and the screen's corner are not
    // rows.
    let gap = (layout.rows[0].rect.bottom() + layout.rows[1].rect.y) / 2.0;
    assert_eq!(
        charm_at(&layout, vec2(layout.rows[0].rect.center().x, gap)),
        None
    );
    assert_eq!(charm_at(&layout, layout.charms_header), None);
    assert_eq!(charm_at(&layout, layout.description[0]), None);
    assert_eq!(charm_at(&layout, vec2(0.0, 0.0)), None);
    assert_eq!(charm_at(&layout, vec2(f32::NAN, f32::NAN)), None);
}

#[test]
fn the_mouse_on_a_scrolled_list_finds_the_charm_not_the_row_number() {
    let layout = layout(VIEW, 0, 20, 7);
    let second = layout.rows[1];
    assert_eq!(second.index, 8);
    assert_eq!(charm_at(&layout, second.rect.center()), Some(8));
}

#[test]
fn the_hit_test_follows_the_panel_when_the_view_is_another_size() {
    for view in [VIEW, vec2(1280.0, 720.0)] {
        let layout = layout(view, 0, 3, 0);
        let row = layout.rows[2];
        assert!(inside(layout.panel, row.rect));
        assert_eq!(charm_at(&layout, row.rect.center()), Some(2));
    }
}

#[test]
fn a_list_with_no_charms_has_a_note_and_no_rows() {
    let layout = layout(VIEW, 0, 0, 0);
    assert!(layout.rows.is_empty());
    assert!(layout.panel.contains(layout.empty_note));
    assert!(layout.panel.contains(layout.footer));
    assert!(layout.empty_note.y > layout.charms_header.y);
    assert!(layout.footer.y > layout.empty_note.y);
    assert!(!layout.more_above && !layout.more_below);
    assert_eq!(charm_at(&layout, layout.empty_note), None);
    // Less to show means a smaller panel than one with charms.
    assert!(layout.panel.h < self::layout(VIEW, 0, 1, 0).panel.h);
}

#[test]
fn the_row_columns_are_inside_the_row_and_in_order() {
    let layout = layout(VIEW, 0, 3, 0);
    for row in &layout.rows {
        let columns = row_columns(row.rect);
        assert!(inside(row.rect, columns.swatch));
        close(columns.swatch.center().y, row.rect.center().y);
        assert!(columns.swatch.right() < columns.name_x);
        assert!(columns.name_x + columns.name_width <= columns.cost_x);
        assert!(columns.cost_x < columns.effect_x);
        assert!(columns.effect_width > 0.0);
        assert!(columns.effect_x + columns.effect_width < columns.marker_right);
        assert!(columns.marker_right <= row.rect.right());
    }
}
