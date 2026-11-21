//! What the canvas writes for locked exits and keys, as a pure function of the markers.
use super::lock_labels::{labels, LockLabel};
use super::markers::editor_markers;
use super::*;
use hollow_steps::editor::{EditorSession, Placement};
use hollow_steps::item::Item;
use hollow_steps::room::{RoomEdge, RoomId};

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn written(editor: &EditorSession) -> Vec<LockLabel> {
    labels(editor, &editor_markers(editor, None, None))
}

fn texts(editor: &EditorSession) -> Vec<String> {
    written(editor)
        .into_iter()
        .map(|label| label.text)
        .collect()
}

#[test]
fn a_room_without_locks_or_keys_has_no_lock_labels() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    assert!(
        !editor.draft.layout.items.is_empty(),
        "the shard and the Dash are there"
    );
    assert_eq!(written(editor), []);
}

#[test]
fn a_locked_exit_says_its_key_beside_its_band_and_unlocking_removes_it() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(3)).unwrap();

    let all = written(editor);

    assert_eq!(all.len(), 1);
    let label = &all[0];
    assert_eq!(label.text, "Key 3");
    assert_eq!(label.edge, RoomEdge::Right);
    assert!(!label.centered);
    let band = editor.draft.layout.exits[0]
        .exit()
        .zone(editor.draft.layout.rect());
    assert_eq!(label.marker, band, "it belongs to the exit's band");

    editor.draft.set_exit_lock(0, None).unwrap();
    assert_eq!(written(editor), []);
}

#[test]
fn a_locked_door_is_labelled_in_its_middle() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let (target, layout) = editor.rooms[0].clone();
    editor.draft.add_door(target, layout.spawn.into()).unwrap();
    let door = editor.draft.layout.exits.len() - 1;
    editor.draft.set_exit_lock(door, Some(2)).unwrap();

    let all = written(editor);

    assert_eq!(all.len(), 1);
    assert_eq!(all[0].text, "Key 2");
    assert!(all[0].centered);
}

#[test]
fn every_key_is_labelled_with_its_number_on_its_marker_and_other_items_are_not() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.layout.items.len();
    for key in 1..=8u8 {
        editor
            .place(
                Placement::Item(Item::Key(key)),
                vec2(100.0 + 40.0 * f32::from(key), 440.0),
            )
            .unwrap();
    }
    let markers = editor_markers(editor, None, None);

    let all = labels(editor, &markers);

    assert_eq!(
        all.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["1", "2", "3", "4", "5", "6", "7", "8"],
        "the shard and the Dash have none"
    );
    for (index, label) in all.iter().enumerate() {
        assert!(label.centered);
        let marker = markers
            .iter()
            .find(|(selection, _, _)| *selection == Selection::Item(before + index))
            .map(|(_, rect, _)| *rect)
            .unwrap();
        assert_eq!(label.marker, marker);
    }
}

#[test]
fn locks_and_keys_are_labelled_together_in_marker_order() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(1)).unwrap();
    editor
        .place(Placement::Item(Item::Key(1)), vec2(300.0, 440.0))
        .unwrap();

    assert_eq!(texts(editor), ["1", "Key 1"], "items come before exits");
}

#[test]
fn no_lock_labels_while_one_exit_s_arrival_is_shown() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.set_exit_lock(0, Some(3)).unwrap();
    editor
        .place(Placement::Item(Item::Key(3)), vec2(300.0, 440.0))
        .unwrap();

    let markers = editor_markers(editor, Some(0), None);

    assert_eq!(
        labels(editor, &markers),
        [],
        "only the purple arrival is drawn"
    );
    assert_eq!(editor.id, RoomId::First);
}
