//! The pause menu during a run: the Abandon run button and its confirm screen. The menu outside
//! a run is tested in `tests`.
use macroquad::math::Vec2;

use super::*;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);

fn run_menu() -> PauseMenu {
    let mut menu = PauseMenu::default();
    menu.set_in_run(true);
    menu.escape();
    menu
}

fn confirm_menu() -> PauseMenu {
    let mut menu = run_menu();
    menu.activate(PauseItem::AbandonRun);
    menu
}

#[test]
fn outside_a_run_the_main_screen_is_unchanged() {
    let mut menu = PauseMenu::default();
    menu.escape();
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
    assert_eq!(
        PauseScreen::Main.items(),
        [PauseItem::Unpause, PauseItem::Settings, PauseItem::Exit]
    );
    // Abandon run is not a button there, so pressing it does nothing.
    assert_eq!(menu.activate(PauseItem::AbandonRun), PauseOutcome::Continue);
    assert_eq!(menu.screen(), Some(PauseScreen::Main));
}

#[test]
fn in_a_run_the_main_screen_lists_abandon_run_before_exit() {
    let menu = run_menu();
    assert_eq!(menu.screen(), Some(PauseScreen::RunMain));
    assert_eq!(menu.selected_item(), Some(PauseItem::Unpause));
    assert_eq!(
        PauseScreen::RunMain.items(),
        [
            PauseItem::Unpause,
            PauseItem::Settings,
            PauseItem::AbandonRun,
            PauseItem::Exit
        ]
    );
    assert_eq!(PauseScreen::RunMain.title(), PauseScreen::Main.title());
}

#[test]
fn abandon_run_asks_first_and_the_safe_button_is_selected() {
    let mut menu = run_menu();
    assert_eq!(menu.activate(PauseItem::AbandonRun), PauseOutcome::Continue);
    assert_eq!(menu.screen(), Some(PauseScreen::Confirm));
    assert_eq!(
        PauseScreen::Confirm.items(),
        [PauseItem::ConfirmAbandon, PauseItem::KeepPlaying]
    );
    assert_eq!(menu.selected_item(), Some(PauseItem::KeepPlaying));
    // Enter straight away keeps playing: back on the main screen, still in the run.
    assert_eq!(menu.activate_selected(), PauseOutcome::Continue);
    assert_eq!(menu.screen(), Some(PauseScreen::RunMain));
    assert_eq!(menu.selected_item(), Some(PauseItem::AbandonRun));
}

#[test]
fn confirming_asks_the_game_to_abandon_and_closes_the_menu() {
    let mut menu = confirm_menu();
    assert_eq!(
        menu.activate(PauseItem::ConfirmAbandon),
        PauseOutcome::AbandonRun
    );
    assert!(!menu.is_open());
    assert_eq!(menu.selected_item(), None);
}

#[test]
fn confirming_with_the_keyboard_needs_a_step_up_and_enter() {
    let mut menu = confirm_menu();
    menu.step(-1);
    assert_eq!(menu.selected_item(), Some(PauseItem::ConfirmAbandon));
    assert_eq!(menu.activate_selected(), PauseOutcome::AbandonRun);
}

#[test]
fn escape_goes_back_one_screen_at_a_time() {
    let mut menu = confirm_menu();
    menu.escape();
    assert_eq!(menu.screen(), Some(PauseScreen::RunMain));
    assert_eq!(menu.selected_item(), Some(PauseItem::AbandonRun));
    menu.escape();
    assert!(!menu.is_open());
}

#[test]
fn settings_leads_back_to_the_run_main_screen() {
    let mut menu = run_menu();
    menu.activate(PauseItem::Settings);
    menu.activate(PauseItem::Back);
    assert_eq!(menu.screen(), Some(PauseScreen::RunMain));
    assert_eq!(menu.selected_item(), Some(PauseItem::Settings));
    menu.activate(PauseItem::Settings);
    menu.escape();
    assert_eq!(menu.screen(), Some(PauseScreen::RunMain));
}

#[test]
fn a_menu_closed_and_reopened_outside_a_run_is_the_plain_one_again() {
    let mut menu = run_menu();
    menu.escape();
    assert!(!menu.is_open());
    menu.escape();
    assert_eq!(
        menu.screen(),
        Some(PauseScreen::Main),
        "the run is forgotten"
    );
}

#[test]
fn the_new_screens_have_buttons_the_mouse_can_find_inside_their_panel() {
    for screen in [PauseScreen::RunMain, PauseScreen::Confirm] {
        let panel = panel_rect(screen, VIEW);
        let buttons = button_rects(screen, VIEW);
        assert_eq!(buttons.len(), screen.items().len());
        for (item, rect) in &buttons {
            assert!(panel.contains(rect.point()) && panel.contains(rect.point() + rect.size()));
            assert_eq!(item_at(screen, VIEW, rect.center()), Some(*item));
        }
        for pair in buttons.windows(2) {
            assert!(pair[0].1.bottom() < pair[1].1.y, "buttons do not overlap");
        }
    }
    assert!(panel_rect(PauseScreen::RunMain, VIEW).h > panel_rect(PauseScreen::Main, VIEW).h);
}
