use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::game::GameState;
use hollow_steps::item::Item;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::{RoomEdge, RoomId};
use hollow_steps::room_data::{load_rooms, parse_rooms};
use macroquad::math::{Rect, Vec2};

const VALID: &str = include_str!("fixtures/rooms/valid.ron");

struct RoomDirectory(PathBuf);

impl RoomDirectory {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-rooms-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, contents: &str) {
        fs::write(self.0.join(name), contents).unwrap();
    }
}

impl Drop for RoomDirectory {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn changed(from: &str, to: &str) -> String {
    assert!(VALID.contains(from), "bad test replacement: {from}");
    VALID.replace(from, to)
}

#[test]
fn valid_files_support_named_rooms_all_entity_types_and_airborne_arrivals() {
    let directory = RoomDirectory::new();
    directory.write("workshop.ron", VALID);
    directory.write("notes.txt", "ignored");
    let rooms = load_rooms(&directory.0).unwrap();
    let room = &rooms[&RoomId::from("workshop".to_owned())];
    assert_eq!(rooms.len(), 1);
    assert_eq!(room.bounds, Rect::new(0.0, 0.0, 400.0, 200.0));
    assert_eq!(room.floor_y, 200.0);
    assert_eq!(room.spawn, Vec2::new(140.0, 176.0));
    assert_eq!(room.exits[0].spawn, Vec2::new(200.0, 100.0));
    assert_eq!(room.enemies[0].position(), Vec2::new(220.0, 182.0));
    assert_eq!(room.enemies[0].health(), 3);
    assert_eq!(room.enemies[0].detection_radius(), 180.0);
    assert_eq!(
        room.boss_special.as_ref().unwrap().position(),
        Vec2::new(280.0, 176.0)
    );
    assert!(room.exits_locked());
}

#[test]
fn references_are_resolved_independently_of_file_order() {
    let source = changed("target_room: \"workshop\"", "target_room: \"destination\"");
    let destination = VALID.replace("workshop", "destination");
    for sources in [
        [
            ("source.ron", source.as_str()),
            ("destination.ron", destination.as_str()),
        ],
        [
            ("destination.ron", destination.as_str()),
            ("source.ron", source.as_str()),
        ],
    ] {
        let rooms = parse_rooms(sources).unwrap();
        let room = &rooms[&RoomId::from("workshop".to_owned())];
        assert_eq!(
            room.exits[0].target_room,
            RoomId::from("destination".to_owned())
        );
        assert_eq!(room.exits[0].spawn, Vec2::new(200.0, 100.0));
    }
}

#[test]
fn bad_files_report_filename_room_and_broken_reference() {
    for (from, to, expected) in [
        (
            "target_room: \"workshop\"",
            "target_room: \"missing\"",
            "exit 0: missing target room 'missing'",
        ),
        (
            "target_spawn: \"arrival\"",
            "target_spawn: \"missing\"",
            "exit 0: target room 'workshop' has no spawn point 'missing'",
        ),
        (
            "default_spawn: \"start\"",
            "default_spawn: \"missing\"",
            "default_spawn 'missing' has no matching spawn point",
        ),
    ] {
        let directory = RoomDirectory::new();
        directory.write("broken.ron", &changed(from, to));
        let error = load_rooms(&directory.0).unwrap_err().to_string();
        assert!(error.contains("broken.ron: room 'workshop':"), "{error}");
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn duplicate_ids_and_spawn_names_are_rejected() {
    let error = parse_rooms([("a.ron", VALID), ("b.ron", VALID)])
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("duplicate room id (also in a.ron)"),
        "{error}"
    );
    assert!(error.contains("b.ron"), "{error}");
    for (from, to, expected) in [
        ("id: \"workshop\"", "id: \" \"", "id must not be empty"),
        (
            "name: \"arrival\"",
            "name: \"start\"",
            "spawn point 'start': name is empty or duplicated",
        ),
        (
            "name: \"arrival\"",
            "name: \"\"",
            "spawn point '': name is empty or duplicated",
        ),
    ] {
        let source = changed(from, to);
        let error = parse_rooms([("bad.ron", source.as_str())])
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn malformed_missing_and_unknown_fields_are_rejected_with_source_locations() {
    for source in [
        "(".to_owned(),
        changed("floor_y: 200.0,", ""),
        changed("floor_y: 200.0,", "floor_y: 200.0, typo: 1,"),
        changed("kind: AmberShard", "kind: UnknownItem"),
        changed("Coin(position", "UnknownEntity(position"),
        changed("speed: 90.0,", "speed: 90.0, typo: 1,"),
    ] {
        let error = parse_rooms([("syntax.ron", source.as_str())])
            .unwrap_err()
            .to_string();
        assert!(error.starts_with("syntax.ron:"), "{error}");
        // RON reports line:column for malformed syntax and schema mismatches.
        assert!(
            error
                .split(':')
                .nth(1)
                .unwrap()
                .trim()
                .parse::<usize>()
                .is_ok(),
            "{error}"
        );
    }
}

#[test]
fn invalid_geometry_and_entity_values_fail_validation_without_panicking() {
    for (from, to, expected) in [
        (
            "size: (400.0, 200.0)",
            "size: (-1.0, 200.0)",
            "origin and size",
        ),
        (
            "size: (400.0, 200.0)",
            "size: (31.0, 200.0)",
            "origin and size",
        ),
        (
            "origin: (0.0, 0.0)",
            "origin: (inf, 0.0)",
            "origin and size",
        ),
        ("floor_y: 200.0", "floor_y: NaN", "floor_y"),
        ("floor_y: 200.0", "floor_y: 201.0", "floor_y"),
        ("floor_y: 200.0", "floor_y: 47.0", "floor_y"),
        ("offset: 104.0", "offset: -1.0", "exit 0"),
        ("length: 96.0", "length: 97.0", "exit 0"),
        ("length: 96.0", "length: 0.0", "exit 0"),
        ("length: 96.0", "length: inf", "exit 0"),
        ("(140.0, 176.0)", "(15.0, 176.0)", "spawn point 'start'"),
        ("(140.0, 176.0)", "(140.0, 177.0)", "spawn point 'start'"),
        ("(140.0, 176.0)", "(140.0, 23.0)", "spawn point 'start'"),
        ("(140.0, 176.0)", "(370.0, 176.0)", "overlaps an exit"),
        ("(140.0, 176.0)", "(NaN, 176.0)", "spawn point 'start'"),
        (
            "patrol: (180.0, 260.0)",
            "patrol: (260.0, 180.0)",
            "entity 0 (Enemy)",
        ),
        (
            "patrol: (180.0, 260.0)",
            "patrol: (17.0, 260.0)",
            "entity 0 (Enemy)",
        ),
        (
            "patrol: (180.0, 260.0)",
            "patrol: (180.0, 383.0)",
            "entity 0 (Enemy)",
        ),
        ("speed: 90.0", "speed: -1.0", "entity 0 (Enemy)"),
        (
            "detection_radius: 180.0",
            "detection_radius: NaN",
            "entity 0 (Enemy)",
        ),
        ("(220.0, 182.0)", "(179.0, 182.0)", "entity 0 (Enemy)"),
        ("(220.0, 182.0)", "(220.0, 181.0)", "entity 0 (Enemy)"),
        ("(180.0, 190.0)", "(180.0, inf)", "entity 1"),
        ("(70.0, 192.0)", "(7.0, 192.0)", "entity 2"),
        (
            "ArenaBoss(position: (280.0, 176.0)),",
            "ArenaBoss(position: (280.0, 176.0)), ArenaBoss(position: (100.0, 176.0)),",
            "only one ArenaBoss",
        ),
    ] {
        let source = changed(from, to);
        let error = parse_rooms([("geometry.ron", source.as_str())])
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{from} -> {to}: {error}");
    }
}

#[test]
fn geometry_boundaries_zero_speed_and_exact_edge_spans_are_valid() {
    let source = VALID
        .replace("(140.0, 176.0)", "(16.0, 24.0)")
        .replace("speed: 90.0", "speed: 0.0")
        .replace("detection_radius: 180.0", "detection_radius: 0.0")
        .replace("(220.0, 182.0)", "(180.0, 182.0)");
    parse_rooms([("boundary.ron", source.as_str())]).unwrap();
}

#[test]
fn boss_rooms_must_be_wide_enough_for_the_summon_attack() {
    let source = r#"(
        id: "small_arena", origin: (0.0, 0.0), size: (138.0, 200.0),
        floor_y: 200.0, default_spawn: "start",
        spawns: [(name: "start", position: (69.0, 176.0))], exits: [],
        entities: [ArenaBoss(position: (69.0, 176.0))],
    )"#;
    parse_rooms([("minimum.ron", source)]).unwrap();
    let bad = source.replace("138.0", "137.0");
    let error = parse_rooms([("narrow.ron", bad.as_str())])
        .unwrap_err()
        .to_string();
    assert!(error.contains("ArenaBoss): room is too narrow"), "{error}");
}

#[test]
fn missing_empty_and_unreadable_room_inputs_report_paths() {
    let directory = RoomDirectory::new();
    for (path, expected) in [
        (directory.0.clone(), "no .ron room files"),
        (directory.0.join("missing"), "missing"),
    ] {
        let error = load_rooms(&path).unwrap_err().to_string();
        assert!(
            error.contains(&path.display().to_string()) && error.contains(expected),
            "{error}"
        );
    }
    fs::create_dir(directory.0.join("unreadable.ron")).unwrap();
    let error = load_rooms(&directory.0).unwrap_err().to_string();
    assert!(error.contains("unreadable.ron"), "{error}");
    assert!(parse_rooms([])
        .unwrap_err()
        .to_string()
        .contains("no room files"));
}

#[test]
fn game_requires_a_start_room_and_uses_authored_items_coins_and_spawns() {
    let directory = RoomDirectory::new();
    directory.write("room.ron", VALID);
    let error = GameState::load(&directory.0).unwrap_err().to_string();
    assert!(error.contains("missing starting room 'first'"), "{error}");
    directory.write("room.ron", &VALID.replace("workshop", "first"));
    let mut game = GameState::load(&directory.0).unwrap();
    assert_eq!(game.items()[0].item, Item::AmberShard);
    assert_eq!(game.items()[0].position, Vec2::new(180.0, 190.0));
    assert_eq!(game.coins()[0].position, Vec2::new(70.0, 192.0));
    assert_eq!(game.player().position(), Vec2::new(140.0, 176.0));
    assert!(game.player().is_grounded());
    // The explicitly placed enemy starts at 220, rather than its patrol's left edge.
    game.update(PlayerInput::default(), 0.0);
    assert_eq!(game.room().enemies[0].position(), Vec2::new(220.0, 182.0));
}

#[test]
fn game_travels_to_an_additional_room_defined_only_in_files() {
    let lf = VALID.replace("\r\n", "\n");
    for line_ending in ["\n", "\r\n"] {
        let fixture = lf.replace('\n', line_ending);
        let directory = RoomDirectory::new();
        // Remove the entity without depending on checkout indentation or line endings.
        let source = fixture.replace("ArenaBoss(position: (280.0, 176.0)),", "");
        let first = source.replace("id: \"workshop\"", "id: \"first\"");
        directory.write("first.ron", &first);
        directory.write("another.ron", &source);
        let mut game = GameState::load(&directory.0).unwrap();
        assert!(game.room().boss_special.is_none());
        assert!(!game.room().exits_locked());
        game.update(
            PlayerInput {
                move_axis: 1.0,
                ..PlayerInput::default()
            },
            10.0,
        );
        assert_eq!(game.room().id, RoomId::from("workshop".to_owned()));
        assert_eq!(game.player().position(), Vec2::new(200.0, 100.0));
        assert!(!game.player().is_grounded());
        assert_eq!(game.items()[0].position, Vec2::new(180.0, 190.0));
        assert_eq!(game.coins()[0].position, Vec2::new(70.0, 192.0));
    }
}

#[test]
fn shipped_rooms_retain_the_original_layout_and_content() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/rooms");
    let rooms = load_rooms(&directory).unwrap();
    assert_eq!(rooms.len(), 3);
    for (id, bounds, floor, spawn) in [
        (
            RoomId::First,
            Rect::new(0.0, 0.0, 960.0, 460.0),
            460.0,
            Vec2::new(180.0, 436.0),
        ),
        (
            RoomId::Second,
            Rect::new(80.0, 40.0, 800.0, 360.0),
            400.0,
            Vec2::new(160.0, 376.0),
        ),
        (
            RoomId::Boss,
            Rect::new(0.0, 0.0, 960.0, 460.0),
            460.0,
            Vec2::new(160.0, 436.0),
        ),
    ] {
        let room = &rooms[&id];
        assert_eq!(
            (room.bounds, room.floor_y, room.spawn),
            (bounds, floor, spawn)
        );
        assert!(room.boss.is_none());
    }
    let first = &rooms[&RoomId::First];
    assert_eq!(first.enemies.len(), 1);
    assert_eq!(first.enemies[0].position(), Vec2::new(560.0, 442.0));
    assert_eq!(first.enemies[0].detection_radius(), 180.0);
    let mut rooms_for_patrol = load_rooms(&directory).unwrap();
    let room = rooms_for_patrol.get_mut(&RoomId::First).unwrap();
    let bounds = room.world_bounds();
    room.enemies[0].update(Vec2::ZERO, 240.0 / 90.0, bounds);
    assert_eq!(room.enemies[0].position(), Vec2::new(800.0, 442.0));
    assert!(rooms[&RoomId::Second].enemies.is_empty());
    assert!(rooms[&RoomId::Boss].enemies.is_empty());
    assert!(!first.exits_locked());
    assert!(rooms[&RoomId::Boss].exits_locked());
    let boss = rooms[&RoomId::Boss].boss_special.as_ref().unwrap();
    assert_eq!(boss.position(), Vec2::new(560.0, 436.0));
    assert_eq!(boss.health(), 100);

    for (id, expected) in [
        (
            RoomId::First,
            vec![(
                RoomEdge::Right,
                364.0,
                RoomId::Second,
                Vec2::new(160.0, 376.0),
            )],
        ),
        (
            RoomId::Second,
            vec![
                (
                    RoomEdge::Left,
                    264.0,
                    RoomId::First,
                    Vec2::new(860.0, 436.0),
                ),
                (
                    RoomEdge::Right,
                    264.0,
                    RoomId::Boss,
                    Vec2::new(160.0, 436.0),
                ),
            ],
        ),
        (
            RoomId::Boss,
            vec![(
                RoomEdge::Left,
                364.0,
                RoomId::Second,
                Vec2::new(780.0, 376.0),
            )],
        ),
    ] {
        let exits = &rooms[&id].exits;
        assert_eq!(exits.len(), expected.len());
        for (exit, (edge, offset, target, spawn)) in exits.iter().zip(expected) {
            assert_eq!((exit.edge, exit.offset, exit.length), (edge, offset, 96.0));
            assert_eq!(exit.target_room, target);
            assert_eq!(exit.spawn, spawn);
        }
    }
    let mut game = GameState::load(&directory).unwrap();
    assert_eq!(
        game.items()
            .iter()
            .map(|item| (item.item, item.position))
            .collect::<Vec<_>>(),
        vec![
            (Item::AmberShard, Vec2::new(300.0, 450.0)),
            (Item::Dash, Vec2::new(500.0, 450.0))
        ]
    );
    assert_eq!(
        game.coins()
            .iter()
            .map(|coin| coin.position)
            .collect::<Vec<_>>(),
        vec![
            Vec2::new(360.0, 452.0),
            Vec2::new(410.0, 452.0),
            Vec2::new(460.0, 452.0)
        ]
    );
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Second);
    assert!(game.items().is_empty());
    assert_eq!(
        game.coins()
            .iter()
            .map(|coin| coin.position)
            .collect::<Vec<_>>(),
        vec![
            Vec2::new(300.0, 392.0),
            Vec2::new(350.0, 392.0),
            Vec2::new(400.0, 392.0)
        ]
    );
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(game.room().id, RoomId::Boss);
    assert!(game.items().is_empty());
    assert!(game.coins().is_empty());
}
