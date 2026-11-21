use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use crate::editor_panel::Selection;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::hazard_look::{HazardFacing, HazardStyle};

/// One pass of the whole panel in a tall window; returns what it drew.
fn pass(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> String {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, 3000.0),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            super::super::draw_panel(ctx, editor, state);
        },
    );
    drawn_text(&output)
}

fn ready(editor: &mut EditorSession, state: &mut EditorUi) -> (egui::Context, String) {
    let ctx = egui::Context::default();
    let mut text = String::new();
    for _ in 0..3 {
        text = pass(&ctx, vec![], editor, state);
    }
    (ctx, text)
}

fn click_panel(ctx: &egui::Context, label: &str, editor: &mut EditorSession, state: &mut EditorUi) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("button", label))))
        .unwrap_or_else(|| panic!("no {label} button"));
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    for _ in 0..2 {
        pass(ctx, vec![], editor, state);
    }
}

/// Clicks whatever was last laid out under `id` (a probe) and lets the result settle.
fn click_id(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut EditorUi) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"));
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    for _ in 0..2 {
        pass(ctx, vec![], editor, state);
    }
}

/// A game editing the first room with one hazard selected, and the panel drawn once.
fn with_a_selected_hazard(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
    editor.draft.add_hazard().unwrap();
    state.selected = Some(Selection::Hazard(0));
    ready(editor, state).0
}

fn damage(editor: &EditorSession) -> u16 {
    editor.draft.hazard(0).unwrap().damage
}

#[test]
fn the_panel_offers_add_hazard_and_explains_the_magenta_marker() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let (_, text) = ready(editor, &mut EditorUi::default());
    for expected in ["Add hazard", "Magenta: hazard (hurts, knocks back)"] {
        assert!(text.contains(expected), "missing {expected:?}");
    }
    assert!(!text.contains("Knockback speed"), "no hazard is selected");
}

#[test]
fn add_hazard_puts_one_in_the_room_selects_it_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    click_panel(&ctx, "Add hazard", editor, &mut state);
    assert_eq!(editor.draft.layout.hazards.len(), 1);
    assert_eq!(state.selected, Some(Selection::Hazard(0)));
    assert!(editor.dirty);
    assert_eq!(state.tool, EditorTool::Select);

    click_panel(&ctx, "Undo", editor, &mut state);
    assert_eq!(editor.draft, before);
}

#[test]
fn the_fields_show_for_a_selected_hazard_and_not_for_an_exit() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, _) = ready(editor, &mut state);
    click_panel(&ctx, "Add hazard", editor, &mut state);
    let text = pass(&ctx, vec![], editor, &mut state);
    for expected in [
        "Hazard width",
        "Hazard height",
        "Damage",
        "Knockback speed",
        "Lift speed",
        "Knockback time",
        "Shape",
        "Spikes",
        "Facing",
        "Up",
        "Color",
        "Reset",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    state.selected = Some(Selection::Exit(0));
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(!text.contains("Knockback speed"), "{text}");
}

#[test]
fn an_accepted_edit_changes_the_hazard_marks_it_unsaved_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_hazard().unwrap();
    editor.message = "stale".into();
    let original = editor.draft.clone();
    let current = editor.draft.hazard(0).unwrap().fields();
    let mut edited = current;
    edited.damage = 25;
    edited.height = 32.0;
    apply_fields(editor, &mut state.history, 0, current, edited);
    assert_eq!(damage(editor), 25);
    assert_eq!(editor.draft.hazard(0).unwrap().rect.3, 32.0);
    assert!(editor.dirty && editor.message.is_empty());

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo(), "one edit, one step");
}

#[test]
fn an_unchanged_edit_does_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    editor.draft.add_hazard().unwrap();
    let current = editor.draft.hazard(0).unwrap().fields();
    apply_fields(editor, &mut history, 0, current, current);
    assert!(!editor.dirty && !history.can_undo());
}

#[test]
fn a_refused_edit_leaves_the_draft_says_why_and_records_no_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    editor.draft.add_hazard().unwrap();
    let original = editor.draft.clone();
    let current = editor.draft.hazard(0).unwrap().fields();
    let mut edited = current;
    edited.width = 120.0; // grows around its center into the spawn point
    apply_fields(editor, &mut history, 0, current, edited);
    assert_eq!(editor.draft, original);
    assert!(
        editor.message.contains("spawn must not be inside hazard 0"),
        "{}",
        editor.message
    );
    assert!(!editor.dirty && !history.can_undo());
}

#[test]
fn dragging_the_damage_field_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_hazard().unwrap();
    state.selected = Some(Selection::Hazard(0));
    let original = damage(editor);
    let (ctx, _) = ready(editor, &mut state);
    let field: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("hazard field", "Damage"))))
        .expect("the damage field was laid out");
    let start = field.center();
    let button = |pressed, pos| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    pass(
        &ctx,
        vec![egui::Event::PointerMoved(start)],
        editor,
        &mut state,
    );
    pass(&ctx, vec![button(true, start)], editor, &mut state);
    for i in 1..=4 {
        let to = start + egui::vec2(15.0 * i as f32, 0.0);
        pass(
            &ctx,
            vec![egui::Event::PointerMoved(to)],
            editor,
            &mut state,
        );
    }
    assert!(state.history.gesture_open(), "the drag is open mid-way");
    assert!(!state.history.can_undo(), "undo waits for the drag to end");
    let end = start + egui::vec2(60.0, 0.0);
    pass(&ctx, vec![button(false, end)], editor, &mut state);
    pass(&ctx, vec![], editor, &mut state);

    assert!(
        damage(editor) > original,
        "dragging right raised the damage"
    );
    assert!(!state.history.gesture_open());
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(damage(editor), original, "one undo undoes the whole drag");
    assert!(!state.history.can_undo(), "the drag was a single step");
}

#[test]
fn choosing_a_shape_is_one_undoable_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = with_a_selected_hazard(editor, &mut state);
    let original = editor.draft.clone();
    editor.dirty = false;

    click_id(
        &ctx,
        egui::Id::new(("hazard pick", "Shape")),
        editor,
        &mut state,
    );
    click_id(&ctx, egui::Id::new(("choice", "Lava")), editor, &mut state);
    assert_eq!(editor.draft.hazard(0).unwrap().style, HazardStyle::Lava);
    assert!(editor.dirty);

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo(), "one choice, one step");
}

#[test]
fn choosing_the_shape_already_chosen_changes_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = with_a_selected_hazard(editor, &mut state);
    editor.dirty = false;
    click_id(
        &ctx,
        egui::Id::new(("hazard pick", "Shape")),
        editor,
        &mut state,
    );
    click_id(
        &ctx,
        egui::Id::new(("choice", "Spikes")),
        editor,
        &mut state,
    );
    assert!(!editor.dirty && !state.history.can_undo());
}

#[test]
fn choosing_a_facing_is_one_undoable_step_and_keeps_the_rest() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = with_a_selected_hazard(editor, &mut state);
    let original = editor.draft.clone();

    click_id(
        &ctx,
        egui::Id::new(("hazard pick", "Facing")),
        editor,
        &mut state,
    );
    click_id(&ctx, egui::Id::new(("choice", "Left")), editor, &mut state);
    let now = editor.draft.hazard(0).unwrap();
    assert_eq!(now.facing, HazardFacing::Left);
    assert_eq!(
        (now.rect, now.damage, now.style, now.color),
        {
            let before = original.hazard(0).unwrap();
            (before.rect, before.damage, before.style, before.color)
        },
        "only the facing changed"
    );
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo());
}

#[test]
fn reset_is_disabled_until_the_hazard_has_its_own_color_then_clears_it() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = with_a_selected_hazard(editor, &mut state);
    editor.dirty = false;

    // No color of its own: the disabled Reset does nothing.
    click_id(
        &ctx,
        egui::Id::new(("hazard color reset",)),
        editor,
        &mut state,
    );
    assert!(!editor.dirty && !state.history.can_undo());

    let mut tinted = editor.draft.hazard(0).unwrap().clone();
    tinted.color = Some((0.9, 0.1, 0.1));
    editor.draft.replace_hazard(0, tinted).unwrap();
    editor.dirty = false;
    let original = editor.draft.clone();
    for _ in 0..2 {
        pass(&ctx, vec![], editor, &mut state);
    }
    click_id(
        &ctx,
        egui::Id::new(("hazard color reset",)),
        editor,
        &mut state,
    );
    assert_eq!(editor.draft.hazard(0).unwrap().color, None);
    assert!(editor.dirty);

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo(), "one reset, one step");
}

#[test]
fn opening_the_color_picker_is_not_an_edit() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = with_a_selected_hazard(editor, &mut state);
    editor.dirty = false;
    click_id(&ctx, egui::Id::new(("hazard color",)), editor, &mut state);
    for _ in 0..3 {
        pass(&ctx, vec![], editor, &mut state);
    }
    assert_eq!(
        editor.draft.hazard(0).unwrap().color,
        None,
        "still the shape's color"
    );
    assert!(!editor.dirty && !state.history.can_undo() && !state.history.gesture_open());
}

#[test]
fn a_picked_color_is_one_undo_step_and_a_bad_one_is_refused() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_hazard().unwrap();
    let original = editor.draft.clone();
    let current = editor.draft.hazard(0).unwrap().fields();
    let mut edited = current;
    edited.color = Some((0.2, 0.4, 0.6));
    apply_fields(editor, &mut state.history, 0, current, edited);
    assert_eq!(editor.draft.hazard(0).unwrap().color, Some((0.2, 0.4, 0.6)));
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo());

    let mut bad = current;
    bad.color = Some((1.5, 0.0, 0.0));
    let mut history = History::default();
    apply_fields(editor, &mut history, 0, current, bad);
    assert_eq!(editor.draft, original, "the draft is unchanged");
    assert!(editor.message.contains("color"), "{}", editor.message);
    assert!(!history.can_undo());
}
