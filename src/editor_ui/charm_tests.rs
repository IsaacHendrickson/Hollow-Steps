use super::markers::{editor_markers, selected_by_press};
use super::*;
use hollow_steps::editor::EditorSession;
use hollow_steps::room_charms::{CharmPlacement, CHARM_SIZE};

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn place(editor: &mut EditorSession, id: &str, at: (f32, f32)) {
    editor.draft.layout.charms.push(CharmPlacement {
        charm: id.into(),
        position: at,
    });
}

#[test]
fn a_placed_charm_is_a_marker_drawn_as_its_swatch_and_sized_like_a_charm() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    place(editor, "ward", (330.0, 452.0));
    let markers = editor_markers(editor, None, None);
    let (selection, rect, color) = markers
        .iter()
        .find(|(selection, _, _)| matches!(selection, Selection::Charm(_)))
        .copied()
        .expect("the charm has a marker");
    assert_eq!(selection, Selection::Charm(0));
    assert_eq!(
        (rect.x, rect.y, rect.w, rect.h),
        (
            330.0 - CHARM_SIZE.x / 2.0,
            452.0 - CHARM_SIZE.y / 2.0,
            CHARM_SIZE.x,
            CHARM_SIZE.y
        )
    );
    // The fixture ward's swatch is (120, 120, 230).
    assert_eq!(color, Color::from_rgba(120, 120, 230, 255));
    // A press on it selects the charm itself.
    assert_eq!(selected_by_press(selection), Selection::Charm(0));
}

#[test]
fn the_marker_follows_the_charm_list_being_edited_and_goes_gray_for_a_deleted_charm() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    place(editor, "ward", (330.0, 452.0));
    let ward = editor
        .charms
        .edited()
        .iter()
        .position(|charm| charm.id == "ward")
        .unwrap();
    editor.charms.set_color(ward, [10, 200, 30, 255]).unwrap();
    let color_of = |editor: &EditorSession| {
        editor_markers(editor, None, None)
            .into_iter()
            .find(|(selection, _, _)| matches!(selection, Selection::Charm(_)))
            .map(|(_, _, color)| color)
            .unwrap()
    };
    assert_eq!(color_of(editor), Color::from_rgba(10, 200, 30, 255));
    editor.charms.delete(ward).unwrap();
    assert_eq!(color_of(editor), Color::from_rgba(128, 128, 128, 255));
}

#[test]
fn charm_markers_are_hidden_while_an_arrival_point_is_shown_alone() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    place(editor, "ward", (330.0, 452.0));
    let exits = editor.draft.layout.exits.len();
    assert!(exits > 0, "the fixture room has an exit");
    assert!(
        editor_markers(editor, Some(exits), None).is_empty(),
        "no such exit: nothing"
    );
    let alone = editor_markers(editor, Some(0), None);
    assert_eq!(alone.len(), 1);
    assert!(matches!(alone[0].0, Selection::Destination(0)));
}
