use super::*;
use crate::editor_history::History;
use crate::enemy::EnemyDefinitions;
use crate::piece::{PieceRole, MAX_WEIGHT, MIN_WEIGHT};
use crate::room::test_rooms;

fn draft() -> RoomDraft {
    let [room, _, _] = test_rooms();
    RoomDraft::new_room(&RoomLayout::from_room(&room))
}

fn no_definitions() -> EnemyDefinitions {
    EnemyDefinitions::from_definitions(Vec::new(), Default::default(), Default::default()).unwrap()
}

fn positions(draft: &RoomDraft) -> Vec<(f32, f32)> {
    draft.layout.slots.iter().map(|s| s.position).collect()
}

fn marked(role: PieceRole, weight: u8) -> Option<PieceInfo> {
    Some(PieceInfo { role, weight })
}

// ---- the mark ----

#[test]
fn marking_changing_and_unmarking_a_room_keeps_its_slots() {
    let mut draft = draft();
    draft.add_slot().unwrap();
    draft.set_piece(marked(PieceRole::Start, 1)).unwrap();
    assert!(draft.layout.is_piece());
    draft.set_piece(marked(PieceRole::Boss, 9)).unwrap();
    assert_eq!(draft.layout.piece, marked(PieceRole::Boss, 9));
    draft.set_piece(None).unwrap();
    assert!(!draft.layout.is_piece());
    assert_eq!(draft.layout.slots.len(), 1, "unmarking keeps the slots");
}

#[test]
fn the_ends_of_the_weight_range_are_accepted_and_outside_it_is_refused_unchanged() {
    let mut draft = draft();
    for weight in [MIN_WEIGHT, MAX_WEIGHT] {
        draft.set_piece(marked(PieceRole::Middle, weight)).unwrap();
    }
    let before = draft.clone();
    for weight in [0, MAX_WEIGHT + 1] {
        let error = draft
            .set_piece(marked(PieceRole::Middle, weight))
            .unwrap_err();
        assert!(error.contains("weight"), "{error}");
        assert_eq!(draft, before, "a refused edit changes nothing");
    }
}

#[test]
fn a_mark_is_one_undo_step_and_redo_puts_it_back() {
    let mut draft = draft();
    let mut history = History::default();
    let before = draft.clone();
    draft.set_piece(marked(PieceRole::Middle, 1)).unwrap();
    history.record(before.clone(), &draft);
    let marked_draft = draft.clone();
    assert!(history.undo(&mut draft, |_| Ok(())).unwrap());
    assert_eq!(draft, before);
    assert!(history.redo(&mut draft, |_| Ok(())).unwrap());
    assert_eq!(draft, marked_draft);
}

// ---- adding ----

#[test]
fn add_puts_an_any_slot_on_the_floor_right_of_the_spawn_and_selects_it() {
    let mut draft = draft();
    let (spawn_x, _) = draft.layout.spawn;
    assert_eq!(draft.add_slot().unwrap(), Selection::Slot(0));
    let slot = draft.slot(0).unwrap();
    assert_eq!(slot.kind, SlotKind::Any);
    assert_eq!(slot.position.0, spawn_x + SPAWN_CLEARANCE);
    assert_eq!(slot.position.1 + ITEM_SIZE.y / 2.0, draft.layout.floor_y);
    draft.layout.validate_geometry().unwrap();
    assert!(
        draft.layout.piece.is_none(),
        "a slot does not mark the room"
    );
}

#[test]
fn each_new_slot_takes_a_free_spot_of_its_own() {
    let mut draft = draft();
    for number in 0..MAX_SLOTS {
        assert_eq!(draft.add_slot().unwrap(), Selection::Slot(number));
    }
    assert_eq!(draft.layout.slots.len(), MAX_SLOTS);
    draft.layout.validate_geometry().unwrap();
    let mut seen = positions(&draft);
    seen.dedup();
    assert_eq!(seen.len(), MAX_SLOTS, "no two on one spot");
}

#[test]
fn add_goes_left_of_the_spawn_when_solids_fill_the_right() {
    let mut draft = draft();
    let (spawn_x, _) = draft.layout.spawn;
    let floor = draft.layout.floor_y;
    draft.layout.terrain = vec![(spawn_x + 17.0, floor - 40.0, 960.0 - spawn_x - 17.0, 40.0)];
    draft.layout.validate_geometry().unwrap();
    draft.add_slot().unwrap();
    assert_eq!(draft.slot(0).unwrap().position.0, spawn_x - SPAWN_CLEARANCE);
}

#[test]
fn add_says_so_when_there_is_no_free_spot_and_changes_nothing() {
    let mut draft = draft();
    let (spawn_x, _) = draft.layout.spawn;
    let floor = draft.layout.floor_y;
    draft.layout.terrain = vec![
        (0.0, floor - 40.0, spawn_x - 17.0, 40.0),
        (spawn_x + 17.0, floor - 40.0, 960.0 - spawn_x - 17.0, 40.0),
    ];
    draft.layout.validate_geometry().unwrap();
    let before = draft.clone();
    let error = draft.add_slot().unwrap_err();
    assert!(error.contains("no free spot"), "{error}");
    assert_eq!(draft, before);
}

#[test]
fn the_ninth_slot_is_refused_with_the_limit_in_the_message() {
    let mut draft = draft();
    for _ in 0..MAX_SLOTS {
        draft.add_slot().unwrap();
    }
    let before = draft.clone();
    let error = draft.add_slot().unwrap_err();
    assert!(error.contains("at most 8"), "{error}");
    assert_eq!(draft, before);
}

// ---- moving, changing, deleting ----

#[test]
fn moving_a_slot_sets_its_center_and_keeps_its_kind() {
    let definitions = no_definitions();
    let mut draft = draft();
    draft.add_slot().unwrap();
    draft.set_slot_kind(0, SlotKind::Key).unwrap();
    draft
        .move_to(Selection::Slot(0), Vec2::new(600.0, 300.0), &definitions)
        .unwrap();
    assert_eq!(
        draft.slot(0).unwrap(),
        &PickupSlot::new((600.0, 300.0), SlotKind::Key)
    );
}

#[test]
fn a_move_the_rules_refuse_leaves_the_slot_where_it_was_and_says_why() {
    let definitions = no_definitions();
    let mut draft = draft();
    draft.add_slot().unwrap();
    draft.add_slot().unwrap();
    let floor = draft.layout.floor_y;
    draft.layout.terrain = vec![(700.0, floor - 40.0, 100.0, 40.0)];
    let before = draft.clone();
    let first = draft.slot(0).unwrap().position;
    for (to, expected) in [
        (Vec2::new(-50.0, 300.0), "inside the room"),
        (Vec2::new(750.0, floor - 20.0), "terrain"),
        (
            Vec2::new(positions(&draft)[1].0 + 3.0, positions(&draft)[1].1),
            "closer than an item",
        ),
        (Vec2::new(f32::NAN, 300.0), "finite"),
    ] {
        let error = draft
            .move_to(Selection::Slot(0), to, &definitions)
            .unwrap_err();
        assert!(error.contains(expected), "{to:?}: {error}");
        assert_eq!(draft, before);
        assert_eq!(draft.slot(0).unwrap().position, first);
    }
    assert!(draft
        .move_to(Selection::Slot(9), Vec2::new(300.0, 300.0), &definitions)
        .is_err());
}

#[test]
fn changing_a_kind_changes_only_that_slot() {
    let mut draft = draft();
    draft.add_slot().unwrap();
    draft.add_slot().unwrap();
    draft.set_slot_kind(1, SlotKind::Ability).unwrap();
    assert_eq!(draft.slot(0).unwrap().kind, SlotKind::Any);
    assert_eq!(draft.slot(1).unwrap().kind, SlotKind::Ability);
    assert!(draft.set_slot_kind(2, SlotKind::Key).is_err());
}

#[test]
fn deleting_a_slot_removes_it_and_later_ones_move_up() {
    let mut draft = draft();
    for _ in 0..3 {
        draft.add_slot().unwrap();
    }
    draft.set_slot_kind(2, SlotKind::Key).unwrap();
    let kept = (positions(&draft)[0], positions(&draft)[2]);
    draft.delete(Selection::Slot(1)).unwrap();
    assert_eq!(positions(&draft), vec![kept.0, kept.1]);
    assert_eq!(draft.slot(1).unwrap().kind, SlotKind::Key);
    assert!(draft.delete(Selection::Slot(2)).is_err(), "nothing there");
}

#[test]
fn a_slot_is_part_of_the_room_being_edited() {
    assert!(Selection::Slot(0).edits_current_room());
    assert!(!Selection::Slot(0).is_puzzle());
}

#[test]
fn slots_and_the_mark_are_in_the_room_a_draft_builds() {
    let mut draft = draft();
    draft.set_piece(marked(PieceRole::Middle, 2)).unwrap();
    draft.add_slot().unwrap();
    let room = draft
        .build(crate::room::RoomId::First, &no_definitions())
        .unwrap();
    assert_eq!(RoomLayout::from_room(&room), draft.layout);
    assert!(room.items.is_empty(), "a slot is not an item");
}
