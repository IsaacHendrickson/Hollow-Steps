//! The "Locked by" control of the Room tools panel: a dropdown on the selected exit (an edge
//! exit or a door) that picks the key needed to go through it, or None. Choosing is one undo
//! step and marks the room unsaved; choosing what is already set changes nothing.
use super::widgets::{choice, panel_label, probe};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::tracked;
use hollow_steps::exit_lock::{lock_label, FIRST_KEY, LAST_KEY};

/// Room for "None" and the eight keys at once; the popup's default height would scroll the last.
const LIST_HEIGHT: f32 = 9.0 * 28.0;

/// What the dropdown shows for a lock: "None" or "Key 3".
fn shown(locked_by: Option<u8>) -> String {
    lock_label(locked_by).unwrap_or_else(|| "None".to_owned())
}

/// The dropdown for exit `index`; nothing when there is no such exit.
pub(super) fn draw_exit_lock(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
) {
    let Some(current) = editor.draft.layout.exits.get(index).map(|e| e.locked_by) else {
        return;
    };
    panel_label(ui, "Locked by");
    let mut picked = None;
    let combo = egui::ComboBox::from_id_salt(egui::Id::new(("exit lock",)))
        .width(ui.available_width())
        .height(LIST_HEIGHT)
        .selected_text(shown(current))
        .show_ui(ui, |ui| {
            let options = std::iter::once(None).chain((FIRST_KEY..=LAST_KEY).map(Some));
            for option in options {
                if choice(ui, &shown(option), option == current).clicked() && option != current {
                    picked = Some(option);
                    ui.close_menu();
                }
            }
        });
    probe(ui, || (egui::Id::new(("exit lock",)), combo.response.rect));
    if let Some(lock) = picked {
        set_lock(editor, state, index, lock);
    }
}

/// Sets the lock of exit `index` as one undo step; a refused edit says why and changes nothing.
pub(super) fn set_lock(
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
    lock: Option<u8>,
) {
    match tracked(&mut state.history, editor, |e| {
        e.draft.set_exit_lock(index, lock)
    }) {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
        }
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests;
