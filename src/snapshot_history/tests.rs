use super::*;
use crate::upgrade_editor::UpgradeDraft;
use crate::upgrades::{Upgrade, UpgradeCatalog};

type History = SnapshotHistory<Upgrade>;

/// Two upgrades written for the tests, never the shipped numbers.
const SAMPLE: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "",
     effect: MoveSpeedPercent(5.0), costs: [40, 80]),
])"#;

fn draft() -> UpgradeDraft {
    UpgradeDraft::from_catalog(&UpgradeCatalog::from_ron(SAMPLE).unwrap())
}

/// Edits the first price of upgrade 0 to `price` the way a frame would: observed with `held`.
fn edit(history: &mut History, draft: &mut UpgradeDraft, price: f32, held: bool) {
    let before = draft.edited().to_vec();
    draft.set_cost(0, 0, price).unwrap();
    history.observe(before, draft.edited(), held);
}

fn idle(history: &mut History, draft: &UpgradeDraft) {
    history.observe(draft.edited().to_vec(), draft.edited(), false);
}

#[test]
fn a_whole_drag_is_one_step() {
    let (mut history, mut draft) = (History::default(), draft());
    for price in [31.0, 32.0, 33.0, 34.0] {
        edit(&mut history, &mut draft, price, true);
    }
    assert!(history.gesture_open());
    idle(&mut history, &draft);
    assert!(!history.gesture_open());
    assert_eq!(history.undo_steps(), 1);
    assert!(history.undo(&mut draft));
    assert_eq!(draft.edited()[0].costs[0], 30);
    assert!(!draft.is_dirty());
    assert!(history.redo(&mut draft));
    assert_eq!(draft.edited()[0].costs[0], 34);
}

#[test]
fn edits_with_nothing_held_are_a_step_each() {
    let (mut history, mut draft) = (History::default(), draft());
    edit(&mut history, &mut draft, 31.0, false);
    edit(&mut history, &mut draft, 32.0, false);
    assert_eq!(history.undo_steps(), 2);
    assert!(history.undo(&mut draft) && history.undo(&mut draft));
    assert!(!history.undo(&mut draft), "nothing left to undo");
    assert_eq!(draft.edited()[0].costs[0], 30);
}

#[test]
fn a_new_edit_clears_redo_and_a_no_change_frame_records_nothing() {
    let (mut history, mut draft) = (History::default(), draft());
    edit(&mut history, &mut draft, 31.0, false);
    history.undo(&mut draft);
    assert!(history.can_redo());
    idle(&mut history, &draft);
    assert!(history.can_redo(), "a frame with no edit keeps redo");
    edit(&mut history, &mut draft, 35.0, false);
    assert!(!history.can_redo());
    assert!(!history.redo(&mut draft));
    // Setting the value it already has is no edit.
    edit(&mut history, &mut draft, 35.0, false);
    assert_eq!(history.undo_steps(), 1);
}

#[test]
fn undo_during_a_drag_closes_it_and_undoes_the_whole_drag() {
    let (mut history, mut draft) = (History::default(), draft());
    edit(&mut history, &mut draft, 31.0, false);
    for price in [40.0, 41.0, 42.0] {
        edit(&mut history, &mut draft, price, true);
    }
    assert!(history.can_undo() && !history.can_redo());
    assert!(history.undo(&mut draft));
    assert_eq!(draft.edited()[0].costs[0], 31, "back to before the drag");
    assert!(history.redo(&mut draft));
    assert_eq!(draft.edited()[0].costs[0], 42);
}

#[test]
fn structure_edits_are_steps_too_and_undo_brings_back_a_deleted_upgrade() {
    let (mut history, mut draft) = (History::default(), draft());
    let before = draft.edited().to_vec();
    draft.delete(0).unwrap();
    history.observe(before, draft.edited(), false);
    let before = draft.edited().to_vec();
    draft.new_upgrade();
    history.observe(before, draft.edited(), false);
    assert_eq!(draft.edited().len(), 2);
    assert!(history.undo(&mut draft) && history.undo(&mut draft));
    assert_eq!(draft.edited()[0].id, "vitality");
    assert!(!draft.is_dirty());
}

#[test]
fn the_oldest_steps_are_dropped_past_the_limit() {
    let (mut history, mut draft) = (History::default(), draft());
    for price in 1..=(HISTORY_LIMIT as u32 + 20) {
        edit(&mut history, &mut draft, price as f32 + 100.0, false);
    }
    assert_eq!(history.undo_steps(), HISTORY_LIMIT);
    for _ in 0..HISTORY_LIMIT {
        assert!(history.undo(&mut draft));
    }
    assert!(!history.undo(&mut draft));
    // The oldest 20 are gone, so undo stops 20 edits after the start.
    assert_eq!(draft.edited()[0].costs[0], 100 + 20);
}

#[test]
fn clear_forgets_everything() {
    let (mut history, mut draft) = (History::default(), draft());
    edit(&mut history, &mut draft, 31.0, false);
    edit(&mut history, &mut draft, 32.0, true);
    history.clear();
    assert!(!history.can_undo() && !history.can_redo() && !history.gesture_open());
    assert!(!history.undo(&mut draft));
}
