//! The preview's text, on layouts built in code (no files).
use std::sync::OnceLock;

use macroquad::math::Vec2;

use super::grid::{self, MARGIN};
use super::*;
use crate::ability::Ability;
use crate::editor::{ExitPlacement, ItemPlacement, RoomLayout};
use crate::gates::{Entrance, ExitRef, Gate};
use crate::hazard::HazardPlacement;
use crate::item::Item;
use crate::player::PlayerConfig;
use crate::room::{RoomEdge, RoomId};

/// 10 x 10 cells at 16 px.
fn room() -> RoomLayout {
    ron::from_str(
        "(bounds: (0.0, 0.0, 160.0, 160.0), floor_y: 160.0, spawn: (24.0, 136.0), items: [], exits: [])",
    )
    .unwrap()
}

fn ruler() -> Ruler {
    static RULER: OnceLock<Ruler> = OnceLock::new();
    *RULER.get_or_init(|| {
        let base = PlayerConfig::default();
        let mut buffed = base;
        buffed.move_speed *= 1.2;
        Ruler::from_configs(&base, &buffed)
    })
}

/// The picture's rows without the margin and the axis.
fn rows(scene: &Scene, cell: f32) -> Vec<String> {
    let mut lines = grid::draw(scene, &scene.things(), cell);
    lines.pop();
    lines.iter().map(|line| line[MARGIN..].to_owned()).collect()
}

fn with_terrain(solids: &[(f32, f32, f32, f32)]) -> Scene {
    let mut layout = room();
    layout.terrain = solids.to_vec();
    Scene::new(layout)
}

fn exit(edge: RoomEdge, offset: f32, length: f32) -> ExitPlacement {
    ExitPlacement {
        edge,
        offset,
        length,
        target_room: RoomId::Custom(3),
        spawn: (480.0, 436.0),
        door: None,
        locked_by: None,
    }
}

#[test]
fn terrain_fills_the_cells_it_covers() {
    // y 48 to 64 is row 3 and x 32 to 96 is columns 2 to 5; the rest is air.
    let picture = rows(&with_terrain(&[(32.0, 48.0, 64.0, 16.0)]), 16.0);
    assert_eq!(picture.len(), 10);
    assert_eq!(picture[3], "..####....");
    assert_eq!(picture[2], "..........");
    assert_eq!(picture[4], "..........");
}

#[test]
fn a_cell_is_solid_at_exactly_half_and_thin_below_it() {
    let half = rows(&with_terrain(&[(32.0, 48.0, 64.0, 8.0)]), 16.0);
    assert_eq!(half[3], "..####....", "half a cell is solid");
    let thin = rows(&with_terrain(&[(32.0, 48.0, 64.0, 4.0)]), 16.0);
    assert_eq!(thin[3], "..====....", "a thin platform keeps a character");
    let wall = rows(&with_terrain(&[(80.0, 16.0, 4.0, 64.0)]), 16.0);
    assert!(wall[1..5].iter().all(|row| row == ".....|...."), "{wall:?}");
    let sliver = rows(&with_terrain(&[(112.0, 16.0, 4.0, 4.0)]), 16.0);
    assert_eq!(sliver[1], ".......+..");
}

#[test]
fn a_platform_split_across_two_rows_does_not_disappear() {
    // 6 px in row 3 and 6 px in row 4: neither row is half covered.
    let picture = rows(&with_terrain(&[(32.0, 58.0, 64.0, 12.0)]), 16.0);
    assert_eq!(picture[3], "..====....");
    assert_eq!(picture[4], "..====....");
}

#[test]
fn overlapping_terrain_is_not_counted_twice() {
    // Each is 40% of a cell; together they are still 40%, not 80%.
    let same = rows(
        &with_terrain(&[(32.0, 48.0, 16.0, 6.4), (32.0, 48.0, 16.0, 6.4)]),
        16.0,
    );
    assert_eq!(same[3], "..=.......");
    // Side by side they do add up.
    let both = rows(
        &with_terrain(&[(32.0, 48.0, 16.0, 6.4), (32.0, 54.4, 16.0, 6.4)]),
        16.0,
    );
    assert_eq!(both[3], "..#.......");
}

#[test]
fn the_floor_body_is_drawn_when_the_floor_line_is_above_the_bottom() {
    let mut layout = room();
    layout.floor_y = 128.0;
    layout.spawn = (24.0, 100.0);
    let picture = rows(&Scene::new(layout), 16.0);
    assert_eq!(picture[8], "##########");
    assert_eq!(picture[9], "##########");
    assert_eq!(picture[7].matches('#').count(), 0);
}

#[test]
fn the_spawn_is_the_players_body() {
    // The player is 32 x 48 px, centered on (24, 136): x 8 to 40, y 112 to 160.
    let picture = rows(&Scene::new(room()), 16.0);
    for row in &picture[7..10] {
        assert_eq!(row, "@@@.......");
    }
    assert_eq!(picture[6], "..........");
}

#[test]
fn each_exit_edge_and_a_door_get_a_mark_and_a_table_row() {
    let mut layout = room();
    layout.exits = vec![
        exit(RoomEdge::Left, 48.0, 32.0),
        ExitPlacement {
            locked_by: Some(2),
            ..exit(RoomEdge::Right, 48.0, 32.0)
        },
        exit(RoomEdge::Top, 64.0, 32.0),
        exit(RoomEdge::Bottom, 64.0, 32.0),
        ExitPlacement {
            door: Some((64.0, 64.0, 32.0, 48.0)),
            ..exit(RoomEdge::Bottom, 0.0, 0.0)
        },
    ];
    layout.gates = vec![Gate {
        from: Entrance::Spawn,
        exit: ExitRef::of(&layout.exits[4]),
        needs: vec![Ability::Dash],
    }];
    let scene = Scene::new(layout);
    let picture = rows(&scene, 16.0);
    assert_eq!(&picture[3][..2], "11", "left edge exit");
    assert_eq!(&picture[4][..2], "11");
    assert_eq!(&picture[3][8..], "22", "right edge exit");
    assert_eq!(&picture[0][4..6], "33", "top edge exit");
    assert_eq!(&picture[1][4..6], "33");
    assert_eq!(&picture[8][4..6], "44", "bottom edge exit");
    assert_eq!(&picture[9][4..6], "44");
    assert_eq!(&picture[5][4..6], "55", "the door");

    let text = render(&scene, &ruler(), 16.0);
    let exit_rows: Vec<&str> = text
        .lines()
        .filter(|line| line.contains("-> room_3"))
        .collect();
    assert_eq!(exit_rows.len(), 5, "{text}");
    assert!(
        exit_rows[0].contains("left edge, offset 48, length 32"),
        "{}",
        exit_rows[0]
    );
    assert!(exit_rows[1].contains("right edge") && exit_rows[1].contains("locked by Key 2"));
    assert!(exit_rows[2].contains("top edge") && exit_rows[3].contains("bottom edge"));
    assert!(exit_rows[4].contains("door"), "{}", exit_rows[4]);
    assert!(
        exit_rows[4].contains("gate: needs Dash from the spawn"),
        "{}",
        exit_rows[4]
    );
    assert!(exit_rows[0].contains("arrival (480, 436)"));
    assert!(!exit_rows[0].contains("gate") && !exit_rows[0].contains("locked"));
}

#[test]
fn a_gate_from_another_rooms_arrival_names_that_exit() {
    let mut layout = room();
    layout.exits = vec![exit(RoomEdge::Right, 48.0, 32.0)];
    layout.gates = vec![Gate {
        from: Entrance::Arrival {
            room: RoomId::First,
            exit: ExitRef::Edge {
                edge: RoomEdge::Left,
                offset: 40.0,
            },
        },
        exit: ExitRef::of(&layout.exits[0]),
        needs: vec![Ability::Dash, Ability::DoubleJump],
    }];
    let text = render(&Scene::new(layout), &ruler(), 16.0);
    assert!(
        text.contains("gate: needs Dash + Double Jump from first's left edge exit at 40"),
        "{text}"
    );
}

#[test]
fn a_hazard_marks_every_cell_it_touches_and_has_a_row() {
    let mut layout = room();
    let mut hazard = HazardPlacement::standard(0.0, 0.0);
    hazard.rect = (64.0, 144.0, 64.0, 16.0);
    layout.hazards = vec![hazard];
    let scene = Scene::new(layout);
    assert_eq!(rows(&scene, 16.0)[9], "@@@.^^^^..");
    let text = render(&scene, &ruler(), 16.0);
    let row = text.lines().find(|line| line.contains("hazard")).unwrap();
    assert!(
        row.contains("64") && row.contains("144") && row.contains("damage"),
        "{row}"
    );
}

#[test]
fn an_item_is_marked_by_its_kind_in_the_cell_it_mostly_covers() {
    let mut layout = room();
    layout.items = vec![
        ItemPlacement {
            item: Item::Key(2),
            position: (88.0, 132.0),
        },
        ItemPlacement {
            item: Item::Dash,
            position: (136.0, 60.0),
        },
    ];
    let scene = Scene::new(layout);
    let picture = rows(&scene, 16.0);
    // The key is x 80 to 96, y 122 to 142: mostly row 8 (128 to 144).
    assert_eq!(picture[8].chars().nth(5), Some('K'), "{picture:?}");
    assert_eq!(
        picture
            .iter()
            .map(|row| row.matches('K').count())
            .sum::<usize>(),
        1
    );
    assert_eq!(
        picture
            .iter()
            .map(|row| row.matches('D').count())
            .sum::<usize>(),
        1
    );
    let text = render(&scene, &ruler(), 16.0);
    assert!(text
        .lines()
        .any(|line| line.contains("item") && line.contains("Key 2")));
}

#[test]
fn an_enemy_is_drawn_with_its_patrol_range_as_a_line() {
    let mut scene = Scene::new(room());
    scene.enemies = vec![EnemyView {
        name: "walker".into(),
        center: Vec2::new(96.0, 136.0),
        size: Vec2::new(32.0, 32.0),
        patrol: Some((48.0, 112.0)),
        min_tier: 2,
        group: Some(1),
        chance: 100,
    }];
    // Body x 80 to 112, y 120 to 152: columns 5 and 6, rows 7 and 8. The line runs along row 8
    // (the body's center) from column 3 (x 48) to column 7 (x 112); the bodies win their cells.
    let picture = rows(&scene, 16.0);
    assert_eq!(picture[8], "@@@<-EE>..");
    assert_eq!(&picture[7][5..7], "EE");
    let text = render(&scene, &ruler(), 16.0);
    let row = text.lines().find(|line| line.contains("walker")).unwrap();
    assert!(row.contains("patrol x 48 to 112 (64 px)"), "{row}");
    assert!(
        row.contains("from tier 2") && row.contains("group 1"),
        "{row}"
    );
    assert!(!row.contains("chance"));
}

#[test]
fn an_enemy_that_does_not_move_has_no_line() {
    let mut scene = Scene::new(room());
    scene.enemies = vec![EnemyView {
        name: "statue".into(),
        center: Vec2::new(96.0, 136.0),
        size: Vec2::new(32.0, 32.0),
        patrol: Some((96.0, 96.0)),
        min_tier: 0,
        group: None,
        chance: 50,
    }];
    let picture = rows(&scene, 16.0);
    assert_eq!(picture[8], "@@@..EE...");
    let text = render(&scene, &ruler(), 16.0);
    assert!(text
        .lines()
        .any(|line| line.contains("statue") && line.contains("chance 50%")));
}

#[test]
fn a_boss_is_drawn_by_its_body() {
    let mut scene = Scene::new(room());
    scene.boss = Some(BossView {
        name: "warden".into(),
        center: Vec2::new(80.0, 96.0),
        size: Vec2::new(64.0, 64.0),
    });
    // x 48 to 112 is columns 3 to 6, y 64 to 128 is rows 4 to 7.
    let picture = rows(&scene, 16.0);
    for row in &picture[4..8] {
        assert_eq!(&row[3..7], "BBBB", "{row}");
    }
    assert_eq!(picture[3], "..........");
    let text = render(&scene, &ruler(), 16.0);
    assert!(text
        .lines()
        .any(|line| line.contains("boss") && line.contains("warden")));
}

#[test]
fn puzzle_pieces_coins_charms_and_slots_are_drawn_and_listed() {
    let mut layout = room();
    layout.breakables = vec![ron::from_str("(rect: (32.0, 32.0, 16.0, 32.0), hits: 3)").unwrap()];
    layout.switches = vec![ron::from_str("(rect: (96.0, 48.0, 16.0, 16.0), id: 1)").unwrap()];
    layout.barriers = vec![ron::from_str("(rect: (112.0, 16.0, 16.0, 48.0), switch: 1)").unwrap()];
    layout.charms = vec![ron::from_str("(charm: \"swift\", position: (24.0, 24.0))").unwrap()];
    layout.slots = vec![ron::from_str("(position: (136.0, 136.0), kind: Key)").unwrap()];
    let mut scene = Scene::new(layout);
    scene.coins = vec![Vec2::new(8.0, 8.0)];
    let picture = rows(&scene, 16.0);
    assert_eq!(picture[0].chars().next(), Some('*'));
    assert_eq!(picture[1].chars().nth(1), Some('C'));
    assert_eq!(picture[2].chars().nth(2), Some('%'));
    assert_eq!(picture[3].chars().nth(6), Some('/'));
    assert_eq!(picture[1].chars().nth(7), Some('~'));
    assert_eq!(picture[8].chars().nth(8), Some('S'));
    let text = render(&scene, &ruler(), 16.0);
    for word in [
        "breakable",
        "switch 1",
        "opens with switch 1",
        "charm \"swift\"",
        "pickup slot for Key",
        "coin",
    ] {
        assert!(text.contains(word), "{word}\n{text}");
    }
}

#[test]
fn the_more_important_mark_wins_a_shared_cell_and_the_table_keeps_both() {
    let mut layout = room();
    layout.terrain = vec![(0.0, 128.0, 160.0, 32.0)];
    let mut hazard = HazardPlacement::standard(0.0, 0.0);
    hazard.rect = (64.0, 128.0, 32.0, 16.0);
    layout.hazards = vec![hazard];
    let scene = Scene::new(layout);
    let picture = rows(&scene, 16.0);
    assert_eq!(picture[8], "@@@#^^####", "{picture:?}");
    let text = render(&scene, &ruler(), 16.0);
    assert!(text.lines().any(|line| line.contains("hazard")));
    assert!(text.lines().any(|line| line.contains("terrain")));
}

#[test]
fn the_header_numbers_are_the_reach_code_s() {
    let ruler = ruler();
    let interval = ruler.interval;
    let text = render(&Scene::new(room()), &ruler, 16.0);
    let header: Vec<&str> = text.lines().take(8).collect();
    assert!(
        header[0].contains("160 x 160 px") && header[0].contains("floor y 160"),
        "{text}"
    );
    assert!(header[1].contains("Cell: 16 px per character") && header[1].contains("10 x 10"));
    assert!(
        header[2].contains("player 32 x 48 px (2.0 x 3.0 cells)"),
        "{}",
        header[2]
    );
    let line = |name: &str| {
        header
            .iter()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("{name}"))
    };
    let px = |value: f32| format!("{value:.0} px");
    let plain = line("no ability, no buffs");
    assert!(
        plain.contains(&format!("far {}", px(interval.with.jump_distance))),
        "{plain}"
    );
    assert!(
        plain.contains(&format!("high {}", px(interval.with.jump_height))),
        "{plain}"
    );
    let buffed = line("every buff at max");
    assert!(
        buffed.contains(&format!("far {}", px(interval.low()))),
        "{buffed}"
    );
    assert!(
        buffed.contains(&format!("high {}", px(interval.without.jump_height))),
        "{buffed}"
    );
    assert!(
        interval.low() > interval.with.jump_distance,
        "the fixture buffs the move speed"
    );
    for ability in Ability::ALL {
        let row = line(&format!("with {}", ability.name()));
        let (gap, height) = (interval.gap(*ability).high, interval.height(*ability).high);
        assert!(row.contains(&format!("far {}", px(gap))), "{row}");
        assert!(row.contains(&format!("({:.1} cells)", gap / 16.0)), "{row}");
        assert!(row.contains(&format!("high {}", px(height))), "{row}");
    }
    // What the editor's line says is the same measurement.
    let reach_line = crate::reach::line(&interval);
    assert!(reach_line.contains(&px(interval.low())), "{reach_line}");
}

#[test]
fn the_same_room_gives_the_same_text() {
    let mut layout = room();
    layout.terrain = vec![(32.0, 48.0, 64.0, 16.0), (80.0, 16.0, 4.0, 64.0)];
    layout.exits = vec![exit(RoomEdge::Left, 48.0, 32.0)];
    let scene = Scene::new(layout);
    assert_eq!(
        render(&scene, &ruler(), 16.0),
        render(&scene, &ruler(), 16.0)
    );
}

#[test]
fn an_empty_room_prints_its_header_picture_table_and_legend() {
    let mut layout = room();
    layout.spawn = (80.0, 136.0);
    let text = render(&Scene::new(layout), &ruler(), 16.0);
    assert!(
        text.starts_with("Room (unsaved room): 160 x 160 px"),
        "{text}"
    );
    assert!(text.contains("Placed things"));
    assert!(text
        .lines()
        .any(|line| line.contains("spawn") && line.contains("player start")));
    assert!(text.contains("Legend") && text.contains("Backgrounds and the look"));
    // Ten rows of ten characters, then the axis.
    assert_eq!(rows(&Scene::new(room()), 16.0).len(), 10);
}

#[test]
fn the_legend_is_printed_every_time_and_explains_every_mark() {
    let text = render(&Scene::new(room()), &ruler(), 8.0);
    let legend = text.split("Legend").nth(1).expect("a legend");
    for mark in [
        "#", "=", "|", "+", "@", "^", "%", "/", "~", "$", "D", "J", "K", "C", "*", "S", "E", "B",
        "<-->", "1-9, a-z",
    ] {
        assert!(legend.contains(mark), "{mark}");
    }
}

#[test]
fn a_room_name_and_id_are_in_the_header() {
    let mut layout = room();
    layout.name = Some("The Gate".into());
    let mut scene = Scene::new(layout);
    scene.id = Some(RoomId::Custom(7));
    let text = render(&scene, &ruler(), 16.0);
    assert!(
        text.starts_with("Room room_7 (\"The Gate\"): 160 x 160 px"),
        "{text}"
    );
}

#[test]
fn the_exit_marks_run_through_digits_then_letters() {
    assert_eq!(scene::exit_mark(0), '1');
    assert_eq!(scene::exit_mark(8), '9');
    assert_eq!(scene::exit_mark(9), 'a');
    assert_eq!(scene::exit_mark(34), 'z');
    assert_eq!(scene::exit_mark(35), '?');
}

#[test]
fn the_default_cell_keeps_the_widest_room_within_200_columns() {
    for widest in [100.0, 800.0, 960.0, 1_600.0, 3_000.0, 12_800.0, 50_000.0] {
        let cell = default_cell(widest);
        assert!(
            columns(widest, cell) <= DEFAULT_MAX_COLUMNS,
            "{widest}: {cell}"
        );
    }
    assert_eq!(default_cell(800.0), 4.0, "the smallest cell that fits");
    assert_eq!(default_cell(960.0), 8.0);
    assert_eq!(columns(960.0, 8.0), 120);
    assert_eq!(columns(100.0, 16.0), 7);
    assert_eq!(columns(160.0, 16.0), 10, "an exact multiple adds no column");
}

#[test]
fn a_cell_that_cannot_be_drawn_is_refused_with_the_reason() {
    let scene = Scene::new(room());
    assert_eq!(check_cell(&scene, 16.0), Ok(16.0));
    for bad in [0.0, 0.5, -4.0, f32::NAN, f32::INFINITY] {
        assert!(
            check_cell(&scene, bad).unwrap_err().contains("--cell"),
            "{bad}"
        );
    }
    let mut huge = room();
    huge.bounds = (0.0, 0.0, 40_000.0, 160.0);
    let error = check_cell(&Scene::new(huge), 1.0).unwrap_err();
    assert!(
        error.contains("40000") && error.contains("bigger cell"),
        "{error}"
    );
    // `render` itself never panics on a bad cell.
    assert!(render(&scene, &ruler(), f32::NAN).starts_with("Room"));
}
