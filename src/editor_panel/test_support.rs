//! Helpers shared by the editor panel's headless tests.
use super::canvas_tools::{draw_room_size_section, draw_view_section};
use super::history_panel::draw_history_section;
use super::tool_panel::draw_tool_section;
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::room_size::RoomSize;

/// All text drawn in one egui pass, one line per text shape.
pub(super) fn drawn_text(output: &egui::FullOutput) -> String {
    fn collect(shape: &egui::epaint::Shape, out: &mut String) {
        match shape {
            egui::epaint::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let mut text = String::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

pub(super) fn click(at: egui::Pos2) -> [Vec<egui::Event>; 2] {
    let button = |pressed| {
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
    [button(true), button(false)]
}

/// The fixture game with its editor open.
pub(super) fn editing_game() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

/// One egui pass over the Tools, History, Room size and View sections in a narrow window.
pub(super) fn frame(
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
                draw_tool_section(ui, editor, state);
                draw_history_section(ui, editor, state);
                draw_room_size_section(ui, editor, &mut state.history);
                draw_view_section(ui, state);
            });
        },
    );
    drawn_text(&output)
}

/// Clicks a button drawn by `frame`, then lets one more frame run.
pub(super) fn click_button(
    ctx: &egui::Context,
    label: &str,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("button", label))))
        .unwrap_or_else(|| panic!("no {label} button"));
    let [press, release] = click(rect.center());
    frame(ctx, press, editor, state);
    frame(ctx, release, editor, state);
    frame(ctx, vec![], editor, state);
}

/// A fresh context that has run enough frames for every widget to be laid out.
pub(super) fn laid_out(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
    let ctx = egui::Context::default();
    for _ in 0..3 {
        frame(&ctx, vec![], editor, state);
    }
    ctx
}

/// The current room size and the same size made `by` wider.
pub(super) fn wider(editor: &EditorSession, by: f32) -> (RoomSize, RoomSize) {
    let current = RoomSize::of(&editor.draft.layout);
    let mut edited = current;
    edited.width += by;
    (current, edited)
}
