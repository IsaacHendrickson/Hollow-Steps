use super::super::history_panel::apply_history;
use super::super::test_support::{click, editing_game as editor, frame, laid_out};
use super::*;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::terrain_styles::RoomStyle;

fn rect(ctx: &egui::Context, id: egui::Id) -> egui::Rect {
    ctx.data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"))
}

fn color_id(label: &str) -> egui::Id {
    egui::Id::new(("look color", label))
}

fn click_on(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut EditorUi) {
    let [press, release] = click(rect(ctx, id).center());
    frame(ctx, press, editor, state);
    frame(ctx, release, editor, state);
    frame(ctx, vec![], editor, state);
}

fn style(editor: &EditorSession) -> RoomStyle {
    editor.draft.layout.terrain_style.clone()
}

fn undo(editor: &mut EditorSession, state: &mut EditorUi) {
    apply_history(editor, state, HistoryKey::Undo);
}

#[test]
fn the_section_shows_every_control_and_they_fit_the_narrow_panel() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    for expected in [
        "Room look",
        "Preset",
        "stone",
        "Fill",
        "Edge",
        "Highlight",
        "Accent",
        "Reset",
        "Moss",
        "Draw boundary",
        "Seed offset",
        "Reroll",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text:?}");
    }
    // Each control is inside the 360 px window and is not squeezed to nothing.
    let mut ids = vec![
        egui::Id::new(("look preset",)),
        egui::Id::new(("look seed",)),
        egui::Id::new(("look checkbox", "Moss")),
        egui::Id::new(("look checkbox", "Draw boundary")),
        egui::Id::new(("button", "Reroll")),
    ];
    for label in ["Fill", "Edge", "Highlight", "Accent"] {
        ids.push(color_id(label));
        ids.push(egui::Id::new(("look reset", label)));
    }
    for id in ids {
        let r = rect(&ctx, id);
        assert!(r.min.x >= 0.0 && r.max.x <= 360.0, "{id:?}: {r:?}");
        assert!(r.width() >= 16.0 && r.height() >= 14.0, "{id:?}: {r:?}");
    }
    assert!(!editor.dirty, "showing the controls is not an edit");
    assert!(!state.history.can_undo());
}

#[test]
fn the_controls_show_the_looks_resolved_values() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor
        .edit_look(|d, s| d.set_look_preset(s, "ruins"))
        .unwrap();
    editor
        .edit_look(|d, _| {
            d.set_seed_offset(321);
            d.set_draw_boundary(false);
            Ok(())
        })
        .unwrap();
    let mut state = EditorUi::default();
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    assert!(text.contains("ruins"), "{text}");
    assert!(text.contains("321"), "{text}");
}

#[test]
fn choosing_a_preset_from_the_dropdown_is_one_undoable_step_that_clears_overrides() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Fill, (0.9, 0.1, 0.1)))
        .unwrap();
    editor
        .edit_look(|d, _| {
            d.set_seed_offset(5);
            Ok(())
        })
        .unwrap();
    editor.dirty = false;
    let before = style(editor);
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);

    click_on(&ctx, egui::Id::new(("look preset",)), editor, &mut state);
    click_on(&ctx, egui::Id::new(("choice", "cave")), editor, &mut state);

    let after = style(editor);
    assert_eq!(after.preset, "cave");
    assert_eq!(after.fill, None, "the override is cleared");
    assert_eq!(after.seed_offset, 5, "the seed stays");
    assert!(editor.dirty);
    assert!(state.history.can_undo());

    undo(editor, &mut state);
    assert_eq!(style(editor), before);
    assert!(!state.history.can_undo(), "one step");
}

#[test]
fn choosing_the_preset_already_chosen_changes_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Edge, (0.1, 0.2, 0.3)))
        .unwrap();
    editor.dirty = false;
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);

    click_on(&ctx, egui::Id::new(("look preset",)), editor, &mut state);
    click_on(&ctx, egui::Id::new(("choice", "stone")), editor, &mut state);

    assert_eq!(style(editor).edge, Some((0.1, 0.2, 0.3)));
    assert!(!editor.dirty);
    assert!(!state.history.can_undo());
}

#[test]
fn reset_works_only_on_an_overridden_color_and_goes_back_to_the_preset() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);

    // Nothing overridden: the disabled Reset does nothing.
    click_on(
        &ctx,
        egui::Id::new(("look reset", "Fill")),
        editor,
        &mut state,
    );
    assert!(!editor.dirty);
    assert!(!state.history.can_undo());

    editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Fill, (0.9, 0.1, 0.1)))
        .unwrap();
    editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Edge, (0.1, 0.9, 0.1)))
        .unwrap();
    editor.dirty = false;
    frame(&ctx, vec![], editor, &mut state);

    click_on(
        &ctx,
        egui::Id::new(("look reset", "Fill")),
        editor,
        &mut state,
    );

    assert_eq!(style(editor).fill, None);
    assert_eq!(
        style(editor).edge,
        Some((0.1, 0.9, 0.1)),
        "only Fill resets"
    );
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(style(editor).fill, Some((0.9, 0.1, 0.1)));
}

#[test]
fn the_moss_and_boundary_checkboxes_edit_the_look_one_step_each() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    assert!(editor.look().palette.moss && editor.look().draw_boundary);

    click_on(
        &ctx,
        egui::Id::new(("look checkbox", "Moss")),
        editor,
        &mut state,
    );
    assert_eq!(style(editor).moss, Some(false));
    assert!(!editor.look().palette.moss);

    click_on(
        &ctx,
        egui::Id::new(("look checkbox", "Draw boundary")),
        editor,
        &mut state,
    );
    assert!(!style(editor).draw_boundary);
    assert!(!editor.look().draw_boundary);
    assert!(editor.dirty);

    undo(editor, &mut state);
    assert!(style(editor).draw_boundary);
    assert_eq!(style(editor).moss, Some(false));
    undo(editor, &mut state);
    assert_eq!(style(editor), RoomStyle::default());
    assert!(!state.history.can_undo());

    // Back to the preset's moss: no override is kept.
    click_on(
        &ctx,
        egui::Id::new(("look checkbox", "Moss")),
        editor,
        &mut state,
    );
    click_on(
        &ctx,
        egui::Id::new(("look checkbox", "Moss")),
        editor,
        &mut state,
    );
    assert_eq!(style(editor), RoomStyle::default());
}

#[test]
fn reroll_moves_the_seed_and_each_click_is_an_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);

    click_on(
        &ctx,
        egui::Id::new(("button", "Reroll")),
        editor,
        &mut state,
    );
    click_on(
        &ctx,
        egui::Id::new(("button", "Reroll")),
        editor,
        &mut state,
    );

    assert_eq!(style(editor).seed_offset, 2);
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(style(editor).seed_offset, 1);
    undo(editor, &mut state);
    assert_eq!(style(editor).seed_offset, 0);
    assert!(!state.history.can_undo());
}

#[test]
fn dragging_the_seed_field_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    let field = rect(&ctx, egui::Id::new(("look seed",)));
    let start = field.center();
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let mut seen = vec![0];
    let mut step = |events: Vec<egui::Event>, state: &mut EditorUi, editor: &mut EditorSession| {
        frame(&ctx, events, editor, state);
        seen.push(style(editor).seed_offset);
    };
    step(vec![egui::Event::PointerMoved(start)], &mut state, editor);
    step(vec![button(start, true)], &mut state, editor);
    for i in 1..=4 {
        let to = start + egui::vec2(15.0 * i as f32, 0.0);
        step(vec![egui::Event::PointerMoved(to)], &mut state, editor);
    }
    assert!(state.history.gesture_open(), "the drag is open mid-way");
    assert!(!state.history.can_undo(), "undo waits for the drag to end");
    let end = start + egui::vec2(60.0, 0.0);
    step(vec![button(end, false)], &mut state, editor);
    step(vec![], &mut state, editor);

    let mut distinct = seen.clone();
    distinct.dedup();
    assert!(
        distinct.len() > 3,
        "the seed changed over several frames: {seen:?}"
    );
    let dragged = style(editor).seed_offset;
    assert!(dragged > 0);
    assert!(!state.history.gesture_open());
    assert!(editor.dirty);

    undo(editor, &mut state);
    assert_eq!(
        style(editor).seed_offset,
        0,
        "one undo takes back the whole drag"
    );
    assert!(!state.history.can_undo(), "the drag was a single step");
    apply_history(editor, &mut state, HistoryKey::Redo);
    assert_eq!(style(editor).seed_offset, dragged);
}

#[test]
fn an_edit_made_while_another_drag_is_open_joins_that_step() {
    // A canvas drag has the history open; an edit from the panel then is part of the same step.
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let original = editor.draft.clone();

    state.history.begin_gesture(&editor.draft);
    apply(editor, &mut state, false, |d, _| {
        d.reroll_seed();
        Ok(())
    });
    assert!(editor.dirty);
    assert!(!state.history.can_undo(), "the open drag has not ended");
    state.history.end_gesture(&editor.draft);

    undo(editor, &mut state);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo(), "one step");
}

#[test]
fn a_refused_edit_says_why_and_records_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let before = editor.draft.clone();

    apply(editor, &mut state, false, |d, s| {
        d.set_look_preset(s, "granite")
    });

    assert!(editor.message.contains("granite"), "{}", editor.message);
    assert_eq!(editor.draft, before);
    assert!(!editor.dirty);
    assert!(!state.history.can_undo());
}

#[test]
fn an_unknown_preset_is_shown_and_can_be_replaced_from_the_dropdown() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.draft.layout.terrain_style = RoomStyle::named("granite");
    let mut state = EditorUi::default();
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    assert!(text.contains("granite"), "{text}");
    assert!(text.contains("not in terrain_styles.ron"), "{text}");
    // The canvas draws the default look rather than failing.
    assert_eq!(editor.look(), hollow_steps::terrain_style::Look::default());

    click_on(&ctx, egui::Id::new(("look preset",)), editor, &mut state);
    click_on(&ctx, egui::Id::new(("choice", "stone")), editor, &mut state);

    assert_eq!(style(editor).preset, "stone");
    assert!(editor.dirty);
}

/// The rectangle of the open color picker: the one popup above the panel.
fn picker(ctx: &egui::Context) -> egui::Rect {
    let layers = ctx.memory(|memory| memory.areas().visible_layer_ids());
    layers
        .into_iter()
        .filter(|layer| layer.order == egui::Order::Foreground)
        .find_map(|layer| ctx.memory(|memory| memory.area_rect(layer.id)))
        .expect("the color picker is open")
}

#[test]
fn dragging_in_the_color_picker_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    assert_eq!(style(editor).fill, None);

    // Click the swatch: the picker opens and nothing is edited yet.
    click_on(&ctx, color_id("Fill"), editor, &mut state);
    let popup = picker(&ctx);
    assert!(!editor.dirty && !state.history.can_undo());

    // Press in the saturation/brightness square and drag across it, then let go.
    let start = popup.min + egui::vec2(30.0, 90.0);
    let button = |pos, pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    let mut seen = vec![style(editor).fill];
    let mut step = |events: Vec<egui::Event>, state: &mut EditorUi, editor: &mut EditorSession| {
        frame(&ctx, events, editor, state);
        seen.push(style(editor).fill);
    };
    step(vec![egui::Event::PointerMoved(start)], &mut state, editor);
    step(vec![button(start, true)], &mut state, editor);
    for i in 1..=5 {
        let to = start + egui::vec2(25.0 * i as f32, 12.0 * i as f32);
        step(vec![egui::Event::PointerMoved(to)], &mut state, editor);
    }
    assert!(state.history.gesture_open(), "the drag is one open step");
    assert!(!state.history.can_undo(), "undo waits for the release");
    let end = start + egui::vec2(125.0, 60.0);
    step(vec![button(end, false)], &mut state, editor);
    step(vec![], &mut state, editor);

    let mut distinct = seen.clone();
    distinct.dedup();
    assert!(
        distinct.len() > 3,
        "the fill changed over several frames: {seen:?}"
    );
    assert!(!state.history.gesture_open());
    assert!(editor.dirty);
    let picked = style(editor).fill;
    assert!(picked.is_some());

    undo(editor, &mut state);
    assert_eq!(
        style(editor).fill,
        None,
        "one undo takes back the whole drag"
    );
    assert!(!state.history.can_undo(), "the drag was a single step");
    apply_history(editor, &mut state, HistoryKey::Redo);
    assert_eq!(style(editor).fill, picked);
}

#[test]
fn opening_the_color_picker_and_leaving_it_open_changes_nothing() {
    // The picker rounds the color it is shown and calls that a change; it must not become an
    // override of the preset, mark the room unsaved or add an undo step.
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    for label in ["Fill", "Edge", "Highlight", "Accent"] {
        click_on(&ctx, color_id(label), editor, &mut state);
        for _ in 0..5 {
            frame(&ctx, vec![], editor, &mut state);
        }
        picker(&ctx);
        assert_eq!(style(editor), RoomStyle::default(), "{label}");
        assert!(!editor.dirty, "{label}");
        assert!(!state.history.can_undo(), "{label}");
        // Close it: an open picker covers the swatches below its own.
        let escape = egui::Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        };
        frame(&ctx, vec![escape], editor, &mut state);
        frame(&ctx, vec![], editor, &mut state);
    }
}
