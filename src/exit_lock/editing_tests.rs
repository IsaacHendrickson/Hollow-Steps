//! Locking exits in the room editor's draft: set and clear, what a refusal leaves alone, what
//! a link or a new target keeps, and keys in the Place list. The room is the fixture's first
//! room, opened in the editor; its first exit is the edge exit on the right, to the second room.
use super::*;
use crate::editor::{EditorSession, ExitPlacement, Placement, RoomLayout, Selection};
use crate::editor_history::{tracked, History};
use crate::game::GameState;
use crate::item::Item;
use crate::room::{RoomEdge, RoomId};

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn exit_lock(editor: &EditorSession, index: usize) -> Option<u8> {
    editor.draft.layout.exits[index].locked_by
}

#[test]
fn an_exit_is_locked_and_unlocked_in_the_draft_and_nothing_else_about_it_changes() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.layout.exits[0].clone();
    assert_eq!(before.locked_by, None, "the fixture exit starts unlocked");

    editor.draft.set_exit_lock(0, Some(3)).unwrap();
    let locked = editor.draft.layout.exits[0].clone();
    assert_eq!(locked.locked_by, Some(3));
    assert_eq!(
        ExitPlacement {
            locked_by: None,
            ..locked
        },
        before,
        "only the lock changed"
    );

    editor.draft.set_exit_lock(0, Some(8)).unwrap();
    assert_eq!(exit_lock(editor, 0), Some(8), "another key replaces it");
    editor.draft.set_exit_lock(0, None).unwrap();
    assert_eq!(editor.draft.layout.exits[0], before);
}

#[test]
fn a_bad_key_or_a_missing_exit_is_refused_and_the_draft_is_unchanged() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.clone();
    for key in [0, 9, 255] {
        let error = editor.draft.set_exit_lock(0, Some(key)).unwrap_err();
        assert!(error.contains("1"), "{error}");
        assert_eq!(editor.draft, before, "key {key}");
    }
    assert!(editor.draft.set_exit_lock(99, Some(1)).is_err());
    assert_eq!(editor.draft, before);
}

#[test]
fn a_door_is_locked_like_an_edge_exit() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let target = editor.rooms[0].clone();
    editor
        .draft
        .add_door(target.0, target.1.spawn.into())
        .unwrap();
    let door = editor.draft.layout.exits.len() - 1;
    assert!(editor.draft.layout.exits[door].is_door());

    editor.draft.set_exit_lock(door, Some(2)).unwrap();

    let exit = editor.draft.layout.exits[door].exit();
    assert_eq!(exit.locked_by, Some(2));
    assert!(exit.door.is_some());
    assert_eq!(editor.draft.layout.exits[0].locked_by, None);
}

#[test]
fn a_lock_set_in_the_editor_is_the_lock_of_the_live_exit_and_comes_back_through_from_room() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(5)).unwrap();

    let exit = editor.draft.layout.exits[0].exit();
    assert_eq!(exit.locked_by, Some(5));

    let [mut room, ..] = crate::room::test_rooms();
    room.exits = vec![exit];
    let layout = RoomLayout::from_room(&room);
    assert_eq!(layout.exits[0], editor.draft.layout.exits[0]);
    layout.validate_geometry().unwrap();
}

#[test]
fn the_lock_is_one_undo_step_and_undo_and_redo_bring_it_back() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    let before = editor.draft.clone();

    tracked(&mut history, editor, |e| e.draft.set_exit_lock(0, Some(4))).unwrap();
    let locked = editor.draft.clone();
    assert_eq!(exit_lock(editor, 0), Some(4));
    assert!(history.can_undo());

    let mut draft = locked.clone();
    assert_eq!(history.undo(&mut draft, |_| Ok(())), Ok(true));
    assert_eq!(draft, before);
    assert_eq!(history.redo(&mut draft, |_| Ok(())), Ok(true));
    assert_eq!(draft, locked);
    assert_eq!(history.undo(&mut draft, |_| Ok(())), Ok(true));
    assert_eq!(
        history.undo(&mut draft, |_| Ok(())),
        Ok(false),
        "it was a single step"
    );
}

#[test]
fn setting_the_lock_it_already_has_adds_no_undo_step() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();

    tracked(&mut history, editor, |e| e.draft.set_exit_lock(0, None)).unwrap();

    assert!(!history.can_undo());
}

#[test]
fn a_new_exit_target_keeps_the_lock() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(3)).unwrap();
    let boss_spawn = editor
        .rooms
        .iter()
        .find(|(id, _)| *id == RoomId::Boss)
        .map(|(_, layout)| layout.spawn)
        .unwrap();

    editor
        .draft
        .set_exit_target(0, RoomId::Boss, boss_spawn.into())
        .unwrap();

    assert_eq!(exit_lock(editor, 0), Some(3));
    assert_eq!(editor.draft.layout.exits[0].target_room, RoomId::Boss);
}

#[test]
fn a_link_that_replaces_the_target_keeps_the_lock_of_the_exit_it_started_from() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(3)).unwrap();
    let from = editor.id;
    assert_eq!(editor.draft.layout.exits[0].target_room, RoomId::Second);

    editor.begin_link(0).unwrap();
    editor.link_choose(RoomId::Boss).unwrap();
    editor.link_connect(0).unwrap();

    assert_eq!(editor.id, from, "back in the room the link started from");
    assert_eq!(editor.draft.layout.exits[0].target_room, RoomId::Boss);
    assert_eq!(exit_lock(editor, 0), Some(3), "the lock is still there");
}

#[test]
fn a_two_way_link_keeps_the_lock_of_both_exits() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(3)).unwrap();

    editor.begin_link(0).unwrap();
    editor.link_choose(RoomId::Boss).unwrap();
    // The boss room's own exit is locked too, then linked back both ways.
    editor.draft.set_exit_lock(0, Some(7)).unwrap();
    editor.link_set_two_way(true);
    editor.link_connect(0).unwrap();

    assert_eq!(exit_lock(editor, 0), Some(3));
    let boss = editor
        .parked
        .iter()
        .find(|work| work.id == RoomId::Boss)
        .expect("the linked room waits among the parked drafts");
    assert_eq!(boss.draft.layout.exits[0].locked_by, Some(7));
    assert_eq!(boss.draft.layout.exits[0].target_room, editor.id);
}

#[test]
fn a_new_exit_starts_unlocked_even_next_to_a_locked_one() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(3)).unwrap();
    let target = editor.rooms[1].clone();

    editor
        .draft
        .add_exit(RoomEdge::Left, target.0, target.1.spawn.into());

    let last = editor.draft.layout.exits.len() - 1;
    assert_eq!(exit_lock(editor, last), None);
    assert_eq!(exit_lock(editor, 0), Some(3));
}

#[test]
fn every_key_is_in_the_place_list_and_places_as_an_item() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let catalog = editor.placement_catalog();

    for key in FIRST_KEY..=LAST_KEY {
        let label = format!("Item: Key {key}");
        let entry = catalog
            .iter()
            .find(|(name, _)| *name == label)
            .unwrap_or_else(|| panic!("no {label} in the Place list"));
        assert!(
            matches!(entry.1, Placement::Item(item) if item == Item::Key(key)),
            "{label} places the wrong thing"
        );
    }
    assert_eq!(
        catalog.iter().filter(|(n, _)| n.contains("Key ")).count(),
        8
    );

    let items = editor.draft.layout.items.len();
    let mut history = History::default();
    let selection = tracked(&mut history, editor, |e| {
        e.place(Placement::Item(Item::Key(6)), Vec2::new(300.0, 440.0))
    })
    .unwrap();
    assert_eq!(selection, Selection::Item(items));
    assert_eq!(editor.draft.layout.items[items].item, Item::Key(6));
    assert!(history.can_undo(), "placing a key is one undo step");
}

#[test]
fn a_placed_key_with_a_bad_number_is_refused_when_the_room_is_checked() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.clone();

    let refused = editor.place(Placement::Item(Item::Key(9)), Vec2::new(300.0, 440.0));

    assert!(refused.is_err());
    assert_eq!(editor.draft, before);
}
