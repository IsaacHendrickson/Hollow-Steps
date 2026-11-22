//! The puzzle controls of the Room tools panel: add a breakable wall, a switch or a barrier
//! (`editor_puzzle`), and edit the selected piece: its width and height, a wall's hits, a
//! switch's number, and the switch that opens a barrier (a dropdown of the room's switches). A
//! piece is moved by dragging it on the canvas, like a hazard.
use std::ops::RangeInclusive;

use super::widgets::{choice, panel_button, panel_label, probe, section};
use super::{EditorSession, EditorUi};
use egui_macroquad::egui;
use hollow_steps::editor::{RoomDraft, Selection};
use hollow_steps::editor_history::{tracked, History};
use hollow_steps::editor_puzzle::PieceFields;
use hollow_steps::editor_tools::EditorTool;
use hollow_steps::puzzle::{FIRST_SWITCH, LAST_SWITCH, MAX_BREAKABLE_HITS, MIN_PIECE_SIZE};

/// The widest or tallest a piece can be set to in the panel; the room still has to fit it.
const MAX_SIDE: f32 = 2000.0;
const SIZE: RangeInclusive<f32> = MIN_PIECE_SIZE..=MAX_SIDE;
const HITS: RangeInclusive<u8> = 1..=MAX_BREAKABLE_HITS;
const SWITCHES: RangeInclusive<u8> = FIRST_SWITCH..=LAST_SWITCH;

/// The buttons and, for a selected piece, its fields.
pub(super) fn draw_puzzle_controls(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    draw_add_buttons(ui, editor, state);
    if let Some(
        selection @ (Selection::Breakable(_) | Selection::Switch(_) | Selection::Barrier(_)),
    ) = state.selected
    {
        draw_piece_fields(ui, editor, state, selection);
    }
}

/// The legend lines for the three markers, once the room has a piece: the panel is exactly as
/// tall as the editor's layout tests allow, so a room without pieces stays as it was.
pub(super) fn draw_legend(ui: &mut egui::Ui, editor: &EditorSession) {
    let layout = &editor.draft.layout;
    if layout.breakables.is_empty() && layout.switches.is_empty() && layout.barriers.is_empty() {
        return;
    }
    panel_label(ui, "Brown: breakable wall (hits to break)");
    panel_label(ui, "Teal: switch (S and its number)");
    panel_label(ui, "Slate: barrier (S of the switch that opens it)");
}

type Add = fn(&mut RoomDraft) -> Result<Selection, String>;

/// "Add breakable wall", "Add switch" and "Add barrier": a standard piece standing on the floor
/// next to the room's spawn point (drag it where it belongs). Each is one undo step.
fn draw_add_buttons(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    let buttons: [(&str, Add); 3] = [
        ("Add breakable wall", RoomDraft::add_breakable),
        ("Add switch", RoomDraft::add_switch),
        ("Add barrier", RoomDraft::add_barrier),
    ];
    for (label, add) in buttons {
        if !panel_button(ui, label) {
            continue;
        }
        match tracked(&mut state.history, editor, |e| add(&mut e.draft)) {
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
}

fn title(selection: Selection) -> &'static str {
    match selection {
        Selection::Breakable(_) => "Breakable wall",
        Selection::Switch(_) => "Switch",
        _ => "Barrier",
    }
}

fn hint(selection: Selection) -> &'static str {
    match selection {
        Selection::Breakable(_) => "Breaks after this many nail hits, one hit per swing.",
        Selection::Switch(_) => "The nail turns it on for the rest of the run.",
        _ => "Goes away when its switch is on. The switch must be in this room.",
    }
}

/// Width, height and the number of the selected piece. Dragging a field edits every frame and the
/// whole drag is one undo step, like the hazard fields; choosing a switch for a barrier is one step.
fn draw_piece_fields(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    selection: Selection,
) {
    let Some(current) = editor.draft.piece_fields(selection) else {
        return;
    };
    let switches: Vec<u8> = editor.draft.layout.switches.iter().map(|s| s.id).collect();
    section(ui, title(selection), |ui| {
        let mut edited = current;
        let rows = egui::Grid::new("puzzle fields")
            .num_columns(2)
            .show(ui, |ui| {
                let mut rows = vec![
                    size_row(ui, "Piece width", &mut edited.width),
                    size_row(ui, "Piece height", &mut edited.height),
                ];
                match selection {
                    Selection::Breakable(_) => {
                        rows.push(number_row(ui, "Hits", &mut edited.number, HITS));
                    }
                    Selection::Switch(_) => {
                        rows.push(number_row(
                            ui,
                            "Switch number",
                            &mut edited.number,
                            SWITCHES,
                        ));
                    }
                    _ => edited.number = switch_row(ui, edited.number, &switches),
                }
                rows
            });
        if rows.inner.iter().any(|row| row.drag_started()) {
            state.history.begin_gesture(&editor.draft);
        }
        apply_fields(editor, &mut state.history, selection, current, edited);
        if rows.inner.iter().any(|row| row.drag_stopped()) {
            state.history.end_gesture(&editor.draft);
        }
        panel_label(ui, hint(selection));
    });
}

fn size_row(ui: &mut egui::Ui, label: &str, value: &mut f32) -> egui::Response {
    panel_label(ui, label);
    let response = ui.add(
        egui::DragValue::new(value)
            .range(SIZE)
            .speed(1.0)
            .fixed_decimals(0)
            .suffix(" px"),
    );
    probe(ui, || {
        (egui::Id::new(("puzzle field", label)), response.rect)
    });
    ui.end_row();
    response
}

fn number_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut u8,
    range: RangeInclusive<u8>,
) -> egui::Response {
    panel_label(ui, label);
    let response = ui.add(egui::DragValue::new(value).range(range).speed(0.1));
    probe(ui, || {
        (egui::Id::new(("puzzle field", label)), response.rect)
    });
    ui.end_row();
    response
}

/// The dropdown that says which of the room's `switches` opens the barrier; returns the one
/// now chosen. A barrier can only point at a switch of its own room.
fn switch_row(ui: &mut egui::Ui, current: u8, switches: &[u8]) -> u8 {
    const LABEL: &str = "Opened by switch";
    panel_label(ui, LABEL);
    let mut picked = current;
    let combo = egui::ComboBox::from_id_salt(egui::Id::new(("puzzle pick", LABEL)))
        .width(ui.available_width())
        .truncate()
        .selected_text(format!("Switch {current}"))
        .show_ui(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for &id in switches {
                if choice(ui, &format!("Switch {id}"), id == current).clicked() {
                    picked = id;
                    ui.close_menu();
                }
            }
        });
    probe(ui, || {
        (egui::Id::new(("puzzle pick", LABEL)), combo.response.rect)
    });
    ui.end_row();
    picked
}

/// Applies an edit made this frame. A refused value leaves the draft as it was, so the field
/// snaps back next frame, and says why in the editor message. Outside a drag each accepted
/// edit is its own undo step; during one, the drag as a whole is recorded when it ends.
fn apply_fields(
    editor: &mut EditorSession,
    history: &mut History,
    selection: Selection,
    current: PieceFields,
    edited: PieceFields,
) {
    if edited == current {
        return;
    }
    let change = |e: &mut EditorSession| e.draft.set_piece_fields(selection, edited);
    let result = if history.gesture_open() {
        change(editor)
    } else {
        tracked(history, editor, change)
    };
    match result {
        Ok(()) => {
            editor.dirty = true;
            editor.message.clear();
        }
        Err(error) => editor.message = error,
    }
}

#[cfg(test)]
mod tests;
