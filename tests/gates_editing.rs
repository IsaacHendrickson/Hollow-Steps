//! Gate annotations in the room editor's session: the entrances of the room being edited,
//! moving an arrival under a gate, gates that name an entrance that is gone, and saving.
//! Editing First: its exit 0 leads to Second; Second's exit 0 leads back and arrives at
//! (860, 436).
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::ability::Ability;
use hollow_steps::editor::RoomLayout;
use hollow_steps::editor_session::EditorSession;
use hollow_steps::game::GameState;
use hollow_steps::gates::{Entrance, ExitRef, Gate};
use hollow_steps::room::{RoomEdge, RoomEnemies, RoomId};
use macroquad::math::Vec2;

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-gates-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }

    fn text(&self, file: &str) -> String {
        std::fs::read_to_string(self.0.join("rooms").join(file)).unwrap()
    }

    fn layout(&self, file: &str) -> RoomLayout {
        let document: RoomEnemies = ron::de::from_str(&self.text(file)).unwrap();
        document.layout.unwrap()
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn editing(temp: &TempAssets) -> GameState {
    let mut game = temp.game();
    game.toggle_editor().unwrap();
    game
}

fn editor(game: &mut GameState) -> &mut EditorSession {
    game.editor_mut().unwrap()
}

/// Second's exit 0 as an entrance of First.
fn from_second(session: &EditorSession) -> Entrance {
    let (_, second) = session
        .rooms
        .iter()
        .find(|(id, _)| *id == RoomId::Second)
        .unwrap();
    Entrance::Arrival {
        room: RoomId::Second,
        exit: ExitRef::of(&second.exits[0]),
    }
}

fn mark_arrival_needs_dash(session: &mut EditorSession) -> Entrance {
    let from = from_second(session);
    session
        .draft
        .set_gate(from, 0, Ability::Dash, true)
        .unwrap();
    from
}

// ---- the entrances ----

#[test]
fn the_entrances_are_the_spawn_then_each_arrival_with_where_it_lands() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let from = from_second(session);
    let found = session.gate_entrances();
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].entrance, Entrance::Spawn);
    assert_eq!(found[0].label, "Room spawn");
    assert_eq!(found[1].entrance, from);
    assert!(
        found[1].label.starts_with("Arrival from Second"),
        "{}",
        found[1].label
    );
    assert_eq!(found[1].position, Vec2::new(860.0, 436.0));
}

// ---- moving an arrival ----

#[test]
fn moving_an_arrival_keeps_its_annotation_attached() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let from = mark_arrival_needs_dash(session);
    let before = session.draft.layout.gates.clone();

    session
        .move_inbound_arrival(RoomId::Second, 0, Vec2::new(820.0, 436.0))
        .unwrap();

    assert_eq!(session.draft.layout.gates, before, "the gate is untouched");
    assert!(session.orphan_gates().is_empty());
    let entrance = &session.gate_entrances()[1];
    assert_eq!(
        entrance.entrance, from,
        "same entrance, named by its source"
    );
    assert_eq!(
        entrance.position,
        Vec2::new(820.0, 436.0),
        "now landing elsewhere"
    );
    let exit = ExitRef::of(&session.draft.layout.exits[0]);
    assert_eq!(
        session.draft.layout.gate_needs(&from, &exit),
        [Ability::Dash]
    );
}

// ---- an annotation that names no entrance ----

#[test]
fn a_gate_naming_no_entrance_is_reported_kept_and_does_not_block_saving() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let exit = ExitRef::of(&session.draft.layout.exits[0]);
    let lost = Entrance::Arrival {
        room: RoomId::Second,
        exit: ExitRef::Edge {
            edge: RoomEdge::Left,
            offset: 7.0,
        },
    };
    session.draft.layout.gates.push(Gate {
        from: lost,
        exit,
        needs: vec![Ability::Dash],
    });

    let orphans = session.orphan_gates();
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0].index, 0);
    assert!(orphans[0].text.contains("Second") && orphans[0].text.contains("left edge exit at 7"));
    assert!(orphans[0].text.contains("Dash"), "{}", orphans[0].text);
    assert_eq!(session.draft.layout.gates.len(), 1, "kept, not thrown away");

    session.dirty = true;
    game.save_editor().unwrap();
    assert!(
        temp.text("first.ron").contains("gates"),
        "still in the file"
    );
}

#[test]
fn removing_an_orphan_clears_it_and_the_spawn_is_never_one() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let exit = ExitRef::of(&session.draft.layout.exits[0]);
    session
        .draft
        .set_gate(Entrance::Spawn, 0, Ability::Dash, true)
        .unwrap();
    assert!(session.orphan_gates().is_empty());
    session.draft.layout.gates.push(Gate {
        from: Entrance::Arrival {
            room: RoomId::Boss,
            exit: ExitRef::Edge {
                edge: RoomEdge::Top,
                offset: 1.0,
            },
        },
        exit,
        needs: vec![Ability::Dash],
    });
    let orphans = session.orphan_gates();
    assert_eq!(orphans.len(), 1);
    assert_eq!(orphans[0].index, 1);
    session.draft.remove_gate(orphans[0].index).unwrap();
    assert!(session.orphan_gates().is_empty());
    assert_eq!(session.draft.layout.gates.len(), 1);
}

// ---- saving ----

#[test]
fn gates_are_saved_with_the_room_and_survive_the_rebuild_after_saving() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let from = mark_arrival_needs_dash(session);
    session
        .draft
        .set_gate(Entrance::Spawn, 0, Ability::Dash, true)
        .unwrap();
    session.dirty = true;
    let expected = session.draft.layout.gates.clone();

    game.save_editor().unwrap();

    assert_eq!(temp.layout("first.ron").gates, expected);
    assert!(
        temp.text("second.ron").find("gates").is_none(),
        "no gates, no field"
    );
    let session = editor(&mut game);
    let saved = session
        .rooms
        .iter()
        .find(|(id, _)| *id == RoomId::First)
        .unwrap();
    assert_eq!(
        saved.1.gates, expected,
        "the editor's copy of the saved room keeps them"
    );
    assert_eq!(session.draft.layout.gates, expected);
    assert_eq!(session.gate_entrances()[1].entrance, from);

    // A fresh game reads them from the file.
    let mut again = temp.game();
    again.toggle_editor().unwrap();
    assert_eq!(editor(&mut again).draft.layout.gates, expected);
}

#[test]
fn a_gate_on_an_exit_the_room_does_not_have_stops_the_save_and_writes_nothing() {
    let temp = TempAssets::new();
    let mut game = editing(&temp);
    let session = editor(&mut game);
    let before = temp.text("first.ron");
    session.draft.layout.gates.push(Gate {
        from: Entrance::Spawn,
        exit: ExitRef::Edge {
            edge: RoomEdge::Top,
            offset: 3.0,
        },
        needs: vec![Ability::Dash],
    });
    session.dirty = true;

    let error = game.save_editor().unwrap_err();

    assert!(
        error.contains("gate 1") && error.contains("top edge exit at 3"),
        "{error}"
    );
    assert_eq!(temp.text("first.ron"), before);
}
