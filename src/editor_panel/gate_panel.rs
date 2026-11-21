//! The gate controls of the Room tools panel (`docs/gates.md`): with an exit selected,
//! "Reaching this exit" lists every entrance with a checkbox per ability for what getting from
//! that entrance to this exit needs; and the gates whose entrance no longer exists are listed as
//! problems with a Remove button. Ticking is one undo step and marks the room unsaved.
use super::widgets::{panel_button, panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::ability::Ability;
use hollow_steps::editor_history::tracked;
use hollow_steps::gates::{Entrance, ExitRef};

/// The section for exit `index`; nothing when there is no such exit.
pub(super) fn draw_exit_gates(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
) {
    let Some(exit) = editor.draft.layout.exits.get(index).map(ExitRef::of) else {
        return;
    };
    let entrances = editor.gate_entrances();
    section(ui, "Reaching this exit", |ui| {
        for (row, info) in entrances.iter().enumerate() {
            panel_label(ui, &info.label);
            let needs = editor
                .draft
                .layout
                .gate_needs(&info.entrance, &exit)
                .to_vec();
            for ability in Ability::ALL {
                let was = needs.contains(ability);
                let mut needed = was;
                let response = ui.checkbox(&mut needed, format!("Needs {}", ability.name()));
                probe(ui, || {
                    (egui::Id::new(("gate", row, ability.name())), response.rect)
                });
                if needed != was {
                    set_gate(editor, state, info.entrance, index, *ability, needed);
                }
            }
        }
    });
}

/// Ticks or clears one ability of a gate as one undo step; a refused edit says why.
pub(super) fn set_gate(
    editor: &mut EditorSession,
    state: &mut EditorUi,
    from: Entrance,
    exit: usize,
    ability: Ability,
    needed: bool,
) {
    match tracked(&mut state.history, editor, |e| {
        e.draft.set_gate(from, exit, ability, needed)
    }) {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
        }
        Err(error) => editor.message = error,
    }
}

/// The gates that come from an entrance the room no longer has, each with a Remove button.
/// Draws nothing when there are none.
pub(super) fn draw_gate_problems(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    let orphans = editor.orphan_gates();
    if orphans.is_empty() {
        return;
    }
    section(ui, "Gate problems", |ui| {
        for orphan in orphans {
            ui.add(
                egui::Label::new(egui::RichText::new(&orphan.text).color(egui::Color32::LIGHT_RED))
                    .wrap(),
            );
            if panel_button(ui, format!("Remove gate {}", orphan.index + 1)) {
                match tracked(&mut state.history, editor, |e| {
                    e.draft.remove_gate(orphan.index)
                }) {
                    Ok(()) => {
                        editor.dirty = true;
                        editor.message.clear();
                    }
                    Err(error) => editor.message = error,
                }
            }
        }
    });
}

#[cfg(test)]
mod tests;
