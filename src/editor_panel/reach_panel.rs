//! What the Room tools panel says while the Reach toggle is on (`docs/reach.md`): what the
//! outlines on the canvas mean and where they start, and, for each ability that no gap or ledge
//! can need with the player's numbers, why (in red). Nothing is measured while the toggle is off.
use super::widgets::panel_label;
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;

pub(super) fn draw_reach_notes(ui: &mut egui::Ui, editor: &EditorSession, state: &mut EditorUi) {
    if !state.checks.reach {
        return;
    }
    let interval = state.checks.interval(editor);
    for (_, error) in interval.errors() {
        ui.add(egui::Label::new(egui::RichText::new(error).color(egui::Color32::LIGHT_RED)).wrap());
    }
    let drawn: Vec<&str> = interval
        .usable_abilities()
        .into_iter()
        .map(|ability| ability.name())
        .collect();
    if drawn.is_empty() {
        return;
    }
    let from = match state.selected {
        Some(Selection::Inbound { .. }) => "the selected arrival",
        _ => "the room spawn",
    };
    panel_label(
        ui,
        &format!(
            "Reach from {from}: the outlines are a jump with no ability, and with {}",
            drawn.join(" and ")
        ),
    );
}

#[cfg(test)]
mod tests;
