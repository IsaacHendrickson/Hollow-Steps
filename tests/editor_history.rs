mod support;

use hollow_steps::editor::{Placement, RoomDraft, Selection};
use hollow_steps::editor_history::{
    redo_session, shortcut, tracked, undo_session, History, HistoryKey, HISTORY_LIMIT,
};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::item::Item;
use hollow_steps::room::{test_rooms, RoomId};
use hollow_steps::room_size::{resize_session, RoomSize};
use macroquad::math::Vec2;

fn setup() -> (RoomDraft, EnemyDefinitions) {
    let rooms = test_rooms();
    let path = support::fixture_assets().join("rooms").join("first.ron");
    let document = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    (
        RoomDraft::from_document(document, &rooms[0]),
        EnemyDefinitions::load(&support::fixture_assets()).unwrap(),
    )
}

fn any(_: &RoomDraft) -> Result<(), String> {
    Ok(())
}

/// Runs `edit` on the draft and records it, the way the editor does for one-step changes.
fn step(history: &mut History, draft: &mut RoomDraft, edit: impl FnOnce(&mut RoomDraft)) {
    let before = draft.clone();
    edit(draft);
    history.record(before, draft);
}

#[test]
fn undo_after_place_removes_the_enemy_and_redo_brings_it_back() {
    let (mut draft, definitions) = setup();
    let original = draft.clone();
    let mut history = History::default();
    step(&mut history, &mut draft, |d| {
        d.place(
            Placement::Enemy("patrol".into()),
            Vec2::new(200.0, 50.0),
            &definitions,
        )
        .unwrap();
    });
    let placed = draft.clone();
    assert_eq!(placed.enemies.len(), original.enemies.len() + 1);

    assert!(history.undo(&mut draft, any).unwrap());
    assert_eq!(draft, original);
    assert!(history.redo(&mut draft, any).unwrap());
    assert_eq!(draft, placed);
}

#[test]
fn undo_after_delete_restores_the_item() {
    let (mut draft, _) = setup();
    let original = draft.clone();
    let mut history = History::default();
    step(&mut history, &mut draft, |d| {
        d.delete(Selection::Item(0)).unwrap();
    });
    assert_eq!(draft.layout.items.len(), original.layout.items.len() - 1);
    history.undo(&mut draft, any).unwrap();
    assert_eq!(draft, original);
}

#[test]
fn a_whole_drag_is_one_undo_step() {
    let (mut draft, definitions) = setup();
    let original = draft.clone();
    let mut history = History::default();
    history.begin_gesture(&draft);
    assert!(history.gesture_open());
    // Thirty frames of dragging the first item.
    for frame in 1..=30 {
        draft
            .move_to(
                Selection::Item(0),
                Vec2::new(300.0 + frame as f32 * 4.0, 300.0),
                &definitions,
            )
            .unwrap();
    }
    let dragged = draft.clone();
    assert_ne!(dragged, original);
    history.end_gesture(&draft);
    assert!(!history.gesture_open());

    assert!(history.undo(&mut draft, any).unwrap());
    assert_eq!(draft, original, "one undo undoes the whole drag");
    assert!(!history.can_undo(), "the drag was a single entry");
    assert!(history.redo(&mut draft, any).unwrap());
    assert_eq!(draft, dragged);
}

#[test]
fn pressing_and_releasing_without_changing_anything_adds_no_step() {
    let (mut draft, _) = setup();
    let mut history = History::default();
    history.begin_gesture(&draft);
    history.end_gesture(&draft);
    assert!(!history.can_undo());
    // Ending a drag that was never begun does nothing either.
    history.end_gesture(&draft);
    assert!(!history.can_undo());
    assert!(!history.undo(&mut draft, any).unwrap());
}

#[test]
fn a_new_edit_clears_redo() {
    let (mut draft, definitions) = setup();
    let mut history = History::default();
    for x in [200.0, 260.0] {
        step(&mut history, &mut draft, |d| {
            d.place(
                Placement::Item(Item::Dash),
                Vec2::new(x, 100.0),
                &definitions,
            )
            .unwrap();
        });
    }
    history.undo(&mut draft, any).unwrap();
    assert!(history.can_redo());
    step(&mut history, &mut draft, |d| {
        d.delete(Selection::Item(0)).unwrap();
    });
    assert!(!history.can_redo());
    assert!(!history.redo(&mut draft, any).unwrap());
}

#[test]
fn a_change_that_changes_nothing_is_not_recorded_and_keeps_redo() {
    let (mut draft, definitions) = setup();
    let mut history = History::default();
    step(&mut history, &mut draft, |d| {
        d.place(
            Placement::Item(Item::Dash),
            Vec2::new(200.0, 100.0),
            &definitions,
        )
        .unwrap();
    });
    history.undo(&mut draft, any).unwrap();
    step(&mut history, &mut draft, |_| {});
    assert!(!history.can_undo());
    assert!(history.can_redo(), "a no-op is not a new edit");
}

#[test]
fn only_the_last_hundred_steps_are_kept() {
    assert_eq!(HISTORY_LIMIT, 100);
    let (mut draft, definitions) = setup();
    let mut history = History::default();
    let mut before_each = Vec::new();
    for i in 0..150 {
        before_each.push(draft.clone());
        step(&mut history, &mut draft, |d| {
            d.move_to(
                Selection::Item(0),
                Vec2::new(100.0 + i as f32, 300.0),
                &definitions,
            )
            .unwrap();
        });
    }
    let mut undone = 0;
    while history.undo(&mut draft, any).unwrap() {
        undone += 1;
    }
    assert_eq!(undone, HISTORY_LIMIT);
    // The 50 oldest states are gone: the furthest back we can reach is the 51st.
    assert_eq!(draft, before_each[150 - HISTORY_LIMIT]);
    // And all hundred can be redone.
    let mut redone = 0;
    while history.redo(&mut draft, any).unwrap() {
        redone += 1;
    }
    assert_eq!(redone, HISTORY_LIMIT);
}

#[test]
fn undo_and_redo_with_nothing_to_do_leave_the_draft_alone() {
    let (mut draft, _) = setup();
    let original = draft.clone();
    let mut history = History::default();
    assert!(!history.undo(&mut draft, any).unwrap());
    assert!(!history.redo(&mut draft, any).unwrap());
    assert_eq!(draft, original);
    assert!(!history.can_undo() && !history.can_redo());
}

#[test]
fn undo_and_redo_are_unavailable_mid_drag() {
    let (mut draft, definitions) = setup();
    let mut history = History::default();
    step(&mut history, &mut draft, |d| {
        d.delete(Selection::Item(1)).unwrap();
    });
    history.undo(&mut draft, any).unwrap();
    step(&mut history, &mut draft, |d| {
        d.delete(Selection::Item(1)).unwrap();
    });
    history.undo(&mut draft, any).unwrap();
    assert!(history.can_undo() || history.can_redo());

    history.begin_gesture(&draft);
    draft
        .move_to(Selection::Item(0), Vec2::new(150.0, 300.0), &definitions)
        .unwrap();
    let during = draft.clone();
    assert!(!history.can_undo() && !history.can_redo());
    assert!(!history.undo(&mut draft, any).unwrap());
    assert!(!history.redo(&mut draft, any).unwrap());
    assert_eq!(draft, during);
    history.end_gesture(&draft);
    assert!(history.can_undo());
}

#[test]
fn a_one_step_edit_during_a_drag_keeps_the_steps_in_order() {
    let (mut draft, definitions) = setup();
    let start = draft.clone();
    let mut history = History::default();
    history.begin_gesture(&draft);
    draft
        .move_to(Selection::Item(0), Vec2::new(150.0, 300.0), &definitions)
        .unwrap();
    let dragged = draft.clone();
    // Delete (say, with the Delete key) while the mouse is still down.
    step(&mut history, &mut draft, |d| {
        d.delete(Selection::Item(1)).unwrap();
    });
    let deleted = draft.clone();
    // Releasing afterwards must not add a stale step.
    history.end_gesture(&draft);

    history.undo(&mut draft, any).unwrap();
    assert_eq!(draft, dragged);
    history.undo(&mut draft, any).unwrap();
    assert_eq!(draft, start);
    assert!(!history.can_undo());
    history.redo(&mut draft, any).unwrap();
    history.redo(&mut draft, any).unwrap();
    assert_eq!(draft, deleted);
}

#[test]
fn a_snapshot_that_is_no_longer_valid_is_refused_and_dropped() {
    let (mut draft, _) = setup();
    let mut history = History::default();
    let mut states = vec![draft.clone()];
    for index in [1, 0] {
        step(&mut history, &mut draft, |d| {
            d.delete(Selection::Item(index)).unwrap();
        });
        states.push(draft.clone());
    }
    // states: [both items, one item, no items]. Refuse the middle snapshot.
    let refuse = |d: &RoomDraft| {
        if d.layout.items.len() == 1 {
            Err("stale".to_owned())
        } else {
            Ok(())
        }
    };
    let error = history.undo(&mut draft, refuse).unwrap_err();
    assert_eq!(error, "stale");
    assert_eq!(draft, states[2], "the draft is untouched");
    // The refused step is gone, so the next undo goes one step further back.
    assert!(history.undo(&mut draft, refuse).unwrap());
    assert_eq!(draft, states[0]);
}

#[test]
fn clearing_forgets_everything() {
    let (mut draft, _) = setup();
    let mut history = History::default();
    step(&mut history, &mut draft, |d| {
        d.delete(Selection::Item(0)).unwrap();
    });
    history.undo(&mut draft, any).unwrap();
    history.begin_gesture(&draft);
    history.clear();
    assert!(!history.can_undo() && !history.can_redo() && !history.gesture_open());
}

fn editing() -> GameState {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

#[test]
fn session_edits_undo_redo_and_mark_the_session_unsaved() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let original = session.draft.clone();
    tracked(&mut history, session, |s| {
        s.place(Placement::Item(Item::Dash), Vec2::new(200.0, 100.0))
    })
    .unwrap();
    assert_eq!(
        session.draft.layout.items.len(),
        original.layout.items.len() + 1
    );
    session.dirty = false;

    assert!(undo_session(&mut history, session).unwrap());
    assert_eq!(session.draft, original);
    assert!(session.dirty, "undo is an edit to the file");
    session.dirty = false;
    assert!(redo_session(&mut history, session).unwrap());
    assert_eq!(
        session.draft.layout.items.len(),
        original.layout.items.len() + 1
    );
    assert!(session.dirty);
    assert!(!redo_session(&mut history, session).unwrap());
}

#[test]
fn nothing_to_undo_does_not_mark_the_session_unsaved() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    assert!(!undo_session(&mut history, session).unwrap());
    assert!(!redo_session(&mut history, session).unwrap());
    assert!(!session.dirty);
}

#[test]
fn a_failed_edit_records_nothing() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let result = tracked(&mut history, session, |s| {
        s.place(Placement::Item(Item::Dash), Vec2::new(f32::NAN, 100.0))
    });
    assert!(result.is_err());
    assert!(!history.can_undo());
}

#[test]
fn room_size_edits_are_undoable() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let original = session.draft.clone();
    let mut size = RoomSize::of(&session.draft.layout);
    size.width += 100.0;
    tracked(&mut history, session, |s| resize_session(s, size)).unwrap();
    assert_eq!(session.draft.layout.bounds.2, size.width);
    // Applying the same size again is not a new step.
    tracked(&mut history, session, |s| resize_session(s, size)).unwrap();
    undo_session(&mut history, session).unwrap();
    assert_eq!(session.draft, original);
    assert!(!history.can_undo());
}

#[test]
fn undoing_to_an_entity_that_was_renamed_since_is_refused_not_applied() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let before_place = session.draft.clone();
    let placed = tracked(&mut history, session, |s| {
        s.place(Placement::Enemy("patrol".into()), Vec2::new(300.0, 50.0))
    })
    .unwrap();
    tracked(&mut history, session, |s| {
        s.move_selection(placed, Vec2::new(340.0, 50.0))
    })
    .unwrap();

    // Definition edits are outside undo: rename the entity the placement uses.
    let path = session.content.entity_path("patrol").unwrap();
    let mut entity = session.content.entities.get(&path).unwrap().clone();
    entity.name = "renamed patrol".into();
    session.update_entity(&path, entity).unwrap();
    let after_rename = session.draft.clone();
    assert!(after_rename
        .enemies
        .iter()
        .any(|e| e.definition == "renamed patrol"));

    // The move snapshot still says "patrol", which no longer exists.
    let error = undo_session(&mut history, session).unwrap_err();
    assert!(error.contains("cannot restore"), "{error}");
    assert_eq!(session.draft, after_rename, "the draft is untouched");
    // The next step back (before the enemy was placed) is fine.
    assert!(undo_session(&mut history, session).unwrap());
    assert_eq!(session.draft.enemies, before_place.enemies);
}

#[test]
fn deleting_the_boss_can_be_undone() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.edit_room(RoomId::Boss).unwrap();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let boss = session.draft.boss.clone();
    assert!(boss.is_some());
    tracked(&mut history, session, |s| s.draft.delete(Selection::Boss)).unwrap();
    assert!(session.draft.boss.is_none());
    assert!(undo_session(&mut history, session).unwrap());
    assert_eq!(session.draft.boss, boss);
}

#[test]
fn ctrl_z_undoes_and_ctrl_y_or_ctrl_shift_z_redo() {
    assert_eq!(shortcut(true, false, true, false), Some(HistoryKey::Undo));
    assert_eq!(shortcut(true, false, false, true), Some(HistoryKey::Redo));
    assert_eq!(shortcut(true, true, true, false), Some(HistoryKey::Redo));
    assert_eq!(shortcut(true, true, false, true), Some(HistoryKey::Redo));
}

#[test]
fn history_keys_need_ctrl_and_a_fresh_press() {
    assert_eq!(shortcut(false, false, true, false), None);
    assert_eq!(shortcut(false, true, true, true), None);
    assert_eq!(shortcut(true, false, false, false), None);
    assert_eq!(shortcut(true, true, false, false), None);
    // Both pressed at once: undo wins rather than doing both.
    assert_eq!(shortcut(true, false, true, true), Some(HistoryKey::Undo));
}
