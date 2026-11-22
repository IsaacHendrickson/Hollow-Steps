use super::super::test_support::{click, drawn_text, editing_game as editor};
use super::*;
use hollow_steps::charm_editor::EffectKind;
use hollow_steps::reach::{Interval, Reach};

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

/// An interval where a jump with every buff crosses 200 px and rises 160 px, a dash jump crosses
/// `dash_far`, and a double jump crosses `double_far` and rises `double_high`.
fn interval(dash_far: f32, double_far: f32, double_high: f32) -> Interval {
    Interval {
        without: Reach {
            jump_height: 160.0,
            jump_distance: 200.0,
            dash_jump_distance: 210.0,
            double_jump_height: 200.0,
            double_jump_distance: 220.0,
        },
        with: Reach {
            jump_height: 160.0,
            jump_distance: 190.0,
            dash_jump_distance: dash_far,
            double_jump_height: double_high,
            double_jump_distance: double_far,
        },
    }
}

/// The panel with the Reach toggle on and `found` as what was measured for the numbers held.
fn with_measured(found: Interval) -> (egui::Context, EditorUi, String, bool) {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    state.checks.reach = true;
    state.checks.measured = Some((editor.reach_key(), found));
    let (ctx, text) = ready(editor, &mut state);
    (ctx, state, text, editor.dirty)
}

#[test]
fn the_note_names_every_ability_that_has_an_outline() {
    let (_, _, text, dirty) = with_measured(interval(400.0, 250.0, 300.0));
    assert!(
        text.contains("Reach from the room spawn: the outlines are a jump with no ability, and with Dash and Double Jump"),
        "{text}"
    );
    assert!(!text.contains("no gap can need"), "{text}");
    assert!(!text.contains("no ledge can need"), "{text}");
    assert!(!dirty, "looking is not an edit");
}

#[test]
fn a_double_jump_nothing_can_need_says_why_in_red_and_the_dash_outline_stays() {
    // It crosses farther but rises no higher than a jump.
    let (_, _, text, _) = with_measured(interval(400.0, 250.0, 160.0));
    assert!(text.contains("no ledge can need Double Jump"), "{text}");
    assert!(text.contains("160 px"), "both numbers: {text}");
    assert!(!text.contains("no gap can need Dash"), "{text}");
    assert!(text.contains("and with Dash"), "{text}");
    assert!(!text.contains("Dash and Double Jump"), "{text}");
}

#[test]
fn a_dash_nothing_can_need_says_why_and_the_double_jump_outline_stays() {
    let (_, _, text, _) = with_measured(interval(190.0, 250.0, 300.0));
    assert!(text.contains("no gap can need Dash"), "{text}");
    assert!(!text.contains("no gap can need Double Jump"), "{text}");
    assert!(text.contains("and with Double Jump"), "{text}");
}

#[test]
fn when_no_ability_has_anything_to_need_it_every_reason_is_listed_and_nothing_is_drawn() {
    let (_, _, text, _) = with_measured(interval(190.0, 190.0, 100.0));
    assert!(text.contains("no gap can need Dash"), "{text}");
    assert!(text.contains("no gap can need Double Jump"), "{text}");
    assert!(text.contains("no ledge can need Double Jump"), "{text}");
    assert!(!text.contains("Reach from"), "nothing to draw: {text}");
}
