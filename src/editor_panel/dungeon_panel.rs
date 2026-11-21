//! The "Check dungeon" button of the Room tools panel's Room section (`docs/dungeon-check.md`)
//! and what it found: one line per problem, or that there are none. The list is as of the press;
//! pressing again checks again. It has no section of its own: the panel has to fit the layout
//! test's window.
use super::widgets::{panel_button, panel_label};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;

pub(super) fn draw_dungeon_check(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    if panel_button(ui, "Check dungeon") {
        state.dungeon_check = Some(editor.check_dungeon().iter().map(|p| p.text()).collect());
    }
    match &state.dungeon_check {
        None => {}
        Some(lines) if lines.is_empty() => panel_label(ui, "No problems found"),
        Some(lines) => {
            panel_label(ui, &format!("{} problem(s) found", lines.len()));
            for line in lines {
                ui.add(
                    egui::Label::new(egui::RichText::new(line).color(egui::Color32::LIGHT_RED))
                        .wrap(),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests;
