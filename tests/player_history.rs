mod support;

use hollow_steps::editor::EditorSession;
use hollow_steps::player_history::{PlayerHistory, PlayerSnapshot, HISTORY_LIMIT};
use hollow_steps::tuning::{Field, TuningConfig};
use support::player_assets::Assets;

fn jump_speed(session: &EditorSession) -> f32 {
    session.player.edited().player.jump_speed
}

/// One edit as the panel makes it: the values before, the edit, the values after.
fn edit(history: &mut PlayerHistory, session: &mut EditorSession, value: f32) {
    let before = PlayerSnapshot::capture(session);
    session.player.set_field(Field::JumpSpeed, value).unwrap();
    history.record(before, &PlayerSnapshot::capture(session));
}

#[test]
fn undo_and_redo_step_through_edits_and_a_new_edit_clears_redo() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = PlayerHistory::default();
    assert!(!history.can_undo() && !history.can_redo());
    assert!(!history.undo(session), "nothing to undo yet");

    edit(&mut history, session, 500.0);
    edit(&mut history, session, 600.0);
    edit(&mut history, session, 700.0);
    assert_eq!(history.undo_steps(), 3);
    assert!(history.undo(session));
    assert_eq!(jump_speed(session), 600.0);
    assert!(history.undo(session));
    assert!(history.undo(session));
    assert_eq!(jump_speed(session), 480.0, "back to the saved value");
    assert!(!session.player.is_dirty());
    assert!(!history.undo(session));

    assert!(history.redo(session));
    assert_eq!(jump_speed(session), 500.0);
    assert!(history.redo(session));
    assert_eq!(jump_speed(session), 600.0);
    // A new edit drops what was left to redo.
    edit(&mut history, session, 550.0);
    assert!(!history.can_redo());
    assert!(!history.redo(session));
    assert_eq!(jump_speed(session), 550.0);
}

#[test]
fn an_edit_that_changes_nothing_records_nothing_and_the_limit_drops_the_oldest() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = PlayerHistory::default();
    let same = PlayerSnapshot::capture(session);
    history.record(same.clone(), &same);
    assert_eq!(history.undo_steps(), 0);

    for step in 0..HISTORY_LIMIT + 5 {
        edit(&mut history, session, 400.0 + step as f32);
    }
    assert_eq!(history.undo_steps(), HISTORY_LIMIT);
    for _ in 0..HISTORY_LIMIT {
        assert!(history.undo(session));
    }
    assert!(!history.undo(session));
    // The five oldest steps were dropped, so the earliest state reachable is the sixth.
    assert_eq!(jump_speed(session), 404.0);
}

#[test]
fn a_whole_drag_is_one_step_whatever_the_number_of_frames() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = PlayerHistory::default();
    let start = PlayerSnapshot::capture(session);

    // Frames of a drag: the mouse stays down and the value changes each frame.
    for value in [500.0, 520.0, 560.0, 600.0] {
        let before = PlayerSnapshot::capture(session);
        session.player.set_field(Field::JumpSpeed, value).unwrap();
        history.observe(before, &PlayerSnapshot::capture(session), true);
        assert!(history.gesture_open());
        assert!(!history.can_undo(), "no undo in the middle of a drag");
        assert!(!history.undo(session));
    }
    // A frame with the mouse down and no change keeps the drag open.
    let same = PlayerSnapshot::capture(session);
    history.observe(same.clone(), &same, true);
    assert!(history.gesture_open());
    assert_eq!(history.undo_steps(), 0);
    // The release ends it as one step.
    history.observe(same.clone(), &same, false);
    assert!(!history.gesture_open());
    assert_eq!(history.undo_steps(), 1);
    assert!(history.undo(session));
    assert_eq!(PlayerSnapshot::capture(session), start);
    assert_eq!(jump_speed(session), 480.0);
    assert!(history.redo(session));
    assert_eq!(jump_speed(session), 600.0);
}

#[test]
fn an_edit_with_no_button_down_is_its_own_step_and_idle_frames_add_none() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = PlayerHistory::default();
    for _ in 0..5 {
        let same = PlayerSnapshot::capture(session);
        history.observe(same.clone(), &same, false);
    }
    assert_eq!(history.undo_steps(), 0);
    // A typed value, a checkbox or a Reset lands on a frame with the button up.
    for value in [500.0, 600.0] {
        let before = PlayerSnapshot::capture(session);
        session.player.set_field(Field::JumpSpeed, value).unwrap();
        history.observe(before, &PlayerSnapshot::capture(session), false);
    }
    assert_eq!(history.undo_steps(), 2);
    // A discrete edit right after a drag closes the drag first, so the order is kept.
    let before = PlayerSnapshot::capture(session);
    session.player.set_field(Field::DashSpeed, 900.0).unwrap();
    history.observe(before, &PlayerSnapshot::capture(session), true);
    let before = PlayerSnapshot::capture(session);
    session.player.set_field(Field::JumpSpeed, 700.0).unwrap();
    history.observe(before, &PlayerSnapshot::capture(session), false);
    assert_eq!(history.undo_steps(), 4);
    assert!(history.undo(session));
    assert_eq!(jump_speed(session), 600.0);
    assert_eq!(session.player.edited().player.dash_speed, 900.0);
    assert!(history.undo(session));
    assert_eq!(
        session.player.edited().player.dash_speed,
        TuningConfig::default().player.dash_speed,
        "the dash drag was one step of its own"
    );
}

#[test]
fn the_look_and_the_body_template_come_back_with_undo() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = PlayerHistory::default();
    let primary = session.look.edited().visual.primary;
    let cloak = |session: &EditorSession| {
        session.content.bodies["wanderer"]
            .parts
            .iter()
            .find(|part| part.name == "cloak")
            .unwrap()
            .size
    };
    let size = cloak(session);
    assert!(!session.dirty);

    let before = PlayerSnapshot::capture(session);
    let mut visual = session.look.edited().visual.clone();
    visual.primary = (0.1, 0.9, 0.1, 1.0);
    session.look.set_visual(visual);
    let template = session.content.bodies.get_mut("wanderer").unwrap();
    std::sync::Arc::make_mut(template)
        .parts
        .iter_mut()
        .find(|part| part.name == "cloak")
        .unwrap()
        .size = (0.7, 0.8);
    history.record(before, &PlayerSnapshot::capture(session));
    assert!(session.look.is_dirty());

    assert!(history.undo(session));
    assert_eq!(session.look.edited().visual.primary, primary);
    assert!(!session.look.is_dirty());
    assert_eq!(cloak(session), size);
    assert!(
        session.dirty,
        "the template changed back: the editor's Save has work"
    );
    assert!(history.redo(session));
    assert_eq!(session.look.edited().visual.primary, (0.1, 0.9, 0.1, 1.0));
    assert_eq!(cloak(session), (0.7, 0.8));
}

#[test]
fn the_rooms_own_history_is_not_touched() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let mut history = PlayerHistory::default();
    let draft = session.draft.clone();
    edit(&mut history, session, 500.0);
    assert_eq!(session.draft, draft, "a player edit changes no room");
    assert!(history.undo(session));
    assert_eq!(session.draft, draft);
}
