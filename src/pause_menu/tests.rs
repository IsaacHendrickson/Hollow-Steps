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
        let panel = panel_rect(view);
        assert!((panel.center().x - view.x / 2.0).abs() < 0.01);
        assert!((panel.center().y - view.y / 2.0).abs() < 0.01);
        for screen in [PauseScreen::Main, PauseScreen::Settings] {
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
    let panel = panel_rect(VIEW);
    let first = button_rects(PauseScreen::Main, VIEW)[0].1;
    assert!(title_baseline() > 0.0);
    assert!(panel.y + title_baseline() < first.y);
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
    // The Exit button is not there on the Settings screen.
    assert_eq!(
        item_at(PauseScreen::Settings, VIEW, main[2].1.center()),
        None
    );
}
