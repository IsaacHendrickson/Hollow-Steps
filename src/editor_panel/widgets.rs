//! Widgets shared by every panel of the editor, and `probe`, the one place production code
//! records where a widget was drawn so the headless tests can find it again.
use crate::SearchList;
use egui_macroquad::egui;

/// Records, in a test build, where a widget was drawn so the headless tests can find it again.
/// `record` gives the egui id to store under and the value; in a normal build it is never
/// called. Every test-only layout hook in the panel goes through here.
pub(super) fn probe<T: Clone + Send + Sync + 'static>(
    ui: &egui::Ui,
    record: impl FnOnce() -> (egui::Id, T),
) {
    #[cfg(test)]
    {
        let (id, value) = record();
        ui.data_mut(|data| data.insert_temp(id, value));
    }
    #[cfg(not(test))]
    let _ = (ui, record);
}

pub(super) fn section<R>(ui: &mut egui::Ui, name: &str, contents: impl FnOnce(&mut egui::Ui) -> R) {
    let response = egui::CollapsingHeader::new(name)
        .default_open(true)
        .show(ui, contents);
    probe(ui, || {
        (
            egui::Id::new(("section", name)),
            response.header_response.id,
        )
    });
}

// All searchable pickers go through this widget. SearchList remains the only filter/selection logic.
pub(super) fn searchable_list(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    names: &[impl AsRef<str>],
    list: &mut SearchList,
) {
    panel_label(ui, label);
    panel_input(ui, id.with("search"), "Filter", &mut list.query);
    let matches = list.matches(names);
    panel_label(ui, &format!("{} of {}", matches.len(), names.len()));
    let caption = list
        .selected
        .and_then(|i| names.get(i))
        .map(|n| n.as_ref())
        .unwrap_or("Select…");
    let width = ui.available_width();
    // The popup uses egui's above/below placement near the window edge. The filter stays visible
    // in the panel; this boundary is the place to add keyboard search/navigation later.
    let dropdown = egui::ComboBox::from_id_salt(id)
        .width(width)
        .truncate()
        .height(300.0)
        .selected_text(caption)
        .show_ui(ui, |ui| {
            ui.set_width((width - 16.0).max(1.0));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
            let height = 8.0 * (24.0 + ui.spacing().item_spacing.y);
            let results = egui::ScrollArea::vertical()
                .id_salt(id.with("results"))
                .min_scrolled_height(height)
                .max_height(height)
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    if matches.is_empty() {
                        choice(ui, "No matches", false);
                    }
                    for &index in &matches {
                        if choice(ui, names[index].as_ref(), list.selected == Some(index)).clicked()
                        {
                            list.selected = Some(index);
                            ui.close_menu();
                        }
                    }
                });
            probe(ui, || {
                (
                    id.with("result metrics"),
                    (
                        results.inner_rect,
                        results.content_size,
                        results.state.offset,
                        results.id,
                    ),
                )
            });
        })
        .response
        .on_hover_text(caption);
    probe(ui, || (id.with("dropdown"), (dropdown.rect, dropdown.id)));
}

pub(super) fn panel_label(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(text).wrap());
}
pub(super) fn panel_input(ui: &mut egui::Ui, id: egui::Id, label: &str, value: &mut String) {
    panel_label(ui, label);
    ui.add(
        egui::TextEdit::singleline(value)
            .id(id)
            .desired_width(ui.available_width()),
    )
    .on_hover_text(value.as_str());
}
pub(super) fn panel_combo(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    names: &[impl AsRef<str>],
    index: &mut usize,
) {
    panel_label(ui, label);
    if names.is_empty() {
        return;
    }
    *index = (*index).min(names.len() - 1);
    // Small fixed catalogs share the same dropdown boundary as searchable pickers.
    egui::ComboBox::from_id_salt(id)
        .width(ui.available_width())
        .truncate()
        .selected_text(names[*index].as_ref())
        .show_ui(ui, |ui| {
            ui.set_max_width(ui.available_width());
            for (i, name) in names.iter().enumerate() {
                if choice(ui, name.as_ref(), *index == i).clicked() {
                    *index = i;
                    ui.close_menu();
                }
            }
        })
        .response
        .on_hover_text(names[*index].as_ref());
}
pub(super) fn panel_button(ui: &mut egui::Ui, text: impl AsRef<str>) -> bool {
    let response = ui.add_sized(
        [ui.available_width(), 24.0],
        egui::Button::new(text.as_ref()).wrap(),
    );
    probe(ui, || {
        (egui::Id::new(("button", text.as_ref())), response.rect)
    });
    response.clicked()
}
pub(super) fn row_controls(ui: &mut egui::Ui) -> Option<u8> {
    let mut command = None;
    ui.horizontal_wrapped(|ui| {
        for (op, label) in ["Up", "Down", "Delete"].iter().enumerate() {
            if ui.button(*label).clicked() {
                command = Some(op as u8);
            }
        }
    });
    command
}
pub(super) fn choice(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let response = ui
        .add_sized(
            [ui.available_width(), 24.0],
            egui::Button::new(text).selected(selected).truncate(),
        )
        .on_hover_text(text);
    probe(ui, || (egui::Id::new(("choice", text)), response.rect));
    response
}

pub(super) fn pose_dropdown(
    ui: &mut egui::Ui,
    id: egui::Id,
    label: &str,
    value: &mut String,
    names: &[String],
) {
    panel_label(ui, label);
    let response = egui::ComboBox::from_id_salt(id)
        .selected_text(if value.is_empty() {
            "None"
        } else {
            value.as_str()
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, String::new(), "None");
            for name in names {
                let response = ui.selectable_value(value, name.clone(), name);
                probe(ui, || (id.with(name), response.rect));
            }
        });
    probe(ui, || (id, response.response.rect));
}
