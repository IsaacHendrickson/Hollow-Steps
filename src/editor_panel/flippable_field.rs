//! The "Seed flip" section of the Room tools panel: one checkbox that marks the room as one a
//! run's seed may play mirrored left to right (`RoomDraft::set_flippable`, `docs/variety.md`).
//!
//! The editor, Save and the Test room always show the room as authored; only a run mirrors it,
//! by its own draw against `room_flip_percent` in `variety.ron`. A change is one undo step and
//! marks the room unsaved.
use super::widgets::{panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::tracked;

const HINT: &str = "A run's seed may play this room mirrored left to right, by chance \
     (room_flip_percent in variety.ron, 50 by default). The editor, Save and the Test room \
     always show it as authored, and the hub is never mirrored.";

/// The "Seed flip" section with its "Flippable" checkbox.
pub(super) fn draw_flippable(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    section(ui, "Seed flip", |ui| {
        let mut flippable = editor.draft.layout.flippable;
        let tick = ui.checkbox(&mut flippable, "Flippable").on_hover_text(HINT);
        probe(ui, || {
            (egui::Id::new(("flip field", "flippable")), tick.rect)
        });
        if tick.changed() {
            let result = tracked(&mut state.history, editor, |e| {
                e.draft.set_flippable(flippable)
            });
            match result {
                Ok(()) => {
                    editor.dirty = true;
                    editor.message.clear();
                }
                Err(error) => editor.message = error,
            }
        }
        panel_label(ui, "Mirrored in some runs, by the seed.");
    });
}

#[cfg(test)]
mod tests;
