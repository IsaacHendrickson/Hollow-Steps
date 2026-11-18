use super::*;
use crate::charms::CharmCatalog;

fn draft() -> CharmDraft {
    CharmDraft::from_catalog(&CharmCatalog::default())
}

/// Edits charm 0's cost to `cost` the way a frame would: observed with `held`.
fn edit(history: &mut CharmHistory, draft: &mut CharmDraft, cost: f32, held: bool) {
    let before = draft.edited().to_vec();
    draft.set_mana_cost(0, cost).unwrap();
    history.observe(before, draft.edited(), held);
}

fn idle(history: &mut CharmHistory, draft: &CharmDraft) {
    history.observe(draft.edited().to_vec(), draft.edited(), false);
}

#[test]
fn a_whole_drag_is_one_step() {
    let (mut history, mut draft) = (CharmHistory::default(), draft());
    for cost in [16.0, 17.0, 18.0, 19.0] {
        edit(&mut history, &mut draft, cost, true);
    }
    assert!(history.gesture_open());
    idle(&mut history, &draft);
    assert!(!history.gesture_open());
    assert_eq!(history.undo_steps(), 1);
    assert!(history.undo(&mut draft));
    assert_eq!(draft.edited()[0].mana_cost, 15.0);
    assert!(!draft.is_dirty());
    assert!(history.redo(&mut draft));
    assert_eq!(draft.edited()[0].mana_cost, 19.0);
}

#[test]
fn edits_with_nothing_held_are_a_step_each() {
    let (mut history, mut draft) = (CharmHistory::default(), draft());
    edit(&mut history, &mut draft, 16.0, false);
    edit(&mut history, &mut draft, 17.0, false);
    assert_eq!(history.undo_steps(), 2);
    assert!(history.undo(&mut draft) && history.undo(&mut draft));
    assert!(!history.undo(&mut draft), "nothing left to undo");
    assert_eq!(draft.edited()[0].mana_cost, 15.0);
}

#[test]
fn a_new_edit_clears_redo_and_a_no_change_frame_records_nothing() {
    let (mut history, mut draft) = (CharmHistory::default(), draft());
    edit(&mut history, &mut draft, 16.0, false);
    history.undo(&mut draft);
    assert!(history.can_redo());
    idle(&mut history, &draft);
    assert!(history.can_redo(), "a frame with no edit keeps redo");
    edit(&mut history, &mut draft, 18.0, false);
    assert!(!history.can_redo());
    assert!(!history.redo(&mut draft));
    // Setting the value it already has is no edit.
    edit(&mut history, &mut draft, 18.0, false);
    assert_eq!(history.undo_steps(), 1);
}

#[test]
fn undo_during_a_drag_closes_it_and_undoes_it() {
    let (mut history, mut draft) = (CharmHistory::default(), draft());
    edit(&mut history, &mut draft, 16.0, true);
    edit(&mut history, &mut draft, 17.0, true);
    assert!(history.can_undo() && !history.can_redo());
    assert!(history.undo(&mut draft));
    assert_eq!(draft.edited()[0].mana_cost, 15.0);
    assert!(!history.gesture_open());
}

#[test]
fn new_charm_and_delete_are_steps_and_come_back() {
    let (mut history, mut draft) = (CharmHistory::default(), draft());
    let before = draft.edited().to_vec();
    draft.new_charm();
    history.observe(before, draft.edited(), false);
    let before = draft.edited().to_vec();
    draft.delete(0).unwrap();
    history.observe(before, draft.edited(), false);
    assert_eq!(draft.edited().len(), 3);
    history.undo(&mut draft);
    assert_eq!(draft.edited()[0].id, "heart");
    history.undo(&mut draft);
    assert_eq!(draft.edited().len(), 3);
    assert!(!draft.is_dirty());
}

#[test]
fn only_the_newest_hundred_steps_are_kept() {
    let (mut history, mut draft) = (CharmHistory::default(), draft());
    for n in 0..HISTORY_LIMIT + 20 {
        edit(&mut history, &mut draft, (n % 50) as f32 + 0.5, false);
    }
    assert_eq!(history.undo_steps(), HISTORY_LIMIT);
    let mut undone = 0;
    while history.undo(&mut draft) {
        undone += 1;
    }
    assert_eq!(undone, HISTORY_LIMIT);
    history.clear();
    assert!(!history.can_undo() && !history.can_redo());
}
