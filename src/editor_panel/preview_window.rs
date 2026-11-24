//! The Preview generated dungeon window of the editor (`docs/dungeon-gen.md`): a seed box and
//! Roll next, and the dungeon that seed builds from the pieces as the editor holds them, unsaved
//! drafts included. It draws the plan with the Dungeon map window's drawing (rooms named by
//! their piece, locked exits tagged with their key number) and lists where each key and ability
//! lies, or says why there is no dungeon. Read-only: it writes no file and never touches the
//! running game; the model is `hollow_steps::dungeon_preview`, which this only draws.
use super::map_window::draw_map_canvas;
use super::widgets::{panel_button, panel_label, probe};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::dungeon_preview::{DungeonPreview, Outcome, PlanView};

const WINDOW_SIZE: [f32; 2] = [520.0, 560.0];
const MAP_HEIGHT: f32 = 260.0;
const SEED_WIDTH: f32 = 190.0;
const ROLL_NEXT: &str = "Roll next";

/// The button of the Dungeon piece section that opens and closes the window.
pub(super) fn draw_preview_button(ui: &mut egui::Ui, state: &mut EditorUi) {
    if panel_button(ui, "Preview generated dungeon") {
        state.checks.preview_window = !state.checks.preview_window;
    }
}

/// Draws the window while it is open.
pub(super) fn draw_preview_window(
    ctx: &egui::Context,
    editor: &EditorSession,
    state: &mut EditorUi,
) {
    if !state.checks.preview_window {
        return;
    }
    let mut shown = true;
    egui::Window::new("Generated dungeon preview")
        .open(&mut shown)
        .default_size(WINDOW_SIZE)
        .min_size([260.0, 220.0])
        .collapsible(false)
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    draw_contents(ui, editor, &mut state.checks.preview)
                });
        });
    state.checks.preview_window = shown;
}

fn draw_contents(ui: &mut egui::Ui, editor: &EditorSession, preview: &mut DungeonPreview) {
    panel_label(
        ui,
        "Read-only: plans a dungeon from the pieces as the editor holds them, unsaved edits \
         included. Nothing is saved and the game is not touched.",
    );
    // The box and the button come first, so the plan below is for what was typed this frame.
    let rolled = ui
        .horizontal(|ui| {
            panel_label(ui, "Seed");
            let field = ui.add(
                egui::TextEdit::singleline(preview.text_mut())
                    .desired_width(SEED_WIDTH)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("hex, 1 to 16 digits"),
            );
            probe(ui, || (egui::Id::new(("preview", "seed")), field.rect));
            let roll = ui.button(ROLL_NEXT);
            probe(ui, || (egui::Id::new(("button", ROLL_NEXT)), roll.rect));
            roll.clicked()
        })
        .inner;
    if rolled {
        preview.roll_next();
    }
    preview.refresh(editor.preview_inputs());

    if let Some(note) = preview.note() {
        panel_label(ui, note);
    }
    match preview.outcome() {
        None => {}
        Some(Outcome::BadSeed(reason)) => problem(ui, &format!("Not a seed: {reason}")),
        Some(Outcome::Failed(reason)) => {
            problem(ui, &format!("No dungeon: {reason}"));
        }
        Some(Outcome::Plan(view)) => draw_plan(ui, view),
    }
}

fn problem(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(egui::RichText::new(text).color(egui::Color32::LIGHT_RED)).wrap());
}

fn draw_plan(ui: &mut egui::Ui, view: &PlanView) {
    ui.separator();
    panel_label(ui, &view.summary());
    panel_label(ui, "Tags on the links are the key a locked exit needs.");
    let size = egui::vec2(ui.available_width().max(200.0), MAP_HEIGHT);
    draw_map_canvas(ui, &view.map, size, false);
    ui.separator();
    ui.strong("Keys and abilities");
    if view.pickups.is_empty() {
        panel_label(ui, "None: this dungeon has no key or ability.");
    }
    for pickup in &view.pickups {
        panel_label(ui, &pickup.line());
    }
}
