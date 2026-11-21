use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use hollow_steps::charm_editor::EffectKind;

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

fn toggle_reach(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) -> String {
    let rect: egui::Rect = ctx
        .data(|data| data.get_temp(egui::Id::new(("reach toggle",))))
        .expect("a Reach toggle");
    let [press, release] = click(rect.center());
    pass(ctx, press, editor, state);
    pass(ctx, release, editor, state);
    pass(ctx, vec![], editor, state);
    pass(ctx, vec![], editor, state)
}

#[test]
fn the_panel_says_nothing_about_reach_until_the_toggle_is_on() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let (ctx, text) = ready(editor, &mut state);
    assert!(text.contains("Reach"), "the toggle is there: {text}");
    assert!(!text.contains("Reach from"), "{text}");
    assert!(!state.checks.reach);

    let text = toggle_reach(&ctx, editor, &mut state);
    assert!(state.checks.reach);
    assert!(text.contains("Reach from the room spawn"), "{text}");
    assert!(!text.contains("no gap can need Dash"), "{text}");
    assert!(!editor.dirty, "looking is not an edit");
    assert!(!state.history.can_undo());

    let text = toggle_reach(&ctx, editor, &mut state);
    assert!(!state.checks.reach);
    assert!(!text.contains("Reach from"), "{text}");
}

#[test]
fn a_selected_arrival_is_named_as_where_the_outlines_start() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.checks.reach = true;
    let (ctx, text) = ready(editor, &mut state);
    assert!(text.contains("Reach from the room spawn"), "{text}");

    state.selected = editor
        .inbound_arrivals()
        .first()
        .map(|inbound| Selection::Inbound {
            room: inbound.room,
            exit: inbound.exit,
        });
    assert!(state.selected.is_some(), "the fixture has an arrival");
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Reach from the selected arrival"), "{text}");
}

#[test]
fn numbers_that_leave_no_gap_for_the_dash_say_why_in_red_text_instead() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    // Two charms of the largest move speed buff: without Dash the jump outruns the dash jump.
    for _ in 0..2 {
        let charm = editor.charms.new_charm();
        editor
            .charms
            .set_effect_kind(charm, EffectKind::MoveSpeedPercent)
            .unwrap();
        editor.charms.set_effect_amount(charm, 200.0).unwrap();
    }
    let mut state = EditorUi::default();
    state.checks.reach = true;

    let (ctx, text) = ready(editor, &mut state);

    assert!(text.contains("no gap can need Dash"), "{text}");
    assert!(!text.contains("Reach from"), "nothing to draw: {text}");
    // Taking the buffs away brings the preview back, without closing the panel.
    editor.charms.reset_all();
    let text = pass(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Reach from the room spawn"), "{text}");
    assert!(!text.contains("no gap can need Dash"), "{text}");
}
