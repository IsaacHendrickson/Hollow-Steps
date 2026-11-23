//! `mirror_layout` and `mirror_document`: one test for each kind of thing that moves.
use super::rich::rich;
use crate::editor::RoomLayout;
use crate::gates::{Entrance, ExitRef};
use crate::hazard_look::HazardFacing;
use crate::mirror::{mirror_document, mirror_layout, mirror_point};
use crate::room::{BossPlacement, EnemyPlacement, RoomEdge, RoomEnemies};
use macroquad::math::Rect;

fn mirrored() -> RoomLayout {
    mirror_layout(&rich())
}

#[test]
fn the_rich_room_is_valid_before_and_after() {
    rich().validate_geometry().unwrap();
    mirrored().validate_geometry().unwrap();
}

#[test]
fn mirroring_twice_gives_the_room_back_exactly() {
    let room = rich();
    assert_eq!(mirror_layout(&mirror_layout(&room)), room);
}

#[test]
fn mirroring_twice_gives_fractional_coordinates_back_within_a_thousandth() {
    let mut room = rich();
    room.spawn = (200.37, 406.0);
    room.terrain = vec![(500.61, 380.0, 99.77, 50.0)];
    room.items[0].position = (350.13, 400.0);
    room.exits[2].offset = 200.29;
    room.exits[2].length = 99.91;
    room.exits[4].door = Some((400.43, 358.0, 47.77, 72.0));
    let back = mirror_layout(&mirror_layout(&room));
    let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
    assert!(close(back.spawn.0, room.spawn.0) && back.spawn.1 == room.spawn.1);
    assert!(close(back.terrain[0].0, room.terrain[0].0) && back.terrain[0].2 == 99.77);
    assert!(close(back.items[0].position.0, 350.13));
    assert!(close(back.exits[2].offset, 200.29) && back.exits[2].length == 99.91);
    let (x, _, width, _) = back.exits[4].door.unwrap();
    assert!(close(x, 400.43) && width == 47.77);
}

#[test]
fn the_spawn_is_mirrored() {
    assert_eq!(mirrored().spawn, (800.0, 406.0));
}

#[test]
fn terrain_is_mirrored_by_its_far_edge() {
    assert_eq!(mirrored().terrain, vec![(400.0, 380.0, 100.0, 50.0)]);
}

#[test]
fn hazards_are_mirrored_and_turn_to_face_the_other_way() {
    let hazards = mirrored().hazards;
    let rects: Vec<_> = hazards.iter().map(|hazard| hazard.rect).collect();
    assert_eq!(
        rects,
        [
            (236.0, 414.0, 64.0, 16.0),
            (844.0, 200.0, 16.0, 64.0),
            (144.0, 200.0, 16.0, 64.0),
            (536.0, 60.0, 64.0, 16.0),
        ]
    );
    let facing: Vec<_> = hazards.iter().map(|hazard| hazard.facing).collect();
    assert_eq!(
        facing,
        [
            HazardFacing::Up,
            HazardFacing::Right,
            HazardFacing::Left,
            HazardFacing::Down
        ]
    );
    // Everything about the hit is as it was.
    assert_eq!(hazards[1].damage, rich().hazards[1].damage);
    assert_eq!(
        hazards[1].knockback_speed,
        rich().hazards[1].knockback_speed
    );
}

#[test]
fn breakables_switches_and_barriers_are_mirrored() {
    let room = mirrored();
    assert_eq!(room.breakables[0].rect, (336.0, 330.0, 24.0, 100.0));
    assert_eq!(room.breakables[0].hits, 2);
    assert_eq!(room.switches[0].rect, (734.0, 300.0, 16.0, 24.0));
    assert_eq!(room.switches[0].id, 1);
    assert_eq!(room.barriers[0].rect, (176.0, 330.0, 24.0, 100.0));
    assert_eq!(room.barriers[0].switch, 1);
}

#[test]
fn items_and_charms_are_mirrored() {
    let room = mirrored();
    assert_eq!(room.items[0].position, (650.0, 400.0));
    assert_eq!(room.items[0].item, rich().items[0].item);
    assert_eq!(room.charms[0].position, (550.0, 410.0));
    assert_eq!(room.charms[0].charm, "ember");
}

#[test]
fn the_map_cell_runs_the_other_way_and_the_row_stays() {
    assert_eq!(mirrored().map_cell, Some((-3, -2)));
    let mut room = rich();
    room.map_cell = None;
    assert_eq!(mirror_layout(&room).map_cell, None);
    room.map_cell = Some((0, 5));
    assert_eq!(mirror_layout(&room).map_cell, Some((0, 5)));
}

#[test]
fn a_left_exit_and_a_right_exit_swap_edges_and_keep_their_height() {
    let exits = mirrored().exits;
    assert_eq!(
        (exits[0].edge, exits[0].offset, exits[0].length),
        (RoomEdge::Right, 300.0, 96.0)
    );
    assert_eq!(
        (exits[1].edge, exits[1].offset, exits[1].length),
        (RoomEdge::Left, 300.0, 96.0)
    );
}

#[test]
fn a_top_or_bottom_exit_keeps_its_edge_and_ends_where_it_began() {
    let exits = mirrored().exits;
    // 800 wide: 800 - 200 - 100, and 800 - 150 - 80.
    assert_eq!(
        (exits[2].edge, exits[2].offset, exits[2].length),
        (RoomEdge::Top, 500.0, 100.0)
    );
    assert_eq!(
        (exits[3].edge, exits[3].offset, exits[3].length),
        (RoomEdge::Bottom, 570.0, 80.0)
    );
}

#[test]
fn a_door_flips_its_rectangle_and_nothing_else() {
    let exits = mirrored().exits;
    assert_eq!(exits[4].door, Some((552.0, 358.0, 48.0, 72.0)));
    assert_eq!(exits[4].locked_by, None);
    assert_eq!(exits[4].target_room, rich().exits[4].target_room);
}

#[test]
fn every_exit_zone_is_the_mirror_image_of_the_old_one() {
    let (before, after) = (rich(), mirrored());
    for (old, new) in before.exits.iter().zip(&after.exits) {
        let old = old.exit().zone(before.rect());
        let new = new.exit().zone(after.rect());
        let (x, y, w, h) = (old.x, old.y, old.w, old.h);
        let image = crate::mirror::Axis::of(before.bounds).rect((x, y, w, h));
        assert_eq!(new, Rect::new(image.0, image.1, image.2, image.3));
    }
}

#[test]
fn an_exit_keeps_its_destination_arrival_and_lock() {
    let mut room = rich();
    room.exits[1].locked_by = Some(3);
    room.exits[1].spawn = (123.0, 456.0);
    let exit = &mirror_layout(&room).exits[1];
    assert_eq!(exit.spawn, (123.0, 456.0), "the arrival is in another room");
    assert_eq!(exit.locked_by, Some(3));
    assert_eq!(exit.target_room, room.exits[1].target_room);
}

#[test]
fn a_gate_follows_its_exit_and_keeps_its_entrance_and_needs() {
    let room = mirrored();
    // The Right exit at 300 is now the Left one; the Bottom exit at 150 (80 long) is at 570.
    assert_eq!(
        room.gates[0].exit,
        ExitRef::Edge {
            edge: RoomEdge::Left,
            offset: 300.0
        }
    );
    assert_eq!(room.gates[0].from, Entrance::Spawn);
    assert_eq!(
        room.gates[1].exit,
        ExitRef::Edge {
            edge: RoomEdge::Bottom,
            offset: 570.0
        }
    );
    // An arrival names an exit of another room, which this room cannot look up.
    assert_eq!(room.gates[1].from, rich().gates[1].from);
    assert_eq!(room.gates[1].needs, rich().gates[1].needs);
    room.validate_gates().unwrap();
}

#[test]
fn what_is_drawing_only_or_not_in_the_room_is_kept() {
    let (before, after) = (rich(), mirrored());
    assert_eq!(after.name, before.name);
    assert_eq!(after.bounds, before.bounds);
    assert_eq!(after.floor_y, before.floor_y);
    assert_eq!(after.background, before.background);
    assert_eq!(after.terrain_style, before.terrain_style);
}

#[test]
fn a_room_with_nothing_in_it_mirrors_to_itself_apart_from_its_spawn() {
    let mut room = rich();
    room.terrain.clear();
    room.hazards.clear();
    room.exits.clear();
    room.gates.clear();
    room.items.clear();
    room.charms.clear();
    room.clear_puzzles();
    let mut expected = room.clone();
    expected.spawn = (800.0, 406.0);
    expected.map_cell = Some((-3, -2));
    assert_eq!(mirror_layout(&room), expected);
}

#[test]
fn a_point_mirrors_in_the_bounds_of_its_room() {
    assert_eq!(
        mirror_point((100.0, 50.0, 800.0, 400.0), (250.0, 77.0)),
        (750.0, 77.0)
    );
    assert_eq!(
        mirror_point((0.0, 0.0, 960.0, 460.0), (160.0, 376.0)),
        (800.0, 376.0)
    );
    assert_eq!(
        mirror_point((-200.0, 0.0, 400.0, 100.0), (-150.0, 5.0)),
        (150.0, 5.0)
    );
}

fn document() -> RoomEnemies {
    let mut slow = EnemyPlacement::new("walker", 200.0, 400.0);
    slow.min_tier = 2;
    slow.chance = 50;
    let mut grouped = EnemyPlacement::new("flier", 150.0, 150.0);
    grouped.group = Some(3);
    RoomEnemies {
        enemies: vec![slow, grouped],
        boss: Some(BossPlacement::new("warden", (300.0, 380.0))),
        layout: Some(rich()),
    }
}

#[test]
fn patrols_swap_ends_and_flip() {
    let mirrored = mirror_document(&document());
    // 1000 - 400 and 1000 - 200: the old right end is the new left end.
    assert_eq!(
        (
            mirrored.enemies[0].patrol_left,
            mirrored.enemies[0].patrol_right
        ),
        (600.0, 800.0)
    );
    assert_eq!(
        (
            mirrored.enemies[1].patrol_left,
            mirrored.enemies[1].patrol_right
        ),
        (850.0, 850.0)
    );
    for (old, new) in document().enemies.iter().zip(&mirrored.enemies) {
        assert!(new.patrol_left <= new.patrol_right);
        assert_eq!(
            (&new.definition, new.min_tier, new.group, new.chance),
            (&old.definition, old.min_tier, old.group, old.chance)
        );
    }
}

#[test]
fn the_boss_is_mirrored_and_keeps_its_height_and_definition() {
    let boss = mirror_document(&document()).boss.unwrap();
    assert_eq!(boss.position, (700.0, 380.0));
    assert_eq!(boss.definition, "warden");
}

#[test]
fn a_document_carries_the_mirrored_layout_and_mirrors_back() {
    let mirrored = mirror_document(&document());
    assert_eq!(mirrored.layout, Some(mirror_layout(&rich())));
    assert_eq!(mirror_document(&mirrored), document());
}

#[test]
fn a_document_with_no_layout_comes_back_unchanged() {
    let mut plain = document();
    plain.layout = None;
    assert_eq!(mirror_document(&plain), plain);
}
