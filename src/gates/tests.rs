//! Gates in a room file, their validation, and how they follow the exits they name. The room is
//! the first fixture room: 960 x 460, one edge exit on the right.
use std::path::Path;

use macroquad::math::Vec2;

use super::*;
use crate::editor::{ExitPlacement, Selection};
use crate::enemy::EnemyDefinitions;
use crate::room::{test_rooms, RoomEdge, RoomId};

fn layout() -> RoomLayout {
    RoomLayout::from_room(&test_rooms()[0])
}

fn draft_of(layout: RoomLayout) -> RoomDraft {
    RoomDraft {
        enemies: Vec::new(),
        boss: None,
        layout,
    }
}

fn definitions() -> EnemyDefinitions {
    EnemyDefinitions::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"))
        .expect("fixed test assets must load")
}

fn written(layout: &RoomLayout) -> (String, RoomLayout) {
    let text = ron::ser::to_string_pretty(layout, ron::ser::PrettyConfig::default()).unwrap();
    let read = ron::from_str(&text).unwrap();
    (text, read)
}

fn door(x: f32, y: f32, w: f32, h: f32) -> ExitPlacement {
    ExitPlacement {
        door: Some((x, y, w, h)),
        ..ExitPlacement::edge_exit(RoomEdge::Bottom, 0.0, 0.0, RoomId::Second, (160.0, 376.0))
    }
}

/// An arrival into this room from exit `offset` on the left edge of Second.
fn arrival(offset: f32) -> Entrance {
    Entrance::Arrival {
        room: RoomId::Second,
        exit: ExitRef::Edge {
            edge: RoomEdge::Left,
            offset,
        },
    }
}

fn gate(from: Entrance, exit: ExitRef, needs: &[Ability]) -> Gate {
    Gate {
        from,
        exit,
        needs: needs.to_vec(),
    }
}

/// The fixture room with its one exit marked as needing Dash from the spawn and from an arrival.
fn gated() -> RoomLayout {
    let mut layout = layout();
    let exit = ExitRef::of(&layout.exits[0]);
    layout.gates = vec![
        gate(Entrance::Spawn, exit, &[Ability::Dash]),
        gate(arrival(40.0), exit, &[Ability::Dash]),
    ];
    layout
}

// ---- the room file ----

#[test]
fn gates_are_written_and_read_back_exactly() {
    let layout = gated();
    let (text, read) = written(&layout);
    assert!(text.contains("gates"), "{text}");
    assert_eq!(read, layout);
}

#[test]
fn a_room_without_gates_writes_no_gates_field_and_a_file_without_one_loads() {
    let (text, read) = written(&layout());
    assert!(!text.contains("gates"), "{text}");
    assert!(read.gates.is_empty());
    assert_eq!(read, layout());
}

#[test]
fn the_example_in_docs_gates_md_reads() {
    let gates: Vec<Gate> = ron::from_str(
        "[(from: Spawn, exit: Edge(edge: Right, offset: 120.0), needs: [Dash]),
          (from: Arrival(room: Custom(2), exit: Edge(edge: Left, offset: 40.0)),
           exit: Door(x: 600.0, y: 300.0, width: 48.0, height: 72.0), needs: [Dash])]",
    )
    .unwrap();
    assert_eq!(gates.len(), 2);
    assert_eq!(
        gates[1].from,
        Entrance::Arrival {
            room: RoomId::Custom(2),
            exit: ExitRef::Edge {
                edge: RoomEdge::Left,
                offset: 40.0
            },
        }
    );
}

#[test]
fn an_ability_that_does_not_exist_is_a_load_error_naming_it() {
    let error =
        ron::from_str::<Gate>("(from: Spawn, exit: Edge(edge: Right, offset: 1.0), needs: [Fly])")
            .unwrap_err()
            .to_string();
    assert!(error.contains("Fly"), "{error}");
}

#[test]
fn a_gate_carries_through_the_live_room_and_back() {
    let [mut room, ..] = test_rooms();
    gated().apply(&mut room).unwrap();
    assert_eq!(RoomLayout::from_room(&room).gates, gated().gates);
}

#[test]
fn a_new_room_starts_without_gates() {
    assert!(gated().gates.len() == 2);
    assert!(RoomDraft::new_room(&gated()).layout.gates.is_empty());
}

// ---- validation ----

#[test]
fn a_valid_set_of_gates_passes_and_so_does_none() {
    assert_eq!(gated().validate_geometry(), Ok(()));
    assert_eq!(layout().validate_geometry(), Ok(()));
}

#[test]
fn a_gate_on_an_exit_the_room_does_not_have_is_refused() {
    let mut layout = gated();
    layout.gates[0].exit = ExitRef::Edge {
        edge: RoomEdge::Left,
        offset: 5.0,
    };
    let error = layout.validate_geometry().unwrap_err();
    assert!(
        error.contains("gate 1") && error.contains("left edge exit at 5"),
        "{error}"
    );
}

#[test]
fn a_gate_that_needs_nothing_or_lists_an_ability_twice_is_refused() {
    let mut layout = gated();
    layout.gates[1].needs.clear();
    assert!(layout.validate_geometry().unwrap_err().contains("gate 2"));

    let mut layout = gated();
    layout.gates[0].needs = vec![Ability::Dash, Ability::Dash];
    let error = layout.validate_geometry().unwrap_err();
    assert!(
        error.contains("gate 1") && error.contains("Dash twice"),
        "{error}"
    );
}

#[test]
fn two_gates_for_the_same_entrance_and_exit_are_refused() {
    let mut layout = gated();
    layout.gates.push(layout.gates[0].clone());
    let error = layout.validate_geometry().unwrap_err();
    assert!(
        error.contains("gate 3") && error.contains("repeats"),
        "{error}"
    );
}

#[test]
fn the_same_entrance_to_two_exits_and_two_entrances_to_one_are_fine() {
    let mut layout = layout();
    layout.exits.push(ExitPlacement::edge_exit(
        RoomEdge::Left,
        300.0,
        96.0,
        RoomId::Second,
        (160.0, 376.0),
    ));
    let (first, second) = (ExitRef::of(&layout.exits[0]), ExitRef::of(&layout.exits[1]));
    layout.gates = vec![
        gate(Entrance::Spawn, first, &[Ability::Dash]),
        gate(Entrance::Spawn, second, &[Ability::Dash]),
        gate(arrival(40.0), first, &[Ability::Dash]),
    ];
    assert_eq!(layout.validate_geometry(), Ok(()));
}

// ---- reading and marking ----

#[test]
fn gate_needs_is_what_is_marked_and_free_where_nothing_is() {
    let layout = gated();
    let exit = ExitRef::of(&layout.exits[0]);
    assert_eq!(layout.gate_needs(&Entrance::Spawn, &exit), [Ability::Dash]);
    assert_eq!(layout.gate_needs(&arrival(40.0), &exit), [Ability::Dash]);
    assert!(layout.gate_needs(&arrival(41.0), &exit).is_empty());
    assert!(self::layout()
        .gate_needs(&Entrance::Spawn, &exit)
        .is_empty());
}

#[test]
fn ticking_makes_the_gate_and_clearing_the_last_ability_removes_it() {
    let mut draft = draft_of(layout());
    let exit = ExitRef::of(&draft.layout.exits[0]);
    draft
        .set_gate(Entrance::Spawn, 0, Ability::Dash, false)
        .unwrap();
    assert!(
        draft.layout.gates.is_empty(),
        "clearing what is free changes nothing"
    );

    draft
        .set_gate(Entrance::Spawn, 0, Ability::Dash, true)
        .unwrap();
    draft
        .set_gate(Entrance::Spawn, 0, Ability::Dash, true)
        .unwrap();
    assert_eq!(
        draft.layout.gates,
        [gate(Entrance::Spawn, exit, &[Ability::Dash])]
    );

    draft
        .set_gate(Entrance::Spawn, 0, Ability::Dash, false)
        .unwrap();
    assert!(
        draft.layout.gates.is_empty(),
        "no empty gate is left behind"
    );
}

#[test]
fn marking_a_missing_exit_or_removing_a_missing_gate_is_an_error() {
    let mut draft = draft_of(layout());
    assert!(draft
        .set_gate(Entrance::Spawn, 7, Ability::Dash, true)
        .is_err());
    assert!(draft.remove_gate(0).is_err());
    let mut draft = draft_of(gated());
    draft.remove_gate(0).unwrap();
    assert_eq!(draft.layout.gates.len(), 1);
}

// ---- gates follow the exit they name ----

#[test]
fn dragging_an_edge_exit_takes_its_gates_along() {
    let mut draft = draft_of(gated());
    let before = ExitRef::of(&draft.layout.exits[0]);
    draft
        .move_to(Selection::Exit(0), Vec2::new(950.0, 200.0), &definitions())
        .unwrap();
    let after = ExitRef::of(&draft.layout.exits[0]);
    assert_ne!(before, after, "the exit moved");
    assert_eq!(draft.layout.gates.len(), 2);
    assert!(draft.layout.gates.iter().all(|gate| gate.exit == after));
    assert_eq!(draft.layout.validate_geometry(), Ok(()));
}

#[test]
fn resizing_a_door_takes_its_gates_along() {
    let mut layout = layout();
    layout.exits = vec![door(380.0, 388.0, 48.0, 72.0)];
    let before = ExitRef::of(&layout.exits[0]);
    layout.gates = vec![gate(Entrance::Spawn, before, &[Ability::Dash])];
    let mut draft = draft_of(layout);
    draft.resize_door(0, 60.0, 72.0).unwrap();
    let after = ExitRef::of(&draft.layout.exits[0]);
    assert_ne!(before, after);
    assert_eq!(draft.layout.gates[0].exit, after);
}

#[test]
fn deleting_an_exit_deletes_its_gates_and_only_its_gates() {
    let mut layout = layout();
    layout.exits.push(ExitPlacement::edge_exit(
        RoomEdge::Left,
        300.0,
        96.0,
        RoomId::Second,
        (160.0, 376.0),
    ));
    let (first, second) = (ExitRef::of(&layout.exits[0]), ExitRef::of(&layout.exits[1]));
    layout.gates = vec![
        gate(Entrance::Spawn, first, &[Ability::Dash]),
        gate(Entrance::Spawn, second, &[Ability::Dash]),
    ];
    let mut draft = draft_of(layout);
    draft.delete(Selection::Exit(0)).unwrap();
    assert_eq!(
        draft.layout.gates,
        [gate(Entrance::Spawn, second, &[Ability::Dash])]
    );
    assert_eq!(draft.layout.validate_geometry(), Ok(()));
}

#[test]
fn adding_an_exit_leaves_every_gate_where_it_was() {
    let mut draft = draft_of(gated());
    let (before, exits) = (draft.layout.gates.clone(), draft.layout.exits.clone());
    draft.add_exit(RoomEdge::Top, RoomId::Second, Vec2::new(160.0, 376.0));
    draft.layout.follow_exits(&exits);
    assert_eq!(draft.layout.gates, before);
}
