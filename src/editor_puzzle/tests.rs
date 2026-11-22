use super::*;
use crate::editor_history::{redo_session, tracked, undo_session, History};
use crate::enemy::EnemyDefinitions;
use crate::game::GameState;
use crate::room::test_rooms;

/// The first sample room: 960 wide, floor at 460, spawn at (180, 436), no pieces.
fn draft() -> RoomDraft {
    let [room, _, _] = test_rooms();
    RoomDraft::new_room(&RoomLayout::from_room(&room))
}

fn no_definitions() -> EnemyDefinitions {
    EnemyDefinitions::from_definitions(Vec::new(), Default::default(), Default::default()).unwrap()
}

/// A draft with switch 1 and a barrier it opens.
fn with_a_pair() -> RoomDraft {
    let mut draft = draft();
    draft.add_switch().unwrap();
    draft.add_barrier().unwrap();
    draft
}

// ---- add ----

#[test]
fn add_puts_a_standard_piece_on_the_floor_right_of_the_spawn_and_selects_it() {
    let mut draft = draft();
    let (spawn_x, _) = draft.layout.spawn;
    let floor = draft.layout.floor_y;

    assert_eq!(draft.add_breakable().unwrap(), Selection::Breakable(0));
    let wall = &draft.layout.breakables[0];
    assert_eq!(
        wall.rect,
        (spawn_x + SPAWN_CLEARANCE, floor - 64.0, 32.0, 64.0)
    );
    assert_eq!(wall.hits, 3);

    assert_eq!(draft.add_switch().unwrap(), Selection::Switch(0));
    let switch = &draft.layout.switches[0];
    assert_eq!(
        switch.rect.1 + switch.rect.3,
        floor,
        "standing on the floor"
    );
    assert_eq!(switch.id, 1);

    assert_eq!(draft.add_barrier().unwrap(), Selection::Barrier(0));
    let barrier = &draft.layout.barriers[0];
    assert_eq!(barrier.rect.1 + barrier.rect.3, floor);
    assert_eq!(barrier.switch, 1);
    assert_eq!(draft.layout.validate_geometry(), Ok(()));

    // A second one goes to the same spot and is the next index.
    assert_eq!(draft.add_breakable().unwrap(), Selection::Breakable(1));
}

#[test]
fn a_new_switch_takes_the_lowest_free_number_and_a_ninth_is_refused() {
    let mut draft = draft();
    for _ in 0..8 {
        draft.add_switch().unwrap();
    }
    let ids: Vec<u8> = draft.layout.switches.iter().map(|s| s.id).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6, 7, 8]);
    let before = draft.clone();
    let error = draft.add_switch().unwrap_err();
    assert!(error.contains('8'), "{error}");
    assert_eq!(draft, before, "a refused add changes nothing");

    draft.delete(Selection::Switch(2)).unwrap();
    assert_eq!(draft.add_switch().unwrap(), Selection::Switch(7));
    assert_eq!(draft.layout.switches[7].id, 3, "the gap is filled");
}

#[test]
fn a_barrier_is_refused_in_a_room_without_a_switch_and_uses_the_first_one_otherwise() {
    let mut draft = draft();
    let before = draft.clone();
    let error = draft.add_barrier().unwrap_err();
    assert!(error.contains("switch"), "{error}");
    assert_eq!(draft, before);

    draft.add_switch().unwrap();
    draft.add_switch().unwrap();
    draft.add_barrier().unwrap();
    assert_eq!(draft.layout.barriers[0].switch, 1);
}

#[test]
fn add_uses_the_left_side_when_the_right_has_no_room() {
    let mut draft = draft();
    let right = draft.layout.rect().right();
    draft.layout.spawn.0 = right - 40.0;
    draft.add_breakable().unwrap();
    let (x, _, w, _) = draft.layout.breakables[0].rect;
    assert!(x + w <= draft.layout.spawn.0 - 16.0, "left of the player");
}

#[test]
fn add_is_refused_when_the_room_is_too_narrow_for_it() {
    let mut draft = draft();
    draft.layout.bounds.2 = 64.0;
    draft.layout.spawn.0 = 32.0;
    let before = draft.clone();
    assert!(draft.add_breakable().is_err());
    assert_eq!(draft, before, "a refused add changes nothing");
}

// ---- move ----

#[test]
fn move_sets_the_center_keeps_the_size_and_stays_inside_the_room() {
    let definitions = no_definitions();
    let mut draft = with_a_pair();
    draft.add_breakable().unwrap();
    let room = draft.layout.rect();
    for selection in [
        Selection::Breakable(0),
        Selection::Switch(0),
        Selection::Barrier(0),
    ] {
        let size = {
            let fields = draft.piece_fields(selection).unwrap();
            (fields.width, fields.height)
        };
        draft
            .move_to(selection, Vec2::new(600.0, 300.0), &definitions)
            .unwrap();
        let (rect, _) = rect_of(&draft, selection);
        assert_eq!(
            rect,
            (600.0 - size.0 / 2.0, 300.0 - size.1 / 2.0, size.0, size.1)
        );
        draft
            .move_to(selection, Vec2::new(-500.0, 5000.0), &definitions)
            .unwrap();
        let (rect, _) = rect_of(&draft, selection);
        assert_eq!(
            (rect.0, rect.1 + rect.3),
            (room.x, room.bottom()),
            "clamped"
        );
        assert_eq!((rect.2, rect.3), size);
    }
}

fn rect_of(draft: &RoomDraft, selection: Selection) -> ((f32, f32, f32, f32), u8) {
    match selection {
        Selection::Breakable(i) => (
            draft.layout.breakables[i].rect,
            draft.layout.breakables[i].hits,
        ),
        Selection::Switch(i) => (draft.layout.switches[i].rect, draft.layout.switches[i].id),
        Selection::Barrier(i) => (
            draft.layout.barriers[i].rect,
            draft.layout.barriers[i].switch,
        ),
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_move_onto_the_spawn_or_to_a_bad_position_is_refused_and_changes_nothing() {
    let definitions = no_definitions();
    let mut draft = with_a_pair();
    let before = draft.clone();
    let spawn: Vec2 = draft.layout.spawn.into();
    for selection in [Selection::Switch(0), Selection::Barrier(0)] {
        let error = draft.move_to(selection, spawn, &definitions).unwrap_err();
        assert!(error.contains("spawn"), "{error}");
        assert_eq!(draft, before);
        assert!(draft
            .move_to(selection, Vec2::new(f32::NAN, 0.0), &definitions)
            .is_err());
    }
    assert!(draft
        .move_to(Selection::Barrier(7), Vec2::ZERO, &definitions)
        .is_err());
    assert_eq!(draft, before);
}

// ---- change ----

#[test]
fn a_new_size_keeps_the_center_line_and_the_bottom_edge() {
    let mut draft = draft();
    draft.add_breakable().unwrap();
    let (x, y, w, h) = draft.layout.breakables[0].rect;
    let fields = PieceFields {
        width: 48.0,
        height: 128.0,
        number: 5,
    };
    draft
        .set_piece_fields(Selection::Breakable(0), fields)
        .unwrap();
    let wall = &draft.layout.breakables[0];
    assert_eq!(wall.rect, (x - 8.0, y + h - 128.0, 48.0, 128.0));
    assert_eq!(wall.rect.0 + wall.rect.2 / 2.0, x + w / 2.0);
    assert_eq!(wall.hits, 5);
    assert_eq!(draft.piece_fields(Selection::Breakable(0)), Some(fields));
}

#[test]
fn a_refused_change_names_the_reason_and_leaves_the_draft_as_it_was() {
    let mut draft = with_a_pair();
    draft.add_switch().unwrap();
    draft.add_breakable().unwrap();
    let before = draft.clone();
    let fields = |draft: &RoomDraft, selection| draft.piece_fields(selection).unwrap();
    type Change = fn(PieceFields) -> PieceFields;
    let refused: [(Selection, Change); 7] = [
        // Too small.
        (Selection::Switch(0), |f: PieceFields| PieceFields {
            width: 7.0,
            ..f
        }),
        // Out of the room: a wall 2000 tall cannot stand on a 460 floor.
        (Selection::Breakable(0), |f| PieceFields {
            height: 2000.0,
            ..f
        }),
        (Selection::Breakable(0), |f| PieceFields { number: 0, ..f }),
        (Selection::Breakable(0), |f| PieceFields { number: 10, ..f }),
        (Selection::Switch(0), |f| PieceFields { number: 9, ..f }),
        (Selection::Barrier(0), |f| PieceFields { number: 0, ..f }),
        // Switch 2 is already taken.
        (Selection::Switch(0), |f| PieceFields { number: 2, ..f }),
    ];
    for (selection, change) in refused {
        let edited = change(fields(&draft, selection));
        assert!(
            draft.set_piece_fields(selection, edited).is_err(),
            "{selection:?} {edited:?}"
        );
        assert_eq!(draft, before);
    }
    assert!(draft
        .set_piece_fields(
            Selection::Barrier(9),
            PieceFields {
                width: 8.0,
                height: 8.0,
                number: 1
            }
        )
        .is_err());
    assert!(draft
        .set_piece_fields(Selection::Spawn, fields(&draft, Selection::Switch(0)))
        .is_err());
}

#[test]
fn a_barrier_can_be_pointed_at_another_switch_of_the_room_but_not_at_a_missing_one() {
    let mut draft = with_a_pair();
    draft.add_switch().unwrap();
    let fields = draft.piece_fields(Selection::Barrier(0)).unwrap();
    draft
        .set_piece_fields(
            Selection::Barrier(0),
            PieceFields {
                number: 2,
                ..fields
            },
        )
        .unwrap();
    assert_eq!(draft.layout.barriers[0].switch, 2);
    let error = draft
        .set_piece_fields(
            Selection::Barrier(0),
            PieceFields {
                number: 5,
                ..fields
            },
        )
        .unwrap_err();
    assert!(
        error.contains("barrier 0") && error.contains("switch 5"),
        "{error}"
    );
    assert_eq!(draft.layout.barriers[0].switch, 2);
}

#[test]
fn renumbering_a_switch_that_a_barrier_uses_is_refused_with_the_barrier_named() {
    let mut draft = with_a_pair();
    let fields = draft.piece_fields(Selection::Switch(0)).unwrap();
    let error = draft
        .set_piece_fields(
            Selection::Switch(0),
            PieceFields {
                number: 4,
                ..fields
            },
        )
        .unwrap_err();
    assert!(error.contains("barrier 0"), "{error}");
    assert_eq!(draft.layout.switches[0].id, 1);
}

// ---- delete ----

#[test]
fn delete_takes_a_piece_out_and_refuses_one_that_is_not_there() {
    let mut draft = with_a_pair();
    draft.add_breakable().unwrap();
    draft.delete(Selection::Breakable(0)).unwrap();
    draft.delete(Selection::Barrier(0)).unwrap();
    draft.delete(Selection::Switch(0)).unwrap();
    assert!(draft.layout.breakables.is_empty());
    assert!(draft.layout.barriers.is_empty());
    assert!(draft.layout.switches.is_empty());
    let before = draft.clone();
    for selection in [
        Selection::Breakable(0),
        Selection::Switch(0),
        Selection::Barrier(3),
    ] {
        assert!(draft.delete(selection).is_err());
    }
    assert_eq!(draft, before);
}

#[test]
fn a_barrier_whose_switch_is_deleted_is_refused_and_the_barrier_is_named() {
    let mut draft = with_a_pair();
    draft.add_barrier().unwrap();
    let before = draft.clone();
    let error = draft.delete(Selection::Switch(0)).unwrap_err();
    assert!(error.contains("barrier 0"), "{error}");
    assert_eq!(draft, before, "nothing was deleted");
    // Another switch left in the room does not help a barrier that needs this one.
    draft.add_switch().unwrap();
    assert!(draft.delete(Selection::Switch(0)).is_err());
    // Once the barriers are gone the switch can go.
    draft.delete(Selection::Barrier(1)).unwrap();
    draft.delete(Selection::Barrier(0)).unwrap();
    draft.delete(Selection::Switch(0)).unwrap();
    assert_eq!(draft.layout.switches.len(), 1);
}

#[test]
fn deleting_a_piece_ahead_of_others_shifts_the_later_ones_down() {
    let mut draft = draft();
    for _ in 0..3 {
        draft.add_switch().unwrap();
    }
    draft.delete(Selection::Switch(0)).unwrap();
    let ids: Vec<u8> = draft.layout.switches.iter().map(|s| s.id).collect();
    assert_eq!(ids, [2, 3]);
}

// ---- undo ----

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

#[test]
fn add_move_resize_and_delete_are_each_one_undo_step() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    let mut states = vec![session.draft.clone()];

    tracked(&mut history, session, |e| e.draft.add_switch()).unwrap();
    states.push(session.draft.clone());
    tracked(&mut history, session, |e| e.draft.add_barrier()).unwrap();
    states.push(session.draft.clone());
    tracked(&mut history, session, |e| {
        let fields = e.draft.piece_fields(Selection::Barrier(0)).ok_or("gone")?;
        e.draft.set_piece_fields(
            Selection::Barrier(0),
            PieceFields {
                height: 150.0,
                ..fields
            },
        )
    })
    .unwrap();
    states.push(session.draft.clone());
    tracked(&mut history, session, |e| {
        e.draft.move_to(
            Selection::Barrier(0),
            Vec2::new(500.0, 300.0),
            &e.definitions,
        )
    })
    .unwrap();
    states.push(session.draft.clone());
    tracked(&mut history, session, |e| {
        e.draft.delete(Selection::Barrier(0))
    })
    .unwrap();
    assert!(session.draft.layout.barriers.is_empty());

    for expected in states.iter().rev() {
        assert!(undo_session(&mut history, session).unwrap());
        assert_eq!(&session.draft, expected);
    }
    assert!(!history.can_undo(), "five edits, five steps");
}

#[test]
fn a_refused_delete_records_no_undo_step() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    tracked(&mut history, session, |e| e.draft.add_switch()).unwrap();
    tracked(&mut history, session, |e| e.draft.add_barrier()).unwrap();
    let before = session.draft.clone();
    let error = tracked(&mut history, session, |e| {
        e.draft.delete(Selection::Switch(0))
    })
    .unwrap_err();
    assert!(error.contains("barrier 0"), "{error}");
    assert_eq!(session.draft, before);
    undo_session(&mut history, session).unwrap();
    undo_session(&mut history, session).unwrap();
    assert!(!history.can_undo(), "only the two adds were recorded");
}

#[test]
fn a_whole_drag_of_a_piece_marks_the_room_unsaved_and_is_one_undo_step() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    let mut history = History::default();
    for add in [RoomDraft::add_breakable, RoomDraft::add_switch] {
        tracked(&mut history, session, |e| add(&mut e.draft)).unwrap();
    }
    let selections = [Selection::Breakable(0), Selection::Switch(0)];
    for selection in selections {
        session.dirty = false;
        let before = session.draft.clone();
        history.begin_gesture(&session.draft);
        for x in [500.0, 520.0, 540.0, 560.0] {
            session
                .drag_selection(selection, Vec2::new(x, 300.0))
                .unwrap();
        }
        history.end_gesture(&session.draft);
        assert!(session.dirty, "{selection:?} dragged");
        assert!(
            session.has_unsaved(),
            "{selection:?}: the unsaved-changes guard sees it"
        );
        assert_ne!(session.draft, before);

        assert!(undo_session(&mut history, session).unwrap());
        assert_eq!(
            session.draft, before,
            "{selection:?}: one undo undoes the whole drag"
        );
        assert!(redo_session(&mut history, session).unwrap());
        assert_ne!(
            session.draft, before,
            "{selection:?}: and redo brings it back"
        );
        // Leave the drag in place for the next piece, with its undo step on the stack.
    }
}

#[test]
fn a_drag_onto_the_spawn_is_refused_mid_drag_and_leaves_the_piece_where_it_was() {
    let mut game = editing();
    let session = game.editor_mut().unwrap();
    session.draft.add_switch().unwrap();
    let before = session.draft.clone();
    let spawn: Vec2 = session.draft.layout.spawn.into();
    session.dirty = false;
    let error = session
        .drag_selection(Selection::Switch(0), spawn)
        .unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert_eq!(session.draft, before);
    assert!(!session.dirty, "nothing changed");
}
