//! The editor's Charms panel.
//!
//! Presentation only. The values, ranges, validation and file writing live in
//! `hollow_steps::charm_editor`; a number is drawn by the Player panel's `draw_field`, so a typed
//! value is judged the same way here as there. Every field of a charm is a `charm_editor::FIELDS`
//! entry, and the panel draws them in that order, each as its `Control` says.
use egui_macroquad::egui;
use hollow_steps::charm_editor::{
    amount, CharmDraft, CharmField, Control, EffectKind, FIELDS, MANA_COST,
};
use hollow_steps::charm_history::CharmHistory;
use hollow_steps::editor::EditorSession;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::tuning::FieldSpec;

use crate::editor_panel::widgets::{panel_button, panel_label, probe};
use crate::player_panel::{draw_field, FieldEdit, FieldRow, Status};

/// Label column of the panel's rows; the editor's side panel is narrow.
const LABEL_WIDTH: f32 = 110.0;

/// What the Charms panel remembers between frames.
#[derive(Default)]
pub(crate) struct CharmUi {
    /// The Charms panel is shown instead of the room tools.
    pub(crate) open: bool,
    /// Save was clicked this frame; `draw_editor` saves through the game and records the result.
    pub(crate) save_requested: bool,
    pub(crate) status: Status,
    /// Why a typed number was refused: the charm's id, the field and the reason, drawn under
    /// the field until the next edit.
    refused: Option<(String, &'static str, String)>,
    /// Undo and redo for what this panel edits, apart from the room's.
    history: CharmHistory,
    /// The last mouse press was inside this panel: Ctrl+Z and Ctrl+Y are for its history.
    focused: bool,
    /// An Undo or Redo button was clicked this frame; applied after the frame is observed.
    history_request: Option<HistoryKey>,
}

impl CharmUi {
    /// Ctrl+Z / Ctrl+Y: goes through the Charms panel's own history when the panel is open and
    /// has focus, and returns true (the key is taken even if there is nothing to undo). False
    /// means the keys are someone else's.
    pub(crate) fn apply_history(&mut self, editor: &mut EditorSession, key: HistoryKey) -> bool {
        if !(self.open && self.focused) {
            return false;
        }
        self.step_history(editor, key);
        true
    }

    fn step_history(&mut self, editor: &mut EditorSession, key: HistoryKey) {
        match key {
            HistoryKey::Undo => self.history.undo(&mut editor.charms),
            HistoryKey::Redo => self.history.redo(&mut editor.charms),
        };
        self.refused = None;
    }
}

/// Something a charm's own buttons asked for this frame.
enum Request {
    Reset(usize),
    Delete(usize),
}

/// Draws the Charms button and, while it is open, the panel. Returns true when the panel is
/// shown, which means the caller draws nothing else in the editor's side panel. `player_open`
/// is the Player panel's flag: only one of the two panels is open at a time.
pub(crate) fn draw(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut CharmUi,
    player_open: &mut bool,
) -> bool {
    if *player_open {
        state.open = false;
    }
    if panel_button(ui, if state.open { "Close charms" } else { "Charms" }) {
        state.open = !state.open;
        if state.open {
            *player_open = false;
        }
    }
    if !state.open {
        state.focused = false;
        return false;
    }
    let before = editor.charms.edited().to_vec();
    draw_list(ui, editor, state);
    // A press inside the panel gives it the undo keys; a press anywhere else takes them away.
    let (down, pressed_at) = ui.input(|input| {
        (
            input.pointer.any_down(),
            input
                .pointer
                .any_pressed()
                .then(|| input.pointer.interact_pos())
                .flatten(),
        )
    });
    if let Some(at) = pressed_at {
        state.focused = ui.clip_rect().contains(at);
    }
    // A stretch of typing in a text box is one step, like a drag.
    let typing = ui.ctx().wants_keyboard_input();
    state
        .history
        .observe(before, editor.charms.edited(), down || typing);
    if let Some(key) = state.history_request.take() {
        state.step_history(editor, key);
    }
    true
}

fn draw_list(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut CharmUi) {
    ui.heading("Charms");
    panel_label(
        ui,
        if editor.charms.is_dirty() {
            "Unsaved changes."
        } else {
            "Matches the saved charms."
        },
    );
    let base = editor.player.saved_vitals().max_mana;
    if panel_button(ui, "New charm") {
        editor.charms.new_charm();
        state.refused = None;
    }
    let mut request = None;
    for index in 0..editor.charms.edited().len() {
        draw_charm(ui, editor, state, index, base, &mut request);
    }
    match request {
        Some(Request::Reset(index)) => editor.charms.reset_charm(index),
        Some(Request::Delete(index)) => {
            let _ = editor.charms.delete(index);
        }
        None => {}
    }
    if request.is_some() {
        state.refused = None;
    }
    ui.separator();
    if panel_button(ui, "Save charms") {
        state.refused = None;
        state.save_requested = true;
    }
    if panel_button(ui, "Undo charm edit") {
        state.history_request = Some(HistoryKey::Undo);
    }
    if panel_button(ui, "Redo charm edit") {
        state.history_request = Some(HistoryKey::Redo);
    }
    if panel_button(ui, "Reset all") {
        editor.charms.reset_all();
        state.refused = None;
        state.status = Status::Quiet;
    }
    match &state.status {
        Status::Quiet => {}
        Status::Saved(text) => panel_label(ui, &format!("Saved to {text}")),
        Status::Error(message) => {
            ui.add(
                egui::Label::new(
                    egui::RichText::new(message.as_str()).color(egui::Color32::LIGHT_RED),
                )
                .wrap(),
            );
        }
    }
    ui.small(
        "Changes apply in play at once. Save writes charms.ron in the assets folder. A charm \
         that is deleted is taken off and no longer owned when the list is saved.",
    );
}

fn draw_charm(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut CharmUi,
    index: usize,
    base: f32,
    request: &mut Option<Request>,
) {
    let charm = editor.charms.edited()[index].clone();
    let title = if charm.name.trim().is_empty() {
        charm.id.clone()
    } else {
        charm.name.clone()
    };
    let dirty = editor.charms.is_charm_dirty(index);
    // The id, not the name, keys the section: renaming a charm must not rebuild its widgets.
    let section = egui::CollapsingHeader::new(format!("{title}{}", if dirty { " *" } else { "" }))
        .id_salt(("charm", &charm.id))
        .default_open(true);
    section.show(ui, |ui| {
        for field in FIELDS {
            let problem = editor.charms.problem_for(index, field.key, base);
            draw_field_row(ui, editor, state, index, field, base, problem);
        }
        ui.horizontal(|ui| {
            let reset = ui
                .add_enabled(dirty, egui::Button::new("Reset charm"))
                .on_hover_text("Back to the saved charm (a new charm is removed)")
                .on_disabled_hover_text("Same as the saved charm");
            probe(ui, || {
                (egui::Id::new(("charm reset", &charm.id)), reset.rect)
            });
            if reset.clicked() {
                *request = Some(Request::Reset(index));
            }
            let delete = ui
                .button("Delete charm")
                .on_hover_text("Removed from the list; taken off and no longer owned once saved");
            probe(ui, || {
                (egui::Id::new(("charm delete", &charm.id)), delete.rect)
            });
            if delete.clicked() {
                *request = Some(Request::Delete(index));
            }
        });
    });
}

/// One field of the charm, drawn as its `Control` says, with `problem` (if any) under it.
fn draw_field_row(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut CharmUi,
    index: usize,
    field: CharmField,
    base: f32,
    problem: Option<String>,
) {
    let charm = editor.charms.edited()[index].clone();
    let refused = state
        .refused
        .as_ref()
        .filter(|(id, key, _)| *id == charm.id && *key == field.key)
        .map(|(_, _, message)| message.clone());
    let shown = refused.or(problem);
    match field.control {
        Control::ReadOnly => {
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                ui.label(&charm.id)
                    .on_hover_text("A stable name; it is never shown in the game");
            });
        }
        Control::Text(max_chars) => {
            let mut text = if field.key == "name" {
                charm.name.clone()
            } else {
                charm.description.clone()
            };
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut text)
                        .id_salt(("charm text", &charm.id, field.key))
                        .char_limit(max_chars)
                        .desired_width(f32::INFINITY),
                );
                probe(ui, || {
                    (
                        egui::Id::new(("charm text", &charm.id, field.key)),
                        response.rect,
                    )
                });
                if response.changed() {
                    let _ = if field.key == "name" {
                        editor.charms.set_name(index, &text)
                    } else {
                        editor.charms.set_description(index, &text)
                    };
                }
            });
        }
        Control::Number => {
            // The drag box reaches the base maximum mana and no further.
            let spec = FieldSpec {
                max: base.clamp(MANA_COST.min, MANA_COST.max),
                ..MANA_COST
            };
            let edit = number_row(ui, &spec, charm.mana_cost, shown.as_deref());
            apply_number(editor, state, &charm.id, field.key, edit, |draft, value| {
                draft.set_mana_cost(index, value)
            });
            return;
        }
        Control::Effect => {
            let kind = EffectKind::of(&charm.effect);
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                let combo = egui::ComboBox::from_id_salt(("charm effect", &charm.id))
                    .selected_text(kind.label())
                    .show_ui(ui, |ui| {
                        for option in EffectKind::ALL {
                            if ui
                                .selectable_label(option == kind, option.label())
                                .clicked()
                            {
                                let _ = editor.charms.set_effect_kind(index, option);
                            }
                        }
                    });
                probe(ui, || {
                    (
                        egui::Id::new(("charm effect", &charm.id)),
                        combo.response.rect,
                    )
                });
            });
            let edit = number_row(ui, kind.spec(), amount(&charm.effect), shown.as_deref());
            apply_number(editor, state, &charm.id, field.key, edit, |draft, value| {
                draft.set_effect_amount(index, value)
            });
            return;
        }
        Control::Color => {
            let mut color = charm.color;
            ui.horizontal(|ui| {
                ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(field.label));
                let swatch = ui.color_edit_button_srgba_unmultiplied(&mut color);
                probe(ui, || {
                    (egui::Id::new(("charm color", &charm.id)), swatch.rect)
                });
                // Only a real pick is an edit: the picker rounds the color it shows.
                if swatch.changed() {
                    let _ = editor.charms.set_color(index, color);
                }
            });
        }
        Control::Toggle => {
            let mut owned = charm.owned_at_start;
            let response = ui.checkbox(&mut owned, field.label).on_hover_text(
                "Owned from the start of a run, and in a test room. Changing it does not give \
                 or take a charm in a game that is already running.",
            );
            probe(ui, || {
                (egui::Id::new(("charm owned", &charm.id)), response.rect)
            });
            if response.changed() {
                let _ = editor.charms.set_owned_at_start(index, owned);
            }
        }
    }
    if let Some(message) = shown {
        ui.add(
            egui::Label::new(egui::RichText::new(message).color(egui::Color32::LIGHT_RED)).wrap(),
        );
    }
}

/// A drag box with the field's unit and range (the Player panel's row).
fn number_row(ui: &mut egui::Ui, spec: &FieldSpec, value: f32, problem: Option<&str>) -> FieldEdit {
    draw_field(
        ui,
        FieldRow {
            spec,
            value,
            label_width: LABEL_WIDTH,
            dirty: None,
            problem,
        },
    )
}

/// Puts a dragged or typed number into the draft, or remembers why it was refused.
fn apply_number(
    editor: &mut EditorSession,
    state: &mut CharmUi,
    id: &str,
    key: &'static str,
    edit: FieldEdit,
    set: impl FnOnce(&mut CharmDraft, f32) -> Result<(), String>,
) {
    if let Some(value) = edit.value {
        state.refused = set(&mut editor.charms, value)
            .err()
            .map(|message| (id.to_owned(), key, message));
    }
    if let Some(message) = edit.rejected {
        state.refused = Some((id.to_owned(), key, message));
    }
}

#[cfg(test)]
mod tests;
