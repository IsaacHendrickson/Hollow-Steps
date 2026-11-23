//! The "Min tier" field of the Room tools panel: the lowest difficulty tier the selected enemy
//! or the boss is in the room at (`editor_min_tier`). The editor always shows and saves every
//! placement; the game leaves one out below its tier, and the Test room does so at the
//! profile's tier. Dragging the field is one undo step, like the room size fields. An enemy's
//! Group and Chance % rows sit under it (`variety_fields`).
use super::widgets::{panel_label, probe, section};
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;
use hollow_steps::editor_history::tracked;
use hollow_steps::room::MAX_MIN_TIER;

const HINT: &str = "The lowest difficulty tier this is in the room at; 0 is every tier. \
                    The game leaves it out below that tier. The editor and Save keep it, and \
                    the Test room plays at the profile's tier.";

/// The min tier of what is selected; `None` when it is not an enemy or the boss.
fn current(editor: &EditorSession, selected: Option<Selection>) -> Option<u32> {
    match selected? {
        Selection::Enemy(index) => editor.draft.enemy_min_tier(index),
        Selection::Boss => editor.draft.boss_min_tier(),
        _ => None,
    }
}

/// Sets the min tier of what is selected.
fn set(editor: &mut EditorSession, selected: Option<Selection>, tier: u32) -> Result<(), String> {
    match selected {
        Some(Selection::Enemy(index)) => editor.draft.set_enemy_min_tier(index, tier),
        Some(Selection::Boss) => editor.draft.set_boss_min_tier(tier),
        _ => Err("select an enemy or the boss first".into()),
    }
}

pub(super) fn draw_min_tier(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    let Some(current) = current(editor, state.selected) else {
        return;
    };
    section(ui, "Placement", |ui| {
        let mut edited = current;
        let response = egui::Grid::new("placement fields")
            .num_columns(2)
            .show(ui, |ui| {
                panel_label(ui, "Min tier");
                let response = ui
                    .add(
                        egui::DragValue::new(&mut edited)
                            .range(0..=MAX_MIN_TIER)
                            .speed(0.1)
                            .fixed_decimals(0),
                    )
                    .on_hover_text(HINT);
                probe(ui, || {
                    (
                        egui::Id::new(("placement field", "Min tier")),
                        response.rect,
                    )
                });
                ui.end_row();
                // Under Min tier, for an enemy: Group and Chance %.
                super::variety_fields::rows(ui, editor, state);
                response
            })
            .inner;
        if response.drag_started() {
            state.history.begin_gesture(&editor.draft);
        }
        if edited != current {
            let selected = state.selected;
            let change = |e: &mut EditorSession| set(e, selected, edited);
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
    });
}

#[cfg(test)]
mod tests;
