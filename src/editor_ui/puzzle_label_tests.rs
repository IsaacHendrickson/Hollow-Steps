//! What the canvas draws and writes for the puzzle pieces, as pure functions of the draft.
use super::markers::editor_markers;
use super::puzzle_labels::{labels, PuzzleLabel};
use super::*;
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::editor_puzzle::PieceFields;

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn written(editor: &EditorSession) -> Vec<PuzzleLabel> {
    labels(editor, &editor_markers(editor, None, None))
}

fn marker_of(editor: &EditorSession, selection: Selection) -> Option<(Rect, Color)> {
    editor_markers(editor, None, None)
        .into_iter()
        .find(|(marker, _, _)| *marker == selection)
        .map(|(_, rect, color)| (rect, color))
}

#[test]
fn a_room_without_pieces_has_no_puzzle_markers_or_labels() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    assert_eq!(written(editor), []);
    assert!(editor_markers(editor, None, None)
        .iter()
        .all(|(selection, _, _)| !matches!(
            selection,
            Selection::Breakable(_) | Selection::Switch(_) | Selection::Barrier(_)
        )));
}

#[test]
fn every_piece_has_a_marker_over_its_rectangle_in_its_own_color() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.add_breakable().unwrap();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    let layout = &editor.draft.layout;
    let expected = [
        (Selection::Breakable(0), layout.breakables[0].rect()),
        (Selection::Switch(0), layout.switches[0].rect()),
        (Selection::Barrier(0), layout.barriers[0].rect()),
    ];
    let mut colors = Vec::new();
    for (selection, rect) in expected {
        let (marker, color) = marker_of(editor, selection).expect("a marker");
        assert_eq!(marker, rect, "{selection:?}");
        colors.push(color);
    }
    // Three different colors, none of them the hazard's magenta or any other marker's.
    let mut others: Vec<Color> = editor_markers(editor, None, None)
        .into_iter()
        .filter(|(selection, _, _)| {
            !matches!(
                selection,
                Selection::Breakable(_) | Selection::Switch(_) | Selection::Barrier(_)
            )
        })
        .map(|(_, _, color)| color)
        .collect();
    others.push(Color::new(0.95, 0.2, 0.75, 1.0));
    for (i, color) in colors.iter().enumerate() {
        assert!(!others.contains(color), "{color:?} is another marker's");
        assert!(!colors[..i].contains(color), "{color:?} is used twice");
    }
}

#[test]
fn a_wall_says_its_hits_and_a_switch_and_its_barrier_say_the_same_number() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.add_breakable().unwrap();
    editor.draft.add_switch().unwrap();
    editor.draft.add_switch().unwrap();
    editor.draft.add_barrier().unwrap();
    let wall = editor.draft.piece_fields(Selection::Breakable(0)).unwrap();
    editor
        .draft
        .set_piece_fields(Selection::Breakable(0), PieceFields { number: 1, ..wall })
        .unwrap();
    let barrier = editor.draft.piece_fields(Selection::Barrier(0)).unwrap();
    editor
        .draft
        .set_piece_fields(
            Selection::Barrier(0),
            PieceFields {
                number: 2,
                ..barrier
            },
        )
        .unwrap();

    let all = written(editor);

    let texts: Vec<&str> = all.iter().map(|label| label.text.as_str()).collect();
    assert_eq!(
        texts,
        ["1 hit", "S1", "S2", "S2"],
        "wall, switches, barrier"
    );
    for label in &all {
        assert!(
            editor_markers(editor, None, None)
                .iter()
                .any(|(_, rect, _)| *rect == label.marker),
            "each belongs to a marker"
        );
    }
    editor
        .draft
        .set_piece_fields(Selection::Breakable(0), PieceFields { number: 7, ..wall })
        .unwrap();
    assert_eq!(written(editor)[0].text, "7 hits");
}

#[test]
fn deleting_a_piece_removes_its_marker_and_label() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    editor.draft.add_breakable().unwrap();
    assert_eq!(written(editor).len(), 1);
    editor.draft.delete(Selection::Breakable(0)).unwrap();
    assert_eq!(written(editor), []);
    assert!(marker_of(editor, Selection::Breakable(0)).is_none());
}
