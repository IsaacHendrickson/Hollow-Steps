use super::*;
use crate::room::{test_rooms, RoomEnemies};

/// The first sample room's layout: 960 wide, floor at 460, spawn at (180, 436) (a body from
/// x 164 to 196), an exit at the right edge from x 928, no solids and no puzzle pieces.
fn layout() -> RoomLayout {
    let [room, _, _] = test_rooms();
    RoomLayout::from_room(&room)
}

fn wall(x: f32, y: f32, hits: u8) -> BreakablePlacement {
    BreakablePlacement {
        rect: (x, y, 32.0, 64.0),
        hits,
    }
}

fn plate(x: f32, id: u8) -> SwitchPlacement {
    SwitchPlacement {
        rect: (x, 436.0, 24.0, 24.0),
        id,
    }
}

fn bars(x: f32, switch: u8) -> BarrierPlacement {
    BarrierPlacement {
        rect: (x, 364.0, 24.0, 96.0),
        switch,
    }
}

/// A layout with one of each: a 3-hit wall, switch 2 and a barrier opened by switch 2.
fn full() -> RoomLayout {
    let mut layout = layout();
    layout.breakables = vec![wall(400.0, 396.0, 3)];
    layout.switches = vec![plate(500.0, 2)];
    layout.barriers = vec![bars(600.0, 2)];
    layout
}

/// The live puzzles of `layout`.
fn live(layout: &RoomLayout) -> RoomPuzzles {
    RoomPuzzles::from_layout(layout)
}

fn swing_at(x: f32, y: f32) -> Rect {
    Rect::new(x, y, 38.0, 32.0)
}

fn problem(layout: &RoomLayout) -> String {
    layout.validate_geometry().unwrap_err()
}

// ---- the file ----

#[test]
fn a_layout_with_every_kind_of_piece_is_valid() {
    assert_eq!(full().validate_geometry(), Ok(()));
}

#[test]
fn a_piece_must_have_finite_numbers() {
    let mut layout = full();
    layout.breakables[0].rect.0 = f32::NAN;
    assert!(problem(&layout).contains("breakable wall 0"));
    let mut layout = full();
    layout.switches[0].rect.2 = f32::INFINITY;
    assert!(problem(&layout).contains("switch 0"));
    let mut layout = full();
    layout.barriers[0].rect.1 = f32::NEG_INFINITY;
    assert!(problem(&layout).contains("barrier 0"));
}

#[test]
fn a_piece_must_be_at_least_eight_by_eight() {
    for (width, height, ok) in [
        (8.0, 8.0, true),
        (7.9, 20.0, false),
        (20.0, 7.9, false),
        (0.0, 20.0, false),
    ] {
        let mut layout = full();
        layout.breakables[0].rect = (400.0, 440.0, width, height);
        layout.switches[0].rect = (500.0, 440.0, width, height);
        layout.barriers[0].rect = (600.0, 440.0, width, height);
        assert_eq!(layout.validate_geometry().is_ok(), ok, "{width}x{height}");
    }
}

#[test]
fn a_piece_must_be_inside_the_room() {
    let mut layout = full();
    layout.switches[0].rect = (950.0, 436.0, 24.0, 24.0);
    assert!(problem(&layout).contains("switch 0"));
    let mut layout = full();
    layout.barriers[0].rect = (600.0, 420.0, 24.0, 96.0);
    assert!(problem(&layout).contains("barrier 0"));
    let mut layout = full();
    layout.breakables[0].rect = (-1.0, 400.0, 32.0, 60.0);
    assert!(problem(&layout).contains("breakable wall 0"));
    // Touching the edge from the inside is fine.
    let mut layout = full();
    layout.breakables[0].rect = (928.0, 396.0, 32.0, 64.0);
    assert_eq!(layout.validate_geometry(), Ok(()));
}

#[test]
fn a_wall_takes_one_to_nine_hits() {
    for (hits, ok) in [(0, false), (1, true), (9, true), (10, false)] {
        let mut layout = full();
        layout.breakables[0].hits = hits;
        assert_eq!(layout.validate_geometry().is_ok(), ok, "{hits} hits");
    }
    let mut layout = full();
    layout.breakables[0].hits = 0;
    assert!(problem(&layout).contains("breakable wall 0"));
}

#[test]
fn switch_numbers_are_one_to_eight_and_unique_in_the_room() {
    for (id, ok) in [(0, false), (1, true), (8, true), (9, false)] {
        let mut layout = full();
        layout.switches[0].id = id;
        layout.barriers.clear();
        assert_eq!(layout.validate_geometry().is_ok(), ok, "switch {id}");
    }
    let mut layout = full();
    layout.switches.push(plate(700.0, 2));
    let error = problem(&layout);
    assert!(
        error.contains("switch 1") && error.contains("number 2"),
        "{error}"
    );
    layout.switches[1].id = 3;
    assert_eq!(layout.validate_geometry(), Ok(()));
}

#[test]
fn a_barrier_needs_a_switch_in_the_same_room_and_the_error_names_the_barrier() {
    let mut layout = full();
    layout.barriers.push(bars(700.0, 5));
    let error = problem(&layout);
    assert!(
        error.contains("barrier 1") && error.contains("switch 5"),
        "{error}"
    );
    layout.barriers[1].switch = 2;
    assert_eq!(layout.validate_geometry(), Ok(()));
    layout.barriers[0].switch = 9;
    assert!(problem(&layout).contains("barrier 0"));
    layout.switches.clear();
    layout.barriers.truncate(1);
    layout.barriers[0].switch = 2;
    assert!(problem(&layout).contains("barrier 0"), "no switch at all");
}

#[test]
fn no_piece_may_overlap_the_spawn_body_but_touching_it_is_fine() {
    // The spawn body spans x 164..196, y 412..460.
    for (label, place) in [("breakable wall 0", 0), ("switch 0", 1), ("barrier 0", 2)] {
        let at = |x: f32, layout: &mut RoomLayout| match place {
            0 => layout.breakables[0].rect = (x, 396.0, 32.0, 64.0),
            1 => layout.switches[0].rect = (x, 436.0, 24.0, 24.0),
            _ => layout.barriers[0].rect = (x, 364.0, 24.0, 96.0),
        };
        let mut layout = full();
        at(190.0, &mut layout);
        let error = problem(&layout);
        assert!(
            error.contains("spawn") && error.contains(label),
            "{label}: {error}"
        );
        at(196.0, &mut layout);
        assert_eq!(layout.validate_geometry(), Ok(()), "{label} touching");
    }
}

#[test]
fn an_exit_arrival_may_not_land_inside_a_piece_of_the_room_it_enters() {
    let layout = full();
    // A body at x 390..422 reaches the wall (x 400..432); one beside it does not.
    let error = layout.validate_arrival((406.0, 436.0)).unwrap_err();
    assert!(error.contains("breakable wall 0"), "{error}");
    assert!(layout.validate_arrival((500.0, 436.0)).is_err(), "a switch");
    let error = layout.validate_arrival((610.0, 436.0)).unwrap_err();
    assert!(error.contains("barrier 0"), "{error}");
    assert_eq!(layout.validate_arrival((100.0, 436.0)), Ok(()));
    assert_eq!(layout.validate_arrival((384.0, 436.0)), Ok(()), "touching");
}

#[test]
fn a_new_room_does_not_copy_its_templates_pieces() {
    let draft = crate::editor::RoomDraft::new_room(&full());
    assert!(draft.layout.breakables.is_empty());
    assert!(draft.layout.switches.is_empty());
    assert!(draft.layout.barriers.is_empty());
}

// ---- the live pieces ----

#[test]
fn a_wall_takes_exactly_as_many_swings_as_it_has_hits() {
    let mut puzzles = live(&full());
    let hitbox = swing_at(380.0, 410.0);
    for swing in 1..=2 {
        let struck = puzzles.strike(hitbox, swing);
        assert_eq!(struck, Struck::default(), "swing {swing} only cracks it");
        assert!(puzzles.standing_solids().any(|rect| rect.x == 400.0));
    }
    let struck = puzzles.strike(hitbox, 3);
    assert_eq!(struck.broken, vec![Rect::new(400.0, 396.0, 32.0, 64.0)]);
    assert!(!puzzles.standing_solids().any(|rect| rect.x == 400.0));
    assert_eq!(puzzles.breakables()[0].remaining, 0);
    // Nothing more to hit.
    assert_eq!(puzzles.strike(hitbox, 4), Struck::default());
}

#[test]
fn two_hits_in_one_swing_count_once() {
    let mut puzzles = live(&full());
    let hitbox = swing_at(380.0, 410.0);
    for _ in 0..10 {
        puzzles.strike(hitbox, 7);
    }
    assert_eq!(puzzles.breakables()[0].remaining, 2);
    puzzles.strike(hitbox, 8);
    assert_eq!(puzzles.breakables()[0].remaining, 1);
}

#[test]
fn one_swing_over_two_walls_hits_each_once() {
    let mut layout = layout();
    layout.breakables = vec![wall(400.0, 396.0, 2), wall(420.0, 396.0, 1)];
    let mut puzzles = live(&layout);
    // The box (x 395..433) overlaps both walls (x 400..432 and 420..452).
    let struck = puzzles.strike(swing_at(395.0, 410.0), 1);
    assert_eq!(struck.broken, vec![Rect::new(420.0, 396.0, 32.0, 64.0)]);
    assert_eq!(puzzles.breakables()[0].remaining, 1);
}

#[test]
fn a_swing_hits_only_the_piece_its_box_overlaps() {
    let mut puzzles = live(&full());
    // Over the switch only (x 500..524), not the wall or the barrier.
    let struck = puzzles.strike(swing_at(490.0, 436.0), 1);
    assert_eq!(struck.switched_on.len(), 1);
    assert_eq!(puzzles.breakables()[0].remaining, 3);
    // Merely touching the wall's edge (box ends at x 400) is not a hit.
    let mut puzzles = live(&full());
    puzzles.strike(swing_at(362.0, 410.0), 1);
    assert_eq!(puzzles.breakables()[0].remaining, 3);
    puzzles.strike(swing_at(363.0, 410.0), 2);
    assert_eq!(puzzles.breakables()[0].remaining, 2);
    // Above and below it do not reach either.
    puzzles.strike(Rect::new(400.0, 300.0, 32.0, 96.0), 3);
    puzzles.strike(Rect::new(400.0, 460.0, 32.0, 96.0), 4);
    assert_eq!(puzzles.breakables()[0].remaining, 2);
}

#[test]
fn a_barrier_goes_away_when_its_switch_is_hit_and_stays_away() {
    let mut puzzles = live(&full());
    let barrier = Rect::new(600.0, 364.0, 24.0, 96.0);
    assert!(puzzles.standing_solids().any(|rect| rect == barrier));
    assert!(!puzzles.is_open(&puzzles.barriers()[0]));
    // Hitting the barrier itself does nothing.
    puzzles.strike(swing_at(590.0, 400.0), 1);
    assert!(puzzles.standing_solids().any(|rect| rect == barrier));

    let struck = puzzles.strike(swing_at(490.0, 436.0), 2);
    assert_eq!(
        struck.switched_on,
        vec![(Rect::new(500.0, 436.0, 24.0, 24.0), 2)]
    );
    assert!(puzzles.is_open(&puzzles.barriers()[0]));
    assert!(!puzzles.standing_solids().any(|rect| rect == barrier));

    // A second hit on a switch that is on changes nothing and reports nothing.
    assert_eq!(puzzles.strike(swing_at(490.0, 436.0), 3), Struck::default());
    assert!(puzzles.is_open(&puzzles.barriers()[0]));
}

#[test]
fn a_switch_opens_only_the_barriers_with_its_number() {
    let mut layout = full();
    layout.switches.push(plate(700.0, 5));
    layout.barriers.push(bars(800.0, 5));
    let mut puzzles = live(&layout);
    puzzles.strike(swing_at(690.0, 436.0), 1);
    let barriers = puzzles.barriers().to_vec();
    assert!(!puzzles.is_open(&barriers[0]), "switch 2 is still off");
    assert!(puzzles.is_open(&barriers[1]));
}

#[test]
fn collision_is_the_terrain_plus_what_still_stands() {
    let terrain = Terrain::from_tuples(&[(0.0, 300.0, 100.0, 20.0)]);
    let nothing = RoomPuzzles::default();
    assert!(matches!(nothing.collision(&terrain), Cow::Borrowed(_)));

    let mut puzzles = live(&full());
    let all = puzzles.collision(&terrain);
    assert_eq!(all.solids().len(), 3);
    assert_eq!(all.solids()[0], terrain.solids()[0]);
    drop(all);
    puzzles.strike(swing_at(490.0, 436.0), 1);
    assert_eq!(
        puzzles.collision(&terrain).solids().len(),
        2,
        "barrier open"
    );
    for swing in 2..=4 {
        puzzles.strike(swing_at(380.0, 410.0), swing);
    }
    // Everything is open or broken: only the terrain is left, and nothing was copied.
    assert!(matches!(puzzles.collision(&terrain), Cow::Borrowed(_)));
}

// ---- the layout and the live room ----

/// The first sample room built from `layout`, as the game plays it.
fn room_of(layout: &RoomLayout) -> crate::room::Room {
    let [mut room, _, _] = test_rooms();
    layout.apply(&mut room).unwrap();
    room
}

#[test]
fn from_room_gives_back_what_the_file_said_whatever_was_broken_or_switched() {
    let layout = full();
    let mut room = room_of(&layout);
    assert_eq!(RoomLayout::from_room(&room), layout);
    for swing in 1..=3 {
        room.puzzles.strike(swing_at(380.0, 410.0), swing);
    }
    room.puzzles.strike(swing_at(490.0, 436.0), 9);
    assert_eq!(room.puzzles.breakables()[0].remaining, 0);
    assert!(room.puzzles.switches()[0].on);
    assert_eq!(RoomLayout::from_room(&room), layout);
    assert!(room.terrain.is_empty(), "never merged into the terrain");
}

#[test]
fn applying_a_layout_replaces_the_rooms_pieces_with_whole_ones() {
    let mut room = room_of(&full());
    room.puzzles.strike(swing_at(490.0, 436.0), 1);
    layout().apply(&mut room).unwrap();
    assert_eq!(room.puzzles, RoomPuzzles::default());
    full().apply(&mut room).unwrap();
    assert!(!room.puzzles.switches()[0].on);
    assert_eq!(room.puzzles.breakables()[0].remaining, 3);
}

fn written(layout: &RoomLayout) -> String {
    ron::ser::to_string_pretty(layout, Default::default()).unwrap()
}

#[test]
fn a_layout_without_pieces_writes_no_lists_and_round_trips() {
    let layout = layout();
    let text = written(&layout);
    for word in ["breakables", "switches", "barriers"] {
        assert!(!text.contains(word), "{word}: {text}");
    }
    assert_eq!(ron::from_str::<RoomLayout>(&text).unwrap(), layout);
}

#[test]
fn each_list_round_trips_through_ron_on_its_own_and_with_the_others() {
    let full = full();
    let mut only_walls = layout();
    only_walls.breakables = full.breakables.clone();
    let mut only_switches = layout();
    only_switches.switches = full.switches.clone();
    for (word, with) in [
        ("breakables", only_walls),
        ("switches", only_switches),
        ("barriers", full.clone()),
        ("breakables", full),
    ] {
        let text = written(&with);
        assert!(text.contains(word), "{word}: {text}");
        assert_eq!(ron::from_str::<RoomLayout>(&text).unwrap(), with, "{word}");
    }
    let mut only_switches = layout();
    only_switches.switches = vec![plate(500.0, 1)];
    let text = written(&only_switches);
    assert!(
        !text.contains("breakables") && !text.contains("barriers"),
        "{text}"
    );
}

#[test]
fn a_room_file_from_before_puzzles_loads_unchanged() {
    // The sample room's file is older than puzzles and has no layout at all; a layout written
    // before them has none of the three lists.
    let text = include_str!("../../tests/fixtures/rooms/first.ron");
    assert!(ron::from_str::<RoomEnemies>(text).is_ok());
    let old = written(&layout());
    let loaded: RoomLayout = ron::from_str(&old).unwrap();
    assert!(loaded.breakables.is_empty());
    assert!(loaded.switches.is_empty());
    assert!(loaded.barriers.is_empty());
}

#[test]
fn a_piece_in_a_file_must_name_all_its_fields_and_no_others() {
    let walls: Vec<Result<BreakablePlacement, _>> = [
        "(rect: (0.0, 0.0, 8.0, 8.0), hits: 2)",
        "(rect: (0.0, 0.0, 8.0, 8.0))",
        "(rect: (0.0, 0.0, 8.0, 8.0), hits: 2, extra: 1)",
    ]
    .into_iter()
    .map(ron::from_str)
    .collect();
    assert!(walls[0].is_ok() && walls[1].is_err() && walls[2].is_err());
    assert!(ron::from_str::<SwitchPlacement>("(rect: (0.0, 0.0, 8.0, 8.0), id: 1)").is_ok());
    assert!(ron::from_str::<SwitchPlacement>("(rect: (0.0, 0.0, 8.0, 8.0))").is_err());
    assert!(ron::from_str::<BarrierPlacement>("(rect: (0.0, 0.0, 8.0, 8.0), switch: 1)").is_ok());
    assert!(ron::from_str::<BarrierPlacement>("(rect: (0.0, 0.0, 8.0, 8.0), id: 1)").is_err());
}
