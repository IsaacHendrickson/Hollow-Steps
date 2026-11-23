use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use crate::editor_panel::Selection;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::item::ITEM_SIZE;
use hollow_steps::piece::PickupSlot;

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

fn click_panel(ctx: &egui::Context, label: &str, editor: &mut EditorSession, state: &mut EditorUi) {
    click_id(ctx, egui::Id::new(("button", label)), editor, state);
}

fn field(name: &str) -> egui::Id {
    egui::Id::new(("piece field", name))
}

fn choose(
    ctx: &egui::Context,
    dropdown: &str,
    option: &str,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    click_id(ctx, egui::Id::new(("piece pick", dropdown)), editor, state);
    click_id(ctx, egui::Id::new(("choice", option)), editor, state);
}

fn undo(editor: &mut EditorSession, state: &mut EditorUi) {
    crate::editor_panel::apply_history(editor, state, HistoryKey::Undo);
}

fn marked(editor: &mut EditorSession, state: &mut EditorUi) -> egui::Context {
    editor
        .draft
        .set_piece(Some(PieceInfo::new(PieceRole::Middle)))
        .unwrap();
    ready(editor, state).0
}

// ---- the mark ----

#[test]
fn the_panel_offers_the_piece_section_and_shows_role_and_weight_only_for_a_piece() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, text) = ready(editor, &mut state);
    for expected in [
        "Dungeon piece",
        "This room is a piece",
        "Add pickup slot",
        "0 of 8 pickup slots",
        "Teal ring: pickup slot (K key, A ability, ? either)",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    assert!(!text.contains("Weight"), "an unmarked room has no weight");

    editor
        .draft
        .set_piece(Some(PieceInfo::new(PieceRole::Middle)))
        .unwrap();
    let text = pass(&ctx, vec![], editor, &mut state);
    for expected in ["Role", "Middle", "Weight"] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
}

#[test]
fn ticking_the_box_marks_the_room_and_unticking_takes_the_mark_off_each_with_undo() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    assert!(!editor.dirty);

    click_id(&ctx, field("mark"), editor, &mut state);
    assert_eq!(
        editor.draft.layout.piece,
        Some(PieceInfo::new(PieceRole::Middle))
    );
    assert!(editor.dirty);

    click_id(&ctx, field("mark"), editor, &mut state);
    assert_eq!(editor.draft.layout.piece, None);

    undo(editor, &mut state);
    assert_eq!(
        editor.draft.layout.piece,
        Some(PieceInfo::new(PieceRole::Middle))
    );
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo(), "two clicks, two steps");
}

#[test]
fn choosing_a_role_is_one_undo_step_and_keeps_the_weight() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor
        .draft
        .set_piece(Some(PieceInfo {
            role: PieceRole::Start,
            weight: 6,
        }))
        .unwrap();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    editor.dirty = false;

    choose(&ctx, "Role", "Boss", editor, &mut state);
    assert_eq!(
        editor.draft.layout.piece,
        Some(PieceInfo {
            role: PieceRole::Boss,
            weight: 6
        })
    );
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
}

#[test]
fn choosing_the_role_already_chosen_records_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = marked(editor, &mut state);
    choose(&ctx, "Role", "Middle", editor, &mut state);
    assert!(!state.history.can_undo());
}

#[test]
fn dragging_the_weight_is_one_undo_step_and_stays_in_range() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = marked(editor, &mut state);
    let weight = |editor: &EditorSession| editor.draft.layout.piece.unwrap().weight;
    let original = weight(editor);
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(field("Weight")))
        .expect("the weight box was laid out");
    let start = rect.center();
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
    for i in 1..=6 {
        let to = start + egui::vec2(40.0 * i as f32, 0.0);
        pass(
            &ctx,
            vec![egui::Event::PointerMoved(to)],
            editor,
            &mut state,
        );
    }
    assert!(state.history.gesture_open(), "the drag is open mid-way");
    assert!(!state.history.can_undo(), "undo waits for the drag to end");
    let end = start + egui::vec2(240.0, 0.0);
    pass(&ctx, vec![button(false, end)], editor, &mut state);
    pass(&ctx, vec![], editor, &mut state);

    assert!(
        weight(editor) > original,
        "dragging right raised the weight"
    );
    assert!(weight(editor) <= MAX_WEIGHT, "never past 10");
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(weight(editor), original, "one undo undoes the whole drag");
    assert!(!state.history.can_undo(), "the drag was a single step");
}

// ---- slots ----

#[test]
fn add_pickup_slot_adds_one_selects_it_marks_unsaved_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    click_panel(&ctx, "Add pickup slot", editor, &mut state);
    assert_eq!(editor.draft.layout.slots.len(), 1);
    assert_eq!(state.selected, Some(Selection::Slot(0)));
    assert!(editor.dirty);
    assert_eq!(state.tool, EditorTool::Select);
    assert!(
        editor.draft.layout.piece.is_none(),
        "a slot does not mark the room"
    );

    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(text.contains("1 of 8 pickup slots"), "{text}");
    assert!(
        text.contains("Not a piece"),
        "slots of an unmarked room say so: {text}"
    );

    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
}

#[test]
fn a_refused_add_says_why_and_records_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    for _ in 0..MAX_SLOTS {
        editor.draft.add_slot().unwrap();
    }
    let (ctx, _) = ready(editor, &mut state);
    let before = editor.draft.clone();

    click_panel(&ctx, "Add pickup slot", editor, &mut state);
    assert!(editor.message.contains("at most 8"), "{}", editor.message);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
    assert!(!editor.dirty);
}

#[test]
fn the_slot_panel_shows_for_a_selected_slot_only() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_slot().unwrap();
    let (ctx, text) = ready(editor, &mut state);
    assert!(
        !text.contains("Pickup slot\n") && !text.contains("Kind"),
        "{text}"
    );
    state.selected = Some(Selection::Slot(0));
    let text = pass(&ctx, vec![], editor, &mut state);
    for expected in ["Kind", "Any", "Delete selected removes it"] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    state.selected = Some(Selection::Exit(0));
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(!text.contains("Kind"), "{text}");
}

#[test]
fn choosing_a_kind_is_one_undo_step_and_changes_only_that_slot() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_slot().unwrap();
    editor.draft.add_slot().unwrap();
    state.selected = Some(Selection::Slot(1));
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    editor.dirty = false;

    choose(&ctx, "Kind", "Ability", editor, &mut state);
    assert_eq!(editor.draft.slot(1).unwrap().kind, SlotKind::Ability);
    assert_eq!(editor.draft.slot(0).unwrap().kind, SlotKind::Any);
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
}

#[test]
fn delete_selected_removes_the_selected_slot_in_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_slot().unwrap();
    editor.draft.add_slot().unwrap();
    state.selected = Some(Selection::Slot(0));
    let before = editor.draft.clone();
    let kept = editor.draft.slot(1).copied().unwrap();
    let (ctx, _) = ready(editor, &mut state);
    editor.dirty = false;

    click_panel(&ctx, "Delete selected", editor, &mut state);
    assert_eq!(editor.draft.layout.slots, vec![kept]);
    assert_eq!(state.selected, None);
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
}

#[test]
fn dragging_a_slot_on_the_canvas_moves_it_as_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_slot().unwrap();
    let before = editor.draft.clone();
    // What the canvas does for a drag: a gesture around the moves.
    state.history.begin_gesture(&editor.draft);
    for x in [400.0, 450.0, 500.0] {
        editor
            .drag_selection(Selection::Slot(0), macroquad::math::vec2(x, 300.0))
            .unwrap();
    }
    state.history.end_gesture(&editor.draft);
    assert_eq!(editor.draft.slot(0).unwrap().position, (500.0, 300.0));
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
}

// ---- the canvas marker ----

#[test]
fn a_slot_has_a_teal_marker_the_size_of_an_item_that_a_press_selects() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor
        .draft
        .layout
        .slots
        .push(PickupSlot::new((300.0, 450.0), SlotKind::Key));
    let markers = crate::editor_ui::editor_markers(editor, None, None);
    let (selection, rect, _) = markers
        .iter()
        .find(|(selection, _, _)| matches!(selection, Selection::Slot(_)))
        .expect("the slot has a marker");
    assert_eq!(*selection, Selection::Slot(0));
    assert_eq!(rect.center(), macroquad::math::vec2(300.0, 450.0));
    assert_eq!(rect.size(), ITEM_SIZE);
    assert_eq!(
        crate::editor_ui::selected_by_press(*selection),
        Selection::Slot(0)
    );
    // Not while an arrival view is open: only that arrival is shown there.
    assert!(crate::editor_ui::editor_markers(editor, Some(0), None)
        .iter()
        .all(|(selection, _, _)| !matches!(selection, Selection::Slot(_))));
}
