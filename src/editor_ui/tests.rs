use super::*;

#[test]
fn a_new_session_forgets_history_and_selection_but_keeps_grid_settings() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    let mut state = EditorUi::default();
    state.grid.enabled = true;
    state.grid.size = 16.0;
    state.selected = Some(Selection::Item(0));
    state.tool = EditorTool::Terrain;
    state.terrain.select(Some(0));
    state.view = Some(CanvasView {
        offset: Vec2::ZERO,
        scale: 2.0,
    });
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.clone();
    tracked(&mut state.history, editor, |e| {
        e.draft.delete(Selection::Item(0))
    })
    .unwrap();
    assert!(state.history.can_undo());

    let next = state.for_new_session();
    assert!(!next.history.can_undo() && !next.history.can_redo());
    assert_eq!(next.selected, None);
    assert_eq!(
        (next.tool, next.terrain.selected()),
        (EditorTool::Select, None)
    );
    assert!(next.view.is_none());
    assert!(next.grid.enabled);
    assert_eq!(next.grid.size, 16.0);
    // The old state is untouched, so the draft it recorded can still be restored.
    assert!(state.history.can_undo());
    assert_ne!(game.editor().unwrap().draft, before);
}

#[test]
fn authored_boss_has_a_canvas_marker() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.edit_room(hollow_steps::room::RoomId::Boss).unwrap();
    let editor = game.editor().unwrap();
    let placement = editor.draft.boss.as_ref().unwrap();
    assert!(editor_markers(editor, None, None)
        .iter()
        .any(|(selection, rect, _)| {
            *selection == Selection::Boss
                && rect.center() == Vec2::from(placement.position)
                && rect.size() == vec2(48.0, 48.0)
        }));
}

#[test]
fn property_error_only_survives_a_visible_selected_enemy() {
    for (properties, selected, retained) in [
        (true, Some(Selection::Enemy(0)), true),
        (false, Some(Selection::Enemy(0)), false),
        (true, None, false),
        (true, Some(Selection::Item(0)), false),
        (true, Some(Selection::Exit(0)), false),
        (true, Some(Selection::Enemy(1)), false),
    ] {
        let mut state = EditorUi {
            properties,
            selected,
            property: PropertyUi {
                error: Some("invalid text".into()),
                ..Default::default()
            },
            ..Default::default()
        };
        state.clear_hidden_property_error(1);
        assert_eq!(state.property.error.is_some(), retained);
    }
}
