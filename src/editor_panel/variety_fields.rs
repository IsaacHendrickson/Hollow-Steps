//! The "Group" and "Chance %" rows of the Placement section for a selected enemy, under Min tier
//! (`editor_variety`, `crate::variety`). They say how likely the enemy is to be in the room in a
//! run: one member of each group stays, and a chance under 100 leaves the enemy out some of the
//! time. The editor always shows and saves every placement, and the Test room plays them all;
//! only a run with a seed leaves some out.
//!
//! Dragging a box is one undo step (a gesture, like the room size fields); ticking or unticking
//! "None" is one step too. Every change marks the room unsaved. A change the rules refuse (a
//! group for an enemy whose chance is not 100, a chance for an enemy in a group) leaves the draft
//! as it was and says why in the editor's message.
use super::widgets::{panel_label, probe};
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;
use hollow_steps::editor_history::tracked;
use hollow_steps::variety::{FULL_CHANCE, MAX_GROUP};

const GROUP_HINT: &str = "Enemies of one room with the same group number are alternatives: \
                          in a run exactly one of them is in the room. None means no group. \
                          The editor and Save keep every one, and the Test room plays them all.";
const CHANCE_HINT: &str = "The chance, in percent, that this enemy is in the room in a run; \
                           100 is always, 0 is never. An enemy in a group always has 100. The \
                           editor and Save keep it, and the Test room plays it.";

/// Applies one edit of the selected placement as one undo step. A drag keeps a gesture open from
/// its first move to its release and records the whole drag once; anything else is recorded
/// at once. A refused edit changes nothing and its reason goes to the editor's message.
fn commit(
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
        let result = if state.history.gesture_open() {
            change(editor)
        } else {
            tracked(&mut state.history, editor, change)
        };
        match result {
            Ok(()) => {
                editor.dirty = true;
                editor.message.clear();
            }
            Err(error) => editor.message = error,
        }
    }
    if response.drag_stopped() {
        state.history.end_gesture(&editor.draft);
    }
}

/// The Group and Chance % rows, drawn inside the Placement grid right after Min tier. Nothing
/// is drawn unless an enemy placement is selected.
pub(super) fn rows(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    let Some(Selection::Enemy(index)) = state.selected else {
        return;
    };
    let (Some(group), Some(chance)) = (
        editor.draft.enemy_group(index),
        editor.draft.enemy_chance(index),
    ) else {
        return;
    };

    panel_label(ui, "Group");
    ui.horizontal(|ui| {
        let mut none = group.is_none();
        let tick = ui.checkbox(&mut none, "None").on_hover_text(GROUP_HINT);
        probe(ui, || {
            (egui::Id::new(("placement field", "Group none")), tick.rect)
        });
        let mut number = group.unwrap_or(1);
        let drag = ui
            .add_enabled(
                group.is_some(),
                egui::DragValue::new(&mut number)
                    .range(1..=MAX_GROUP)
                    .speed(0.1)
                    .fixed_decimals(0),
            )
            .on_hover_text(GROUP_HINT);
        probe(ui, || {
            (egui::Id::new(("placement field", "Group")), drag.rect)
        });
        if tick.changed() {
            // Unticking None puts the enemy in group 1; ticking it takes it out of its group.
            let to = (!none).then_some(1);
            commit(editor, state, &tick, true, |e| {
                e.draft.set_enemy_group(index, to)
            });
        } else {
            let changed = group.is_some_and(|group| group != number);
            commit(editor, state, &drag, changed, |e| {
                e.draft.set_enemy_group(index, Some(number))
            });
        }
    });
    ui.end_row();

    panel_label(ui, "Chance %");
    let mut edited = chance;
    let drag = ui
        .add(
            egui::DragValue::new(&mut edited)
                .range(0..=FULL_CHANCE)
                .speed(0.5)
                .fixed_decimals(0),
        )
        .on_hover_text(CHANCE_HINT);
    probe(ui, || {
        (egui::Id::new(("placement field", "Chance %")), drag.rect)
    });
    commit(editor, state, &drag, edited != chance, |e| {
        e.draft.set_enemy_chance(index, edited)
    });
    ui.end_row();
}

#[cfg(test)]
mod tests;
