use macroquad::math::Vec2;

use super::*;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);

fn open_menu() -> PauseMenu {
    let mut menu = PauseMenu::default();
    menu.escape();
    menu
}

fn settings_menu() -> PauseMenu {
    let mut menu = open_menu();
    assert_eq!(menu.activate(PauseItem::Settings), PauseOutcome::Continue);
    menu
}

#[test]
fn the_menu_starts_closed_and_escape_opens_it_on_unpause() {
    let mut menu = PauseMenu::default();
    assert!(!menu.is_open());
    assert_eq!(menu.screen(), None);
    assert_eq!(menu.selected_item(), None);
    menu.escape();
    assert!(menu.is_open());
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
    assert_eq!(menu.selected_item(), Some(PauseItem::Unpause));
}

#[test]
fn escape_closes_the_main_screen_and_reopening_starts_on_unpause_again() {
    let mut menu = open_menu();
    menu.step(2);
    assert_eq!(menu.selected_item(), Some(PauseItem::Exit));
    menu.escape();
    assert!(!menu.is_open());
    menu.escape();
    assert_eq!(menu.selected_item(), Some(PauseItem::Unpause));
}

#[test]
fn escape_in_settings_goes_back_to_the_main_screen_instead_of_closing() {
    let mut menu = settings_menu();
    assert_eq!(menu.screen(), Some(PauseScreen::Settings));
    menu.escape();
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
    assert_eq!(menu.selected_item(), Some(PauseItem::Settings));
    assert!(menu.is_open());
}

#[test]
fn the_selection_wraps_in_both_directions() {
    let mut menu = open_menu();
    menu.step(-1);
    assert_eq!(menu.selected_item(), Some(PauseItem::Exit));
    menu.step(1);
    assert_eq!(menu.selected_item(), Some(PauseItem::Unpause));
    menu.step(1);
    assert_eq!(menu.selected_item(), Some(PauseItem::Settings));
    menu.step(7); // two full laps and one more
    assert_eq!(menu.selected_item(), Some(PauseItem::Exit));
    menu.step(0);
    assert_eq!(menu.selected_item(), Some(PauseItem::Exit));
}

#[test]
fn stepping_a_closed_menu_does_nothing() {
    let mut menu = PauseMenu::default();
    menu.step(1);
    menu.step(-1);
    assert_eq!(menu, PauseMenu::default());
}

#[test]
fn unpause_closes_the_menu() {
    let mut menu = open_menu();
    assert_eq!(menu.activate_selected(), PauseOutcome::Continue);
    assert!(!menu.is_open());
}

#[test]
fn settings_opens_the_blank_screen_and_back_returns_to_the_main_one() {
    let mut menu = open_menu();
    menu.step(1);
    assert_eq!(menu.activate_selected(), PauseOutcome::Continue);
    assert_eq!(menu.screen(), Some(PauseScreen::Settings));
    assert_eq!(menu.selected_item(), Some(PauseItem::Back));
    assert_eq!(menu.activate_selected(), PauseOutcome::Continue);
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
    assert_eq!(menu.selected_item(), Some(PauseItem::Settings));
}

#[test]
fn exit_asks_the_front_end_to_end_the_program_and_nothing_else_does() {
    let mut menu = open_menu();
    menu.step(-1);
    assert_eq!(menu.activate_selected(), PauseOutcome::Exit);
    for item in [PauseItem::Unpause, PauseItem::Settings, PauseItem::Back] {
        let mut menu = open_menu();
        assert_eq!(menu.activate(item), PauseOutcome::Continue);
    }
}

#[test]
fn a_button_the_open_screen_does_not_have_does_nothing() {
    let mut menu = settings_menu();
    let before = menu.clone();
    assert_eq!(menu.activate(PauseItem::Exit), PauseOutcome::Continue);
    assert_eq!(menu.activate(PauseItem::Unpause), PauseOutcome::Continue);
    menu.select(PauseItem::Exit);
    assert_eq!(menu, before);
    // Nothing is pressed or selected while the menu is closed either.
    let mut closed = PauseMenu::default();
    assert_eq!(closed.activate(PauseItem::Exit), PauseOutcome::Continue);
    assert_eq!(closed.activate_selected(), PauseOutcome::Continue);
    closed.select(PauseItem::Settings);
    assert_eq!(closed, PauseMenu::default());
}

#[test]
fn resting_the_mouse_on_a_button_selects_it() {
    let mut menu = open_menu();
    menu.select(PauseItem::Exit);
    assert_eq!(menu.selected_item(), Some(PauseItem::Exit));
    menu.select(PauseItem::Unpause);
    assert_eq!(menu.selected_item(), Some(PauseItem::Unpause));
}

#[test]
fn the_buttons_are_stacked_without_overlap_inside_a_centered_panel() {
    for view in [
        VIEW,
        Vec2::new(1280.0, 720.0),
        Vec2::new(640.0, 360.0),
        Vec2::new(1920.0, 540.0),
    ] {
        for screen in [PauseScreen::Main, PauseScreen::Settings] {
            // Each screen has its own panel (Settings lists the keybinds, so it is bigger).
            let panel = panel_rect(screen, view);
            assert!((panel.center().x - view.x / 2.0).abs() < 0.01);
            assert!((panel.center().y - view.y / 2.0).abs() < 0.01);
            let buttons = button_rects(screen, view);
            let items = buttons.iter().map(|(item, _)| *item).collect::<Vec<_>>();
            assert_eq!(items, screen.items());
            for (index, (_, rect)) in buttons.iter().enumerate() {
                assert!(rect.x >= panel.x && rect.right() <= panel.right());
                assert!(rect.y >= panel.y && rect.bottom() <= panel.bottom());
                assert!((rect.center().x - panel.center().x).abs() < 0.01);
                if let Some((_, next)) = buttons.get(index + 1) {
                    assert!(next.y >= rect.bottom());
                }
            }
        }
    }
}

#[test]
fn the_title_sits_above_the_first_button() {
    for screen in [PauseScreen::Main, PauseScreen::Settings] {
        let panel = panel_rect(screen, VIEW);
        let first = button_rects(screen, VIEW)[0].1;
        assert!(title_baseline() > 0.0);
        assert!(panel.y + title_baseline() < first.y);
    }
}

#[test]
fn item_at_finds_the_button_under_a_point_and_misses_the_gaps_and_the_outside() {
    for screen in [PauseScreen::Main, PauseScreen::Settings] {
        let buttons = button_rects(screen, VIEW);
        for (item, rect) in &buttons {
            assert_eq!(item_at(screen, VIEW, rect.center()), Some(*item));
            assert_eq!(item_at(screen, VIEW, rect.point() + Vec2::ONE), Some(*item));
        }
        assert_eq!(item_at(screen, VIEW, Vec2::ZERO), None);
        assert_eq!(item_at(screen, VIEW, VIEW), None);
        let first = buttons[0].1;
        assert_eq!(
            item_at(screen, VIEW, first.center() - Vec2::new(0.0, 100.0)),
            None
        );
    }
    let main = button_rects(PauseScreen::Main, VIEW);
    let gap = Vec2::new(main[0].1.center().x, main[0].1.bottom() + 1.0);
    assert_eq!(item_at(PauseScreen::Main, VIEW, gap), None);
    // The Exit button is not there on the Settings screen: the point where it sits on the main
    // screen is a keybind row there, and no point of that screen finds Exit.
    assert_ne!(
        item_at(PauseScreen::Settings, VIEW, main[2].1.center()),
        Some(PauseItem::Exit)
    );
    for (item, rect) in button_rects(PauseScreen::Settings, VIEW) {
        assert_ne!(item, PauseItem::Exit);
        assert_ne!(item_at(PauseScreen::Main, VIEW, rect.center()), Some(item));
    }
}

fn bind(action: Action) -> PauseItem {
    PauseItem::Bind(action)
}

/// The Settings screen with the first keybind row (Move left) selected.
fn first_row_menu() -> PauseMenu {
    let mut menu = settings_menu();
    menu.step(1); // from Back, around to the top
    assert_eq!(menu.selected_item(), Some(bind(Action::MoveLeft)));
    menu
}

#[test]
fn the_settings_screen_lists_a_row_per_action_then_reset_and_back() {
    let items = PauseScreen::Settings.items();
    assert_eq!(items.len(), Action::ALL.len() + 2);
    for (item, action) in items.iter().zip(Action::ALL) {
        assert_eq!(*item, bind(action));
        assert_eq!(item.label(), action.label());
    }
    assert_eq!(items[Action::ALL.len()], PauseItem::ResetKeybinds);
    assert_eq!(items[Action::ALL.len() + 1], PauseItem::Back);
    assert_eq!(PauseItem::ResetKeybinds.label(), "Reset keybinds");
}

#[test]
fn pressing_a_keybind_row_waits_for_a_key_for_the_highlighted_slot() {
    let mut menu = first_row_menu();
    assert_eq!(menu.slot(), Slot::Primary);
    assert_eq!(menu.listening(), None);
    assert_eq!(menu.activate_selected(), PauseOutcome::Continue);
    assert_eq!(menu.listening(), Some((Action::MoveLeft, Slot::Primary)));
    assert_eq!(menu.screen(), Some(PauseScreen::Settings));
    menu.stop_listening();
    assert_eq!(menu.listening(), None);
    menu.step_slot(1);
    assert_eq!(menu.activate_selected(), PauseOutcome::Continue);
    assert_eq!(menu.listening(), Some((Action::MoveLeft, Slot::Alternate)));
}

#[test]
fn left_and_right_move_between_the_two_keys_of_a_row_and_stop_at_the_ends() {
    let mut menu = first_row_menu();
    menu.step_slot(-1);
    assert_eq!(menu.slot(), Slot::Primary);
    menu.step_slot(1);
    assert_eq!(menu.slot(), Slot::Alternate);
    menu.step_slot(1);
    menu.step_slot(5);
    assert_eq!(menu.slot(), Slot::Alternate);
    // The highlighted key stays when the selection moves to another row.
    menu.step(1);
    assert_eq!(menu.selected_item(), Some(bind(Action::MoveRight)));
    assert_eq!(menu.slot(), Slot::Alternate);
    menu.step_slot(-1);
    assert_eq!(menu.slot(), Slot::Primary);
}

#[test]
fn the_highlighted_key_only_moves_on_a_keybind_row() {
    let mut menu = settings_menu(); // Back is selected
    menu.step_slot(1);
    assert_eq!(menu.slot(), Slot::Primary);
    let mut main = open_menu();
    main.step_slot(1);
    assert_eq!(main, open_menu());
    let mut closed = PauseMenu::default();
    closed.step_slot(1);
    assert_eq!(closed, PauseMenu::default());
}

#[test]
fn while_waiting_for_a_key_the_selection_cannot_move() {
    let mut menu = first_row_menu();
    menu.activate_selected();
    let waiting = menu.clone();
    menu.step(1);
    menu.step(-1);
    menu.step_slot(1);
    menu.select(PauseItem::Back);
    menu.select_slot(Action::Dash, Slot::Alternate);
    assert_eq!(menu, waiting);
}

#[test]
fn escape_cancels_the_wait_first_and_only_then_leaves_settings() {
    let mut menu = first_row_menu();
    menu.activate_selected();
    menu.set_note(Some("old message".to_owned()));
    menu.escape();
    assert_eq!(menu.listening(), None);
    assert_eq!(menu.note(), None);
    assert_eq!(menu.screen(), Some(PauseScreen::Settings));
    assert_eq!(menu.selected_item(), Some(bind(Action::MoveLeft)));
    menu.escape();
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
}

#[test]
fn leaving_the_screen_or_closing_the_menu_drops_the_wait_the_note_and_the_highlighted_key() {
    let mut menu = first_row_menu();
    menu.step_slot(1);
    menu.activate_selected();
    menu.set_note(Some("message".to_owned()));
    assert_eq!(menu.activate(PauseItem::Back), PauseOutcome::Continue);
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
    assert_eq!(
        (menu.listening(), menu.note(), menu.slot()),
        (None, None, Slot::Primary)
    );

    // Closing the menu with Escape from the main screen and reopening starts clean.
    let mut menu = first_row_menu();
    menu.step_slot(1);
    menu.set_note(Some("message".to_owned()));
    menu.escape();
    menu.escape();
    assert!(!menu.is_open());
    menu.escape();
    assert_eq!(
        (menu.listening(), menu.note(), menu.slot()),
        (None, None, Slot::Primary)
    );
}

#[test]
fn reset_keybinds_asks_for_a_reset_and_nothing_else_does() {
    let mut menu = settings_menu();
    menu.select(PauseItem::ResetKeybinds);
    menu.set_note(Some("old".to_owned()));
    assert_eq!(menu.activate_selected(), PauseOutcome::ResetKeybinds);
    assert_eq!(menu.screen(), Some(PauseScreen::Settings));
    assert_eq!((menu.listening(), menu.note()), (None, None));
    assert_eq!(menu.selected_item(), Some(PauseItem::ResetKeybinds));
    for item in PauseScreen::Settings.items() {
        if *item != PauseItem::ResetKeybinds {
            assert_ne!(settings_menu().activate(*item), PauseOutcome::ResetKeybinds);
        }
    }
    // Not on the main screen, and not while the menu is closed.
    assert_eq!(
        open_menu().activate(PauseItem::ResetKeybinds),
        PauseOutcome::Continue
    );
    assert_eq!(
        PauseMenu::default().activate(PauseItem::ResetKeybinds),
        PauseOutcome::Continue
    );
}

#[test]
fn the_mouse_picks_a_key_button_by_resting_on_it_and_waits_for_a_key_when_clicking_it() {
    let mut menu = settings_menu();
    menu.select_slot(Action::Dash, Slot::Alternate);
    assert_eq!(menu.selected_item(), Some(bind(Action::Dash)));
    assert_eq!(menu.slot(), Slot::Alternate);
    assert_eq!(menu.listening(), None);
    assert_eq!(
        menu.activate_slot(Action::Jump, Slot::Primary),
        PauseOutcome::Continue
    );
    assert_eq!(menu.selected_item(), Some(bind(Action::Jump)));
    assert_eq!(menu.listening(), Some((Action::Jump, Slot::Primary)));
    // Clicking another key button while waiting switches the wait to it.
    menu.activate_slot(Action::Heal, Slot::Alternate);
    assert_eq!(menu.listening(), Some((Action::Heal, Slot::Alternate)));
    assert_eq!(menu.selected_item(), Some(bind(Action::Heal)));
}

#[test]
fn key_buttons_do_nothing_on_the_main_screen_or_while_the_menu_is_closed() {
    let mut main = open_menu();
    main.select_slot(Action::Jump, Slot::Alternate);
    assert_eq!(
        main.activate_slot(Action::Jump, Slot::Alternate),
        PauseOutcome::Continue
    );
    assert_eq!(main, open_menu());
    let mut closed = PauseMenu::default();
    closed.select_slot(Action::Jump, Slot::Alternate);
    closed.activate_slot(Action::Jump, Slot::Alternate);
    assert_eq!(closed, PauseMenu::default());
}

#[test]
fn starting_a_wait_clears_the_old_message() {
    let mut menu = first_row_menu();
    menu.set_note(Some("Left Shift swapped".to_owned()));
    assert_eq!(menu.note(), Some("Left Shift swapped"));
    menu.activate_selected();
    assert_eq!(menu.note(), None);
    menu.set_note(Some("again".to_owned()));
    menu.activate_slot(Action::Jump, Slot::Primary);
    assert_eq!(menu.note(), None);
}

#[test]
fn the_settings_panel_fits_the_virtual_screen_and_the_main_one_keeps_its_size() {
    let settings = panel_rect(PauseScreen::Settings, VIEW);
    assert!(settings.x >= 0.0 && settings.y >= 0.0);
    assert!(settings.right() <= VIEW.x && settings.bottom() <= VIEW.y);
    let main = panel_rect(PauseScreen::Main, VIEW);
    assert_eq!((main.w, main.h), (360.0, 292.0));
    assert!(settings.w > main.w && settings.h > main.h);
}

#[test]
fn each_keybind_row_has_two_key_buttons_at_its_right_end_that_do_not_overlap() {
    for (item, row) in button_rects(PauseScreen::Settings, VIEW) {
        let PauseItem::Bind(_) = item else { continue };
        let [primary, alternate] = slot_rects(row);
        assert!(primary.x >= row.x && alternate.right() <= row.right() + 0.01);
        assert!((alternate.right() - row.right()).abs() < 0.01);
        assert!(primary.right() < alternate.x);
        for slot in [primary, alternate] {
            assert_eq!((slot.y, slot.h), (row.y, row.h));
            assert!(slot.w >= 100.0);
        }
        // The label has room to its left.
        assert!(primary.x - row.x >= 150.0);
    }
}

#[test]
fn slot_at_finds_the_key_button_under_a_point_and_nothing_else() {
    for (item, row) in button_rects(PauseScreen::Settings, VIEW) {
        let [primary, alternate] = slot_rects(row);
        match item {
            PauseItem::Bind(action) => {
                assert_eq!(
                    slot_at(PauseScreen::Settings, VIEW, primary.center()),
                    Some((action, Slot::Primary))
                );
                assert_eq!(
                    slot_at(PauseScreen::Settings, VIEW, alternate.center()),
                    Some((action, Slot::Alternate))
                );
                // The label side of the row, and the gap between the two buttons.
                let label = Vec2::new(row.x + 5.0, row.center().y);
                assert_eq!(slot_at(PauseScreen::Settings, VIEW, label), None);
                let gap = Vec2::new(primary.right() + 1.0, row.center().y);
                assert_eq!(slot_at(PauseScreen::Settings, VIEW, gap), None);
                assert_eq!(item_at(PauseScreen::Settings, VIEW, label), Some(item));
            }
            _ => assert_eq!(slot_at(PauseScreen::Settings, VIEW, row.center()), None),
        }
    }
    assert_eq!(slot_at(PauseScreen::Settings, VIEW, Vec2::ZERO), None);
    // Nothing on the main screen is a key button.
    for (_, rect) in button_rects(PauseScreen::Main, VIEW) {
        assert_eq!(slot_at(PauseScreen::Main, VIEW, rect.center()), None);
    }
}

#[test]
fn reset_and_back_are_set_apart_below_the_keybind_rows_and_the_message_line_is_under_them() {
    let buttons = button_rects(PauseScreen::Settings, VIEW);
    let panel = panel_rect(PauseScreen::Settings, VIEW);
    let last_row = buttons[Action::ALL.len() - 1].1;
    let reset = buttons[Action::ALL.len()].1;
    let back = buttons[Action::ALL.len() + 1].1;
    assert_eq!(buttons[Action::ALL.len()].0, PauseItem::ResetKeybinds);
    assert!(reset.y - last_row.bottom() > back.y - reset.bottom());
    let status = status_rect(VIEW);
    assert!(status.y >= back.bottom());
    assert!(status.x >= panel.x && status.right() <= panel.right());
    assert!(status.bottom() <= panel.bottom());
}
