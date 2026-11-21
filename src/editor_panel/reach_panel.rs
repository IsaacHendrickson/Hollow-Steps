//! What the Room tools panel says while the Reach toggle is on (`docs/reach.md`): what the two
//! outlines on the canvas mean and where they start, or, when the player's numbers leave no gap
//! that needs Dash, why there is nothing to draw. Nothing is measured while the toggle is off.
use super::widgets::panel_label;
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;

pub(super) fn draw_reach_notes(ui: &mut egui::Ui, editor: &EditorSession, state: &mut EditorUi) {
    if !state.checks.reach {
        return;
    }
    let interval = state.checks.interval(editor);
    match interval.error() {
        Some(error) => {
            ui.add(
                egui::Label::new(egui::RichText::new(error).color(egui::Color32::LIGHT_RED)).wrap(),
            );
        }
        None => {
            let from = match state.selected {
                Some(Selection::Inbound { .. }) => "the selected arrival",
                _ => "the room spawn",
            };
            panel_label(
                ui,
                &format!("Reach from {from}: the outlines are a jump without Dash and with it"),
            );
        }
    }
}

#[cfg(test)]
mod tests;
