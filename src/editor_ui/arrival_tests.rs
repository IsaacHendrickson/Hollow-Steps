//! The arrival markers as the canvas draws and hit-tests them, without a window: which
//! markers exist when, what is pressable while exits are linked, what a press selects and
//! what the labels say.
use super::*;
use hollow_steps::editor::EditorSession;
use hollow_steps::room::RoomId;

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

const SECOND_TO_FIRST: Selection = Selection::Inbound {
    room: RoomId::Second,
    exit: 0,
};

fn marker_of(
    editor: &EditorSession,
    selected: Option<Selection>,
    wanted: Selection,
) -> Option<(Rect, Color)> {
    editor_markers(editor, None, selected)
        .into_iter()
        .find(|(selection, _, _)| *selection == wanted)
        .map(|(_, rect, color)| (rect, color))
}

fn in_second(editor: &mut EditorSession) {
    editor.begin_link(0).unwrap();
    editor.link_choose(RoomId::Second).unwrap();
}

#[test]
fn an_exit_from_another_room_shows_its_arrival_as_a_purple_player_sized_marker() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let (rect, color) = marker_of(editor, None, SECOND_TO_FIRST).unwrap();
    assert_eq!(rect.center(), vec2(860.0, 436.0));
    assert_eq!(rect.size(), vec2(32.0, 48.0));
    assert_eq!(color, PURPLE);
}

#[test]
fn an_arrival_that_does_not_fit_any_more_is_red() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.layout.terrain.push((820.0, 400.0, 80.0, 60.0));
    let (_, color) = marker_of(editor, None, SECOND_TO_FIRST).unwrap();
    assert_eq!(color, RED);
}

#[test]
fn a_moved_arrival_is_drawn_where_it_was_moved() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor
        .drag_selection(SECOND_TO_FIRST, vec2(700.0, 436.0))
        .unwrap();
    let (rect, _) = marker_of(editor, None, SECOND_TO_FIRST).unwrap();
    assert_eq!(rect.center(), vec2(700.0, 436.0));
}

#[test]
fn no_inbound_markers_while_a_link_is_open_or_on_the_destination_view() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    assert!(editor_markers(editor, Some(0), None)
        .iter()
        .all(|(selection, _, _)| matches!(selection, Selection::Destination(_))));
    editor.begin_link(0).unwrap();
    assert!(marker_of(editor, None, SECOND_TO_FIRST).is_none());
    editor.link_choose(RoomId::Second).unwrap();
    assert!(editor_markers(editor, None, None)
        .iter()
        .all(|(selection, _, _)| !matches!(selection, Selection::Inbound { .. })));
}

#[test]
fn the_link_arrival_marker_follows_the_selected_exit_of_the_open_room() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    in_second(editor);
    assert!(marker_of(editor, None, Selection::LinkArrival(1)).is_none());
    let (rect, color) =
        marker_of(editor, Some(Selection::Exit(1)), Selection::LinkArrival(1)).unwrap();
    assert_eq!(rect.center(), editor.link_arrival(1).unwrap());
    assert_eq!((rect.size(), color), (vec2(32.0, 48.0), PURPLE));
    // A different exit gets its own marker, and only that one.
    let markers = editor_markers(editor, None, Some(Selection::Exit(0)));
    let arrivals: Vec<_> = markers
        .iter()
        .filter(|(selection, _, _)| matches!(selection, Selection::LinkArrival(_)))
        .collect();
    assert_eq!(arrivals.len(), 1);
    assert_eq!(arrivals[0].0, Selection::LinkArrival(0));
}

#[test]
fn the_link_arrival_marker_moves_when_it_is_dragged() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    in_second(editor);
    editor
        .drag_selection(Selection::LinkArrival(1), vec2(600.0, 352.0))
        .unwrap();
    let (rect, _) = marker_of(editor, Some(Selection::Exit(1)), Selection::LinkArrival(1)).unwrap();
    assert_eq!(rect.center(), vec2(600.0, 352.0));
}

#[test]
fn there_is_no_link_arrival_marker_outside_the_link() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    assert!(marker_of(editor, Some(Selection::Exit(0)), Selection::LinkArrival(0)).is_none());
}

#[test]
fn the_link_arrival_is_drawn_above_the_exits_so_it_is_pressed_first() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    in_second(editor);
    let markers = editor_markers(editor, None, Some(Selection::Exit(1)));
    let last = markers.last().unwrap().0;
    assert_eq!(last, Selection::LinkArrival(1));
}

#[test]
fn while_linking_only_the_exits_and_their_arrival_can_be_pressed() {
    assert!(linking::live_while_linking(Selection::Exit(0)));
    assert!(linking::live_while_linking(Selection::LinkArrival(0)));
    for other in [
        SECOND_TO_FIRST,
        Selection::Spawn,
        Selection::Item(0),
        Selection::Enemy(0),
        Selection::Boss,
        Selection::Destination(0),
    ] {
        assert!(!linking::live_while_linking(other), "{other:?}");
    }
}

#[test]
fn pressing_the_link_arrival_keeps_its_exit_selected() {
    assert_eq!(
        selected_by_press(Selection::LinkArrival(3)),
        Selection::Exit(3)
    );
    assert_eq!(
        selected_by_press(Selection::PatrolLeft(2)),
        Selection::Enemy(2)
    );
    assert_eq!(
        selected_by_press(Selection::PatrolRight(2)),
        Selection::Enemy(2)
    );
    assert_eq!(selected_by_press(SECOND_TO_FIRST), SECOND_TO_FIRST);
    assert_eq!(selected_by_press(Selection::Spawn), Selection::Spawn);
}

#[test]
fn the_labels_name_the_room_an_arrival_comes_from_and_what_is_wrong_with_it() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let at = vec2(860.0, 436.0);
    assert_eq!(
        arrival_labels::label(editor, SECOND_TO_FIRST, at).as_deref(),
        Some("from Second")
    );
    editor.draft.layout.terrain.push((820.0, 400.0, 80.0, 60.0));
    assert_eq!(
        arrival_labels::label(editor, SECOND_TO_FIRST, at).as_deref(),
        Some("from Second: inside a solid")
    );
    assert_eq!(
        arrival_labels::label(editor, Selection::LinkArrival(0), at).as_deref(),
        Some("Arrival")
    );
    for none in [Selection::Spawn, Selection::Exit(0), Selection::Item(0)] {
        assert!(arrival_labels::label(editor, none, at).is_none());
    }
}

#[test]
fn an_inbound_arrival_is_not_erasable_and_deleting_it_changes_nothing() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.clone();
    assert!(editor.draft.delete(SECOND_TO_FIRST).is_err());
    assert!(editor.draft.delete(Selection::LinkArrival(0)).is_err());
    assert_eq!(editor.draft, before);
}
