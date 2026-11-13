//! egui presentation only; authored content and filter rules stay in the library.
//! One module per panel: `room_tools` and `entity` are the two views `draw_panel` switches
//! between, `widgets` holds what they share.
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;
use hollow_steps::editor_tools::EditorTool;

mod behavior;
pub(crate) mod body_visual;
mod boss;
mod canvas_tools;
mod death_field;
mod entity;
mod history_panel;
mod key_values;
mod look_panel;
mod movement_field;
#[cfg(test)]
mod panel_tests;
pub(crate) mod property;
mod room_tools;
mod states;
#[cfg(test)]
mod test_support;
mod tool_panel;
mod unsaved_dialog;
mod visual_size_field;
pub(crate) mod widgets;
pub(super) use boss::{draw_boss_panel, BossPropertyUi};
use entity::{draw_definition_panel, pending_error};
pub(super) use history_panel::apply_history;
pub(super) use look_panel::LookUi;
pub(super) use property::PropertyUi;
use room_tools::draw_room_tools;
pub(super) use tool_panel::choose_tool;
pub(super) use unsaved_dialog::draw_unsaved_dialog;
use widgets::{panel_button, panel_label, probe};

pub(super) struct PanelResponse {
    pub save: bool,
    /// The Test room button was clicked.
    pub test: bool,
    pub open: Option<hollow_steps::room::RoomId>,
    pub left: f32,
    pub blocks_pointer: bool,
    pub keyboard: bool,
}

pub(super) fn draw_panel(
    ctx: &egui::Context,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> PanelResponse {
    let entity_paths = editor.content.entities.keys().cloned().collect::<Vec<_>>();
    let definitions = editor
        .content
        .placement_catalog()
        .into_iter()
        .map(|(name, _)| name)
        .collect::<Vec<_>>();
    let room_labels = editor
        .rooms
        .iter()
        .map(|(id, _)| format!("{id:?}"))
        .collect::<Vec<_>>();
    let names = &definitions;
    let mut save = false;
    let mut test = false;
    let mut open = None;
    let width = ctx.screen_rect().width().min(360.0);
    let response = egui::SidePanel::right("editor panel")
        .resizable(false)
        .exact_width(width)
        .show(ctx, |ui| {
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.spacing_mut().interact_size.y = 24.0;
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            // Bottom-up layout reserves the footer before laying out the scroll viewport.
            ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                save = panel_button(ui, "Save");
                test = panel_button(ui, "Test room (F6)");
                ui.separator();
                let scroll = egui::ScrollArea::vertical()
                    .id_salt("editor controls")
                    .auto_shrink([false, false])
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                    .show(ui, |ui| {
                        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
                            ui.heading("Room editor (F2)");
                            for warning in editor.draft.warnings() {
                                panel_label(ui, warning);
                            }
                            if !editor.message.is_empty() {
                                panel_label(ui, &editor.message);
                            }
                            panel_label(
                                ui,
                                &format!("{:?}{}", editor.id, if editor.dirty { " *" } else { "" }),
                            );
                            if panel_button(
                                ui,
                                if state.properties {
                                    "Room tools"
                                } else {
                                    "Entity properties"
                                },
                            ) {
                                state.player.open = false;
                                state.properties = !state.properties;
                                if state.properties {
                                    state.tool = EditorTool::Select;
                                    state.destination = None;
                                    state.dragging = None;
                                    let boss_path = match state.selected {
                                        Some(Selection::Boss) => {
                                            editor.draft.boss.as_ref().and_then(|b| {
                                                editor.content.boss_path(&b.definition)
                                            })
                                        }
                                        Some(Selection::Enemy(_)) => None,
                                        _ => state
                                            .definition
                                            .selected
                                            .and_then(|i| {
                                                editor
                                                    .content
                                                    .bosses
                                                    .keys()
                                                    .nth(i.checked_sub(entity_paths.len())?)
                                            })
                                            .cloned(),
                                    };
                                    match boss_path {
                                        Some(path)
                                            if !state
                                                .boss_property
                                                .as_ref()
                                                .is_some_and(|p| p.path == path) =>
                                        {
                                            state.boss_property =
                                                Some(BossPropertyUi::new(editor, path));
                                        }
                                        None => state.boss_property = None,
                                        _ => {}
                                    }
                                    let path = match state.selected {
                                        Some(Selection::Enemy(index)) => {
                                            editor.draft.enemies.get(index).and_then(|enemy| {
                                                editor.content.entity_path(&enemy.definition)
                                            })
                                        }
                                        _ => state
                                            .definition
                                            .selected
                                            .and_then(|i| entity_paths.get(i))
                                            .cloned(),
                                    };
                                    state.property = PropertyUi::for_entity(editor, path);
                                }
                            }
                            if crate::player_panel::draw(ui, editor, &mut state.player) {
                                // The Player panel replaces the room tools and properties.
                            } else if state.properties {
                                if let Some(p) = &mut state.boss_property {
                                    draw_boss_panel(ui, editor, p);
                                } else {
                                    draw_definition_panel(ui, editor, state);
                                    // Validate pending text even when a collapsed/filtered view returns early.
                                    if let Some(error) = pending_error(editor, &state.property) {
                                        state.property.error = Some(error);
                                    }
                                }
                            } else {
                                draw_room_tools(
                                    ui,
                                    editor,
                                    state,
                                    &entity_paths,
                                    names,
                                    &room_labels,
                                    &mut open,
                                );
                            }
                        });
                    });
                probe(ui, || {
                    (
                        egui::Id::new("panel scroll"),
                        (
                            scroll.inner_rect,
                            scroll.content_size,
                            scroll.state.offset,
                            scroll.id,
                        ),
                    )
                });
            });
        });
    PanelResponse {
        save,
        test,
        open,
        left: response.response.rect.left(),
        blocks_pointer: ctx.is_pointer_over_area() || ctx.wants_pointer_input(),
        keyboard: ctx.wants_keyboard_input(),
    }
}
