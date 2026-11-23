use super::super::test_support::{drawn_text, editing_game as editor};
use super::*;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::room::BossPlacement;

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

fn button(pressed: bool, pos: egui::Pos2) -> egui::Event {
    egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

fn field(ctx: &egui::Context, name: &str) -> egui::Rect {
    ctx.data(|data| data.get_temp(egui::Id::new(("placement field", name))))
        .unwrap_or_else(|| panic!("the {name} field was not laid out"))
}

/// Presses on a field, drags `dx` pixels to the right in four moves and lets go.
fn drag(
    ctx: &egui::Context,
    name: &str,
    dx: f32,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) {
    let start = field(ctx, name).center();
    pass(ctx, vec![egui::Event::PointerMoved(start)], editor, state);
    pass(ctx, vec![button(true, start)], editor, state);
    for i in 1..=4 {
        let to = start + egui::vec2(dx * i as f32 / 4.0, 0.0);
        pass(ctx, vec![egui::Event::PointerMoved(to)], editor, state);
    }
    let end = start + egui::vec2(dx, 0.0);
    pass(ctx, vec![button(false, end)], editor, state);
    pass(ctx, vec![], editor, state);
}

/// A click on a field.
fn click(ctx: &egui::Context, name: &str, editor: &mut EditorSession, state: &mut EditorUi) {
    let at = field(ctx, name).center();
    pass(ctx, vec![egui::Event::PointerMoved(at)], editor, state);
    pass(ctx, vec![button(true, at)], editor, state);
    pass(ctx, vec![button(false, at)], editor, state);
    pass(ctx, vec![], editor, state);
}

fn selecting(index: usize) -> EditorUi {
    EditorUi {
        selected: Some(Selection::Enemy(index)),
        ..Default::default()
    }
}

#[test]
fn the_rows_show_for_a_selected_enemy_only() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.draft.boss = Some(BossPlacement::new("Ash Warden", (560.0, 436.0)));
    let mut state = selecting(0);
    let (_, text) = ready(editor, &mut state);
    assert!(
        text.contains("Group") && text.contains("Chance %"),
        "{text}"
    );
    assert!(text.contains("Min tier"), "next to Min tier:\n{text}");

    for selection in [
        None,
        Some(Selection::Boss),
        Some(Selection::Spawn),
        Some(Selection::Enemy(99)),
    ] {
        state.selected = selection;
        let (_, text) = ready(editor, &mut state);
        assert!(
            !text.contains("Group") && !text.contains("Chance %"),
            "{selection:?}:\n{text}"
        );
    }
}

#[test]
fn dragging_chance_sets_it_marks_the_room_unsaved_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting(0);
    let before = editor.draft.clone();
    assert_eq!(before.enemy_chance(0), Some(100));
    assert!(!editor.dirty && !editor.has_unsaved());
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, "Chance %", -60.0, editor, &mut state);

    let chance = editor.draft.enemy_chance(0).unwrap();
    assert!(chance < 100, "dragging left lowered it to {chance}");
    assert!(editor.dirty, "the unsaved guard sees the edit");
    assert!(editor.has_unsaved());
    assert!(!state.history.gesture_open());

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, before, "one undo undoes the whole drag");
    assert!(!state.history.can_undo(), "the drag was a single step");
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Redo);
    assert_eq!(editor.draft.enemy_chance(0), Some(chance));
}

#[test]
fn chance_stops_at_100_and_at_0() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting(0);
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, "Chance %", -5000.0, editor, &mut state);
    assert_eq!(editor.draft.enemy_chance(0), Some(0));
    assert!(editor.message.is_empty(), "{}", editor.message);
    let (ctx, _) = ready(editor, &mut state);
    drag(&ctx, "Chance %", 5000.0, editor, &mut state);
    assert_eq!(editor.draft.enemy_chance(0), Some(100));
}

#[test]
fn unticking_none_puts_the_enemy_in_group_1_and_ticking_it_again_takes_it_out_each_one_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting(0);
    let before = editor.draft.clone();
    let (ctx, text) = ready(editor, &mut state);
    assert!(text.contains("None"), "{text}");

    click(&ctx, "Group none", editor, &mut state);
    assert_eq!(editor.draft.enemy_group(0), Some(Some(1)));
    assert!(editor.dirty && editor.has_unsaved());

    let (ctx, _) = ready(editor, &mut state);
    click(&ctx, "Group none", editor, &mut state);
    assert_eq!(editor.draft.enemy_group(0), Some(None));

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(
        editor.draft.enemy_group(0),
        Some(Some(1)),
        "the second step"
    );
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, before, "the first step");
    assert!(!state.history.can_undo());
}

#[test]
fn dragging_the_group_box_changes_the_group_in_one_step_and_stays_within_1_to_9() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_enemy_group(0, Some(1)).unwrap();
    let mut state = selecting(0);
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, "Group", 60.0, editor, &mut state);
    let group = editor.draft.enemy_group(0).unwrap().unwrap();
    assert!(
        (2..=9).contains(&group),
        "dragging right raised it to {group}"
    );
    assert!(editor.dirty);
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, before, "one undo undoes the whole drag");
    assert!(!state.history.can_undo());

    // Undo drops the selection (`apply_history`), so pick the enemy again.
    state.selected = Some(Selection::Enemy(0));
    let (ctx, _) = ready(editor, &mut state);
    drag(&ctx, "Group", 5000.0, editor, &mut state);
    assert_eq!(editor.draft.enemy_group(0), Some(Some(9)));
    let (ctx, _) = ready(editor, &mut state);
    drag(&ctx, "Group", -5000.0, editor, &mut state);
    assert_eq!(
        editor.draft.enemy_group(0),
        Some(Some(1)),
        "never 0: None is the checkbox"
    );
}

#[test]
fn the_group_box_cannot_be_dragged_while_the_enemy_is_in_no_group() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = selecting(0);
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    drag(&ctx, "Group", 60.0, editor, &mut state);
    assert_eq!(editor.draft, before);
    assert!(!state.history.can_undo());
}

#[test]
fn a_change_the_rules_refuse_leaves_the_draft_and_the_history_alone_and_says_why() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_enemy_chance(0, 40).unwrap();
    let mut state = selecting(0);
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    click(&ctx, "Group none", editor, &mut state);
    assert_eq!(editor.draft, before, "a chance of 40 cannot be in a group");
    assert!(editor.message.contains("chance 40"), "{}", editor.message);
    assert!(!state.history.can_undo());
    assert!(!editor.dirty);

    // And the other way round: an enemy in a group keeps chance 100.
    editor.draft.set_enemy_chance(0, 100).unwrap();
    editor.draft.set_enemy_group(0, Some(2)).unwrap();
    let grouped = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);
    drag(&ctx, "Chance %", -60.0, editor, &mut state);
    assert_eq!(editor.draft, grouped);
    assert!(editor.message.contains("group 2"), "{}", editor.message);
    assert!(!state.history.can_undo());
}

#[test]
fn two_enemies_of_one_definition_have_their_own_group_and_chance() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let twin = editor.draft.enemies[0].clone();
    editor.draft.enemies.push(twin);
    let mut state = selecting(1);
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, "Chance %", -60.0, editor, &mut state);

    assert_eq!(editor.draft.enemy_chance(0), Some(100));
    assert!(editor.draft.enemy_chance(1).unwrap() < 100);
    assert_eq!(
        editor.draft.enemies[0].definition, editor.draft.enemies[1].definition,
        "group and chance are the placement's own, not the definition's"
    );
}
