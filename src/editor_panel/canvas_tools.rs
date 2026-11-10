//! Panel sections for the canvas view (grid, snap size, reset) and the room size. The
//! rules live in `hollow_steps::editor_view` and `hollow_steps::room_size`.
use std::ops::RangeInclusive;

use super::{panel_button, panel_label, section, EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::{tracked, History};
use hollow_steps::editor_view::{MAX_SNAP, MIN_SNAP};
use hollow_steps::room_size::{resize_session, RoomSize};

/// Loose limits for the drag fields; `RoomLayout::validate_geometry` decides what is valid.
const COORDINATES: RangeInclusive<f32> = -10_000.0..=10_000.0;
const SIZES: RangeInclusive<f32> = 1.0..=20_000.0;

pub(super) fn draw_view_section(ui: &mut egui::Ui, state: &mut EditorUi) {
    section(ui, "View", |ui| {
        ui.checkbox(&mut state.grid.enabled, "Grid and snap");
        ui.horizontal(|ui| {
            panel_label(ui, "Snap size");
            ui.add(
                egui::DragValue::new(&mut state.grid.size)
                    .range(MIN_SNAP..=MAX_SNAP)
                    .speed(1.0)
                    .fixed_decimals(0)
                    .suffix(" px"),
            );
        });
        if panel_button(ui, "Reset view") {
            state.view = None;
        }
    });
}

pub(super) fn draw_room_size_section(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    history: &mut History,
) {
    section(ui, "Room size", |ui| {
        let current = RoomSize::of(&editor.draft.layout);
        let mut edited = current;
        let rows = egui::Grid::new("room size").num_columns(2).show(ui, |ui| {
            [
                size_row(ui, "X", &mut edited.x, COORDINATES),
                size_row(ui, "Y", &mut edited.y, COORDINATES),
                size_row(ui, "Width", &mut edited.width, SIZES),
                size_row(ui, "Height", &mut edited.height, SIZES),
                size_row(ui, "Floor Y", &mut edited.floor_y, COORDINATES),
            ]
        });
        let started = rows.inner.iter().any(|row| row.drag_started());
        let stopped = rows.inner.iter().any(|row| row.drag_stopped());
        if started {
            // Dragging a field resizes every frame; the whole drag is one undo step.
            history.begin_gesture(&editor.draft);
        }
        apply_size(editor, history, current, edited);
        if stopped {
            history.end_gesture(&editor.draft);
        }
    });
}

fn size_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
) -> egui::Response {
    panel_label(ui, label);
    let response = ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(1.0)
            .fixed_decimals(0)
            .suffix(" px"),
    );
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(egui::Id::new(("size field", label)), response.rect));
    ui.end_row();
    response
}

/// Applies an edit made this frame. A refused size leaves the draft as it was, so the
/// field snaps back next frame, and says why in the editor message. Outside a drag each
/// accepted edit is its own undo step; during one, the drag as a whole is recorded when
/// it ends.
fn apply_size(
    editor: &mut EditorSession,
    history: &mut History,
    current: RoomSize,
    edited: RoomSize,
) {
    if edited == current {
        return;
    }
    let result = if history.gesture_open() {
        resize_session(editor, edited)
    } else {
        tracked(history, editor, |e| resize_session(e, edited))
    };
    match result {
        Ok(()) => editor.message.clear(),
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests {
    use super::super::history_panel::apply_history;
    use super::super::test_support::{editing_game as editor, frame, laid_out, wider};
    use super::*;
    use hollow_steps::editor_history::HistoryKey;

    #[test]
    fn sections_show_every_control() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let ctx = egui::Context::default();
        let mut state = EditorUi::default();
        let mut text = String::new();
        for _ in 0..3 {
            text = frame(&ctx, vec![], editor, &mut state);
        }
        for expected in [
            "Room size",
            "X",
            "Y",
            "Width",
            "Height",
            "Floor Y",
            "View",
            "Grid and snap",
            "Snap size",
            "Reset view",
        ] {
            assert!(text.contains(expected), "missing {expected:?} in {text:?}");
        }
        // Showing the controls is not an edit.
        assert!(!editor.dirty);
    }

    #[test]
    fn reset_view_clears_a_manual_view() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let ctx = egui::Context::default();
        let mut state = EditorUi {
            view: Some(hollow_steps::editor_view::CanvasView {
                offset: macroquad::math::Vec2::ZERO,
                scale: 2.0,
            }),
            ..Default::default()
        };
        for _ in 0..3 {
            frame(&ctx, vec![], editor, &mut state);
        }
        let rect: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("button", "Reset view"))))
            .unwrap();
        let [press, release] = super::super::test_support::click(rect.center());
        frame(&ctx, press, editor, &mut state);
        frame(&ctx, release, editor, &mut state);
        frame(&ctx, vec![], editor, &mut state);
        assert!(state.view.is_none());
    }

    #[test]
    fn an_accepted_edit_resizes_the_draft_and_clears_the_message() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        editor.message = "stale".into();
        let current = RoomSize::of(&editor.draft.layout);
        let mut edited = current;
        edited.width += 100.0;
        apply_size(editor, &mut History::default(), current, edited);
        assert_eq!(editor.draft.layout.bounds.2, edited.width);
        assert!(editor.dirty);
        assert!(editor.message.is_empty());
    }

    #[test]
    fn a_refused_edit_keeps_the_draft_and_explains_why() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let before = editor.draft.layout.clone();
        let current = RoomSize::of(&before);
        let mut edited = current;
        edited.width = 300.0;
        apply_size(editor, &mut History::default(), current, edited);
        assert_eq!(editor.draft.layout, before);
        assert!(!editor.dirty);
        assert!(!editor.message.is_empty());
    }

    #[test]
    fn an_unchanged_size_does_nothing() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        editor.message = "keep me".into();
        let current = RoomSize::of(&editor.draft.layout);
        apply_size(editor, &mut History::default(), current, current);
        assert!(!editor.dirty);
        assert_eq!(editor.message, "keep me");
    }

    #[test]
    fn wheel_over_the_canvas_leaves_the_panel_alone_but_scrolls_it_over_the_panel() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let ctx = egui::Context::default();
        let mut state = EditorUi::default();
        let size = egui::vec2(960.0, 540.0);
        let draw = |events: Vec<egui::Event>, editor: &mut EditorSession, state: &mut EditorUi| {
            let mut left = 0.0;
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |ctx| left = super::super::draw_panel(ctx, editor, state).left,
            );
            left
        };
        let offset = || -> f32 {
            let (_, _, offset, _): (egui::Rect, egui::Vec2, egui::Vec2, egui::Id) = ctx
                .data(|data| data.get_temp(egui::Id::new("panel scroll")))
                .unwrap();
            offset.y
        };
        let wheel = |at: egui::Pos2| {
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -28.0),
                    modifiers: Default::default(),
                },
            ]
        };
        let mut left = 0.0;
        for _ in 0..3 {
            left = draw(vec![], editor, &mut state);
        }
        assert!(left > 100.0 && left < size.x, "panel left {left}");
        let start = offset();
        // Pointer on the canvas side of the window: the panel does not move.
        draw(wheel(egui::pos2(left / 2.0, 300.0)), editor, &mut state);
        for _ in 0..20 {
            draw(vec![], editor, &mut state);
        }
        assert_eq!(offset(), start);
        // Pointer over the panel: the same wheel scrolls it.
        draw(wheel(egui::pos2(left + 60.0, 300.0)), editor, &mut state);
        for _ in 0..20 {
            draw(vec![], editor, &mut state);
        }
        assert!(offset() > start, "panel did not scroll: {}", offset());
    }

    #[test]
    fn separate_size_edits_are_separate_steps_and_a_refused_one_records_nothing() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut history = History::default();
        let original = editor.draft.layout.bounds.2;
        let (current, first) = wider(editor, 50.0);
        apply_size(editor, &mut history, current, first);
        let (current, second) = wider(editor, 50.0);
        apply_size(editor, &mut history, current, second);
        let current = RoomSize::of(&editor.draft.layout);
        let mut too_small = current;
        too_small.width = 300.0;
        apply_size(editor, &mut history, current, too_small);
        assert_eq!(editor.draft.layout.bounds.2, second.width);

        let mut state = EditorUi {
            history,
            ..Default::default()
        };
        apply_history(editor, &mut state, HistoryKey::Undo);
        assert_eq!(editor.draft.layout.bounds.2, first.width);
        apply_history(editor, &mut state, HistoryKey::Undo);
        assert_eq!(editor.draft.layout.bounds.2, original);
        assert!(!state.history.can_undo());
    }

    #[test]
    fn dragging_a_size_field_is_one_undo_step() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        let original = editor.draft.layout.bounds.2;
        let ctx = laid_out(editor, &mut state);
        let field: egui::Rect = ctx
            .data(|data| data.get_temp(egui::Id::new(("size field", "Width"))))
            .unwrap();
        let start = field.center();
        let button = |pressed| egui::Event::PointerButton {
            pos: start,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        };
        let mut seen = vec![original];
        let mut step =
            |events: Vec<egui::Event>, state: &mut EditorUi, editor: &mut EditorSession| {
                frame(&ctx, events, editor, state);
                seen.push(editor.draft.layout.bounds.2);
            };
        step(vec![egui::Event::PointerMoved(start)], &mut state, editor);
        step(vec![button(true)], &mut state, editor);
        for i in 1..=4 {
            let to = start + egui::vec2(15.0 * i as f32, 0.0);
            step(vec![egui::Event::PointerMoved(to)], &mut state, editor);
        }
        assert!(state.history.gesture_open(), "the drag is open mid-way");
        assert!(!state.history.can_undo(), "undo waits for the drag to end");
        let end = start + egui::vec2(60.0, 0.0);
        step(
            vec![egui::Event::PointerButton {
                pos: end,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            &mut state,
            editor,
        );
        step(vec![], &mut state, editor);

        let mut distinct = seen.clone();
        distinct.dedup();
        assert!(
            distinct.len() > 3,
            "the field changed over several frames: {seen:?}"
        );
        let resized = editor.draft.layout.bounds.2;
        assert!(resized > original);
        assert!(!state.history.gesture_open());

        apply_history(editor, &mut state, HistoryKey::Undo);
        assert_eq!(
            editor.draft.layout.bounds.2, original,
            "one undo undoes the whole drag"
        );
        assert!(!state.history.can_undo(), "the drag was a single step");
        apply_history(editor, &mut state, HistoryKey::Redo);
        assert_eq!(editor.draft.layout.bounds.2, resized);
    }
}
