//! egui presentation only; authored content and filter rules stay in the library.
use super::{EditorSession, EditorUi, RoomEdge, SearchList, Selection};
use egui_macroquad::egui;

mod boss;
mod canvas_tools;
pub(super) use boss::{draw_boss_panel, BossPropertyUi};

pub(super) struct PanelResponse {
    pub save: bool,
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
                                state.properties = !state.properties;
                                if state.properties {
                                    state.placing = false;
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
                            if state.properties {
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
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new("panel scroll"),
                        (
                            scroll.inner_rect,
                            scroll.content_size,
                            scroll.state.offset,
                            scroll.id,
                        ),
                    )
                });
                #[cfg(not(test))]
                let _ = scroll;
            });
        });
    PanelResponse {
        save,
        open,
        left: response.response.rect.left(),
        blocks_pointer: ctx.is_pointer_over_area() || ctx.wants_pointer_input(),
        keyboard: ctx.wants_keyboard_input(),
    }
}

fn section<R>(ui: &mut egui::Ui, name: &str, contents: impl FnOnce(&mut egui::Ui) -> R) {
    let response = egui::CollapsingHeader::new(name)
        .default_open(true)
        .show(ui, contents);
    #[cfg(test)]
    ui.data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("section", name)),
            response.header_response.id,
        )
    });
    #[cfg(not(test))]
    let _ = response;
}

fn draw_room_tools(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    entity_paths: &[std::path::PathBuf],
    names: &[String],
    room_labels: &[String],
    open: &mut Option<hollow_steps::room::RoomId>,
) {
    section(ui, "Room", |ui| {
        if panel_button(ui, "New room") {
            editor.new_room();
            *state = EditorUi::default();
        }
        searchable_list(
            ui,
            egui::Id::new(("rooms",)),
            "Rooms",
            room_labels,
            &mut state.room,
        );
        if panel_button(ui, "Open room") {
            *open = state
                .room
                .selected
                .and_then(|i| editor.rooms.get(i))
                .map(|(id, _)| *id);
        }
        ui.separator();
        panel_combo(
            ui,
            egui::Id::new(("edge",)),
            "Edge",
            &["Left", "Right", "Top", "Bottom"],
            &mut state.edge,
        );
        searchable_list(
            ui,
            egui::Id::new(("target",)),
            "Exit target",
            room_labels,
            &mut state.target,
        );
        if panel_button(ui, "Add exit") {
            if let Some((id, layout)) = state.target.selected.and_then(|i| editor.rooms.get(i)) {
                state.selected = Some(editor.draft.add_exit(
                    [
                        RoomEdge::Left,
                        RoomEdge::Right,
                        RoomEdge::Top,
                        RoomEdge::Bottom,
                    ][state.edge],
                    *id,
                    layout.spawn.into(),
                ));
                editor.dirty = true;
                state.placing = false;
                state.destination = None;
            }
        }
        if let Some(Selection::Exit(i) | Selection::Destination(i)) = state.selected {
            if panel_button(ui, "Set exit target") {
                if let Some((id, layout)) = state.target.selected.and_then(|i| editor.rooms.get(i))
                {
                    if let Err(error) = editor.draft.set_exit_target(i, *id, layout.spawn.into()) {
                        editor.message = error;
                    } else {
                        editor.dirty = true;
                    }
                }
            }
            if panel_button(ui, "Arrival spawn") {
                state.destination = Some(i);
                state.selected = Some(Selection::Destination(i));
                state.placing = false;
            }
        }
        if state.destination.is_some() && panel_button(ui, "Back to room") {
            state.destination = None;
            state.selected = None;
        }
        ui.separator();
        panel_label(ui, "Blue: room spawn");
        panel_label(ui, "Green: exit span");
        panel_label(ui, "Purple: arrival spawn");
    });
    canvas_tools::draw_room_size_section(ui, editor);
    canvas_tools::draw_view_section(ui, state);
    section(ui, "Entity", |ui| {
        searchable_list(
            ui,
            egui::Id::new(("definitions",)),
            "Entity",
            names,
            &mut state.definition,
        );
        draw_entity_tools(ui, editor, state, entity_paths);
        if panel_button(ui, if state.placing { "[Place]" } else { "Place" }) {
            state.placing = state.definition.selected.is_some();
            state.destination = None;
            state.selected = None;
        }
        if panel_button(
            ui,
            if !state.placing {
                "[Select / drag]"
            } else {
                "Select / drag"
            },
        ) {
            state.placing = false;
        }
        if panel_button(ui, "Delete selected") {
            if let Some(selection) = state.selected {
                match editor.draft.delete(selection) {
                    Ok(()) => {
                        state.selected = None;
                        state.destination = None;
                        editor.dirty = true;
                    }
                    Err(error) => editor.message = error,
                }
            }
        }
    });
}

// All searchable pickers go through this widget. SearchList remains the only filter/selection logic.
fn searchable_list(
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
            #[cfg(test)]
            ui.data_mut(|data| {
                data.insert_temp(
                    id.with("result metrics"),
                    (
                        results.inner_rect,
                        results.content_size,
                        results.state.offset,
                        results.id,
                    ),
                )
            });
            #[cfg(not(test))]
            let _ = results;
        })
        .response
        .on_hover_text(caption);
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(id.with("dropdown"), (dropdown.rect, dropdown.id)));
    #[cfg(not(test))]
    let _ = dropdown;
}

fn draw_entity_tools(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    paths: &[std::path::PathBuf],
) {
    panel_input(
        ui,
        egui::Id::new(("new entity name",)),
        "New / duplicate entity name",
        &mut state.new_entity,
    );
    panel_label(ui, "New entities start with their own blank behavior.");
    let selected = state.definition.selected.and_then(|i| paths.get(i));
    let result = if panel_button(ui, "New entity") {
        editor.content.add_blank_entity(&state.new_entity).map(Some)
    } else if panel_button(ui, "Duplicate entity") {
        selected
            .ok_or_else(|| "Select an entity first".to_owned())
            .and_then(|path| editor.content.duplicate_entity(path, &state.new_entity))
            .map(Some)
    } else if panel_button(ui, "Delete entity") {
        selected
            .ok_or_else(|| "Select an entity first".to_owned())
            .and_then(|path| editor.delete_entity(path))
            .map(|()| None)
    } else {
        return;
    };
    match result {
        Ok(path) => {
            if let Ok(definitions) = editor.content.placement_definitions() {
                editor.definitions = definitions;
            }
            editor.dirty = true;
            editor.message.clear();
            state.definition.query.clear();
            state.definition.selected = path
                .as_ref()
                .and_then(|p| editor.content.entities.keys().position(|key| key == p));
            state.placing = false;
            if path.is_some() {
                state.boss_property = None;
                state.property = PropertyUi::for_entity(editor, path);
                state.properties = true;
            }
        }
        Err(error) => editor.message = error,
    }
}

#[derive(Default)]
pub(super) struct PropertyUi {
    pub(super) path: Option<std::path::PathBuf>,
    pub(super) fields: [String; 6],
    pub(super) body: Option<hollow_steps::body::BodyVisual>,
    pub(super) behavior: String,
    pub(super) behavior_list: SearchList,
    pub(super) behavior_name: String,
    pub(super) new_pose_name: String,
    pub(super) states: SearchList,
    pub(super) loaded_state: Option<(String, usize)>,
    pub(super) state_name: String,
    pub(super) actions: String,
    pub(super) transitions: String,
    pub(super) new_state: String,
    pub(super) error: Option<String>,
    pub(super) structured: hollow_steps::definition_editor::StateInputs,
    pub(super) add_action: SearchList,
    pub(super) add_condition: SearchList,
    pub(super) conditions: Vec<SearchList>,
    pub(super) advanced: bool,
    pub(super) rows: bool,
    pub(super) presets: SearchList,
    pub(super) creation: Option<BehaviorCreation>,
    pub(super) shared_edit: Option<String>,
    pub(super) key_values: Option<(String, hollow_steps::behavior_values::KeyValues)>,
}

pub(super) enum BehaviorSource {
    Preset(usize),
    Copy(String),
}

pub(super) struct BehaviorCreation {
    source: BehaviorSource,
    name: String,
    assign: bool,
}

impl PropertyUi {
    pub(super) fn for_entity(editor: &EditorSession, path: Option<std::path::PathBuf>) -> Self {
        let mut ui = Self {
            path,
            ..Self::default()
        };
        if let Some(entity) = ui
            .path
            .as_ref()
            .and_then(|path| editor.content.entities.get(path))
        {
            ui.body = entity.body.clone();
            ui.fields = [
                entity.name.clone(),
                entity.health.to_string(),
                entity.size.0.to_string(),
                entity.size.1.to_string(),
                entity.contact_damage.to_string(),
                entity.behavior.clone(),
            ];
            ui.behavior_list.selected = editor
                .content
                .behaviors
                .keys()
                .position(|name| name == &entity.behavior);
            ui.rows = editor
                .content
                .behaviors
                .get(&entity.behavior)
                .is_some_and(|b| {
                    b.states
                        .iter()
                        .all(|s| s.actions.is_empty() && s.transitions.is_empty())
                });
        }
        ui
    }
}

fn draw_definition_panel(ui: &mut egui::Ui, editor: &mut EditorSession, state: &mut EditorUi) {
    use hollow_steps::definition_editor::{
        add_state, edit_state_lists, remove_state, rename_state, serialize,
    };
    let p = &mut state.property;
    if let Some(error) = &p.error {
        panel_label(ui, error);
    }
    let mut error = None;
    let path = p
        .path
        .clone()
        .filter(|path| editor.content.entities.contains_key(path));
    section(ui, "Entity", |ui| {
        if path.is_some() {
            panel_label(ui, "Shared definition (all rooms)");
            for (i, label) in ["Name", "Health", "Width", "Height", "Contact damage"]
                .iter()
                .enumerate()
            {
                panel_input(
                    ui,
                    egui::Id::new(("entity field", i)),
                    label,
                    &mut p.fields[i],
                );
                if let Err(e) =
                    hollow_steps::definition_editor::validate_entity_field(i, &p.fields[i])
                {
                    ui.colored_label(egui::Color32::LIGHT_RED, &e);
                }
            }
            panel_label(ui, &format!("Assigned behavior: {}", p.fields[5]));
        } else {
            panel_label(ui, "Select an entity in Room tools or on the canvas");
        }
    });
    if path.is_some() {
        section(ui, "Body visual", |ui| {
            egui::ComboBox::from_id_salt("body template")
                .selected_text(p.body.as_ref().map_or("Box", |b| b.template.as_str()))
                .show_ui(ui, |ui| {
                    if ui.selectable_label(p.body.is_none(), "Box").clicked() {
                        p.body = None;
                    }
                    for name in editor.content.bodies.keys() {
                        if ui
                            .selectable_label(
                                p.body.as_ref().is_some_and(|b| b.template == *name),
                                name,
                            )
                            .clicked()
                        {
                            if let Some(body) = &mut p.body {
                                body.template = name.clone();
                            } else {
                                p.body = Some(hollow_steps::body::BodyVisual::for_template(name));
                            }
                        }
                    }
                });
            if let Some(body) = &mut p.body {
                section(ui, "Damage pose", |ui| {
                    let names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
                    let mut pose = body.hurt.pose.clone().unwrap_or_default();
                    pose_dropdown(
                        ui,
                        egui::Id::new("hurt pose"),
                        "Hurt pose",
                        &mut pose,
                        &names,
                    );
                    body.hurt.pose = if pose.is_empty() { None } else { Some(pose) };
                    for (label, value) in [
                        ("Hurt duration (seconds)", &mut body.hurt.duration_secs),
                        ("Hurt blend time (seconds)", &mut body.hurt.blend_secs),
                        ("White flash duration (seconds)", &mut body.hurt.flash_secs),
                        ("Flash tint strength (0..1)", &mut body.hurt.tint_strength),
                        ("Flash red (0..1)", &mut body.hurt.tint.0),
                        ("Flash green (0..1)", &mut body.hurt.tint.1),
                        ("Flash blue (0..1)", &mut body.hurt.tint.2),
                    ] {
                        ui.horizontal(|ui| {
                            ui.label(label);
                            ui.add(egui::DragValue::new(value));
                        });
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Visual scale");
                    ui.add(egui::DragValue::new(&mut body.scale));
                });
                for (label, color) in [
                    ("Primary", &mut body.primary),
                    ("Secondary", &mut body.secondary),
                    ("Accent", &mut body.accent),
                    ("Eye", &mut body.eye),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        let mut rgba = [color.0, color.1, color.2, color.3];
                        if ui.color_edit_button_rgba_unmultiplied(&mut rgba).changed() {
                            *color = (rgba[0], rgba[1], rgba[2], rgba[3]);
                        }
                    });
                }
                if let Err(e) = body.validate_template(&editor.content.bodies) {
                    ui.colored_label(egui::Color32::LIGHT_RED, &e);
                }
                if let Some(template) = editor.content.bodies.get_mut(&body.template) {
                    let before = template.clone();
                    draw_body_motion(ui, std::sync::Arc::make_mut(template));
                    editor.dirty |= before != *template;
                }
            }
        });
    }
    section(ui, "Pose assets", |ui| {
        for (name, pose) in &mut editor.content.poses {
            let before = pose.clone();
            section(ui, name, |ui| {
                let pose = std::sync::Arc::make_mut(pose);
                for semantic in hollow_steps::pose::SEMANTIC_POINTS {
                    ui.push_id((name, semantic), |ui| {
                        let mut enabled = pose.offsets.contains_key(*semantic);
                        if ui.checkbox(&mut enabled, *semantic).changed() {
                            if enabled {
                                pose.offsets.insert((*semantic).into(), (0.0, 0.0));
                            } else {
                                pose.offsets.remove(*semantic);
                            }
                        }
                        if let Some(offset) = pose.offsets.get_mut(*semantic) {
                            ui.horizontal(|ui| {
                                ui.label("Offset X (box fraction)");
                                ui.add(egui::DragValue::new(&mut offset.0));
                                ui.label("Y");
                                ui.add(egui::DragValue::new(&mut offset.1));
                            });
                        }
                    });
                }
                if let Err(error) = pose.validate() {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
            });
            editor.dirty |= before != *pose;
        }
        panel_input(
            ui,
            egui::Id::new("new pose name"),
            "New pose name",
            &mut p.new_pose_name,
        );
        if panel_button(ui, "Create pose") {
            match editor.content.create_pose(&p.new_pose_name) {
                Ok(()) => {
                    editor.dirty = true;
                    p.new_pose_name.clear();
                    p.error = None;
                }
                Err(error) => p.error = Some(error),
            }
        }
    });
    draw_preset_picker(ui, editor, p, path.is_some());
    if draw_behavior_creation(ui, editor, p, path.as_deref()) {
        return;
    }
    let behavior_names = editor.content.behaviors.keys().cloned().collect::<Vec<_>>();
    let mut selected_behavior = p
        .behavior_list
        .selected
        .and_then(|i| behavior_names.get(i))
        .cloned();
    let mut operation = None;
    section(ui, "Behavior", |ui| {
        searchable_list(
            ui,
            egui::Id::new(("behaviors",)),
            "Behavior",
            &behavior_names,
            &mut p.behavior_list,
        );
        selected_behavior = p
            .behavior_list
            .selected
            .and_then(|i| behavior_names.get(i))
            .cloned();
        if let Some(name) = &selected_behavior {
            if path.is_some() && panel_button(ui, "Use selected behavior") {
                p.fields[5] = name.clone();
            }
        }
        panel_input(
            ui,
            egui::Id::new(("behavior name",)),
            "New / renamed behavior name",
            &mut p.behavior_name,
        );
        operation = if panel_button(ui, "New blank behavior") {
            Some(0)
        } else if ui
            .add_enabled(
                selected_behavior.as_ref().is_some_and(|name| {
                    !editor
                        .content
                        .behavior_needs_warning(name, p.shared_edit.as_deref())
                }),
                egui::Button::new("Rename behavior"),
            )
            .clicked()
        {
            Some(1)
        } else if panel_button(ui, "Delete behavior") {
            Some(2)
        } else {
            None
        };
    });
    if let Some(path) = &path {
        if let Err(e) = hollow_steps::definition_editor::entity_with_body(&p.fields, p.body.clone())
            .and_then(|entity| editor.update_entity(path, entity))
        {
            error = Some(e);
        }
    }

    if let Some(operation) = operation {
        let result = match operation {
            0 => editor
                .content
                .create_blank_behavior(&p.behavior_name, path.as_deref()),
            _ => selected_behavior
                .as_deref()
                .ok_or_else(|| "Select a behavior first".to_owned())
                .and_then(|name| {
                    if operation == 1 {
                        editor.content.rename_behavior(name, &p.behavior_name)
                    } else {
                        editor.content.delete_behavior(name)
                    }
                }),
        };
        match result {
            Ok(()) => {
                editor.dirty = true;
                editor.message.clear();
                p.error = None;
                p.behavior_list.query.clear();
                p.behavior_list.selected = if operation == 2 {
                    None
                } else {
                    editor
                        .content
                        .behaviors
                        .keys()
                        .position(|name| name == p.behavior_name.trim())
                };
                if let Some(path) = &path {
                    p.fields[5] = editor.content.entities[path].behavior.clone();
                }
                p.loaded_state = None;
                p.key_values = None;
                if operation == 0 {
                    p.rows = true;
                }
            }
            Err(e) => {
                editor.message = e.clone();
                p.error = Some(e.clone());
                ui.colored_label(egui::Color32::LIGHT_RED, &e);
            }
        }
        return;
    }
    let Some(selected_behavior) = selected_behavior else {
        panel_label(ui, "Select a behavior to edit its states");
        p.error = error;
        return;
    };
    if p.behavior != selected_behavior {
        p.behavior = selected_behavior;
        p.loaded_state = None;
        p.key_values = None;
        p.shared_edit = None;
        p.states = SearchList {
            selected: Some(0),
            ..SearchList::default()
        };
    }
    ui.separator();
    let users = editor.content.behavior_users(&p.behavior);
    panel_label(ui, &format!("used by {users} entities"));
    let warning = editor
        .content
        .behavior_needs_warning(&p.behavior, p.shared_edit.as_deref());
    if warning {
        panel_label(
            ui,
            "Editing this behavior affects every entity that uses it.",
        );
        if panel_button(ui, "Save as new behavior") {
            p.creation = Some(BehaviorCreation {
                source: BehaviorSource::Copy(p.behavior.clone()),
                name: String::new(),
                assign: path.is_some(),
            });
        }
        if panel_button(ui, "Edit shared behavior") {
            p.shared_edit = Some(p.behavior.clone());
        }
        p.error = error;
        return;
    }
    draw_key_values(ui, editor, p, &mut error);
    if ui
        .checkbox(&mut p.rows, "Advanced: structured rows")
        .changed()
    {
        p.loaded_state = None;
    }
    let pose_names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
    if p.rows {
        let original = editor.content.behaviors.get(&p.behavior).cloned();
        section(ui, "States", |ui| {
            if let Some(behavior) = editor.content.behaviors.get_mut(&p.behavior) {
                let mut names = behavior
                    .states
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>();
                searchable_list(
                    ui,
                    egui::Id::new(("behavior states",)),
                    "State",
                    &names,
                    &mut p.states,
                );
                panel_input(
                    ui,
                    egui::Id::new(("new state",)),
                    "New state",
                    &mut p.new_state,
                );
                if panel_button(ui, "Add state") {
                    match add_state(behavior, p.new_state.clone()) {
                        Ok(()) => {
                            p.states.query.clear();
                            p.states.selected = Some(behavior.states.len() - 1);
                            p.loaded_state = None;
                            editor.dirty = true;
                            names = behavior.states.iter().map(|s| s.name.clone()).collect();
                        }
                        Err(e) => error = Some(e),
                    }
                }
                let Some(index) = p.states.selected else {
                    panel_label(ui, "Select a state to edit");
                    return;
                };
                let key = (p.behavior.clone(), index);
                if p.loaded_state.as_ref() != Some(&key) {
                    let selected = &behavior.states[index];
                    p.state_name = selected.name.clone();
                    p.structured =
                        hollow_steps::definition_editor::StateInputs::from_state(selected);
                    p.conditions = p
                        .structured
                        .transitions
                        .iter()
                        .map(|row| SearchList {
                            selected: Some(row.kind),
                            ..SearchList::default()
                        })
                        .collect();
                    p.actions = serialize(&selected.actions).expect("serializable actions");
                    p.transitions =
                        serialize(&selected.transitions).expect("serializable transitions");
                    p.loaded_state = Some(key);
                }
                panel_input(
                    ui,
                    egui::Id::new(("state name",)),
                    "State name",
                    &mut p.state_name,
                );
                if panel_button(ui, "Rename state") {
                    match rename_state(behavior, index, p.state_name.clone()) {
                        Ok(()) => {
                            editor.dirty = true;
                            p.structured = hollow_steps::definition_editor::StateInputs::from_state(
                                &behavior.states[index],
                            );
                            names = behavior.states.iter().map(|s| s.name.clone()).collect();
                            p.transitions = serialize(&behavior.states[index].transitions)
                                .expect("serializable transitions");
                        }
                        Err(e) => error = Some(e),
                    }
                }
                if panel_button(ui, "Make initial") {
                    hollow_steps::definition_editor::set_initial_state(behavior, index)
                        .expect("selected state");
                    editor.dirty = true;
                }
                panel_label(ui, &format!("Initial: {}", behavior.initial_state));
                let before = behavior.states[index].clone();
                if panel_button(ui, "Advanced: raw RON") {
                    p.advanced = !p.advanced;
                    // Switching views always starts from the last valid model.
                    p.actions = serialize(&before.actions).expect("actions");
                    p.transitions = serialize(&before.transitions).expect("transitions");
                    p.structured =
                        hollow_steps::definition_editor::StateInputs::from_state(&before);
                    p.conditions = p
                        .structured
                        .transitions
                        .iter()
                        .map(|row| SearchList {
                            selected: Some(row.kind),
                            ..SearchList::default()
                        })
                        .collect();
                }
                if p.advanced {
                    section(ui, "Actions", |ui| {
                        panel_label(ui, "Ordered actions (RON list)");
                        ui.add(
                            egui::TextEdit::multiline(&mut p.actions)
                                .id(egui::Id::new("actions"))
                                .desired_width(ui.available_width())
                                .desired_rows(6)
                                .code_editor(),
                        );
                    });
                    section(ui, "Transitions", |ui| {
                        panel_label(ui, "Transitions (RON list)");
                        ui.add(
                            egui::TextEdit::multiline(&mut p.transitions)
                                .id(egui::Id::new("transitions"))
                                .desired_width(ui.available_width())
                                .desired_rows(6)
                                .code_editor(),
                        );
                    });
                    if let Err(e) =
                        edit_state_lists(&mut behavior.states[index], &p.actions, &p.transitions)
                    {
                        error = Some(e);
                    }
                } else {
                    draw_state_rows(ui, p, &names, index, &pose_names);
                    if let Err(e) = p.structured.apply(&mut behavior.states[index], &names) {
                        error = Some(e);
                    }
                }
                editor.dirty |= before != behavior.states[index];
                if panel_button(ui, "Remove state") {
                    match remove_state(behavior, index) {
                        Ok(()) => {
                            p.states.selected = None;
                            p.loaded_state = None;
                            editor.dirty = true;
                        }
                        Err(e) => error = Some(e),
                    }
                }
            } else {
                error = Some(format!("Missing behavior file {:?}", p.behavior));
            }
        });
        if original.as_ref() != editor.content.behaviors.get(&p.behavior) {
            // Structural changes invalidate numeric addresses and labels.
            p.key_values = None;
        }
    }
    if let Some(pending) = pending_error(editor, p) {
        error = Some(pending);
    }
    p.error = error.clone();
    if let Some(e) = &error {
        editor.message = e.clone();
    }
    if let Some(e) = error {
        panel_label(ui, &e);
    }
}

fn pending_error(editor: &EditorSession, p: &PropertyUi) -> Option<String> {
    use hollow_steps::definition_editor::{edit_state_lists, entity_with_body};
    for (name, pose) in &editor.content.poses {
        if let Err(error) = pose.validate() {
            return Some(format!("pose {name:?}: {error}"));
        }
    }
    for (name, template) in &editor.content.bodies {
        if let Err(error) = template.validate() {
            return Some(format!("body template {name:?}: {error}"));
        }
    }
    if p.path
        .as_ref()
        .is_some_and(|path| editor.content.entities.contains_key(path))
    {
        if let Err(error) = entity_with_body(&p.fields, p.body.clone()).and_then(|entity| {
            if let Some(body) = &entity.body {
                body.validate_template(&editor.content.bodies)?;
            }
            if let Some(behavior) = editor.content.behaviors.get(&entity.behavior) {
                hollow_steps::pose::validate_entity(&entity, behavior, &editor.content.poses)?;
            }
            Ok(())
        }) {
            return Some(error);
        }
    }
    if let Some((name, values)) = &p.key_values {
        if let Some(behavior) = editor.content.behaviors.get(name) {
            let mut candidate = behavior.clone();
            if let Err(error) = values.apply(&mut candidate).and_then(|_| {
                hollow_steps::pose::validate_behavior(&candidate, &editor.content.poses)
            }) {
                return Some(error);
            }
        }
    }
    if let Some((behavior, index)) = &p.loaded_state {
        if let Some(behavior) = editor.content.behaviors.get(behavior) {
            if let Some(state) = behavior.states.get(*index) {
                let mut candidate = state.clone();
                let names = behavior
                    .states
                    .iter()
                    .map(|s| s.name.clone())
                    .collect::<Vec<_>>();
                let result = if p.advanced {
                    edit_state_lists(&mut candidate, &p.actions, &p.transitions)
                } else {
                    p.structured.apply(&mut candidate, &names)
                };
                if let Err(error) = result {
                    return Some(error);
                }
                let mut behavior = behavior.clone();
                behavior.states[*index] = candidate;
                if let Err(error) =
                    hollow_steps::pose::validate_behavior(&behavior, &editor.content.poses)
                {
                    return Some(error);
                }
            }
        }
    }
    editor.content.definitions().err()
}

fn draw_body_motion(ui: &mut egui::Ui, template: &mut hollow_steps::body::BodyTemplate) {
    use hollow_steps::body::{Follow, IdleMotion};
    fn follow_fields(ui: &mut egui::Ui, follow: &mut Follow) {
        ui.horizontal(|ui| {
            ui.label("Stiffness");
            ui.add(egui::DragValue::new(&mut follow.stiffness));
        });
        ui.horizontal(|ui| {
            ui.label("Damping");
            ui.add(egui::DragValue::new(&mut follow.damping));
        });
    }
    section(ui, "Pose following", |ui| {
        for (label, value) in [
            (
                "Pose settling factor",
                &mut template.pose_motion.settle_factor,
            ),
            (
                "Minimum pose blend time (seconds)",
                &mut template.pose_motion.min_blend_secs,
            ),
            (
                "Instant strike impulse time (seconds)",
                &mut template.pose_motion.instant_strike_secs,
            ),
        ] {
            ui.horizontal(|ui| {
                ui.label(label);
                ui.add(egui::DragValue::new(value));
            });
        }
        for semantic in hollow_steps::pose::SEMANTIC_POINTS {
            let mut name = template
                .pose_map
                .get(*semantic)
                .cloned()
                .unwrap_or_default();
            let names = template.points.keys().cloned().collect::<Vec<_>>();
            pose_dropdown(
                ui,
                egui::Id::new(("pose map", semantic)),
                semantic,
                &mut name,
                &names,
            );
            if name.is_empty() {
                template.pose_map.remove(*semantic);
            } else {
                template.pose_map.insert((*semantic).into(), name);
            }
        }
    });
    ui.label("Template motion (shared by all users)");
    follow_fields(ui, &mut template.follow);
    section(ui, "Point follow overrides", |ui| {
        for point in template.points.keys() {
            ui.push_id(point, |ui| {
                let mut enabled = template.follow_overrides.contains_key(point);
                let response = ui.checkbox(&mut enabled, point);
                #[cfg(test)]
                ui.data_mut(|data| {
                    data.insert_temp(egui::Id::new(("follow override", point)), response.rect)
                });
                if response.changed() {
                    if enabled {
                        template
                            .follow_overrides
                            .insert(point.clone(), template.follow);
                    } else {
                        template.follow_overrides.remove(point);
                    }
                }
                if let Some(follow) = template.follow_overrides.get_mut(point) {
                    follow_fields(ui, follow);
                }
            });
        }
    });
    section(ui, "Idle motion", |ui| {
        let mut remove = None;
        for (index, idle) in template.idle.iter_mut().enumerate() {
            ui.push_id(index, |ui| {
                egui::ComboBox::from_id_salt("idle point")
                    .selected_text(&idle.point)
                    .show_ui(ui, |ui| {
                        for name in template.points.keys() {
                            ui.selectable_value(&mut idle.point, name.clone(), name);
                        }
                    });
                for (label, value) in [
                    ("Amplitude X (box fraction)", &mut idle.amplitude.0),
                    ("Amplitude Y (box fraction)", &mut idle.amplitude.1),
                    ("Period (seconds)", &mut idle.period_secs),
                    ("Phase offset (seconds)", &mut idle.phase_offset_secs),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value));
                    });
                }
                if panel_button(ui, "Remove idle entry") {
                    remove = Some(index);
                }
            });
        }
        if let Some(index) = remove {
            template.idle.remove(index);
        }
        if let Some(point) = template.points.keys().next() {
            if panel_button(ui, "Add idle entry") {
                template.idle.push(IdleMotion::for_point(point));
            }
        }
    });
    if let Err(error) = template.validate() {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn draw_preset_picker(
    ui: &mut egui::Ui,
    editor: &EditorSession,
    p: &mut PropertyUi,
    can_assign: bool,
) {
    section(ui, "New behavior from preset", |ui| {
        let labels = editor
            .content
            .presets
            .iter()
            .map(|preset| format!("{}: {}", preset.name, preset.description))
            .collect::<Vec<_>>();
        let previous = p.presets.selected;
        searchable_list(
            ui,
            egui::Id::new("presets"),
            "Preset",
            &labels,
            &mut p.presets,
        );
        if p.presets.selected != previous {
            if let Some(index) = p.presets.selected {
                p.creation = Some(BehaviorCreation {
                    source: BehaviorSource::Preset(index),
                    name: String::new(),
                    assign: can_assign,
                });
            }
        }
        if labels.is_empty() {
            panel_label(ui, "No presets in assets/presets");
        }
    });
}

/// The prompt is shared by preset instantiation and copying a shared behavior.
fn draw_behavior_creation(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut PropertyUi,
    path: Option<&std::path::Path>,
) -> bool {
    let Some(creation) = &mut p.creation else {
        return false;
    };
    let title = match &creation.source {
        BehaviorSource::Preset(index) => {
            format!("Create from {}", editor.content.presets[*index].name)
        }
        BehaviorSource::Copy(name) => format!("Save {name} as new behavior"),
    };
    panel_label(ui, &title);
    panel_input(
        ui,
        egui::Id::new("create behavior name"),
        "Behavior name (file stem)",
        &mut creation.name,
    );
    if path.is_some() {
        ui.checkbox(
            &mut creation.assign,
            "Assign to selected entity (all its instances)",
        );
    }
    if panel_button(ui, "Create behavior") {
        let assign = path.filter(|_| creation.assign);
        let result = match &creation.source {
            BehaviorSource::Preset(index) => {
                let preset = editor.content.presets[*index].clone();
                editor
                    .content
                    .create_behavior_from_preset(&preset, &creation.name, assign)
            }
            BehaviorSource::Copy(source) => {
                editor.content.copy_behavior(source, &creation.name, assign)
            }
        };
        match result {
            Ok(()) => {
                p.behavior_list.query.clear();
                p.behavior_list.selected = editor
                    .content
                    .behaviors
                    .keys()
                    .position(|name| name == creation.name.trim());
                if let Some(path) = path {
                    p.fields[5] = editor.content.entities[path].behavior.clone();
                }
                p.loaded_state = None;
                p.key_values = None;
                p.shared_edit = None;
                p.presets.selected = None;
                p.creation = None;
                p.error = None;
                editor.message.clear();
                editor.dirty = true;
                if let Ok(definitions) = editor.content.placement_definitions() {
                    editor.definitions = definitions;
                }
            }
            Err(error) => {
                panel_label(ui, &error);
                p.error = Some(error);
            }
        }
        return true;
    }
    if panel_button(ui, "Cancel creation") {
        p.creation = None;
        p.presets.selected = None;
        p.error = None;
    }
    false
}

fn draw_key_values(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    p: &mut PropertyUi,
    error: &mut Option<String>,
) {
    use hollow_steps::behavior_values::KeyValues;
    let pose_names = editor.content.poses.keys().cloned().collect::<Vec<_>>();
    let Some(behavior) = editor.content.behaviors.get_mut(&p.behavior) else {
        return;
    };
    let (_, values) = p
        .key_values
        .get_or_insert_with(|| (p.behavior.clone(), KeyValues::from_behavior(behavior)));
    section(ui, "Key values", |ui| {
        if values.values.is_empty() {
            panel_label(ui, "This behavior has no numeric parameters.");
            panel_label(
                ui,
                "Enable Advanced: structured rows to add actions, states, and transitions, or create and assign a behavior from a preset.",
            );
        }
        let mut changed = false;
        for (index, entry) in values.values.iter_mut().enumerate() {
            let before = entry.value.clone();
            if entry.pose_choice {
                pose_dropdown(
                    ui,
                    egui::Id::new(("key value", &p.behavior, index)),
                    &entry.label,
                    &mut entry.value,
                    &pose_names,
                );
            } else {
                panel_input(
                    ui,
                    egui::Id::new(("key value", &p.behavior, index)),
                    &entry.label,
                    &mut entry.value,
                );
            }
            changed |= before != entry.value;
        }
        if let Err(e) = values.apply(&mut behavior.clone()) {
            ui.colored_label(egui::Color32::LIGHT_RED, &e);
            *error = Some(e);
        } else if changed {
            let before = behavior.clone();
            values.apply(behavior).expect("validated key values");
            editor.dirty |= before != *behavior;
            p.loaded_state = None;
            editor.message.clear();
        }
    });
}

fn panel_label(ui: &mut egui::Ui, text: &str) {
    ui.add(egui::Label::new(text).wrap());
}
fn panel_input(ui: &mut egui::Ui, id: egui::Id, label: &str, value: &mut String) {
    panel_label(ui, label);
    ui.add(
        egui::TextEdit::singleline(value)
            .id(id)
            .desired_width(ui.available_width()),
    )
    .on_hover_text(value.as_str());
}
fn panel_combo(
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
fn panel_button(ui: &mut egui::Ui, text: impl AsRef<str>) -> bool {
    let response = ui.add_sized(
        [ui.available_width(), 24.0],
        egui::Button::new(text.as_ref()).wrap(),
    );
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(egui::Id::new(("button", text.as_ref())), response.rect));
    response.clicked()
}
fn row_controls(ui: &mut egui::Ui) -> Option<u8> {
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
fn choice(ui: &mut egui::Ui, text: &str, selected: bool) -> egui::Response {
    let response = ui
        .add_sized(
            [ui.available_width(), 24.0],
            egui::Button::new(text).selected(selected).truncate(),
        )
        .on_hover_text(text);
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(egui::Id::new(("choice", text)), response.rect));
    response
}

fn pose_dropdown(
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
                #[cfg(test)]
                ui.data_mut(|data| data.insert_temp(id.with(name), response.rect));
                let _ = response;
            }
        });
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(id, response.response.rect));
    let _ = response;
}

fn draw_state_rows(
    ui: &mut egui::Ui,
    p: &mut PropertyUi,
    names: &[String],
    state_index: usize,
    pose_names: &[String],
) {
    use hollow_steps::behavior::{Action, Condition};
    use hollow_steps::definition_editor::{move_row, remove_row};
    let action_names = Action::TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    let condition_names = Condition::TYPES
        .iter()
        .map(|s| (*s).to_owned())
        .collect::<Vec<_>>();
    section(ui, "Actions", |ui| {
        panel_label(ui, "Actions (in order)");
        let mut command = None;
        for (i, row) in p.structured.actions.iter_mut().enumerate() {
            ui.separator();
            panel_label(ui, &format!("{}. {}", i + 1, Action::TYPES[row.kind]));
            let template = Action::defaults(row.kind).expect("catalog action");
            for (j, label) in template.parameter_labels().iter().enumerate() {
                if hollow_steps::pose::is_pose_parameter(template.parameters()[j].0) {
                    pose_dropdown(
                        ui,
                        egui::Id::new(("action value", i, j)),
                        label,
                        &mut row.values[j],
                        pose_names,
                    );
                } else {
                    panel_input(
                        ui,
                        egui::Id::new(("action value", i, j)),
                        label,
                        &mut row.values[j],
                    );
                }
                let mut check = template.clone();
                if let Err(error) = check.set_parameter(j, &row.values[j]) {
                    ui.colored_label(egui::Color32::LIGHT_RED, &error);
                }
            }
            if let Some(op) = row_controls(ui) {
                command = Some((i, op));
            }
        }
        if let Some((i, op)) = command {
            if op == 2 {
                let _ = remove_row(&mut p.structured.actions, i);
            } else {
                let _ = move_row(&mut p.structured.actions, i, op == 0);
            }
        }
        searchable_list(
            ui,
            egui::Id::new(("add action kind",)),
            "Action type",
            &action_names,
            &mut p.add_action,
        );
        if panel_button(ui, "Add action") {
            if let Some(kind) = p.add_action.selected {
                p.structured.add_action(kind).expect("catalog");
            }
        }
    });
    section(ui, "Transitions", |ui| {
        panel_label(ui, "Transitions (first match wins)");
        let mut command = None;
        for (i, row) in p.structured.transitions.iter_mut().enumerate() {
            ui.separator();
            panel_label(ui, &format!("Transition {}", i + 1));
            let mut target = names.iter().position(|n| *n == row.target).unwrap_or(0);
            panel_combo(
                ui,
                egui::Id::new(("transition target", i)),
                "Target",
                names,
                &mut target,
            );
            row.target = names[target].clone();
            let filter = &mut p.conditions[i];
            searchable_list(
                ui,
                egui::Id::new(("condition kind", i)),
                "Condition",
                &condition_names,
                filter,
            );
            if let Some(kind) = filter.selected {
                if kind != row.kind {
                    row.change_kind(kind).expect("catalog");
                }
            }
            panel_label(ui, &format!("Current: {}", Condition::TYPES[row.kind]));
            if let Some((label, _)) = Condition::defaults(row.kind).expect("catalog").parameter() {
                panel_input(
                    ui,
                    egui::Id::new(("condition value", i)),
                    label,
                    &mut row.value,
                );
                if let Err(error) = row.build(names) {
                    ui.colored_label(egui::Color32::LIGHT_RED, &error);
                }
            }
            ui.checkbox(&mut row.invert, "Invert");
            if let Some(op) = row_controls(ui) {
                command = Some((i, op));
            }
        }
        if let Some((i, op)) = command {
            if op == 2 {
                let _ = remove_row(&mut p.structured.transitions, i);
                let _ = remove_row(&mut p.conditions, i);
            } else {
                let _ = move_row(&mut p.structured.transitions, i, op == 0);
                let _ = move_row(&mut p.conditions, i, op == 0);
            }
        }
        searchable_list(
            ui,
            egui::Id::new(("add condition kind",)),
            "New condition",
            &condition_names,
            &mut p.add_condition,
        );
        if panel_button(ui, "Add transition") {
            if let Some(kind) = p.add_condition.selected {
                p.structured
                    .add_transition(names[state_index].clone(), kind)
                    .expect("catalog");
                p.conditions.push(SearchList {
                    selected: Some(kind),
                    ..SearchList::default()
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hollow_steps::game::GameState;

    type ScrollMetrics = (egui::Rect, egui::Vec2, egui::Vec2, egui::Id);

    fn editor() -> GameState {
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        let mut game = GameState::try_new(&assets).unwrap();
        game.toggle_editor().unwrap();
        game
    }
    fn frame(
        ctx: &egui::Context,
        size: egui::Vec2,
        events: Vec<egui::Event>,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) -> PanelResponse {
        let mut response = None;
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                events,
                ..Default::default()
            },
            |ctx| {
                response = Some(draw_panel(ctx, editor, state));
            },
        );
        response.unwrap()
    }
    fn metrics(ctx: &egui::Context) -> ScrollMetrics {
        ctx.data(|data| data.get_temp(egui::Id::new("panel scroll")).unwrap())
    }
    fn button(ctx: &egui::Context, name: &str) -> egui::Rect {
        ctx.data(|data| data.get_temp(egui::Id::new(("button", name))).unwrap())
    }
    fn pointer(pos: egui::Pos2, pressed: bool) -> Vec<egui::Event> {
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]
    }

    fn click(
        ctx: &egui::Context,
        size: egui::Vec2,
        pos: egui::Pos2,
        editor: &mut EditorSession,
        state: &mut EditorUi,
    ) {
        frame(ctx, size, pointer(pos, true), editor, state);
        frame(ctx, size, pointer(pos, false), editor, state);
        frame(ctx, size, vec![], editor, state);
    }

    #[test]
    fn collapsed_pose_authoring_values_still_block_invalid_saves() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let property = PropertyUi::default();
        editor.content.create_pose("test_pose").unwrap();
        std::sync::Arc::make_mut(editor.content.poses.get_mut("test_pose").unwrap())
            .offsets
            .insert("head".into(), (f32::NAN, 0.0));
        assert!(pending_error(editor, &property)
            .unwrap()
            .contains("offsets must be finite"));
        std::sync::Arc::make_mut(editor.content.poses.get_mut("test_pose").unwrap())
            .offsets
            .insert("head".into(), (0.2, 0.0));
        std::sync::Arc::make_mut(editor.content.bodies.get_mut("small").unwrap())
            .pose_motion
            .settle_factor = 0.0;
        assert!(pending_error(editor, &property)
            .unwrap()
            .contains("pose motion"));
    }

    #[test]
    fn pose_dropdown_selects_names_loaded_from_the_fixture_catalog() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("pose dropdown test");
        let poses = hollow_steps::pose::load_poses(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("poses"),
        )
        .unwrap();
        let names = poses.keys().cloned().collect::<Vec<_>>();
        let mut value = String::new();
        let draw = |value: &mut String, events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(600.0, 800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        pose_dropdown(ui, id, "telegraph_pose", value, &names);
                    });
                },
            );
        };
        for _ in 0..3 {
            draw(&mut value, vec![]);
        }
        let rect: egui::Rect = ctx.data(|d| d.get_temp(id).unwrap());
        draw(&mut value, pointer(rect.center(), true));
        draw(&mut value, pointer(rect.center(), false));
        draw(&mut value, vec![]);
        let option: egui::Rect = ctx.data(|d| d.get_temp(id.with("windup")).unwrap());
        draw(&mut value, pointer(option.center(), true));
        draw(&mut value, pointer(option.center(), false));
        draw(&mut value, vec![]);
        assert_eq!(value, "windup");
    }

    #[test]
    fn new_entity_starts_with_its_own_empty_behavior_and_can_be_assigned_an_existing_one() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let original = editor.content.behaviors.clone();
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 3000.0);
        let mut state = EditorUi {
            new_entity: "blank enemy".into(),
            ..Default::default()
        };
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        click(
            &ctx,
            size,
            button(&ctx, "New entity").center(),
            editor,
            &mut state,
        );
        let path = editor.content.entity_path("blank enemy").unwrap();
        let assigned = editor.content.entities[&path].behavior.clone();
        let blank = &editor.content.behaviors[&assigned];
        assert_eq!(blank.states.len(), 1);
        assert!(blank.states[0].actions.is_empty());
        assert!(blank.states[0].transitions.is_empty());
        assert!(!original.contains_key(&assigned));
        assert!(state.property.rows);
        assert!(editor.definitions.size("blank enemy").is_ok());
        for (name, behavior) in original {
            assert_eq!(editor.content.behaviors[&name], behavior);
        }
        // Authoring from scratch exposes actions immediately and refreshes Key values.
        state.property.add_action.selected = Some(0);
        click(
            &ctx,
            size,
            button(&ctx, "Add action").center(),
            editor,
            &mut state,
        );
        assert_eq!(
            editor.content.behaviors[&assigned].states[0].actions.len(),
            1
        );
        assert_eq!(
            state.property.key_values.as_ref().unwrap().1.values.len(),
            1
        );
        state.property.behavior_list.selected = editor
            .content
            .behaviors
            .keys()
            .position(|name| name == "patrol");
        frame(&ctx, size, vec![], editor, &mut state);
        click(
            &ctx,
            size,
            button(&ctx, "Use selected behavior").center(),
            editor,
            &mut state,
        );
        assert_eq!(editor.content.entities[&path].behavior, "patrol");
        state.property.behavior_name = "fresh_blank".into();
        click(
            &ctx,
            size,
            button(&ctx, "New blank behavior").center(),
            editor,
            &mut state,
        );
        assert_eq!(editor.content.entities[&path].behavior, "fresh_blank");
        assert_eq!(
            editor.content.behaviors["fresh_blank"],
            hollow_steps::behavior::BehaviorDefinition::blank()
        );
        assert!(state.property.rows);
    }

    #[test]
    fn body_settings_survive_entity_edits_and_invalid_collapsed_settings_block_save() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        editor.content.entities.get_mut(&path).unwrap().body =
            Some(hollow_steps::body::BodyVisual::for_template("small"));
        let mut state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path.clone())),
            ..Default::default()
        };
        let original = state.property.body.clone();
        state.property.fields[1] = "7".into();
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 2400.0);
        frame(&ctx, size, vec![], editor, &mut state);
        assert_eq!(editor.content.entities[&path].body, original);
        assert_eq!(editor.content.entities[&path].health, 7);
        state.property.body.as_mut().unwrap().scale = 0.0;
        let section_id: egui::Id = ctx.data(|d| {
            d.get_temp(egui::Id::new(("section", "Body visual")))
                .unwrap()
        });
        let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
            &ctx, section_id, true,
        );
        collapse.set_open(false);
        collapse.store(&ctx);
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(pending_error(editor, &state.property)
            .unwrap()
            .contains("scale"));
        assert_eq!(editor.content.entities[&path].body, original);
        state.property.body = original;
        state.property.body.as_mut().unwrap().template = "missing".into();
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(pending_error(editor, &state.property)
            .unwrap()
            .contains("unknown body template"));
    }

    #[test]
    fn body_motion_controls_add_remove_overrides_and_idle_and_block_invalid_saves() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        editor.content.entities.get_mut(&path).unwrap().body =
            Some(hollow_steps::body::BodyVisual::for_template("small"));
        let mut state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path)),
            ..Default::default()
        };
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 3000.0);
        frame(&ctx, size, vec![], editor, &mut state);
        let checkbox: egui::Rect = ctx.data(|data| {
            data.get_temp(egui::Id::new(("follow override", "root")))
                .unwrap()
        });
        click(&ctx, size, checkbox.center(), editor, &mut state);
        assert_eq!(
            editor.content.bodies["small"].follow_overrides["root"],
            editor.content.bodies["small"].follow
        );
        click(
            &ctx,
            size,
            button(&ctx, "Add idle entry").center(),
            editor,
            &mut state,
        );
        assert_eq!(editor.content.bodies["small"].idle.len(), 1);
        assert_eq!(editor.content.bodies["small"].idle[0].point, "root");
        assert!(editor.dirty);
        click(
            &ctx,
            size,
            button(&ctx, "Remove idle entry").center(),
            editor,
            &mut state,
        );
        assert!(editor.content.bodies["small"].idle.is_empty());
        std::sync::Arc::make_mut(editor.content.bodies.get_mut("small").unwrap())
            .follow_overrides
            .get_mut("root")
            .unwrap()
            .damping = -1.0;
        let id: egui::Id = ctx.data(|data| {
            data.get_temp(egui::Id::new(("section", "Body visual")))
                .unwrap()
        });
        let mut collapsed =
            egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, id, true);
        collapsed.set_open(false);
        collapsed.store(&ctx);
        frame(&ctx, size, vec![], editor, &mut state);
        let error = pending_error(editor, &state.property).unwrap();
        assert!(
            error.contains("small") && error.contains("root") && error.contains("damping"),
            "{error}"
        );
    }

    #[test]
    fn choosing_preset_prompts_before_creation_and_optional_assignment() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("patrol").unwrap();
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 2400.0);
        let mut state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path.clone())),
            ..Default::default()
        };
        state.property.presets.query = "CHASER".into();
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        let (dropdown, _): (egui::Rect, egui::Id) = ctx.data(|d| {
            d.get_temp(egui::Id::new("presets").with("dropdown"))
                .unwrap()
        });
        click(&ctx, size, dropdown.center(), editor, &mut state);
        let preset_index = editor
            .content
            .presets
            .iter()
            .position(|p| p.name == "Chaser")
            .unwrap();
        let preset = &editor.content.presets[preset_index];
        let label = format!("{}: {}", preset.name, preset.description);
        let choice: egui::Rect = ctx.data(|d| {
            d.get_temp(egui::Id::new(("choice", label.as_str())))
                .unwrap()
        });
        click(&ctx, size, choice.center(), editor, &mut state);
        assert!(
            matches!(state.property.creation.as_ref().unwrap().source, BehaviorSource::Preset(i) if i == preset_index)
        );
        assert_eq!(editor.content.behaviors.len(), 2);
        assert_eq!(editor.content.entities[&path].behavior, "patrol");
        state.property.creation.as_mut().unwrap().name = "new_chaser".into();
        frame(&ctx, size, vec![], editor, &mut state);
        click(
            &ctx,
            size,
            button(&ctx, "Create behavior").center(),
            editor,
            &mut state,
        );
        assert!(state.property.creation.is_none());
        assert_eq!(editor.content.entities[&path].behavior, "new_chaser");
        assert_eq!(state.property.fields[5], "new_chaser");
        assert_eq!(
            editor.content.behaviors["new_chaser"],
            editor.content.presets[preset_index].behavior
        );
        assert!(editor.dirty);
        state.property.creation = Some(BehaviorCreation {
            source: BehaviorSource::Preset(preset_index),
            name: "unassigned".into(),
            assign: false,
        });
        frame(&ctx, size, vec![], editor, &mut state);
        click(
            &ctx,
            size,
            button(&ctx, "Create behavior").center(),
            editor,
            &mut state,
        );
        assert!(editor.content.behaviors.contains_key("unassigned"));
        assert_eq!(editor.content.entities[&path].behavior, "new_chaser");
    }

    #[test]
    fn shared_behavior_blocks_state_edits_until_choice_and_copy_reassigns_one_entity() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("patrol").unwrap();
        let other = editor.content.duplicate_entity(&path, "second").unwrap();
        let original = editor.content.behaviors["patrol"].clone();
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 2400.0);
        let mut state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path.clone())),
            ..Default::default()
        };
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        assert!(state.property.key_values.is_none());
        assert!(state.property.loaded_state.is_none());
        state.property.rows = true;
        state.property.advanced = true;
        state.property.loaded_state = Some(("patrol".into(), 0));
        state.property.actions = "[wait(seconds: 999.0)]".into();
        state.property.transitions = "[]".into();
        frame(&ctx, size, vec![], editor, &mut state);
        assert_eq!(editor.content.behaviors["patrol"], original);
        click(
            &ctx,
            size,
            button(&ctx, "Edit shared behavior").center(),
            editor,
            &mut state,
        );
        assert_eq!(state.property.shared_edit.as_deref(), Some("patrol"));
        assert_ne!(editor.content.behaviors["patrol"], original);
        state.property.shared_edit = None;
        frame(&ctx, size, vec![], editor, &mut state);
        click(
            &ctx,
            size,
            button(&ctx, "Save as new behavior").center(),
            editor,
            &mut state,
        );
        state.property.creation.as_mut().unwrap().name = "personal".into();
        frame(&ctx, size, vec![], editor, &mut state);
        click(
            &ctx,
            size,
            button(&ctx, "Create behavior").center(),
            editor,
            &mut state,
        );
        assert_eq!(editor.content.entities[&path].behavior, "personal");
        assert_eq!(editor.content.entities[&other].behavior, "patrol");
        assert_eq!(editor.content.behavior_users("personal"), 1);
        assert_eq!(editor.content.behavior_users("patrol"), 1);
    }

    #[test]
    fn key_value_input_updates_model_and_invalid_collapsed_input_blocks_save() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("patrol").unwrap();
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 2400.0);
        let mut state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path)),
            ..Default::default()
        };
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        let id = egui::Id::new(("key value", "patrol", 0_usize));
        assert!(ctx.read_response(id).is_some());
        ctx.memory_mut(|m| m.request_focus(id));
        frame(
            &ctx,
            size,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                },
                egui::Event::Text("123".into()),
            ],
            editor,
            &mut state,
        );
        assert_eq!(
            hollow_steps::behavior_values::KeyValues::from_behavior(
                &editor.content.behaviors["patrol"]
            )
            .values[0]
                .value,
            "123"
        );
        assert!(editor.dirty);
        let valid = editor.content.behaviors["patrol"].clone();
        state.property.key_values.as_mut().unwrap().1.values[0].value = "NaN".into();
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(state.property.error.is_some());
        assert_eq!(editor.content.behaviors["patrol"], valid);
        let section_id: egui::Id = ctx.data(|d| {
            d.get_temp(egui::Id::new(("section", "Key values")))
                .unwrap()
        });
        let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
            &ctx, section_id, true,
        );
        collapse.set_open(false);
        collapse.store(&ctx);
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(pending_error(editor, &state.property).is_some());
        assert_eq!(editor.content.behaviors["patrol"], valid);
        state.property.key_values.as_mut().unwrap().1.values[0].value = "123".into();
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(state.property.error.is_none());
    }

    #[test]
    fn panel_fits_viewport_and_reserves_save_outside_scroll_bounds() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let ctx = egui::Context::default();
        let mut state = EditorUi {
            new_entity: "雪longname".repeat(512),
            ..Default::default()
        };
        for size in [
            egui::vec2(1920.0, 1080.0),
            egui::vec2(960.0, 540.0),
            egui::vec2(640.0, 360.0),
            egui::vec2(360.0, 240.0),
            egui::vec2(200.0, 100.0),
        ] {
            for _ in 0..3 {
                frame(&ctx, size, vec![], editor, &mut state);
            }
            let save = button(&ctx, "Save");
            let (viewport, _, _, _) = metrics(&ctx);
            assert!(
                save.min.y >= 0.0 && save.max.y <= size.y,
                "{size:?}: {save:?}"
            );
            assert!(viewport.max.y <= save.min.y, "scroll area covers Save");
            assert!(save.max.x <= size.x && save.min.x >= 0.0);
            let open = button(&ctx, "Open room");
            let room_picker: (egui::Rect, egui::Id) = ctx.data(|d| {
                d.get_temp(egui::Id::new(("rooms",)).with("dropdown"))
                    .unwrap()
            });
            assert!(
                room_picker.0.max.y <= open.min.y,
                "room list covers Open room"
            );
            let input = ctx
                .read_response(egui::Id::new(("new entity name",)))
                .unwrap()
                .rect;
            let entity_picker: (egui::Rect, egui::Id) = ctx.data(|d| {
                d.get_temp(egui::Id::new(("definitions",)).with("dropdown"))
                    .unwrap()
            });
            assert!(
                entity_picker.0.max.y <= input.min.y,
                "entity list covers name input"
            );
            assert!(input.max.x <= size.x, "long text expands panel");
        }
    }

    #[test]
    fn wheel_and_scrollbar_move_controls_and_resize_clamps_scroll() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let ctx = egui::Context::default();
        let mut state = EditorUi::default();
        let size = egui::vec2(960.0, 540.0);
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        let (viewport, _, before, _) = metrics(&ctx);
        frame(
            &ctx,
            size,
            vec![
                egui::Event::PointerMoved(viewport.center()),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: egui::vec2(0.0, -28.0),
                    modifiers: Default::default(),
                },
            ],
            editor,
            &mut state,
        );
        for _ in 0..20 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        let after = metrics(&ctx).2.y;
        assert!(
            after > before.y && after <= before.y + 29.0,
            "wheel moved {after}"
        );
        // Drag the real egui scrollbar, rather than injecting its saved state.
        let bar_top = egui::pos2(viewport.right() + 6.0, viewport.top() + 20.0);
        let bar_bottom = egui::pos2(bar_top.x, viewport.bottom() - 12.0);
        frame(&ctx, size, pointer(bar_top, true), editor, &mut state);
        frame(
            &ctx,
            size,
            vec![egui::Event::PointerMoved(bar_bottom)],
            editor,
            &mut state,
        );
        frame(&ctx, size, pointer(bar_bottom, false), editor, &mut state);
        assert!(
            metrics(&ctx).2.y > after + 50.0,
            "scrollbar did not drag: {:?}",
            metrics(&ctx)
        );
        let expanded = egui::vec2(1920.0, 1800.0);
        for _ in 0..3 {
            frame(&ctx, expanded, vec![], editor, &mut state);
        }
        assert_eq!(metrics(&ctx).2.y, 0.0);
    }

    #[test]
    fn searchable_dropdown_has_eight_rows_and_reaches_last_result_near_bottom() {
        let ctx = egui::Context::default();
        let id = egui::Id::new("test picker");
        let names = (0..40)
            .map(|i| format!("Enemy {i}"))
            .chain(["Item: Dash".to_owned()])
            .collect::<Vec<_>>();
        let mut list = SearchList::default();
        let size = egui::vec2(360.0, 360.0);
        let draw = |events: Vec<egui::Event>, list: &mut SearchList| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.add_space(200.0);
                        searchable_list(ui, id, "Entity", &names, list);
                    });
                },
            );
        };
        for _ in 0..3 {
            draw(vec![], &mut list);
        }
        let (rect, _): (egui::Rect, egui::Id) =
            ctx.data(|d| d.get_temp(id.with("dropdown")).unwrap());
        draw(pointer(rect.center(), true), &mut list);
        draw(pointer(rect.center(), false), &mut list);
        for _ in 0..3 {
            draw(vec![], &mut list);
        }
        let (viewport, content, _, scroll_id): ScrollMetrics =
            ctx.data(|d| d.get_temp(id.with("result metrics")).unwrap());
        assert!(viewport.height() >= 8.0 * 24.0, "{viewport:?}");
        assert!(
            viewport.top() >= 0.0 && viewport.bottom() <= size.y,
            "popup offscreen: {viewport:?}"
        );
        let mut scroll = egui::scroll_area::State::load(&ctx, scroll_id).unwrap();
        scroll.offset.y = content.y;
        scroll.store(&ctx, scroll_id);
        for _ in 0..3 {
            draw(vec![], &mut list);
        }
        let last: egui::Rect =
            ctx.data(|d| d.get_temp(egui::Id::new(("choice", "Item: Dash"))).unwrap());
        let (viewport, _, _, _): ScrollMetrics =
            ctx.data(|d| d.get_temp(id.with("result metrics")).unwrap());
        assert!(
            viewport.contains(last.center()),
            "last result unreachable: {last:?} {viewport:?}"
        );
        draw(pointer(last.center(), true), &mut list);
        draw(pointer(last.center(), false), &mut list);
        assert_eq!(list.selected, Some(40));
        list.query = "missing".into();
        draw(vec![], &mut list);
        assert_eq!(list.selected, None);
        draw(pointer(rect.center(), true), &mut list);
        draw(pointer(rect.center(), false), &mut list);
        for _ in 0..3 {
            draw(vec![], &mut list);
        }
        assert!(ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("choice", "No matches"))))
            .is_some());
    }

    #[test]
    fn boss_picker_properties_attack_prompt_and_collapsed_validation_work_headlessly() {
        let mut game = editor();
        game.edit_room(hollow_steps::room::RoomId::Boss).unwrap();
        let editor = game.editor_mut().unwrap();
        let path = editor.content.boss_path("Ash Warden").unwrap();
        let ctx = egui::Context::default();
        let size = egui::vec2(960.0, 4000.0);
        let mut state = EditorUi {
            definition: SearchList {
                selected: Some(editor.content.entities.len()),
                ..Default::default()
            },
            ..Default::default()
        };
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        click(
            &ctx,
            size,
            button(&ctx, "Entity properties").center(),
            editor,
            &mut state,
        );
        assert_eq!(state.boss_property.as_ref().unwrap().path, path);
        assert_eq!(
            state.boss_property.as_ref().unwrap().inputs.fields[0],
            "Ash Warden"
        );
        click(
            &ctx,
            size,
            button(&ctx, "Add attack").center(),
            editor,
            &mut state,
        );
        ctx.memory_mut(|m| m.request_focus(egui::Id::new("new boss attack")));
        frame(
            &ctx,
            size,
            vec![egui::Event::Text("New boss attack".into())],
            editor,
            &mut state,
        );
        click(
            &ctx,
            size,
            button(&ctx, "Create attack").center(),
            editor,
            &mut state,
        );
        let p = state.boss_property.as_ref().unwrap();
        assert_eq!(p.inputs.attacks.last().unwrap().name, "New boss attack");
        assert!(p
            .error
            .as_ref()
            .unwrap()
            .contains("actions must not be empty"));
        assert_eq!(editor.content.bosses[&path].attacks.len(), 3);
        // Filter the shared searchable widget to a boss-only action and add it through the UI.
        let filter = egui::Id::new("boss action catalog").with("search");
        ctx.memory_mut(|m| m.request_focus(filter));
        frame(
            &ctx,
            size,
            vec![egui::Event::Text("shockwave".into())],
            editor,
            &mut state,
        );
        let (rect, _): (egui::Rect, egui::Id) = ctx.data(|d| {
            d.get_temp(egui::Id::new("boss action catalog").with("dropdown"))
                .unwrap()
        });
        click(&ctx, size, rect.center(), editor, &mut state);
        let row: egui::Rect = ctx.data(|d| {
            d.get_temp(egui::Id::new(("choice", "spawn_shockwave")))
                .unwrap()
        });
        click(&ctx, size, row.center(), editor, &mut state);
        click(
            &ctx,
            size,
            button(&ctx, "Add action").center(),
            editor,
            &mut state,
        );
        assert!(state.boss_property.as_ref().unwrap().error.is_none());
        assert_eq!(editor.content.bosses[&path].attacks.len(), 4);
        assert!(matches!(
            editor.content.bosses[&path].attacks[3].actions[0],
            hollow_steps::behavior::Action::SpawnShockwave { .. }
        ));
        let before = editor.content.bosses[&path].clone();
        state.boss_property.as_mut().unwrap().inputs.attacks[3].actions[0].values[0] = "-1".into();
        // Changing attack selection and collapsing the editor must not hide invalid text from Save.
        state.boss_property.as_mut().unwrap().attack = Some(0);
        let header = ctx.data(|d| {
            d.get_temp::<egui::Id>(egui::Id::new(("section", "Attack editor")))
                .unwrap()
        });
        let mut collapsed =
            egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, header, true);
        collapsed.set_open(false);
        collapsed.store(&ctx);
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(state
            .boss_property
            .as_ref()
            .unwrap()
            .error
            .as_ref()
            .unwrap()
            .contains("Shockwave radius"));
        assert_eq!(editor.content.bosses[&path], before);
        assert!(button(&ctx, "Save").bottom() <= size.y);
    }

    #[test]
    fn panel_captures_pointer_and_collapsed_invalid_fields_still_block_save() {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let ctx = egui::Context::default();
        let path = editor.content.entities.keys().next().cloned();
        let mut state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, path),
            ..Default::default()
        };
        let size = egui::vec2(960.0, 540.0);
        for _ in 0..3 {
            frame(&ctx, size, vec![], editor, &mut state);
        }
        let response = frame(
            &ctx,
            size,
            pointer(button(&ctx, "Save").center(), true),
            editor,
            &mut state,
        );
        assert!(response.blocks_pointer);
        frame(
            &ctx,
            size,
            pointer(button(&ctx, "Save").center(), false),
            editor,
            &mut state,
        );
        state.property.fields[1] = "invalid".into();
        let section_id: egui::Id =
            ctx.data(|d| d.get_temp(egui::Id::new(("section", "Entity"))).unwrap());
        let mut collapse = egui::collapsing_header::CollapsingState::load_with_default_open(
            &ctx, section_id, true,
        );
        collapse.set_open(false);
        collapse.store(&ctx);
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(state.property.error.is_some());
        assert!(pending_error(editor, &state.property).is_some());
        state.property.fields[1] = "3".into();
        frame(&ctx, size, vec![], editor, &mut state);
        assert!(state.property.error.is_none());
        let response = frame(
            &ctx,
            size,
            vec![egui::Event::PointerMoved(egui::pos2(50.0, 100.0))],
            editor,
            &mut state,
        );
        assert!(!response.blocks_pointer);
    }
}
