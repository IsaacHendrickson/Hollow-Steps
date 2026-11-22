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

/// Presses on the Min tier field, drags `dx` pixels to the right in four moves and lets go.
fn drag(
    ctx: &egui::Context,
    dx: f32,
    editor: &mut EditorSession,
    state: &mut EditorUi,
) -> egui::Pos2 {
    let field: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("placement field", "Min tier"))))
        .expect("the Min tier field was laid out");
    let start = field.center();
    pass(ctx, vec![egui::Event::PointerMoved(start)], editor, state);
    pass(ctx, vec![button(true, start)], editor, state);
    for i in 1..=4 {
        let to = start + egui::vec2(dx * i as f32 / 4.0, 0.0);
        pass(ctx, vec![egui::Event::PointerMoved(to)], editor, state);
    }
    let end = start + egui::vec2(dx, 0.0);
    pass(ctx, vec![button(false, end)], editor, state);
    pass(ctx, vec![], editor, state);
    end
}

fn with_boss(editor: &mut EditorSession) {
    editor.draft.boss = Some(BossPlacement::new("Ash Warden", (560.0, 436.0)));
}

#[test]
fn the_field_shows_for_a_selected_enemy_and_for_the_boss() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_boss(editor);
    let mut state = EditorUi {
        selected: Some(Selection::Enemy(0)),
        ..Default::default()
    };
    let (_, text) = ready(editor, &mut state);
    assert!(text.contains("Min tier"), "{text}");

    state.selected = Some(Selection::Boss);
    let (_, text) = ready(editor, &mut state);
    assert!(text.contains("Min tier"), "{text}");
}

#[test]
fn the_field_is_not_there_for_anything_else_or_for_a_missing_placement() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (_, text) = ready(editor, &mut state);
    assert!(!text.contains("Min tier"), "nothing selected:\n{text}");

    for selection in [Selection::Spawn, Selection::Exit(0), Selection::Boss] {
        state.selected = Some(selection);
        let (_, text) = ready(editor, &mut state);
        assert!(
            !text.contains("Min tier"),
            "{selection:?} (the room has no boss):\n{text}"
        );
    }
    state.selected = Some(Selection::Enemy(99));
    let (_, text) = ready(editor, &mut state);
    assert!(!text.contains("Min tier"), "{text}");
}

#[test]
fn dragging_the_field_sets_the_enemy_min_tier_marks_it_unsaved_and_is_one_undo_step() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi {
        selected: Some(Selection::Enemy(0)),
        ..Default::default()
    };
    let before = editor.draft.clone();
    assert_eq!(before.enemy_min_tier(0), Some(0));
    assert!(!editor.dirty && !editor.has_unsaved());
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, 60.0, editor, &mut state);

    let tier = editor.draft.enemy_min_tier(0).unwrap();
    assert!(
        tier > 0 && tier <= MAX_MIN_TIER,
        "dragging right raised it to {tier}"
    );
    assert!(editor.dirty, "the unsaved guard sees the edit");
    assert!(editor.has_unsaved());
    assert!(!state.history.gesture_open());

    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, before, "one undo undoes the whole drag");
    assert!(!state.history.can_undo(), "the drag was a single step");
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Redo);
    assert_eq!(editor.draft.enemy_min_tier(0), Some(tier));
}

#[test]
fn dragging_the_field_sets_the_boss_min_tier_and_only_that() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_boss(editor);
    let mut state = EditorUi {
        selected: Some(Selection::Boss),
        ..Default::default()
    };
    let before = editor.draft.clone();
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, 60.0, editor, &mut state);

    assert!(editor.draft.boss_min_tier().unwrap() > 0);
    assert_eq!(editor.draft.enemy_min_tier(0), Some(0));
    assert_eq!(editor.draft.layout, before.layout);
    crate::editor_panel::apply_history(editor, &mut state, HistoryKey::Undo);
    assert_eq!(editor.draft, before);
}

#[test]
fn the_field_stops_at_the_highest_tier_and_at_zero() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi {
        selected: Some(Selection::Enemy(0)),
        ..Default::default()
    };
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, 5000.0, editor, &mut state);
    assert_eq!(editor.draft.enemy_min_tier(0), Some(MAX_MIN_TIER));
    assert!(editor.message.is_empty(), "{}", editor.message);

    let (ctx, _) = ready(editor, &mut state);
    drag(&ctx, -5000.0, editor, &mut state);
    assert_eq!(editor.draft.enemy_min_tier(0), Some(0));
}

#[test]
fn two_enemies_of_one_definition_have_their_own_min_tier() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let twin = editor.draft.enemies[0].clone();
    editor.draft.enemies.push(twin);
    let mut state = EditorUi {
        selected: Some(Selection::Enemy(1)),
        ..Default::default()
    };
    let (ctx, _) = ready(editor, &mut state);

    drag(&ctx, 60.0, editor, &mut state);

    assert_eq!(editor.draft.enemy_min_tier(0), Some(0));
    assert!(editor.draft.enemy_min_tier(1).unwrap() > 0);
    assert_eq!(
        editor.draft.enemies[0].definition, editor.draft.enemies[1].definition,
        "min_tier is the placement's own, not the definition's"
    );
}
