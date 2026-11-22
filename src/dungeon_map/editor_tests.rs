//! The editor's map window model: which rooms are marked and what a click hits. The session side
//! (set a cell, undo, drafts on the map, the conflict problem) is in `tests/editor_map.rs`.
use macroquad::math::{vec2, Rect};

use super::*;
use crate::room::RoomEdge;

fn id(n: usize) -> RoomId {
    RoomId::Custom(n)
}

fn room(n: usize, cell: Option<Cell>, exits: &[(RoomEdge, usize)]) -> MapRoomInput {
    MapRoomInput {
        id: id(n),
        name: format!("R{n}"),
        cell,
        exits: exits
            .iter()
            .map(|&(edge, target)| MapExit {
                edge: Some(edge),
                target: id(target),
                lock: None,
            })
            .collect(),
    }
}

fn entry(map: &EditorMap, n: usize) -> &EditorMapRoom {
    map.rooms.iter().find(|room| room.id == id(n)).unwrap()
}

#[test]
fn every_room_is_listed_with_its_cell_and_the_current_one_is_marked() {
    let layout = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1)]),
        room(1, None, &[(RoomEdge::Left, 0)]),
    ]);
    let map = EditorMap::new(&layout, id(1));
    assert_eq!(map.rooms.len(), 2);
    assert_eq!(entry(&map, 0).cell, (0, 0));
    assert_eq!(entry(&map, 1).cell, (1, 0));
    assert_eq!(entry(&map, 1).name, "R1");
    assert!(entry(&map, 1).current && !entry(&map, 0).current);
    assert_eq!(map.cells(), [(0, 0), (1, 0)]);
    assert_eq!(map.links.len(), 2);
}

#[test]
fn a_room_no_exit_leads_to_is_marked() {
    // 0 -> 1, nothing leads into 0; 2 leads nowhere and nothing leads to it.
    let layout = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1)]),
        room(1, None, &[]),
        room(2, None, &[]),
    ]);
    let map = EditorMap::new(&layout, id(0));
    assert!(entry(&map, 0).no_entrance);
    assert!(!entry(&map, 1).no_entrance);
    assert!(entry(&map, 2).no_entrance);
}

#[test]
fn a_room_that_only_leads_to_itself_still_has_no_entrance() {
    let layout = MapLayout::build(&[room(0, None, &[(RoomEdge::Right, 0)])]);
    assert!(entry(&EditorMap::new(&layout, id(0)), 0).no_entrance);
}

#[test]
fn rooms_on_one_cell_are_marked_as_a_conflict() {
    let layout = MapLayout::build(&[
        room(0, Some((1, 1)), &[]),
        room(1, Some((1, 1)), &[]),
        room(2, Some((4, 4)), &[]),
    ]);
    let map = EditorMap::new(&layout, id(2));
    assert!(entry(&map, 0).conflict && entry(&map, 1).conflict);
    assert!(!entry(&map, 2).conflict);
    assert_ne!(entry(&map, 1).cell, (1, 1), "the second one was moved");
}

#[test]
fn an_empty_map_has_no_rooms_and_nothing_to_hit() {
    let map = EditorMap::new(&MapLayout::default(), id(0));
    assert!(map.rooms.is_empty() && map.cells().is_empty());
    let grid = MapGrid::fit(Rect::new(0.0, 0.0, 300.0, 200.0), &map.cells());
    assert_eq!(map.room_at(&grid, vec2(150.0, 100.0)), None);
}

#[test]
fn a_click_hits_the_box_it_is_in_and_nothing_in_the_gaps_or_outside() {
    let layout = MapLayout::build(&[
        room(0, None, &[(RoomEdge::Right, 1)]),
        room(1, None, &[(RoomEdge::Bottom, 2)]),
        room(2, None, &[]),
    ]);
    let map = EditorMap::new(&layout, id(0));
    let area = Rect::new(0.0, 0.0, 400.0, 300.0);
    let grid = MapGrid::fit(area, &map.cells());
    for room in &map.rooms {
        let at = grid.center(room.cell);
        assert_eq!(map.room_at(&grid, at), Some(room.id), "{}", room.name);
        let rect = grid.room_rect(room.cell);
        assert_eq!(
            map.room_at(&grid, vec2(rect.x + 1.0, rect.y + 1.0)),
            Some(room.id),
            "just inside the corner"
        );
        assert_eq!(
            map.room_at(&grid, vec2(rect.x - 1.0, rect.y - 1.0)),
            None,
            "just outside"
        );
    }
    // Between the boxes of (0, 0) and (1, 0).
    let gap = (grid.center((0, 0)) + grid.center((1, 0))) / 2.0;
    assert_eq!(map.room_at(&grid, gap), None);
    assert_eq!(map.room_at(&grid, vec2(-50.0, -50.0)), None);
}

#[test]
fn a_draft_cell_is_checked_like_a_loaded_one() {
    let template = crate::editor::RoomLayout::from_room(&crate::room::test_rooms()[0]);
    let mut draft = crate::editor::RoomDraft::new_room(&template);
    assert_eq!(draft.layout.map_cell, None);
    assert_eq!(draft.set_map_cell(Some((64, -64))), Ok(()));
    assert_eq!(draft.layout.map_cell, Some((64, -64)));
    let error = draft.set_map_cell(Some((65, 0))).unwrap_err();
    assert!(
        error.contains("(65, 0)") && error.contains("out of range"),
        "{error}"
    );
    assert_eq!(draft.layout.map_cell, Some((64, -64)), "refused, unchanged");
    assert_eq!(draft.set_map_cell(None), Ok(()));
    assert_eq!(draft.layout.map_cell, None);
}
