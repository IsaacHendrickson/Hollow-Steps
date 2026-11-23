//! The dungeon-piece controls of the Room tools panel (`hollow_steps::piece`, `editor_piece`):
//! the "This room is a piece" box with its role and weight, "Add pickup slot", and the kind of
//! the selected slot. A slot is moved by dragging it on the canvas and deleted like any marker
//! (Delete selected, the Delete key, right click).
//!
//! Every change is one undo step and marks the room unsaved, and a dragged weight is one step for
//! the whole drag, like the room size fields. A change the rules refuse leaves the draft as it
//! was and says why in the editor's message.
use super::widgets::{choice, panel_button, panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor_history::tracked;
use hollow_steps::editor_tools::EditorTool;
use hollow_steps::piece::{PieceInfo, PieceRole, SlotKind, MAX_SLOTS, MAX_WEIGHT, MIN_WEIGHT};

const MARK_HINT: &str = "Marks this room as one piece of a dungeon a generator can build. \
     The game never reads the mark; Check dungeon checks the pieces together.";
const ROLE_HINT: &str =
    "Start: where the run begins. Middle: anywhere between. Boss: the last room.";
const WEIGHT_HINT: &str =
    "How likely the generator is to pick this piece, 1 (rarely) to 10 (often).";
const KIND_HINT: &str = "What the generator may put in this slot: a key, an ability, or either. \
     The Test room and the game never show a slot.";

/// Reports the result of an edit: an accepted one marks the room unsaved, a refused one says why.
fn report(editor: &mut EditorSession, result: Result<(), String>) {
    match result {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
        }
        Err(error) => editor.message = error,
    }
}

/// Applies one edit as one undo step.
fn apply(
    editor: &mut EditorSession,
    state: &mut EditorUi,
    change: impl FnOnce(&mut EditorSession) -> Result<(), String>,
) {
    let result = tracked(&mut state.history, editor, change);
    report(editor, result);
}

/// Applies an edit of a draggable box. A drag keeps a gesture open from its first move to its
/// release and records the whole drag once; anything else is recorded at once.
fn commit_drag(
    editor: &mut EditorSession,
    state: &mut EditorUi,
    response: &egui::Response,
    changed: bool,
    change: impl FnOnce(&mut EditorSession) -> Result<(), String>,
) {
    if response.drag_started() {
        state.history.begin_gesture(&editor.draft);
    }
    if changed {
        if state.history.gesture_open() {
            let result = change(editor);
            report(editor, result);
        } else {
            apply(editor, state, change);
        }
    }
    if response.drag_stopped() {
        state.history.end_gesture(&editor.draft);
    }
}

/// The "Dungeon piece" section: the mark, its role and weight, and "Add pickup slot".
pub(super) fn draw_piece_section(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    section(ui, "Dungeon piece", |ui| {
        let current = editor.draft.layout.piece;
        let mut marked = current.is_some();
        let tick = ui
            .checkbox(&mut marked, "This room is a piece")
            .on_hover_text(MARK_HINT);
        probe(ui, || (egui::Id::new(("piece field", "mark")), tick.rect));
        if tick.changed() {
            // A new mark starts as a Middle piece at the lightest weight.
            let to = marked.then(|| PieceInfo::new(PieceRole::Middle));
            apply(editor, state, |e| e.draft.set_piece(to));
        }
        if let Some(info) = current {
            draw_role_and_weight(ui, editor, state, info);
        }
        let slots = editor.draft.layout.slots.len();
        if panel_button(ui, "Add pickup slot") {
            add_slot(editor, state);
        }
        panel_label(ui, &format!("{slots} of {MAX_SLOTS} pickup slots"));
        if current.is_none() && slots > 0 {
            panel_label(ui, "Not a piece: its slots are kept and do nothing.");
        }
        panel_label(ui, "Teal ring: pickup slot (K key, A ability, ? either)");
    });
}

fn draw_role_and_weight(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    info: PieceInfo,
) {
    egui::Grid::new("piece fields")
        .num_columns(2)
        .show(ui, |ui| {
            panel_label(ui, "Role");
            let role = pick(
                ui,
                "Role",
                info.role,
                &PieceRole::ALL,
                PieceRole::name,
                ROLE_HINT,
            );
            if role != info.role {
                apply(editor, state, |e| {
                    e.draft.set_piece(Some(PieceInfo { role, ..info }))
                });
            }
            ui.end_row();

            panel_label(ui, "Weight");
            let mut weight = info.weight;
            let drag = ui
                .add(
                    egui::DragValue::new(&mut weight)
                        .range(MIN_WEIGHT..=MAX_WEIGHT)
                        .speed(0.1)
                        .fixed_decimals(0),
                )
                .on_hover_text(WEIGHT_HINT);
            probe(ui, || (egui::Id::new(("piece field", "Weight")), drag.rect));
            commit_drag(editor, state, &drag, weight != info.weight, |e| {
                e.draft.set_piece(Some(PieceInfo { weight, ..info }))
            });
            ui.end_row();
        });
}

/// "Add pickup slot": an Any slot on the floor next to the spawn point (drag it where it
/// belongs). One undo step.
fn add_slot(editor: &mut EditorSession, state: &mut EditorUi) {
    match tracked(&mut state.history, editor, |e| e.draft.add_slot()) {
        Ok(selection) => {
            state.selected = Some(selection);
            editor.dirty = true;
            editor.message.clear();
            state.tool = EditorTool::Select;
            state.destination = None;
        }
        Err(error) => editor.message = error,
    }
}

/// The kind of slot `index`: what the generator may put in it.
pub(super) fn draw_slot_fields(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    index: usize,
) {
    let Some(current) = editor.draft.slot(index).map(|slot| slot.kind) else {
        return;
    };
    section(ui, "Pickup slot", |ui| {
        egui::Grid::new("slot fields")
            .num_columns(2)
            .show(ui, |ui| {
                panel_label(ui, "Kind");
                let kind = pick(
                    ui,
                    "Kind",
                    current,
                    &SlotKind::ALL,
                    SlotKind::name,
                    KIND_HINT,
                );
                if kind != current {
                    apply(editor, state, |e| e.draft.set_slot_kind(index, kind));
                }
                ui.end_row();
            });
        panel_label(
            ui,
            "Drag it on the canvas to move it; Delete selected removes it.",
        );
    });
}

/// A dropdown: the choices by name, `current` shown; returns the one now chosen.
fn pick<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    current: T,
    all: &[T],
    name: fn(T) -> &'static str,
    hint: &str,
) -> T {
    let mut picked = current;
    let id = egui::Id::new(("piece pick", label));
    let combo = egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .truncate()
        .selected_text(name(current))
        .show_ui(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for &option in all {
                if choice(ui, name(option), option == current).clicked() {
                    picked = option;
                    ui.close_menu();
                }
            }
        });
    combo.response.clone().on_hover_text(hint);
    probe(ui, || (id, combo.response.rect));
    picked
}

#[cfg(test)]
mod tests;
