//! Door exits in room files and in the editor: how they are stored, which doors are valid, and
//! adding, moving and resizing one. A door is `(x, y, width, height)` in room coordinates.
use std::path::Path;

use macroquad::math::{vec2, Rect, Vec2};

use super::*;
use crate::enemy::EnemyDefinitions;
use crate::exit_arrival::{arrival_error, arrival_next_to};
use crate::room::{test_rooms, Exit, RoomEdge, RoomId};

/// The first fixture room as a layout: 960 x 460, floor at 460, spawn near (180, 436).
fn layout() -> RoomLayout {
    RoomLayout::from_room(&test_rooms()[0])
}

fn draft() -> RoomDraft {
    let mut layout = layout();
    layout.exits.clear();
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

fn door(x: f32, y: f32, w: f32, h: f32) -> ExitPlacement {
    ExitPlacement {
        door: Some((x, y, w, h)),
        ..ExitPlacement::edge_exit(RoomEdge::Bottom, 0.0, 0.0, RoomId::Second, (160.0, 376.0))
    }
}

fn with_exit(exit: ExitPlacement) -> RoomLayout {
    let mut layout = layout();
    layout.exits = vec![exit];
    layout
}

// ---- the room file ----

#[test]
fn an_edge_exit_is_written_exactly_as_before() {
    let exit =
        ExitPlacement::edge_exit(RoomEdge::Right, 364.0, 96.0, RoomId::Second, (160.0, 376.0));
    assert_eq!(
        ron::to_string(&exit).unwrap(),
        "(edge:Right,offset:364.0,length:96.0,target_room:Second,spawn:(160.0,376.0))"
    );
}

#[test]
fn a_door_is_written_with_its_rectangle_and_no_edge_fields() {
    let text = ron::to_string(&door(380.0, 388.0, 48.0, 72.0)).unwrap();
    assert_eq!(
        text,
        "(door:(380.0,388.0,48.0,72.0),target_room:Second,spawn:(160.0,376.0))"
    );
}

#[test]
fn doors_and_edge_exits_survive_a_round_trip_through_a_room_file() {
    let mut layout = layout();
    layout.exits.push(door(380.0, 388.0, 48.0, 72.0));
    let text = ron::ser::to_string_pretty(&layout, ron::ser::PrettyConfig::default()).unwrap();
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, layout);
    assert_eq!(back.exits.iter().filter(|e| e.is_door()).count(), 1);
    assert_eq!(back.exits.iter().filter(|e| !e.is_door()).count(), 1);
}

#[test]
fn an_exit_from_an_older_room_file_loads_as_an_edge_exit() {
    let exit: ExitPlacement = ron::from_str(
        "(edge: Left, offset: 12.0, length: 96.0, target_room: First, spawn: (3.0, 4.0))",
    )
    .unwrap();
    assert_eq!(exit.door, None);
    assert_eq!(exit.edge, RoomEdge::Left);
    assert_eq!((exit.offset, exit.length), (12.0, 96.0));
}

#[test]
fn a_door_cannot_also_have_edge_fields_and_an_edge_exit_cannot_miss_one() {
    for (text, hint) in [
        (
            "(edge: Left, door: (1.0, 2.0, 30.0, 40.0), target_room: First, spawn: (3.0, 4.0))",
            "no `edge`",
        ),
        (
            "(offset: 1.0, door: (1.0, 2.0, 30.0, 40.0), target_room: First, spawn: (3.0, 4.0))",
            "no `edge`",
        ),
        (
            "(edge: Left, offset: 1.0, target_room: First, spawn: (3.0, 4.0))",
            "needs `edge`",
        ),
        ("(target_room: First, spawn: (3.0, 4.0))", "needs `edge`"),
    ] {
        let error = ron::from_str::<ExitPlacement>(text)
            .unwrap_err()
            .to_string();
        assert!(error.contains(hint), "{text}: {error}");
    }
    assert!(ron::from_str::<ExitPlacement>(
        "(door: (1.0, 2.0, 30.0, 40.0), target_room: First, spawn: (3.0, 4.0), extra: 1)"
    )
    .is_err());
}

// ---- which doors are valid ----

#[test]
fn a_door_anywhere_in_the_room_above_the_floor_is_valid() {
    // Standing on the floor, hanging in the middle of the room, and flush with a wall.
    for (x, y) in [(380.0, 388.0), (500.0, 100.0), (0.0, 388.0), (912.0, 388.0)] {
        assert_eq!(
            with_exit(door(x, y, 48.0, 72.0)).validate_geometry(),
            Ok(()),
            "({x}, {y})"
        );
    }
}

#[test]
fn a_door_outside_the_room_or_below_the_floor_is_rejected() {
    for (x, y, w, h) in [
        (-1.0, 388.0, 48.0, 72.0),
        (913.0, 388.0, 48.0, 72.0),
        (380.0, -1.0, 48.0, 72.0),
        (380.0, 389.0, 48.0, 72.0), // bottom 461 is below the floor
    ] {
        let error = with_exit(door(x, y, w, h)).validate_geometry().unwrap_err();
        assert!(error.contains("inside the room"), "({x}, {y}): {error}");
    }
}

#[test]
fn a_door_that_is_tiny_or_not_a_number_is_rejected() {
    for (w, h) in [
        (0.0, 72.0),
        (48.0, 15.0),
        (-48.0, 72.0),
        (f32::NAN, 72.0),
        (48.0, f32::INFINITY),
    ] {
        assert!(
            with_exit(door(380.0, 388.0, w, h))
                .validate_geometry()
                .is_err(),
            "{w} x {h}"
        );
    }
    let mut bad_spawn = door(380.0, 388.0, 48.0, 72.0);
    bad_spawn.spawn = (f32::NAN, 0.0);
    assert!(with_exit(bad_spawn).validate_geometry().is_err());
}

#[test]
fn a_door_may_stand_on_a_platform_but_not_inside_one() {
    let mut on_top = with_exit(door(380.0, 328.0, 48.0, 72.0)); // bottom 400
    on_top.terrain.push((300.0, 400.0, 200.0, 20.0));
    assert_eq!(on_top.validate_geometry(), Ok(()));

    let mut inside = with_exit(door(380.0, 340.0, 48.0, 72.0)); // bottom 412, in the platform
    inside.terrain.push((300.0, 400.0, 200.0, 20.0));
    assert!(inside.validate_geometry().unwrap_err().contains("terrain"));
}

// ---- adding, moving and resizing ----

/// The bottom edge `add_door` gave the first door of `draft`.
fn door_bottom(draft: &RoomDraft) -> f32 {
    let (_, y, _, h) = draft.layout.exits[0].door.unwrap();
    y + h
}

#[test]
fn add_door_stands_a_door_on_the_floor_under_the_spawn_point_and_selects_it() {
    let mut draft = draft();
    let (px, _) = draft.layout.spawn;
    let selection = draft.add_door(RoomId::Second, vec2(160.0, 376.0)).unwrap();
    assert_eq!(selection, Selection::Exit(0));
    let exit = &draft.layout.exits[0];
    assert_eq!(exit.target_room, RoomId::Second);
    assert_eq!(exit.spawn, (160.0, 376.0));
    let (x, _, w, h) = exit.door.unwrap();
    assert_eq!((w, h), (DOOR_SIZE.x, DOOR_SIZE.y));
    assert_eq!(x + w / 2.0, px, "centered on the spawn point");
    assert_eq!(door_bottom(&draft), draft.layout.floor_y);
}

#[test]
fn a_spawn_point_in_mid_air_does_not_leave_the_door_hanging_there() {
    // The shipped first room spawns the player at y = 273 with its floor at 460: a door
    // placed at the spawn would hang 160 px above anything the player can stand on.
    let mut draft = draft();
    draft.layout.spawn = (252.0, 273.14285);
    draft.add_door(RoomId::Second, Vec2::ZERO).unwrap();
    assert_eq!(door_bottom(&draft), draft.layout.floor_y);
    assert_eq!(draft.layout.exits[0].door.unwrap().0 + 24.0, 252.0);
}

#[test]
fn a_door_stands_on_a_platform_below_the_spawn_but_not_one_beside_or_above_it() {
    let platform = |x: f32, y: f32| (x, y, 120.0, 20.0);
    for (terrain, expected_bottom, why) in [
        (platform(200.0, 400.0), 400.0, "under the door"),
        (platform(0.0, 400.0), 460.0, "beside it, out of its width"),
        (platform(200.0, 100.0), 460.0, "above the spawn"),
    ] {
        let mut draft = draft();
        draft.layout.spawn = (252.0, 273.0);
        draft.layout.terrain.push(terrain);
        draft.add_door(RoomId::Second, Vec2::ZERO).unwrap();
        assert_eq!(door_bottom(&draft), expected_bottom, "{why}");
    }
}

#[test]
fn a_door_for_a_spawn_next_to_a_wall_is_kept_inside_the_room() {
    let mut draft = draft();
    draft.layout.spawn = (20.0, 436.0);
    draft.add_door(RoomId::Second, Vec2::ZERO).unwrap();
    assert_eq!(draft.layout.exits[0].door.unwrap().0, 0.0);
}

#[test]
fn add_door_in_a_room_too_small_for_it_changes_nothing() {
    let mut draft = draft();
    draft.layout.bounds = (0.0, 0.0, 200.0, 60.0);
    draft.layout.floor_y = 60.0;
    draft.layout.spawn = (100.0, 36.0);
    draft.layout.items.clear();
    draft.layout.charms.clear();
    let before = draft.clone();
    let error = draft.add_door(RoomId::Second, Vec2::ZERO).unwrap_err();
    assert!(error.contains("inside the room"), "{error}");
    assert_eq!(draft, before);
}

#[test]
fn dragging_a_door_moves_it_by_its_center_and_keeps_its_size() {
    let mut draft = draft();
    draft.layout.exits.push(door(380.0, 388.0, 48.0, 72.0));
    draft
        .move_to(Selection::Exit(0), vec2(600.0, 300.0), &definitions())
        .unwrap();
    assert_eq!(draft.layout.exits[0].door, Some((576.0, 264.0, 48.0, 72.0)));
}

#[test]
fn dragging_a_door_out_of_the_room_is_refused_and_leaves_it_where_it_was() {
    let mut draft = draft();
    draft.layout.exits.push(door(380.0, 388.0, 48.0, 72.0));
    let before = draft.clone();
    assert!(draft
        .move_to(Selection::Exit(0), vec2(5000.0, 300.0), &definitions())
        .is_err());
    assert_eq!(draft, before);
}

#[test]
fn resizing_a_door_keeps_its_bottom_and_its_center_line() {
    let mut draft = draft();
    draft.layout.exits.push(door(380.0, 388.0, 48.0, 72.0));
    draft.resize_door(0, 64.0, 96.0).unwrap();
    assert_eq!(draft.layout.exits[0].door, Some((372.0, 364.0, 64.0, 96.0)));
}

#[test]
fn a_door_resize_that_is_too_small_or_does_not_fit_is_refused() {
    let mut draft = draft();
    draft.layout.exits.push(door(380.0, 388.0, 48.0, 72.0));
    let before = draft.clone();
    assert!(draft.resize_door(0, 8.0, 72.0).is_err());
    assert!(
        draft.resize_door(0, 48.0, 1000.0).is_err(),
        "taller than the room"
    );
    assert!(draft.resize_door(7, 48.0, 72.0).is_err(), "no such exit");
    assert_eq!(draft, before);

    let mut edge = ExitPlacement::edge_exit(RoomEdge::Left, 0.0, 96.0, RoomId::Second, (0.0, 0.0));
    assert!(
        edge.resize_door(48.0, 72.0).is_err(),
        "an edge exit has no size"
    );
}

#[test]
fn deleting_a_selected_door_removes_it() {
    let mut draft = draft();
    draft.layout.exits.push(door(380.0, 388.0, 48.0, 72.0));
    draft.delete(Selection::Exit(0)).unwrap();
    assert!(draft.layout.exits.is_empty());
}

// ---- where the player arrives ----

#[test]
fn arriving_at_a_door_is_in_front_of_it_standing_on_its_bottom_edge() {
    let layout = with_exit(door(380.0, 388.0, 48.0, 72.0));
    let point = arrival_next_to(&layout, 0).unwrap();
    assert_eq!(point, vec2(404.0, 436.0));
    assert_eq!(arrival_error(&layout, point), None, "inside a door is fine");
}

#[test]
fn arriving_in_front_of_a_door_next_to_a_wall_slides_to_the_free_side() {
    let mut layout = with_exit(door(380.0, 388.0, 48.0, 72.0));
    // A pillar over the spot straight in front of the door.
    layout.terrain.push((395.0, 410.0, 20.0, 50.0));
    let point = arrival_next_to(&layout, 0).unwrap();
    assert_ne!(point, vec2(404.0, 436.0));
    assert_eq!(arrival_error(&layout, point), None);
    assert!(
        (point.x - 404.0).abs() <= 48.0,
        "still next to the door: {point}"
    );
}

#[test]
fn an_edge_exit_is_still_not_a_valid_place_to_arrive() {
    let mut layout = layout();
    layout.exits.push(ExitPlacement::edge_exit(
        RoomEdge::Left,
        364.0,
        96.0,
        RoomId::Second,
        (0.0, 0.0),
    ));
    assert_eq!(
        arrival_error(&layout, vec2(16.0, 436.0)),
        Some("inside an exit")
    );
}

#[test]
fn a_destination_in_front_of_a_door_is_no_return_trip_but_one_inside_an_edge_exit_is() {
    let first = layout();
    let mut second = layout();
    second.exits = vec![door(380.0, 388.0, 48.0, 72.0)];
    let rooms = [(RoomId::First, first.clone()), (RoomId::Second, second)];

    let mut a = draft();
    a.layout.exits = vec![ExitPlacement::edge_exit(
        RoomEdge::Right,
        364.0,
        96.0,
        RoomId::Second,
        (404.0, 436.0), // inside B's door
    )];
    assert_eq!(a.validate(&definitions(), &rooms), Ok(()));

    // The same spot inside B's right-edge exit still bounces the player straight back.
    let mut edge_room = layout();
    edge_room.exits = vec![ExitPlacement::edge_exit(
        RoomEdge::Right,
        364.0,
        96.0,
        RoomId::First,
        (0.0, 0.0),
    )];
    let rooms = [(RoomId::First, first), (RoomId::Second, edge_room)];
    a.layout.exits[0].spawn = (940.0, 436.0);
    assert!(a
        .validate(&definitions(), &rooms)
        .unwrap_err()
        .contains("immediate return"));
}

// ---- what the room is built from it ----

#[test]
fn a_built_room_holds_the_door_as_a_free_rectangle_that_only_up_uses() {
    let mut layout = layout();
    layout.exits = vec![door(380.0, 388.0, 48.0, 72.0)];
    let [mut room, _, _] = test_rooms();
    layout.apply(&mut room).unwrap();
    let exit = room.exits[0];
    let zone = Rect::new(380.0, 388.0, 48.0, 72.0);
    assert_eq!(
        exit,
        Exit::doorway(zone, RoomId::Second, vec2(160.0, 376.0))
    );
    assert_eq!(exit.zone(room.bounds), zone);
    let size = vec2(32.0, 48.0);
    assert!(
        room.exit_at(vec2(404.0, 436.0), size).is_none(),
        "walking in does nothing"
    );
    assert!(room.door_at(vec2(404.0, 436.0), size).is_some());
    assert_eq!(RoomLayout::from_room(&room).exits, layout.exits);
}
