//! The dungeon map controls of the Room tools panel (`docs/dungeon-map.md`): the "Map cell" row
//! of the room being edited, and the button that opens the Dungeon map window
//! (`map_window`). A change of the cell is one undo step and marks the room unsaved.
use super::widgets::{panel_button, panel_label, probe};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::dungeon_map::{Cell, CELL_LIMIT};
use hollow_steps::editor_history::{tracked, History};

pub(super) fn draw_map_controls(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    draw_map_cell(ui, editor, state);
    if panel_button(ui, "Dungeon map") {
        state.checks.map_window = !state.checks.map_window;
    }
}

/// The row: an Auto box (no cell: the exits place the room), the two numbers when it has one, and
/// where the map puts the room either way.
fn draw_map_cell(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    panel_label(ui, "Map cell");
    let own = editor.draft.layout.map_cell;
    let map = editor.dungeon_map();
    let shown = map.room(editor.id).map(|room| room.cell);

    let mut auto = own.is_none();
    let auto_box = ui.checkbox(&mut auto, "Auto (from the exits)");
    probe(ui, || (egui::Id::new(("map cell", "auto")), auto_box.rect));
    if auto_box.changed() {
        // Turning Auto off starts from where the room is now, so nothing jumps.
        let from = shown.unwrap_or((0, 0));
        let cell = (!auto).then(|| {
            (
                from.0.clamp(-CELL_LIMIT, CELL_LIMIT),
                from.1.clamp(-CELL_LIMIT, CELL_LIMIT),
            )
        });
        set_cell(editor, &mut state.history, cell);
    }

    if let Some(current) = editor.draft.layout.map_cell {
        let mut edited = current;
        let fields = ui
            .horizontal(|ui| {
                [
                    cell_field(ui, "X", &mut edited.0),
                    cell_field(ui, "Y", &mut edited.1),
                ]
            })
            .inner;
        if fields.iter().any(|field| field.drag_started()) {
            // Dragging a number changes it every frame; the whole drag is one undo step.
            state.history.begin_gesture(&editor.draft);
        }
        apply_cell(editor, &mut state.history, current, edited);
        if fields.iter().any(|field| field.drag_stopped()) {
            state.history.end_gesture(&editor.draft);
        }
    }

    match shown {
        Some(cell) if map.in_conflict(editor.id) => panel_label(
            ui,
            &format!(
                "Shown at ({}, {}): another room has this cell",
                cell.0, cell.1
            ),
        ),
        Some(cell) => panel_label(ui, &format!("Shown at ({}, {})", cell.0, cell.1)),
        None => {}
    }
}

fn cell_field(ui: &mut egui::Ui, label: &str, value: &mut i32) -> egui::Response {
    panel_label(ui, label);
    let response = ui.add(
        egui::DragValue::new(value)
            .range(-CELL_LIMIT..=CELL_LIMIT)
            .speed(0.1),
    );
    probe(ui, || (egui::Id::new(("map cell", label)), response.rect));
    response
}

/// Applies a change made this frame to the number fields. Outside a drag each change is its own
/// undo step; during one, the drag as a whole is recorded when it ends.
fn apply_cell(editor: &mut EditorSession, history: &mut History, current: Cell, edited: Cell) {
    if edited == current {
        return;
    }
    if history.gesture_open() {
        let result = editor.draft.set_map_cell(Some(edited));
        finish(editor, result);
    } else {
        set_cell(editor, history, Some(edited));
    }
}

/// Sets the room's cell as one undo step; a refused cell says why and changes nothing.
fn set_cell(editor: &mut EditorSession, history: &mut History, cell: Option<Cell>) {
    let result = tracked(history, editor, |e| e.draft.set_map_cell(cell));
    finish(editor, result);
}

fn finish(editor: &mut EditorSession, result: Result<(), String>) {
    match result {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
        }
        Err(error) => editor.message = error,
    }
}
