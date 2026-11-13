//! The Save / Discard / Cancel window shown when closing the editor or switching rooms
//! with unsaved changes. The decisions live in `hollow_steps::editor_guard`.
use super::widgets::{panel_button, panel_label};
use egui_macroquad::egui;
use hollow_steps::editor_guard::{GuardChoice, UnsavedGuard};

/// Draws the dialog while the guard has one open and returns the button pressed this
/// frame. A modal backdrop blocks the panel and canvas behind it. Escape or a click
/// outside the window cancels, which keeps the session.
pub(crate) fn draw_unsaved_dialog(
    ctx: &egui::Context,
    guard: &UnsavedGuard,
) -> Option<GuardChoice> {
    let prompt = guard.prompt()?;
    let mut choice = None;
    let width = (ctx.screen_rect().width() - 48.0).clamp(120.0, 320.0);
    let modal = egui::Modal::new(egui::Id::new("unsaved changes")).show(ctx, |ui| {
        ui.set_width(width);
        ui.heading("Unsaved changes");
        panel_label(
            ui,
            &format!("Save your changes before you {}?", prompt.action.describe()),
        );
        if !guard.unsaved().is_empty() {
            panel_label(ui, &format!("Unsaved: {}", guard.unsaved().join(", ")));
        }
        if let Some(error) = &prompt.error {
            ui.add(
                egui::Label::new(egui::RichText::new(error).color(egui::Color32::LIGHT_RED)).wrap(),
            );
        }
        for (label, button) in [
            ("Save", GuardChoice::Save),
            ("Discard", GuardChoice::Discard),
            ("Cancel", GuardChoice::Cancel),
        ] {
            if panel_button(ui, label) {
                choice = Some(button);
            }
        }
    });
    if modal.should_close() {
        choice = choice.or(Some(GuardChoice::Cancel));
    }
    choice
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{click, drawn_text};
    use super::*;
    use hollow_steps::editor_guard::GuardedAction;
    use hollow_steps::room::RoomId;

    fn frame(
        ctx: &egui::Context,
        guard: &UnsavedGuard,
        events: Vec<egui::Event>,
    ) -> (Option<GuardChoice>, String) {
        let mut choice = None;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 540.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| choice = draw_unsaved_dialog(ctx, guard),
        );
        (choice, drawn_text(&output))
    }

    fn open_dialog(action: GuardedAction) -> (egui::Context, UnsavedGuard) {
        let mut guard = UnsavedGuard::default();
        guard.request(action, true);
        let ctx = egui::Context::default();
        for _ in 0..3 {
            frame(&ctx, &guard, vec![]);
        }
        (ctx, guard)
    }

    fn button(ctx: &egui::Context, name: &str) -> egui::Pos2 {
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", name))))
            .unwrap();
        rect.center()
    }

    fn pressed(ctx: &egui::Context, guard: &UnsavedGuard, at: egui::Pos2) -> Option<GuardChoice> {
        let [press, release] = click(at);
        let first = frame(ctx, guard, press).0;
        let second = frame(ctx, guard, release).0;
        let third = frame(ctx, guard, vec![]).0;
        first.or(second).or(third)
    }

    #[test]
    fn nothing_is_drawn_without_an_open_prompt() {
        let ctx = egui::Context::default();
        let guard = UnsavedGuard::default();
        for _ in 0..3 {
            let (choice, text) = frame(&ctx, &guard, vec![]);
            assert_eq!(choice, None);
            assert!(text.is_empty(), "{text:?}");
        }
    }

    #[test]
    fn the_dialog_says_what_the_user_is_about_to_do() {
        let (ctx, guard) = open_dialog(GuardedAction::OpenRoom(RoomId::Second));
        let (_, text) = frame(&ctx, &guard, vec![]);
        for expected in [
            "Unsaved changes",
            "open room Second",
            "Save",
            "Discard",
            "Cancel",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text:?}");
        }
        let (ctx, guard) = open_dialog(GuardedAction::Close);
        assert!(frame(&ctx, &guard, vec![]).1.contains("close the editor"));
    }

    #[test]
    fn each_button_reports_its_choice() {
        for (label, expected) in [
            ("Save", GuardChoice::Save),
            ("Discard", GuardChoice::Discard),
            ("Cancel", GuardChoice::Cancel),
        ] {
            let (ctx, guard) = open_dialog(GuardedAction::NewRoom);
            let at = button(&ctx, label);
            assert_eq!(pressed(&ctx, &guard, at), Some(expected), "{label}");
        }
    }

    #[test]
    fn idle_frames_report_no_choice() {
        let (ctx, guard) = open_dialog(GuardedAction::Close);
        assert_eq!(frame(&ctx, &guard, vec![]).0, None);
        assert_eq!(frame(&ctx, &guard, vec![]).0, None);
    }

    #[test]
    fn escape_and_a_click_outside_cancel() {
        let (ctx, guard) = open_dialog(GuardedAction::Close);
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: Some(egui::Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        assert_eq!(
            frame(&ctx, &guard, vec![escape]).0,
            Some(GuardChoice::Cancel)
        );
        let (ctx, guard) = open_dialog(GuardedAction::Close);
        assert_eq!(
            pressed(&ctx, &guard, egui::pos2(6.0, 6.0)),
            Some(GuardChoice::Cancel)
        );
    }

    #[test]
    fn a_failed_save_is_shown_in_the_dialog() {
        let mut guard = UnsavedGuard::default();
        guard.request(GuardedAction::Close, true);
        guard.choose(GuardChoice::Save);
        guard.save_finished(Err("Save failed: room file is read-only".into()));
        let ctx = egui::Context::default();
        let mut text = String::new();
        for _ in 0..3 {
            text = frame(&ctx, &guard, vec![]).1;
        }
        assert!(text.contains("room file is read-only"), "{text:?}");
        assert!(text.contains("close the editor"));
    }
}
