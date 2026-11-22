use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use hollow_steps::editor::RoomDraft;
use hollow_steps::editor_history::HistoryKey;

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

fn settle(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) {
    for _ in 0..2 {
        pass(ctx, vec![], editor, state);
    }
}

fn click_panel(ctx: &egui::Context, label: &str, editor: &mut EditorSession, state: &mut EditorUi) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("button", label))))
        .unwrap_or_else(|| panic!("no {label} button"));
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    settle(ctx, editor, state);
}

/// Clicks whatever was last laid out under `id` (a probe) and lets the result settle.
fn click_id(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut EditorUi) {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"));
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    settle(ctx, editor, state);
}

/// A drag across a field, laid out under `id`, from its middle by `distance` pixels to the right.
fn drag_field(
    ctx: &egui::Context,
    id: egui::Id,
    distance: f32,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    let field: egui::Rect = ctx
        .data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"));
    let start = field.center();
    let button = |pressed, pos| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    };
    pass(ctx, vec![egui::Event::PointerMoved(start)], editor, state);
    pass(ctx, vec![button(true, start)], editor, state);
    for i in 1..=4 {
        let to = start + egui::vec2(distance / 4.0 * i as f32, 0.0);
        pass(ctx, vec![egui::Event::PointerMoved(to)], editor, state);
    }
    assert!(state.history.gesture_open(), "the drag is open mid-way");
    assert!(!state.history.can_undo(), "undo waits for the drag to end");
    let end = start + egui::vec2(distance, 0.0);
    pass(ctx, vec![button(false, end)], editor, state);
    pass(ctx, vec![], editor, state);
}

fn field(label: &str) -> egui::Id {
    egui::Id::new(("puzzle field", label))
}

#[test]
fn the_panel_offers_the_three_buttons_and_explains_the_markers_once_the_room_has_pieces() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, text) = ready(editor, &mut state);
    for expected in ["Add breakable wall", "Add switch", "Add barrier"] {
        assert!(text.contains(expected), "missing {expected:?}");
    }
    assert!(text.contains("Magenta: hazard (hurts, knocks back)"));
    // The legend appears with the first piece: the panel has no room for it before that.
    assert!(!text.contains("Brown:"), "no piece, no legend");
    assert!(!text.contains("Piece width"), "no piece is selected");

    click_panel(&ctx, "Add breakable wall", editor, &mut state);
    state.selected = None;
    let text = pass(&ctx, vec![], editor, &mut state);
    for expected in [
        "Brown: breakable wall",
        "Teal: switch",
        "Slate: barrier",
        "Magenta: hazard (hurts, knocks back)",
    ] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    assert!(!text.contains("Piece width"), "nothing is selected");
}

#[test]
fn each_add_button_puts_one_piece_in_the_room_selects_it_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    click_panel(&ctx, "Add breakable wall", editor, &mut state);
    assert_eq!(editor.draft.layout.breakables.len(), 1);
    assert_eq!(state.selected, Some(Selection::Breakable(0)));
    assert!(editor.dirty);
    assert_eq!(state.tool, EditorTool::Select);

    click_panel(&ctx, "Add switch", editor, &mut state);
    assert_eq!(state.selected, Some(Selection::Switch(0)));
    click_panel(&ctx, "Add barrier", editor, &mut state);
    assert_eq!(state.selected, Some(Selection::Barrier(0)));
    assert_eq!(editor.draft.layout.switches.len(), 1);
    assert_eq!(editor.draft.layout.barriers.len(), 1);

    for _ in 0..3 {
        click_panel(&ctx, "Undo", editor, &mut state);
    }
    assert_eq!(editor.draft, before, "three buttons, three steps");
    assert!(!state.history.can_undo());
}

#[test]
fn add_barrier_without_a_switch_says_why_and_changes_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    click_panel(&ctx, "Add barrier", editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(editor.message.contains("switch"), "{}", editor.message);
    assert!(!editor.dirty && !state.history.can_undo());
    assert_eq!(state.selected, None);
}

#[test]
fn the_fields_follow_the_kind_of_piece_selected() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, _) = ready(editor, &mut state);
    click_panel(&ctx, "Add breakable wall", editor, &mut state);
    let text = pass(&ctx, vec![], editor, &mut state);
    for expected in ["Breakable wall", "Piece width", "Piece height", "Hits"] {
        assert!(text.contains(expected), "missing {expected:?} in {text}");
    }
    assert!(!text.contains("Switch number") && !text.contains("Opened by switch"));

    click_panel(&ctx, "Add switch", editor, &mut state);
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(
        text.contains("Switch number") && text.contains("Piece width"),
        "{text}"
    );
    assert!(!text.contains("Hits"), "{text}");

    click_panel(&ctx, "Add barrier", editor, &mut state);
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(
        text.contains("Opened by switch") && text.contains("Switch 1"),
        "{text}"
    );
    assert!(
        !text.contains("Switch number") && !text.contains("Hits"),
        "{text}"
    );

    state.selected = Some(Selection::Exit(0));
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(!text.contains("Piece width"), "{text}");
}

#[test]
fn dragging_the_hits_field_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_breakable().unwrap();
    state.selected = Some(Selection::Breakable(0));
    let original = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    drag_field(&ctx, field("Hits"), 60.0, editor, &mut state);

    let hits = editor.draft.layout.breakables[0].hits;
    assert!(
        hits > 3 && hits <= MAX_BREAKABLE_HITS,
        "dragging right raised the hits: {hits}"
    );
    assert!(editor.dirty && !state.history.gesture_open());
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original, "one undo undoes the whole drag");
    assert!(!state.history.can_undo(), "the drag was a single step");
}

#[test]
fn dragging_the_height_resizes_the_piece_in_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    state.selected = Some(Selection::Barrier(0));
    let original = editor.draft.clone();
    let floor = editor.draft.layout.floor_y;
    let (ctx, _) = ready(editor, &mut state);

    drag_field(&ctx, field("Piece height"), 60.0, editor, &mut state);

    let (_, y, _, h) = editor.draft.layout.barriers[0].rect;
    assert!(h > 96.0, "dragging right made it taller: {h}");
    assert_eq!(y + h, floor, "it still stands on the floor");
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo());
}

#[test]
fn choosing_a_switch_for_a_barrier_is_one_undoable_step_and_lists_the_rooms_switches() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_switch().unwrap();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    state.selected = Some(Selection::Barrier(0));
    let original = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    editor.dirty = false;

    click_id(
        &ctx,
        egui::Id::new(("puzzle pick", "Opened by switch")),
        editor,
        &mut state,
    );
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(
        text.contains("Switch 1") && text.contains("Switch 2"),
        "{text}"
    );
    assert!(
        !text.contains("Switch 3"),
        "only the room's switches: {text}"
    );
    click_id(
        &ctx,
        egui::Id::new(("choice", "Switch 2")),
        editor,
        &mut state,
    );

    assert_eq!(editor.draft.layout.barriers[0].switch, 2);
    assert!(editor.dirty);
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, original);
    assert!(!state.history.can_undo(), "one choice, one step");
}

#[test]
fn choosing_the_switch_already_chosen_changes_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    state.selected = Some(Selection::Barrier(0));
    let (ctx, _) = ready(editor, &mut state);
    editor.dirty = false;
    click_id(
        &ctx,
        egui::Id::new(("puzzle pick", "Opened by switch")),
        editor,
        &mut state,
    );
    click_id(
        &ctx,
        egui::Id::new(("choice", "Switch 1")),
        editor,
        &mut state,
    );
    assert!(!editor.dirty && !state.history.can_undo());
}

#[test]
fn a_refused_edit_leaves_the_draft_says_why_and_records_no_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    let original = editor.draft.clone();
    let current = editor.draft.piece_fields(Selection::Switch(0)).unwrap();
    // The barrier uses switch 1, so switch 1 cannot be renumbered.
    let edited = PieceFields {
        number: 4,
        ..current
    };
    apply_fields(
        editor,
        &mut state.history,
        Selection::Switch(0),
        current,
        edited,
    );
    assert_eq!(editor.draft, original);
    assert!(editor.message.contains("barrier 0"), "{}", editor.message);
    assert!(!editor.dirty && !state.history.can_undo());
}

#[test]
fn an_unchanged_edit_does_nothing() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    editor.draft.add_breakable().unwrap();
    let current = editor.draft.piece_fields(Selection::Breakable(0)).unwrap();
    apply_fields(
        editor,
        &mut history,
        Selection::Breakable(0),
        current,
        current,
    );
    assert!(!editor.dirty && !history.can_undo());
}

#[test]
fn the_delete_key_and_button_remove_the_selected_piece_as_one_undo_step() {
    let pieces: [(Selection, Add); 3] = [
        (Selection::Breakable(0), RoomDraft::add_breakable),
        (Selection::Switch(0), RoomDraft::add_switch),
        (Selection::Barrier(0), |draft| {
            draft.add_switch()?;
            draft.add_barrier()
        }),
    ];
    for (selection, add) in pieces {
        let mut game = editor();
        let editor = game.editor_mut().unwrap();
        let mut state = EditorUi::default();
        add(&mut editor.draft).unwrap();
        editor.dirty = false;
        let original = editor.draft.clone();
        state.selected = Some(selection);

        crate::terrain_canvas::delete_selected(&mut state, editor);

        assert_eq!(state.selected, None, "{selection:?}");
        assert!(editor.dirty, "{selection:?}");
        assert_ne!(editor.draft, original, "{selection:?} is gone");
        assert_eq!(editor.draft.piece_fields(selection), None, "{selection:?}");
        crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
        assert_eq!(
            editor.draft, original,
            "{selection:?}: one undo brings it back"
        );
        assert!(!state.history.can_undo(), "{selection:?}: one step");
    }
}

#[test]
fn deleting_a_switch_a_barrier_needs_is_refused_and_names_the_barrier() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    editor.dirty = false;
    let original = editor.draft.clone();
    state.selected = Some(Selection::Switch(0));

    crate::terrain_canvas::delete_selected(&mut state, editor);

    assert_eq!(editor.draft, original, "the switch is still there");
    assert!(editor.message.contains("barrier 0"), "{}", editor.message);
    assert_eq!(
        state.selected,
        Some(Selection::Switch(0)),
        "and still selected"
    );
    assert!(!editor.dirty && !state.history.can_undo());
}
