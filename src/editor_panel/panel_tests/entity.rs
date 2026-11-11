//! The Entity properties panel: fields, body visual and motion, poses, and new entities.
use super::*;

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
    state.property.fields.health = "7".into();
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
    let mut collapse =
        egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, section_id, true);
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
    state.property.fields.health = "invalid".into();
    let section_id: egui::Id =
        ctx.data(|d| d.get_temp(egui::Id::new(("section", "Entity"))).unwrap());
    let mut collapse =
        egui::collapsing_header::CollapsingState::load_with_default_open(&ctx, section_id, true);
    collapse.set_open(false);
    collapse.store(&ctx);
    frame(&ctx, size, vec![], editor, &mut state);
    assert!(state.property.error.is_some());
    assert!(pending_error(editor, &state.property).is_some());
    state.property.fields.health = "3".into();
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
