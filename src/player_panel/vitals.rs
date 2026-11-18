//! The Mana, Health trail and Heal sections of the Player panel: the numbers of `vitals.ron`
//! (mana per kill, the mana cap, healing, the trailing health bar), drawn with the same field row
//! as the stats and edited in the same draft (`PlayerDraft::set_vitals_field`).
//!
//! Presentation only. The ranges are `hollow_steps::vitals::VitalsField`; one rule spans two
//! fields (the starting mana may not be above the mana cap), and its message is drawn under the
//! field that was being edited.
use super::{draw_field, FieldRow, LABEL_WIDTH};
use crate::editor_panel::widgets::section;
use egui_macroquad::egui;
use hollow_steps::editor::EditorSession;
use hollow_steps::vitals::VitalsField;

/// Why an edit to a vitals field was refused, drawn under that field until the next edit.
pub(super) type Refused = Option<(VitalsField, String)>;

/// Draws one collapsing section per group of `VitalsField::ALL`, in table order.
pub(super) fn draw(ui: &mut egui::Ui, editor: &mut EditorSession, refused: &mut Refused) {
    let all = VitalsField::ALL;
    let mut start = 0;
    while start < all.len() {
        let group = all[start].spec().group;
        let end = all[start..]
            .iter()
            .position(|field| field.spec().group != group)
            .map_or(all.len(), |count| start + count);
        section(ui, group, |ui| {
            for &field in &all[start..end] {
                field_row(ui, editor, refused, field);
            }
        });
        start = end;
    }
}

fn field_row(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    refused: &mut Refused,
    field: VitalsField,
) {
    let shown = refused
        .as_ref()
        .filter(|(refused, _)| *refused == field)
        .map(|(_, message)| message.as_str());
    let edit = draw_field(
        ui,
        FieldRow {
            spec: field.spec(),
            value: field.get(editor.player.edited_vitals()),
            label_width: LABEL_WIDTH,
            dirty: Some(editor.player.is_vitals_field_dirty(field)),
            problem: shown,
        },
    );
    if let Some(value) = edit.value {
        *refused = editor
            .player
            .set_vitals_field(field, value)
            .err()
            .map(|message| (field, message));
    }
    if let Some(message) = edit.rejected {
        *refused = Some((field, message));
    }
    if edit.reset {
        *refused = editor
            .player
            .reset_vitals_field(field)
            .err()
            .map(|message| (field, message));
    }
}
