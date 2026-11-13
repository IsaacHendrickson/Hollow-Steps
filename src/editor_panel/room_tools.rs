//! The Room tools panel: tools, history, rooms and exits, room size, view and the entity list.
use super::property::PropertyUi;
use super::room_name_field::draw_room_name_field;
use super::widgets::{
    panel_button, panel_combo, panel_input, panel_label, searchable_list, section,
};
use super::{canvas_tools, history_panel, look_panel, tool_panel};
use super::{EditorSession, EditorUi, Selection};
use egui_macroquad::egui;
use hollow_steps::editor_history::tracked;
use hollow_steps::editor_tools::EditorTool;
use hollow_steps::room::RoomEdge;

pub(super) fn draw_room_tools(
    ui: &mut egui::Ui,
    editor: &mut EditorSession,
    state: &mut EditorUi,
    entity_paths: &[std::path::PathBuf],
    names: &[String],
    room_labels: &[String],
    open: &mut Option<hollow_steps::room::RoomId>,
) {
    draw_room_name_field(ui, editor, state);
    tool_panel::draw_tool_section(ui, editor, state);
    history_panel::draw_history_section(ui, editor, state);
    section(ui, "Room", |ui| {
        if panel_button(ui, "New room") {
            // Asks for the name first; `draw_editor` then asks to save if there are unsaved
            // changes and starts the room.
            state.name_prompt = Some(hollow_steps::room_name::NamePrompt::new(
                editor.suggested_room_name(),
            ));
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
            let target = state
                .target
                .selected
                .and_then(|i| editor.rooms.get(i))
                .map(|(id, layout)| (*id, layout.spawn));
            if let Some((id, spawn)) = target {
                let edge = [
                    RoomEdge::Left,
                    RoomEdge::Right,
                    RoomEdge::Top,
                    RoomEdge::Bottom,
                ][state.edge];
                let added = tracked(&mut state.history, editor, |e| {
                    Ok(e.draft.add_exit(edge, id, spawn.into()))
                });
                if let Ok(selection) = added {
                    state.selected = Some(selection);
                    editor.dirty = true;
                    state.tool = EditorTool::Select;
                    state.destination = None;
                }
            }
        }
        if panel_button(ui, "Add exit and link...") {
            // Places the exit and asks which room it leads to; Cancel takes it out again.
            let edge = [
                RoomEdge::Left,
                RoomEdge::Right,
                RoomEdge::Top,
                RoomEdge::Bottom,
            ][state.edge];
            state.start_link_new_exit(editor, edge);
        }
        if let Some(Selection::Exit(i) | Selection::Destination(i)) = state.selected {
            if let Some(leads_to) = editor.exit_leads_to(i) {
                panel_label(ui, &format!("Leads to {leads_to}"));
            }
            if panel_button(ui, "Link...") {
                state.start_link(editor, i);
            }
            if panel_button(ui, "Set exit target") {
                let target = state
                    .target
                    .selected
                    .and_then(|i| editor.rooms.get(i))
                    .map(|(id, layout)| (*id, layout.spawn));
                if let Some((id, spawn)) = target {
                    let result = tracked(&mut state.history, editor, |e| {
                        e.draft.set_exit_target(i, id, spawn.into())
                    });
                    match result {
                        Ok(()) => editor.dirty = true,
                        Err(error) => editor.message = error,
                    }
                }
            }
            if panel_button(ui, "Arrival spawn") {
                state.destination = Some(i);
                state.selected = Some(Selection::Destination(i));
                state.tool = EditorTool::Select;
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
    canvas_tools::draw_room_size_section(ui, editor, &mut state.history);
    look_panel::draw_look_section(ui, editor, state);
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
    });
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
            state.tool = EditorTool::Select;
            if path.is_some() {
                state.boss_property = None;
                state.property = PropertyUi::for_entity(editor, path);
                state.properties = true;
            }
        }
        Err(error) => editor.message = error,
    }
}
