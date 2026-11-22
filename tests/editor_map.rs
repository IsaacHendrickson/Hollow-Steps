//! The editor's side of the dungeon map: a room's own cell on its draft (set, clear, undo), the
//! map of every room the session knows (saved rooms and unsaved drafts), and Check dungeon
//! reporting two rooms that name one cell. The window and its click are tested with the panel.
//! Editing First (exit 0 leads to Second, a Right exit); the fixture rooms have no cell.
mod support;

use hollow_steps::dungeon_check::Problem;
use hollow_steps::dungeon_map::CellSource;
use hollow_steps::editor_history::{redo_session, tracked, undo_session, History};
use hollow_steps::editor_session::EditorSession;
use hollow_steps::game::GameState;
use hollow_steps::room::RoomId;

fn editing() -> GameState {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn editor(game: &mut GameState) -> &mut EditorSession {
    game.editor_mut().expect("the editor is open")
}

fn same_cell_problems(session: &EditorSession) -> Vec<Problem> {
    session
        .check_dungeon()
        .into_iter()
        .filter(|problem| matches!(problem, Problem::SameMapCell { .. }))
        .collect()
}

#[test]
fn a_cell_is_set_and_cleared_as_one_undo_step_each() {
    let mut game = editing();
    let session = editor(&mut game);
    let mut history = History::default();
    assert_eq!(session.draft.layout.map_cell, None);

    tracked(&mut history, session, |e| {
        e.draft.set_map_cell(Some((2, -1)))
    })
    .unwrap();
    assert_eq!(session.draft.layout.map_cell, Some((2, -1)));
    tracked(&mut history, session, |e| e.draft.set_map_cell(None)).unwrap();
    assert_eq!(session.draft.layout.map_cell, None);

    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert_eq!(
        session.draft.layout.map_cell,
        Some((2, -1)),
        "one step back"
    );
    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert_eq!(session.draft.layout.map_cell, None, "and the other");
    assert_eq!(
        undo_session(&mut history, session),
        Ok(false),
        "nothing else was recorded"
    );
    assert_eq!(redo_session(&mut history, session), Ok(true));
    assert_eq!(session.draft.layout.map_cell, Some((2, -1)));
}

#[test]
fn setting_the_cell_it_already_has_records_nothing() {
    let mut game = editing();
    let session = editor(&mut game);
    let mut history = History::default();
    tracked(&mut history, session, |e| e.draft.set_map_cell(None)).unwrap();
    assert!(!history.can_undo());
}

#[test]
fn a_cell_out_of_range_is_refused_and_the_draft_is_unchanged() {
    let mut game = editing();
    let session = editor(&mut game);
    session.draft.set_map_cell(Some((3, 4))).unwrap();
    let mut history = History::default();
    for cell in [(65, 0), (0, -65), (i32::MAX, 0)] {
        let refused = tracked(&mut history, session, |e| e.draft.set_map_cell(Some(cell)));
        assert!(refused.unwrap_err().contains("out of range"), "{cell:?}");
        assert_eq!(session.draft.layout.map_cell, Some((3, 4)));
    }
    assert!(!history.can_undo(), "a refused edit is not a step");
    session.draft.set_map_cell(Some((64, -64))).unwrap();
}

#[test]
fn the_cell_is_part_of_the_draft_that_save_would_write() {
    let mut game = editing();
    let session = editor(&mut game);
    session.draft.set_map_cell(Some((-7, 9))).unwrap();
    let text = session
        .draft
        .serialize(&session.definitions, &session.catalog())
        .unwrap();
    assert!(text.contains("map_cell: Some((-7, 9))"), "{text}");
}

#[test]
fn the_map_lists_every_saved_room_and_the_room_being_edited_with_its_unsaved_change() {
    let mut game = editing();
    let session = editor(&mut game);
    let before = session.dungeon_map();
    let ids: Vec<_> = before.rooms.iter().map(|room| room.id).collect();
    assert_eq!(ids, [RoomId::First, RoomId::Second, RoomId::Boss]);

    session.draft.set_map_cell(Some((5, 5))).unwrap();
    assert_eq!(
        session.dungeon_map().room(RoomId::First).unwrap().cell,
        (5, 5)
    );
    assert_eq!(
        session
            .rooms
            .iter()
            .find(|(id, _)| *id == RoomId::First)
            .unwrap()
            .1
            .map_cell,
        None,
        "the saved layout is untouched until Save"
    );
}

#[test]
fn the_map_lists_a_new_room_and_a_parked_draft_that_are_not_saved() {
    let mut game = editing();
    let session = editor(&mut game);
    // A new room first (starting one clears the parked list), then a parked draft: Second, edited
    // and set aside with its change.
    session.new_room_named("Cellar").unwrap();
    let mut second = session.current_work();
    second.id = RoomId::Second;
    second.draft = session
        .rooms
        .iter()
        .find(|(id, _)| *id == RoomId::Second)
        .map(|(_, layout)| {
            let mut draft = session.draft.clone();
            draft.layout = layout.clone();
            draft
        })
        .unwrap();
    second.draft.layout.map_cell = Some((-3, 0));
    second.dirty = true;
    session.parked.push(second);

    let map = session.dungeon_map();
    let cellar = session.id;
    assert!(matches!(cellar, RoomId::Custom(_)));
    let names: Vec<_> = map.rooms.iter().map(|room| room.name.as_str()).collect();
    assert!(names.contains(&"Cellar"), "{names:?}");
    assert_eq!(
        map.room(RoomId::Second).unwrap().cell,
        (-3, 0),
        "the parked draft's cell"
    );
    assert_eq!(
        map.room(RoomId::Second).unwrap().source,
        CellSource::Explicit
    );
    assert!(
        map.room(cellar).is_some(),
        "the new room is on the map before it has a file"
    );
    assert_eq!(map.rooms.len(), 4);
}

#[test]
fn the_resolved_cell_is_the_rooms_own_when_it_has_one_and_where_the_exits_put_it_when_not() {
    let mut game = editing();
    let session = editor(&mut game);
    // First has a Right exit to Second, which has an exit back; nothing names a cell.
    let walked = session.resolved_map_cell().unwrap();
    let second = session.dungeon_map().room(RoomId::Second).unwrap().cell;
    assert_ne!(walked, second);

    session.draft.set_map_cell(Some((7, 7))).unwrap();
    assert_eq!(session.resolved_map_cell(), Some((7, 7)));
    session.draft.set_map_cell(None).unwrap();
    assert_eq!(session.resolved_map_cell(), Some(walked));
}

#[test]
fn check_dungeon_names_both_rooms_that_have_the_same_cell() {
    let mut game = editing();
    let session = editor(&mut game);
    assert_eq!(
        same_cell_problems(session),
        [],
        "the fixture rooms name no cell"
    );

    // First (being edited) and Second (a saved room with a cell of its own) name one cell.
    session.draft.set_map_cell(Some((2, 2))).unwrap();
    session
        .rooms
        .iter_mut()
        .find(|(id, _)| *id == RoomId::Second)
        .unwrap()
        .1
        .map_cell = Some((2, 2));
    let found = same_cell_problems(session);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(
        found[0].text(),
        "First and Second both have the map cell (2, 2); the map moves Second"
    );
    let map = session.dungeon_map();
    assert!(map.in_conflict(RoomId::First) && map.in_conflict(RoomId::Second));
    assert_ne!(map.room(RoomId::Second).unwrap().cell, (2, 2));

    // Giving one of them another cell, or none, ends the problem.
    session.draft.set_map_cell(None).unwrap();
    assert_eq!(same_cell_problems(session), []);
}

#[test]
fn three_rooms_on_one_cell_give_two_problems_that_all_name_the_room_that_keeps_it() {
    let mut game = editing();
    let session = editor(&mut game);
    session.draft.set_map_cell(Some((0, 0))).unwrap();
    for (id, layout) in &mut session.rooms {
        if matches!(id, RoomId::Second | RoomId::Boss) {
            layout.map_cell = Some((0, 0));
        }
    }
    let texts: Vec<_> = same_cell_problems(session)
        .iter()
        .map(Problem::text)
        .collect();
    assert_eq!(
        texts,
        [
            "First and Second both have the map cell (0, 0); the map moves Second",
            "First and Boss both have the map cell (0, 0); the map moves Boss",
        ]
    );
}
