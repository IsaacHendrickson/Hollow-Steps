use super::super::history_panel::apply_history;
use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use crate::editor_panel::Selection;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::gates::Gate;
use hollow_steps::room::{RoomEdge, RoomId};

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

fn click_on(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut EditorUi) {
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

fn tick(ctx: &egui::Context, row: usize, editor: &mut EditorSession, state: &mut EditorUi) {
    click_on(ctx, egui::Id::new(("gate", row, "Dash")), editor, state);
}

fn undo(editor: &mut EditorSession, state: &mut EditorUi) {
    apply_history(editor, state, HistoryKey::Undo);
}

fn selecting_the_edge_exit() -> EditorUi {
    EditorUi {
        selected: Some(Selection::Exit(0)),
        ..EditorUi::default()
    }
}

#[test]
fn a_selected_exit_lists_every_entrance_with_an_ability_checkbox_each() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let (_, text) = ready(editor, &mut EditorUi::default());
    assert!(
        !text.contains("Reaching this exit"),
        "no exit selected: {text}"
    );

    let mut state = selecting_the_edge_exit();
    let (_, text) = ready(editor, &mut state);
    assert!(text.contains("Reaching this exit"), "{text}");
    assert!(text.contains("Room spawn"), "{text}");
    assert!(text.contains("Arrival from Second"), "{text}");
    assert_eq!(text.matches("Needs Dash").count(), 2, "{text}");
    assert!(!editor.dirty, "showing the gates is not an edit");
    assert!(!state.history.can_undo());
}

#[test]
fn ticking_the_spawn_marks_the_exit_the_room_unsaved_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    tick(&ctx, 0, editor, &mut state);

    let exit = ExitRef::of(&editor.draft.layout.exits[0]);
    assert_eq!(
        editor.draft.layout.gate_needs(&Entrance::Spawn, &exit),
        [Ability::Dash]
    );
    assert_eq!(editor.draft.layout.gates.len(), 1, "only that entrance");
    assert!(editor.dirty);
    assert!(state.history.can_undo());
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo(), "one step");
}

#[test]
fn ticking_an_arrival_marks_that_arrival_only_and_unticking_frees_it_again() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let arrival = editor.gate_entrances()[1].entrance;
    let (ctx, _) = ready(editor, &mut state);

    tick(&ctx, 1, editor, &mut state);
    let exit = ExitRef::of(&editor.draft.layout.exits[0]);
    assert_eq!(
        editor.draft.layout.gate_needs(&arrival, &exit),
        [Ability::Dash]
    );
    assert!(editor
        .draft
        .layout
        .gate_needs(&Entrance::Spawn, &exit)
        .is_empty());

    tick(&ctx, 1, editor, &mut state);
    assert!(
        editor.draft.layout.gates.is_empty(),
        "the last ability cleared removes the gate"
    );
    undo(editor, &mut state);
    assert_eq!(editor.draft.layout.gates.len(), 1);
}

#[test]
fn a_gate_naming_no_entrance_is_shown_as_a_problem_and_can_be_removed() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (_, text) = ready(editor, &mut state);
    assert!(!text.contains("Gate problems"), "{text}");

    let exit = ExitRef::of(&editor.draft.layout.exits[0]);
    editor.draft.layout.gates.push(Gate {
        from: Entrance::Arrival {
            room: RoomId::Second,
            exit: ExitRef::Edge {
                edge: RoomEdge::Left,
                offset: 7.0,
            },
        },
        exit,
        needs: vec![Ability::Dash],
    });
    let (ctx, text) = ready(editor, &mut state);
    assert!(text.contains("Gate problems"), "{text}");
    assert!(text.contains("no longer exists"), "{text}");
    assert!(text.contains("left edge exit at 7"), "{text}");
    assert!(!editor.dirty, "showing the problem is not an edit");

    click_on(
        &ctx,
        egui::Id::new(("button", "Remove gate 1")),
        editor,
        &mut state,
    );

    assert!(editor.draft.layout.gates.is_empty());
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(
        editor.draft.layout.gates.len(),
        1,
        "one undo step brings it back"
    );
}

fn tick_double_jump(
    ctx: &egui::Context,
    row: usize,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    click_on(
        ctx,
        egui::Id::new(("gate", row, "Double Jump")),
        editor,
        state,
    );
}

#[test]
fn every_entrance_has_a_checkbox_for_the_double_jump_beside_the_dash() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let (_, text) = ready(editor, &mut state);
    assert_eq!(text.matches("Needs Double Jump").count(), 2, "{text}");
    assert_eq!(text.matches("Needs Dash").count(), 2, "{text}");
}

#[test]
fn ticking_the_double_jump_marks_that_ability_alone_in_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    tick_double_jump(&ctx, 0, editor, &mut state);

    let exit = ExitRef::of(&editor.draft.layout.exits[0]);
    assert_eq!(
        editor.draft.layout.gate_needs(&Entrance::Spawn, &exit),
        [Ability::DoubleJump],
        "the Dash is not ticked along with it"
    );
    assert!(editor.dirty);
    undo(editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo(), "one step");
}

#[test]
fn ticking_both_abilities_marks_both_and_clearing_one_leaves_the_other() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting_the_edge_exit();
    let (ctx, _) = ready(editor, &mut state);

    tick_double_jump(&ctx, 0, editor, &mut state);
    tick(&ctx, 0, editor, &mut state);
    let exit = ExitRef::of(&editor.draft.layout.exits[0]);
    assert_eq!(
        editor.draft.layout.gate_needs(&Entrance::Spawn, &exit),
        [Ability::Dash, Ability::DoubleJump]
    );

    tick(&ctx, 0, editor, &mut state);
    assert_eq!(
        editor.draft.layout.gate_needs(&Entrance::Spawn, &exit),
        [Ability::DoubleJump]
    );
}
