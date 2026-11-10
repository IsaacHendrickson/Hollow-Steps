//! Panel sections for the canvas view (grid, snap size, reset) and the room size. The
//! rules live in `hollow_steps::editor_view` and `hollow_steps::room_size`.
use std::ops::RangeInclusive;

use super::{panel_button, panel_label, section, EditorSession, EditorUi};
use egui_macroquad::egui;
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

pub(super) fn draw_room_size_section(ui: &mut egui::Ui, editor: &mut EditorSession) {
    section(ui, "Room size", |ui| {
        let current = RoomSize::of(&editor.draft.layout);
        let mut edited = current;
        egui::Grid::new("room size").num_columns(2).show(ui, |ui| {
            size_row(ui, "X", &mut edited.x, COORDINATES);
            size_row(ui, "Y", &mut edited.y, COORDINATES);
            size_row(ui, "Width", &mut edited.width, SIZES);
            size_row(ui, "Height", &mut edited.height, SIZES);
            size_row(ui, "Floor Y", &mut edited.floor_y, COORDINATES);
        });
        apply_size(editor, current, edited);
    });
}

fn size_row(ui: &mut egui::Ui, label: &str, value: &mut f32, range: RangeInclusive<f32>) {
    panel_label(ui, label);
    ui.add(
        egui::DragValue::new(value)
            .range(range)
            .speed(1.0)
            .fixed_decimals(0)
            .suffix(" px"),
    );
    ui.end_row();
}

/// Applies an edit made this frame. A refused size leaves the draft as it was, so the
/// field snaps back next frame, and says why in the editor message.
fn apply_size(editor: &mut EditorSession, current: RoomSize, edited: RoomSize) {
    if edited == current {
        return;
    }
    match resize_session(editor, edited) {
        Ok(()) => editor.message.clear(),
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hollow_steps::game::GameState;

    fn editor() -> GameState {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        let mut game = GameState::try_new(&assets).unwrap();
        game.toggle_editor().unwrap();
        game
    }

    fn shape_text(shape: &egui::epaint::Shape, out: &mut String) {
        match shape {
            egui::epaint::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|s| shape_text(s, out)),
            _ => {}
        }
    }

    fn frame(
        ctx: &egui::Context,
        events: Vec<egui::Event>,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) -> String {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(360.0, 1200.0),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw_room_size_section(ui, editor);
                    draw_view_section(ui, state);
                });
            },
        );
        let mut text = String::new();
        for clipped in &output.shapes {
            shape_text(&clipped.shape, &mut text);
        }
        text
    }

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
        let at = rect.center();
        let press = |pressed| {
            vec![
                egui::Event::PointerMoved(at),
                egui::Event::PointerButton {
                    pos: at,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: Default::default(),
                },
            ]
        };
        frame(&ctx, press(true), editor, &mut state);
        frame(&ctx, press(false), editor, &mut state);
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
        apply_size(editor, current, edited);
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
        apply_size(editor, current, edited);
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
        apply_size(editor, current, current);
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
}
