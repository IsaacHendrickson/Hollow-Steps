//! Piece marks and pickup slots as data: the room file, the checks, `from_room`, the mirror.
use super::*;
use crate::editor::RoomDraft;
use crate::mirror::mirror_layout;
use crate::room::test_rooms;

/// The first sample room's layout: bounds 960 wide, floor at 460, nothing solid, spawn at x 180.
fn layout() -> RoomLayout {
    RoomLayout::from_room(&test_rooms()[0])
}

/// A free spot on the floor: an item standing there has its feet on the floor.
fn on_floor(layout: &RoomLayout, x: f32) -> (f32, f32) {
    (x, layout.floor_y - ITEM_SIZE.y / 2.0)
}

fn slot(position: (f32, f32), kind: SlotKind) -> PickupSlot {
    PickupSlot::new(position, kind)
}

fn with_slots(positions: &[(f32, f32)]) -> RoomLayout {
    let mut layout = layout();
    layout.slots = positions
        .iter()
        .map(|&at| slot(at, SlotKind::Any))
        .collect();
    layout
}

fn terrain_of(layout: &RoomLayout) -> Terrain {
    Terrain::from_tuples(&layout.terrain)
}

fn refused(layout: &RoomLayout) -> PieceError {
    layout.check_piece(&terrain_of(layout)).unwrap_err()
}

fn written(layout: &RoomLayout) -> String {
    ron::ser::to_string_pretty(layout, ron::ser::PrettyConfig::default()).unwrap()
}

fn read(text: &str) -> RoomLayout {
    ron::from_str(text).unwrap()
}

// ---- the room file ----

#[test]
fn a_room_with_neither_mark_nor_slots_writes_neither() {
    let text = written(&layout());
    assert!(!text.contains("piece") && !text.contains("slots"), "{text}");
}

#[test]
fn a_piece_round_trips_alone() {
    let mut piece = layout();
    piece.piece = Some(PieceInfo {
        role: PieceRole::Boss,
        weight: 7,
    });
    let text = written(&piece);
    assert!(text.contains("piece") && !text.contains("slots"), "{text}");
    assert_eq!(read(&text), piece);
}

#[test]
fn slots_round_trip_alone_in_order_with_their_kinds() {
    let mut slotted = layout();
    slotted.slots = vec![
        slot(on_floor(&slotted, 300.0), SlotKind::Key),
        slot(on_floor(&slotted, 400.0), SlotKind::Ability),
        slot(on_floor(&slotted, 500.0), SlotKind::Any),
    ];
    let text = written(&slotted);
    assert!(text.contains("slots") && !text.contains("piece"), "{text}");
    assert_eq!(read(&text), slotted);
}

#[test]
fn a_piece_with_slots_round_trips_and_stays_valid() {
    let mut both = with_slots(&[(300.0, 450.0), (400.0, 450.0)]);
    both.piece = Some(PieceInfo::new(PieceRole::Start));
    assert_eq!(read(&written(&both)), both);
    both.validate_geometry().unwrap();
}

#[test]
fn an_older_room_file_loads_without_a_mark_or_slots() {
    let old = read(&file_with(""));
    assert_eq!(old.piece, None);
    assert!(old.slots.is_empty() && !old.is_piece());
    old.validate_geometry().unwrap();
}

/// An older room file's text with `extra` fields added after `exits`.
fn file_with(extra: &str) -> String {
    format!(
        "(bounds: (0.0, 0.0, 960.0, 540.0), floor_y: 460.0, spawn: (180.0, 436.0), \
         items: [], exits: []{extra})"
    )
}

#[test]
fn a_mark_without_a_weight_reads_as_weight_one() {
    let loaded = read(&file_with(", piece: Some((role: Middle))"));
    assert_eq!(loaded.piece, Some(PieceInfo::new(PieceRole::Middle)));
    assert_eq!(loaded.piece.unwrap().weight, MIN_WEIGHT);
}

#[test]
fn a_misspelled_field_in_a_mark_or_slot_is_refused() {
    for extra in [
        ", piece: Some((role: Middle, wieght: 3))",
        ", slots: [(position: (300.0, 450.0), kind: Key, colour: 1)]",
        ", slots: [(position: (300.0, 450.0), kind: Gold)]",
        ", piece: Some((role: Hub))",
    ] {
        assert!(
            ron::from_str::<RoomLayout>(&file_with(extra)).is_err(),
            "{extra}"
        );
    }
}

// ---- the checks ----

#[test]
fn a_valid_piece_and_slots_pass_and_so_do_the_limits_themselves() {
    for weight in [MIN_WEIGHT, 5, MAX_WEIGHT] {
        let mut room = layout();
        room.piece = Some(PieceInfo {
            role: PieceRole::Middle,
            weight,
        });
        room.validate_geometry().unwrap();
    }
    let eight: Vec<(f32, f32)> = (0..MAX_SLOTS)
        .map(|i| on_floor(&layout(), 100.0 + 40.0 * i as f32))
        .collect();
    with_slots(&eight).validate_geometry().unwrap();
}

#[test]
fn a_weight_outside_one_to_ten_is_refused() {
    for weight in [0, MAX_WEIGHT + 1, 255] {
        let mut room = layout();
        room.piece = Some(PieceInfo {
            role: PieceRole::Start,
            weight,
        });
        assert_eq!(refused(&room), PieceError::WeightOutOfRange { weight });
        let message = room.validate_geometry().unwrap_err();
        assert!(
            message.contains("weight") && message.contains("1 to 10"),
            "{message}"
        );
    }
}

#[test]
fn more_than_eight_slots_are_refused() {
    let nine: Vec<(f32, f32)> = (0..=MAX_SLOTS)
        .map(|i| on_floor(&layout(), 100.0 + 40.0 * i as f32))
        .collect();
    let room = with_slots(&nine);
    assert_eq!(refused(&room), PieceError::TooManySlots { count: 9 });
    assert!(room.validate_geometry().unwrap_err().contains("at most 8"));
}

#[test]
fn a_slot_that_is_not_a_number_is_refused_and_named() {
    for bad in [
        (f32::NAN, 440.0),
        (300.0, f32::INFINITY),
        (f32::NEG_INFINITY, 440.0),
    ] {
        let room = with_slots(&[(100.0, 450.0), bad]);
        assert_eq!(refused(&room), PieceError::SlotNotFinite { slot: 2 });
    }
}

#[test]
fn a_slot_outside_the_room_or_below_the_floor_is_refused() {
    let room = layout();
    let (left, top, width) = (room.bounds.0, room.bounds.1, room.bounds.2);
    let right = left + width;
    let half = ITEM_SIZE / 2.0;
    for outside in [
        (left + half.x - 1.0, 400.0),         // sticks out of the left wall
        (right - half.x + 1.0, 400.0),        // sticks out of the right wall
        (300.0, top + half.y - 1.0),          // sticks out of the ceiling
        (300.0, room.floor_y - half.y + 1.0), // sinks into the floor
        (-500.0, 400.0),
    ] {
        let room = with_slots(&[outside]);
        assert_eq!(
            refused(&room),
            PieceError::SlotOutsideRoom { slot: 1 },
            "{outside:?}"
        );
    }
    // Touching the wall and standing on the floor is inside.
    let touching = [
        (left + half.x, 400.0),
        (right - half.x, 400.0),
        on_floor(&room, 300.0),
    ];
    for inside in touching {
        with_slots(&[inside]).validate_geometry().unwrap();
    }
}

#[test]
fn a_slot_inside_a_solid_is_refused_but_one_resting_on_it_is_not() {
    let mut room = layout();
    room.terrain = vec![(600.0, 400.0, 100.0, 20.0)]; // top edge at y 400
    room.slots = vec![slot((650.0, 405.0), SlotKind::Key)];
    assert_eq!(refused(&room), PieceError::SlotInsideSolid { slot: 1 });
    assert!(room.validate_geometry().unwrap_err().contains("terrain"));
    room.slots = vec![slot((650.0, 400.0 - ITEM_SIZE.y / 2.0), SlotKind::Key)];
    room.validate_geometry().unwrap();
}

#[test]
fn two_slots_closer_than_an_item_are_refused_in_either_direction() {
    let (w, h) = (ITEM_SIZE.x, ITEM_SIZE.y);
    let too_close = [
        ((300.0, 300.0), (300.0 + w - 0.5, 300.0)), // side by side, overlapping
        ((300.0, 300.0), (300.0, 300.0 + h - 0.5)), // one above the other
        ((300.0, 300.0), (300.0 + w - 0.5, 300.0 + h - 0.5)), // diagonal
        ((300.0, 300.0), (300.0, 300.0)),           // on the same spot
    ];
    for (first, second) in too_close {
        let room = with_slots(&[first, second]);
        assert_eq!(
            refused(&room),
            PieceError::SlotsTooClose {
                first: 1,
                second: 2
            },
            "{first:?} {second:?}"
        );
    }
    // Exactly an item apart, they touch and do not overlap.
    for (first, second) in [
        ((300.0, 300.0), (300.0 + w, 300.0)),
        ((300.0, 300.0), (300.0, 300.0 + h)),
    ] {
        with_slots(&[first, second]).validate_geometry().unwrap();
    }
}

#[test]
fn the_error_names_the_pair_that_collides_not_the_first_slot() {
    let room = with_slots(&[(100.0, 300.0), (200.0, 300.0), (205.0, 300.0)]);
    assert_eq!(
        refused(&room),
        PieceError::SlotsTooClose {
            first: 2,
            second: 3
        }
    );
    let message = room.validate_geometry().unwrap_err();
    assert!(message.contains("slots 2 and 3"), "{message}");
}

#[test]
fn an_unmarked_room_still_has_its_slots_checked() {
    let mut room = with_slots(&[(-10.0, 300.0)]);
    room.piece = None;
    assert!(room.validate_geometry().is_err());
}

#[test]
fn removing_the_mark_keeps_the_slots_and_they_stay_valid() {
    let mut room = with_slots(&[(300.0, 450.0)]);
    room.piece = Some(PieceInfo::new(PieceRole::Middle));
    assert!(room.is_piece());
    room.piece = None;
    assert!(!room.is_piece() && room.slots.len() == 1);
    room.validate_geometry().unwrap();
}

// ---- carried through a Room, a new room and the mirror ----

#[test]
fn from_room_keeps_the_mark_and_the_slots() {
    let mut authored = with_slots(&[(300.0, 450.0), (400.0, 450.0)]);
    authored.slots[0].kind = SlotKind::Key;
    authored.piece = Some(PieceInfo {
        role: PieceRole::Boss,
        weight: 4,
    });
    let [mut room, _, _] = test_rooms();
    authored.apply(&mut room).unwrap();
    let back = RoomLayout::from_room(&room);
    assert_eq!(back.piece, authored.piece);
    assert_eq!(back.slots, authored.slots);
    assert_eq!(back, authored);
}

#[test]
fn a_new_room_does_not_inherit_the_templates_mark_or_slots() {
    let mut template = with_slots(&[(300.0, 450.0)]);
    template.piece = Some(PieceInfo::new(PieceRole::Start));
    let fresh = RoomDraft::new_room(&template);
    assert_eq!(fresh.layout.piece, None);
    assert!(fresh.layout.slots.is_empty());
}

#[test]
fn the_mirror_flips_each_slot_keeps_its_kind_and_leaves_the_mark() {
    let mut room = layout();
    let (left, width) = (room.bounds.0, room.bounds.2);
    room.slots = vec![
        slot((100.0, 300.0), SlotKind::Key),
        slot((700.0, 200.0), SlotKind::Ability),
    ];
    room.piece = Some(PieceInfo {
        role: PieceRole::Middle,
        weight: 3,
    });
    let flipped = mirror_layout(&room);
    let across = |x: f32| 2.0 * left + width - x;
    assert_eq!(
        flipped.slots[0],
        slot((across(100.0), 300.0), SlotKind::Key)
    );
    assert_eq!(
        flipped.slots[1],
        slot((across(700.0), 200.0), SlotKind::Ability)
    );
    assert_eq!(flipped.piece, room.piece);
    assert_eq!(mirror_layout(&flipped), room, "twice gives the room back");
    flipped.validate_geometry().unwrap();
}

#[test]
fn the_mirror_of_a_room_without_slots_has_none() {
    let flipped = mirror_layout(&layout());
    assert!(flipped.slots.is_empty() && flipped.piece.is_none());
}

#[test]
fn names_and_letters_are_distinct_and_the_kinds_know_what_they_hold() {
    let letters: Vec<_> = SlotKind::ALL.iter().map(|k| k.letter()).collect();
    assert_eq!(letters, ["K", "A", "?"]);
    assert_eq!(
        PieceRole::ALL.map(PieceRole::name),
        ["Start", "Middle", "Boss"]
    );
    assert!(SlotKind::Key.holds_key() && !SlotKind::Key.holds_ability());
    assert!(SlotKind::Ability.holds_ability() && !SlotKind::Ability.holds_key());
    assert!(SlotKind::Any.holds_key() && SlotKind::Any.holds_ability());
}
