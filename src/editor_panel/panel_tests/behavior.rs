//! Behaviors (presets, sharing), key values and the boss panel, driven through the UI.
use super::*;

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
    assert_eq!(state.property.fields.behavior, "new_chaser");
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
    let mut collapse =
        egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, section_id, true);
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
