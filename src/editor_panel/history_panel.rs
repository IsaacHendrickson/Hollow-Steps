//! The Undo and Redo buttons and what they do to the editor state. The snapshot logic
//! lives in `hollow_steps::editor_history`; this is presentation and wiring only.
use super::widgets::{probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::{redo_session, undo_session, HistoryKey};

/// Shown on hover next to the Undo and Redo buttons.
pub(super) const HISTORY_TOOLTIP: &str = "Undo and redo cover changes to the room: placing, \
    moving, resizing and deleting things and solids, exits, room size and the room look. Entity, behavior and boss \
    definition edits are not covered. Shortcuts: Ctrl+Z, Ctrl+Y or Ctrl+Shift+Z, with the canvas focused.";

/// Undo and Redo buttons, enabled only when there is a step to take.
pub(super) fn draw_history_section(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    section(ui, "History", |ui| {
        let can_undo = state.history.can_undo();
        let can_redo = state.history.can_redo();
        ui.horizontal(|ui| {
            if history_button(ui, "Undo", can_undo) {
                apply_history(editor, state, HistoryKey::Undo);
            }
            if history_button(ui, "Redo", can_redo) {
                apply_history(editor, state, HistoryKey::Redo);
            }
        });
    });
}

fn history_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> bool {
    let response = ui
        .add_enabled(enabled, egui::Button::new(text))
        .on_hover_text(HISTORY_TOOLTIP)
        .on_disabled_hover_text(HISTORY_TOOLTIP);
    probe(ui, || (egui::Id::new(("button", text)), response.rect));
    response.clicked()
}

/// Undoes or redoes one step. A restored draft can have different contents, so the
/// selection (and any arrival-spawn view) is dropped; a step that cannot be restored
/// is reported in the editor message and the draft is left as it was.
pub(crate) fn apply_history(editor: &mut EditorSession, state: &mut EditorUi, key: HistoryKey) {
    if editor.link_active() {
        // The room is not changed by undo while an exit link is open.
        return;
    }
    let result = match key {
        HistoryKey::Undo => undo_session(&mut state.history, editor),
        HistoryKey::Redo => redo_session(&mut state.history, editor),
    };
    match result {
        Ok(true) => {
            state.selected = None;
            state.dragging = None;
            state.destination = None;
            state.terrain.clear();
            editor.message.clear();
        }
        Ok(false) => {}
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{
        click_button, editing_game as editor, frame, laid_out, wider,
    };
    use super::*;
    use hollow_steps::editor::Selection;
    use hollow_steps::editor_history::tracked;
    use hollow_steps::room_size::resize_session;

    #[test]
    fn undo_and_redo_buttons_step_through_edits_and_drop_the_selection() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let original = editor.draft.layout.bounds.2;
        let (_, edited) = wider(editor, 100.0);
        tracked(&mut state.history, editor, |e| resize_session(e, edited)).unwrap();
        editor.dirty = false;
        state.selected = Some(Selection::Exit(0));
        state.destination = Some(0);
        let ctx = laid_out(editor, &mut state);

        click_button(&ctx, "Undo", editor, &mut state);
        assert_eq!(editor.draft.layout.bounds.2, original);
        assert!(editor.dirty, "undo is an unsaved change");
        assert_eq!((state.selected, state.destination), (None, None));

        // Nothing left to undo: the button is disabled and does nothing.
        click_button(&ctx, "Undo", editor, &mut state);
        assert_eq!(editor.draft.layout.bounds.2, original);

        click_button(&ctx, "Redo", editor, &mut state);
        assert_eq!(editor.draft.layout.bounds.2, edited.width);
        click_button(&ctx, "Redo", editor, &mut state);
        assert_eq!(editor.draft.layout.bounds.2, edited.width);
    }

    #[test]
    fn the_buttons_do_nothing_with_an_empty_history() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let before = editor.draft.clone();
        let ctx = laid_out(editor, &mut state);
        click_button(&ctx, "Undo", editor, &mut state);
        click_button(&ctx, "Redo", editor, &mut state);
        assert_eq!(editor.draft, before);
        assert!(!editor.dirty);
    }

    #[test]
    fn a_step_that_cannot_be_restored_is_reported_and_leaves_the_draft_alone() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let placed = hollow_steps::editor_history::tracked(&mut state.history, editor, |s| {
            s.place(
                hollow_steps::editor::Placement::Enemy("patrol".into()),
                macroquad::math::Vec2::new(300.0, 50.0),
            )
        })
        .unwrap();
        hollow_steps::editor_history::tracked(&mut state.history, editor, |s| {
            s.move_selection(placed, macroquad::math::Vec2::new(340.0, 50.0))
        })
        .unwrap();
        let path = editor.content.entity_path("patrol").unwrap();
        let mut entity = editor.content.entities.get(&path).unwrap().clone();
        entity.name = "renamed patrol".into();
        editor.update_entity(&path, entity).unwrap();
        let after_rename = editor.draft.clone();
        state.selected = Some(placed);

        apply_history(editor, &mut state, HistoryKey::Undo);
        assert!(
            editor.message.contains("cannot restore"),
            "{}",
            editor.message
        );
        assert_eq!(editor.draft, after_rename);
        assert_eq!(
            state.selected,
            Some(placed),
            "a refused step keeps the selection"
        );
    }

    #[test]
    fn hovering_undo_or_redo_says_definition_edits_are_not_covered() {
        for (label, edited) in [("Undo", false), ("Undo", true), ("Redo", false)] {
            let mut game = editor();
            let editor = game.editor_mut().unwrap();
            let mut state = EditorUi::default();
            if edited {
                let (_, bigger) = wider(editor, 10.0);
                tracked(&mut state.history, editor, |e| resize_session(e, bigger)).unwrap();
            }
            let ctx = egui::Context::default();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(360.0, 1200.0));
            let run = |time: f64,
                       events: Vec<egui::Event>,
                       editor: &mut EditorSession,
                       state: &mut EditorUi| {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        time: Some(time),
                        events,
                        ..Default::default()
                    },
                    |ctx| {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            draw_history_section(ui, editor, state);
                        });
                    },
                );
                super::super::test_support::drawn_text(&output)
            };
            for i in 0..3 {
                run(i as f64 * 0.1, vec![], editor, &mut state);
            }
            let rect: egui::Rect = ctx
                .data(|data| data.get_temp(egui::Id::new(("button", label))))
                .unwrap();
            let mut text = run(
                0.3,
                vec![egui::Event::PointerMoved(rect.center())],
                editor,
                &mut state,
            );
            for time in [0.4, 1.0, 1.5, 2.0] {
                text = run(time, vec![], editor, &mut state);
            }
            assert!(
                text.contains("Entity, behavior and boss definition edits are not covered"),
                "{label} (edited: {edited}) shows no tooltip: {text:?}"
            );
        }
    }

    #[test]
    fn the_history_section_shows_undo_and_redo() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let ctx = egui::Context::default();
        let mut text = String::new();
        for _ in 0..3 {
            text = frame(&ctx, vec![], editor, &mut state);
        }
        for expected in ["History", "Undo", "Redo"] {
            assert!(text.contains(expected), "missing {expected:?} in {text:?}");
        }
    }

    fn panel_frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 3000.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                super::super::draw_panel(ctx, editor, state);
            },
        );
    }

    /// Clicks a button of the whole panel, then lets the next frames lay out.
    fn click_panel(
        ctx: &egui::Context,
        label: &str,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) {
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", label))))
            .unwrap_or_else(|| panic!("no {label} button"));
        let [press, release] = super::super::test_support::click(rect.center());
        panel_frame(ctx, press, editor, state);
        panel_frame(ctx, release, editor, state);
        for _ in 0..2 {
            panel_frame(ctx, vec![], editor, state);
        }
    }

    fn panel_ready(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
        let ctx = egui::Context::default();
        for _ in 0..3 {
            panel_frame(&ctx, vec![], editor, state);
        }
        ctx
    }

    #[test]
    fn add_exit_from_the_panel_is_one_undo_step() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        state.target.selected = Some(0);
        let before = editor.draft.clone();
        let ctx = panel_ready(editor, &mut state);

        click_panel(&ctx, "Add exit", editor, &mut state);
        assert_eq!(
            editor.draft.layout.exits.len(),
            before.layout.exits.len() + 1
        );
        assert!(editor.dirty);

        click_panel(&ctx, "Undo", editor, &mut state);
        assert_eq!(editor.draft, before);
        click_panel(&ctx, "Redo", editor, &mut state);
        assert_eq!(
            editor.draft.layout.exits.len(),
            before.layout.exits.len() + 1
        );
    }

    #[test]
    fn set_exit_target_from_the_panel_is_one_undo_step() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        state.target.selected = Some(0);
        let ctx = panel_ready(editor, &mut state);
        click_panel(&ctx, "Add exit", editor, &mut state);
        let Some(Selection::Exit(index)) = state.selected else {
            panic!("the new exit is selected: {:?}", state.selected);
        };
        let added = editor.draft.clone();
        let first_target = added.layout.exits[index].target_room;

        state.target.selected = Some(1);
        assert_ne!(
            editor.rooms[1].0, first_target,
            "the test needs a different room"
        );
        click_panel(&ctx, "Set exit target", editor, &mut state);
        assert_eq!(editor.message, "", "the retarget was accepted");
        assert_eq!(
            editor.draft.layout.exits[index].target_room,
            editor.rooms[1].0
        );

        // One undo undoes only the retarget; the exit stays.
        click_panel(&ctx, "Undo", editor, &mut state);
        assert_eq!(editor.draft, added);
    }

    #[test]
    fn delete_selected_from_the_panel_is_one_undo_step() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let before = editor.draft.clone();
        assert!(
            !before.layout.items.is_empty(),
            "the fixture room has an item"
        );
        state.selected = Some(Selection::Item(0));
        let ctx = panel_ready(editor, &mut state);

        click_panel(&ctx, "Delete selected", editor, &mut state);
        assert_eq!(
            editor.draft.layout.items.len(),
            before.layout.items.len() - 1
        );
        assert_eq!(state.selected, None);

        click_panel(&ctx, "Undo", editor, &mut state);
        assert_eq!(editor.draft, before);
    }
}
