mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{save_room, EditorSession, RoomDraft, RoomLayout};
use hollow_steps::editor_history::{redo_session, undo_session, History};
use hollow_steps::editor_terrain::{
    clamp_to_room, corners_rect, handle_at, resized, Handle, MIN_TERRAIN_SIZE,
};
use hollow_steps::editor_tools::{tool_shortcut, EditorTool, TerrainHit, TerrainTool};
use hollow_steps::editor_view::Grid;
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::room::{test_rooms, RoomEnemies, RoomId};
use macroquad::math::{Rect, Vec2};

fn assets() -> PathBuf {
    support::fixture_assets()
}

fn draft() -> RoomDraft {
    let rooms = test_rooms();
    let document =
        ron::from_str(&std::fs::read_to_string(assets().join("rooms/first.ron")).unwrap()).unwrap();
    RoomDraft::from_document(document, &rooms[0])
}

fn editing_game() -> GameState {
    let mut game = GameState::try_new(&assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

/// Clear floor space away from the spawn (x 164..196), items (x 300 and 500) and the exit.
const PLATFORM: Rect = Rect {
    x: 600.0,
    y: 380.0,
    w: 120.0,
    h: 20.0,
};

fn plain() -> Grid {
    Grid::default()
}

// ---- pure draft edits -------------------------------------------------------------------

#[test]
fn adding_a_solid_appends_it_and_returns_its_index() {
    let mut draft = draft();
    assert!(draft.layout.terrain.is_empty());
    assert_eq!(draft.add_terrain(PLATFORM), Ok(0));
    assert_eq!(
        draft.add_terrain(Rect::new(700.0, 300.0, 20.0, 80.0)),
        Ok(1)
    );
    assert_eq!(
        draft.layout.terrain,
        vec![(600.0, 380.0, 120.0, 20.0), (700.0, 300.0, 20.0, 80.0)]
    );
    assert_eq!(draft.terrain_rect(0), Some(PLATFORM));
    assert_eq!(draft.terrain_rects().len(), 2);
}

#[test]
fn a_solid_over_the_spawn_point_is_refused_and_the_draft_is_unchanged() {
    let mut draft = draft();
    let spawn = Vec2::from(draft.layout.spawn);
    let before = draft.clone();
    let error = draft
        .add_terrain(Rect::new(spawn.x - 40.0, spawn.y - 10.0, 80.0, 20.0))
        .unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert_eq!(draft, before);
}

#[test]
fn a_solid_over_an_item_or_an_exit_is_refused() {
    let mut draft = draft();
    let item = Vec2::from(draft.layout.items[0].position);
    let error = draft
        .add_terrain(Rect::new(item.x - 20.0, item.y - 30.0, 40.0, 30.0))
        .unwrap_err();
    assert!(error.contains("item"), "{error}");
    let room = draft.layout.rect();
    let error = draft
        .add_terrain(Rect::new(
            room.right() - 16.0,
            room.bottom() - 60.0,
            16.0,
            40.0,
        ))
        .unwrap_err();
    assert!(error.contains("exit"), "{error}");
    assert!(draft.layout.terrain.is_empty());
}

#[test]
fn solids_must_be_big_enough_and_inside_the_room() {
    let mut draft = draft();
    let room = draft.layout.rect();
    let thin = Rect::new(600.0, 380.0, MIN_TERRAIN_SIZE - 1.0, 40.0);
    assert!(draft.add_terrain(thin).unwrap_err().contains("at least"));
    assert!(draft
        .add_terrain(Rect::new(600.0, 380.0, f32::NAN, 40.0))
        .is_err());
    let outside = Rect::new(room.right() - 10.0, 300.0, 40.0, 40.0);
    assert!(draft.add_terrain(outside).unwrap_err().contains("inside"));
    assert!(draft.layout.terrain.is_empty());
}

#[test]
fn moving_keeps_the_size_and_stays_inside_the_room() {
    let mut draft = draft();
    let index = draft.add_terrain(PLATFORM).unwrap();
    draft.move_terrain(index, Vec2::new(650.0, 300.0)).unwrap();
    assert_eq!(
        draft.terrain_rect(index),
        Some(Rect::new(650.0, 300.0, 120.0, 20.0))
    );
    // Dragged past the right and top walls: pushed back inside, size unchanged.
    let room = draft.layout.rect();
    draft
        .move_terrain(index, Vec2::new(5_000.0, -500.0))
        .unwrap();
    assert_eq!(
        draft.terrain_rect(index),
        Some(Rect::new(room.right() - 120.0, room.y, 120.0, 20.0))
    );
    assert!(draft.move_terrain(index, Vec2::new(f32::NAN, 0.0)).is_err());
    assert!(draft.move_terrain(9, Vec2::ZERO).is_err());
}

#[test]
fn moving_a_solid_onto_the_spawn_is_refused_and_leaves_it_where_it_was() {
    let mut draft = draft();
    let index = draft.add_terrain(PLATFORM).unwrap();
    let spawn = Vec2::from(draft.layout.spawn);
    let before = draft.clone();
    let error = draft
        .move_terrain(index, Vec2::new(spawn.x - 60.0, spawn.y - 10.0))
        .unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert_eq!(draft, before);
}

#[test]
fn each_handle_moves_only_its_own_edges() {
    let rect = Rect::new(100.0, 100.0, 80.0, 40.0);
    let pointer = Vec2::new(150.0, 90.0);
    let check = |handle, expected: (f32, f32, f32, f32)| {
        let r = resized(&rect, handle, pointer, 8.0);
        assert_eq!((r.x, r.y, r.w, r.h), expected, "{handle:?}");
    };
    check(Handle::North, (100.0, 90.0, 80.0, 50.0));
    check(Handle::South, (100.0, 100.0, 80.0, 8.0));
    check(Handle::West, (150.0, 100.0, 30.0, 40.0));
    check(Handle::East, (100.0, 100.0, 50.0, 40.0));
    check(Handle::NorthWest, (150.0, 90.0, 30.0, 50.0));
    check(Handle::NorthEast, (100.0, 90.0, 50.0, 50.0));
    check(Handle::SouthWest, (150.0, 100.0, 30.0, 8.0));
    check(Handle::SouthEast, (100.0, 100.0, 50.0, 8.0));
}

#[test]
fn dragging_a_handle_past_the_far_edge_stops_at_the_minimum_size() {
    let rect = Rect::new(100.0, 100.0, 80.0, 40.0);
    let r = resized(&rect, Handle::East, Vec2::new(0.0, 0.0), 8.0);
    assert_eq!((r.x, r.w), (100.0, 8.0));
    let r = resized(&rect, Handle::North, Vec2::new(0.0, 500.0), 8.0);
    assert_eq!((r.y, r.h, r.bottom()), (132.0, 8.0, 140.0));
}

#[test]
fn resizing_changes_the_draft_and_is_kept_inside_the_room() {
    let mut draft = draft();
    // High enough that stretching it to the wall stays above the exit on the right edge.
    let index = draft
        .add_terrain(Rect::new(600.0, 300.0, 120.0, 20.0))
        .unwrap();
    draft
        .resize_terrain(index, Handle::SouthEast, Vec2::new(760.0, 340.0))
        .unwrap();
    assert_eq!(
        draft.terrain_rect(index),
        Some(Rect::new(600.0, 300.0, 160.0, 40.0))
    );
    let room = draft.layout.rect();
    draft
        .resize_terrain(index, Handle::East, Vec2::new(99_999.0, 0.0))
        .unwrap();
    assert_eq!(draft.terrain_rect(index).unwrap().right(), room.right());
    assert!(draft
        .resize_terrain(index, Handle::East, Vec2::new(f32::INFINITY, 0.0))
        .is_err());
    assert!(draft
        .resize_terrain(7, Handle::East, Vec2::new(700.0, 0.0))
        .is_err());
}

#[test]
fn resizing_over_the_spawn_is_refused() {
    let mut draft = draft();
    let index = draft.add_terrain(PLATFORM).unwrap();
    let before = draft.clone();
    // Stretching the west edge across the floor to x=150 covers the spawn at x 164..196.
    let error = draft
        .resize_terrain(index, Handle::SouthWest, Vec2::new(150.0, 440.0))
        .unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert_eq!(draft, before);
}

#[test]
fn deleting_removes_the_solid_and_shifts_later_ones() {
    let mut draft = draft();
    draft.add_terrain(PLATFORM).unwrap();
    draft
        .add_terrain(Rect::new(300.0, 300.0, 50.0, 20.0))
        .unwrap();
    draft
        .add_terrain(Rect::new(800.0, 300.0, 50.0, 20.0))
        .unwrap();
    draft.delete_terrain(0).unwrap();
    assert_eq!(
        draft.layout.terrain,
        vec![(300.0, 300.0, 50.0, 20.0), (800.0, 300.0, 50.0, 20.0)]
    );
    assert!(draft.delete_terrain(2).is_err());
    assert_eq!(draft.layout.terrain.len(), 2);
}

#[test]
fn the_topmost_solid_is_picked_and_the_nearest_handle_wins() {
    let mut draft = draft();
    draft.add_terrain(PLATFORM).unwrap();
    draft
        .add_terrain(Rect::new(640.0, 360.0, 60.0, 40.0))
        .unwrap();
    assert_eq!(draft.terrain_at(Vec2::new(650.0, 390.0)), Some(1));
    assert_eq!(draft.terrain_at(Vec2::new(610.0, 390.0)), Some(0));
    assert_eq!(draft.terrain_at(Vec2::new(100.0, 100.0)), None);

    let rect = PLATFORM;
    assert_eq!(
        handle_at(&rect, Vec2::new(601.0, 381.0), 6.0),
        Some(Handle::NorthWest)
    );
    assert_eq!(
        handle_at(&rect, Vec2::new(660.0, 379.0), 6.0),
        Some(Handle::North)
    );
    assert_eq!(
        handle_at(&rect, Vec2::new(719.0, 390.0), 6.0),
        Some(Handle::East)
    );
    assert_eq!(handle_at(&rect, Vec2::new(660.0, 390.0), 6.0), None);
    assert_eq!(handle_at(&rect, Vec2::new(601.0, 381.0), f32::NAN), None);
}

#[test]
fn corners_and_clamping_are_order_independent() {
    let a = Vec2::new(300.0, 200.0);
    let b = Vec2::new(100.0, 250.0);
    assert_eq!(corners_rect(a, b), Rect::new(100.0, 200.0, 200.0, 50.0));
    assert_eq!(corners_rect(b, a), corners_rect(a, b));
    let room = Rect::new(0.0, 0.0, 960.0, 460.0);
    assert_eq!(
        clamp_to_room(&room, Vec2::new(-5.0, 900.0)),
        Vec2::new(0.0, 460.0)
    );
    assert!(!clamp_to_room(&room, Vec2::new(f32::NAN, 0.0)).is_finite());
}

// ---- save and reload --------------------------------------------------------------------

fn catalog(draft: &RoomDraft) -> Vec<(RoomId, RoomLayout)> {
    test_rooms()
        .iter()
        .map(|room| {
            if room.id == RoomId::First {
                (room.id, draft.layout.clone())
            } else {
                (room.id, RoomLayout::from_room(room))
            }
        })
        .collect()
}

#[test]
fn terrain_survives_a_ron_save_and_reload() {
    let mut draft = draft();
    draft.add_terrain(PLATFORM).unwrap();
    draft
        .add_terrain(Rect::new(300.0, 300.0, 50.0, 20.0))
        .unwrap();
    let definitions = EnemyDefinitions::load(&assets()).unwrap();
    let source = draft.serialize(&definitions, &catalog(&draft)).unwrap();
    let document: RoomEnemies = ron::from_str(&source).unwrap();
    let reloaded = RoomDraft::from_document(document, &test_rooms()[0]);
    assert_eq!(reloaded.layout.terrain, draft.layout.terrain);
    assert_eq!(reloaded, draft);
    let room = reloaded.build(RoomId::First, &definitions).unwrap();
    assert_eq!(room.terrain.solids().len(), 2);
    assert_eq!(room.terrain.solids()[0], PLATFORM);
}

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-terrain-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
}
impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn a_saved_room_file_loads_with_its_terrain_and_a_refused_edit_never_reaches_disk() {
    let temp = TempAssets::new();
    let path = temp.0.join("rooms").join("first.ron");
    let definitions = EnemyDefinitions::load(&temp.0).unwrap();
    let mut draft = draft();
    draft.add_terrain(PLATFORM).unwrap();
    save_room(&path, &draft, &definitions, &catalog(&draft), false).unwrap();
    let saved = std::fs::read(&path).unwrap();

    let game = GameState::try_new(&temp.0).unwrap();
    assert_eq!(game.room().terrain.solids(), [PLATFORM]);

    // A solid over the spawn never gets as far as the file.
    let spawn = Vec2::from(draft.layout.spawn);
    assert!(draft
        .add_terrain(Rect::new(spawn.x - 40.0, spawn.y - 10.0, 80.0, 20.0))
        .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), saved);
}

// ---- tool state -------------------------------------------------------------------------

#[test]
fn tool_shortcuts_pick_one_tool_and_ignore_modifiers() {
    use EditorTool::*;
    assert_eq!(
        tool_shortcut(false, true, false, false, false),
        Some(Select)
    );
    assert_eq!(tool_shortcut(false, false, true, false, false), Some(Place));
    assert_eq!(
        tool_shortcut(false, false, false, true, false),
        Some(Terrain)
    );
    assert_eq!(tool_shortcut(false, false, false, false, true), Some(Erase));
    assert_eq!(tool_shortcut(false, false, false, false, false), None);
    // Ctrl+E and friends are left alone.
    assert_eq!(tool_shortcut(true, false, false, false, true), None);
    assert_eq!(EditorTool::default(), Select);
    let keys: Vec<char> = EditorTool::ALL.iter().map(|t| t.key()).collect();
    assert_eq!(keys, ['V', 'P', 'T', 'E']);
}

fn draw(
    tool: &mut TerrainTool,
    session: &mut EditorSession,
    history: &mut History,
    grid: &Grid,
    from: Vec2,
    to: Vec2,
) -> Result<(), String> {
    tool.begin_draw(session, from, grid);
    tool.drag(session, (from + to) / 2.0, grid)?;
    tool.drag(session, to, grid)?;
    tool.release(session, history)
}

#[test]
fn drawing_a_rectangle_adds_one_solid_selects_it_and_is_one_undo_step() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    session.dirty = false;
    let from = Vec2::new(720.0, 400.0);
    let to = Vec2::new(600.0, 380.0);
    draw(&mut tool, session, &mut history, &plain(), from, to).unwrap();
    assert_eq!(
        session.draft.layout.terrain,
        vec![(600.0, 380.0, 120.0, 20.0)]
    );
    assert_eq!(tool.selected(), Some(0));
    assert!(session.dirty);
    assert!(!tool.has_gesture());

    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert!(session.draft.layout.terrain.is_empty());
    assert_eq!(redo_session(&mut history, session), Ok(true));
    assert_eq!(session.draft.layout.terrain.len(), 1);
}

#[test]
fn the_drawn_rectangle_snaps_to_the_grid_and_previews_while_dragging() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    let grid = Grid {
        enabled: true,
        size: 16.0,
    };
    tool.begin_draw(session, Vec2::new(605.0, 383.0), &grid);
    tool.drag(session, Vec2::new(711.0, 405.0), &grid).unwrap();
    assert_eq!(tool.preview(), Some(Rect::new(608.0, 384.0, 96.0, 16.0)));
    assert!(
        session.draft.layout.terrain.is_empty(),
        "nothing is added until release"
    );
    tool.release(session, &mut history).unwrap();
    assert_eq!(
        session.draft.layout.terrain,
        vec![(608.0, 384.0, 96.0, 16.0)]
    );
    assert_eq!(tool.preview(), None);
}

#[test]
fn drawing_past_the_room_edge_stops_at_the_wall() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    let room = session.draft.layout.rect();
    draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        Vec2::new(room.right() - 100.0, 300.0),
        Vec2::new(room.right() + 400.0, 320.0),
    )
    .unwrap();
    let solid = session.draft.terrain_rect(0).unwrap();
    assert_eq!((solid.right(), solid.w), (room.right(), 100.0));
}

#[test]
fn a_plain_click_adds_nothing_and_a_sliver_is_refused_with_a_message() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    session.dirty = false;
    let at = Vec2::new(650.0, 390.0);
    draw(&mut tool, session, &mut history, &plain(), at, at).unwrap();
    assert!(session.draft.layout.terrain.is_empty());
    assert!(!session.dirty && !history.can_undo());

    let error = draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        at,
        at + Vec2::new(3.0, 40.0),
    )
    .unwrap_err();
    assert!(error.contains("at least"), "{error}");
    assert!(session.draft.layout.terrain.is_empty());
    assert!(!history.can_undo() && !tool.has_gesture());
}

#[test]
fn drawing_over_the_spawn_is_refused_and_records_no_undo_step() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    let spawn = Vec2::from(session.draft.layout.spawn);
    let error = draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        spawn - Vec2::new(40.0, 10.0),
        spawn + Vec2::new(40.0, 10.0),
    )
    .unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert!(session.draft.layout.terrain.is_empty());
    assert!(!history.can_undo());
    assert_eq!(tool.selected(), None);
}

fn with_platform(session: &mut EditorSession) -> (TerrainTool, History) {
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        PLATFORM.point(),
        PLATFORM.point() + PLATFORM.size(),
    )
    .unwrap();
    tool.select(None);
    (tool, history)
}

#[test]
fn dragging_a_solid_moves_it_by_the_grab_offset_as_one_undo_step() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    session.dirty = false;
    let press = Vec2::new(620.0, 390.0);
    let hit = TerrainTool::hit_body(&session.draft, press).unwrap();
    assert_eq!(hit, TerrainHit::Body(0));
    tool.grab(session, &mut history, hit, press);
    assert_eq!(tool.selected(), Some(0));
    for step in 1..=5 {
        let at = press + Vec2::new(10.0 * step as f32, -4.0 * step as f32);
        tool.drag(session, at, &plain()).unwrap();
    }
    tool.release(session, &mut history).unwrap();
    assert_eq!(
        session.draft.terrain_rect(0),
        Some(Rect::new(650.0, 360.0, 120.0, 20.0))
    );
    assert!(session.dirty);
    // The five drag frames collapse into one step: a single undo returns to the start.
    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert_eq!(session.draft.terrain_rect(0), Some(PLATFORM));
    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert!(session.draft.layout.terrain.is_empty());
}

#[test]
fn a_click_on_a_solid_selects_it_without_marking_the_room_changed() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    session.dirty = false;
    let steps_before = history.can_undo();
    let press = Vec2::new(620.0, 390.0);
    let hit = TerrainTool::hit_body(&session.draft, press).unwrap();
    tool.grab(session, &mut history, hit, press);
    tool.release(session, &mut history).unwrap();
    assert_eq!(tool.selected(), Some(0));
    assert!(!session.dirty);
    assert_eq!(session.draft.terrain_rect(0), Some(PLATFORM));
    // Only the drawing step is on the stack, not a second no-op step.
    assert_eq!(steps_before, history.can_undo());
    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert_eq!(undo_session(&mut history, session), Ok(false));
}

#[test]
fn a_move_the_rules_refuse_keeps_the_last_valid_spot_and_reports_why() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    let spawn = Vec2::from(session.draft.layout.spawn);
    let press = Vec2::new(620.0, 390.0);
    tool.grab(session, &mut history, TerrainHit::Body(0), press);
    tool.drag(session, Vec2::new(500.0, 390.0), &plain())
        .unwrap();
    let valid = session.draft.terrain_rect(0).unwrap();
    // The solid would now cover the player's spawn.
    let error = tool
        .drag(session, spawn + Vec2::new(10.0, 0.0), &plain())
        .unwrap_err();
    assert!(error.contains("spawn"), "{error}");
    assert_eq!(session.draft.terrain_rect(0), Some(valid));
    tool.release(session, &mut history).unwrap();
    assert_eq!(session.draft.terrain_rect(0), Some(valid));
}

#[test]
fn a_snapped_move_lands_on_grid_lines() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    let grid = Grid {
        enabled: true,
        size: 8.0,
    };
    let press = Vec2::new(620.0, 390.0);
    tool.grab(session, &mut history, TerrainHit::Body(0), press);
    tool.drag(session, Vec2::new(633.0, 391.0), &grid).unwrap();
    tool.release(session, &mut history).unwrap();
    let moved = session.draft.terrain_rect(0).unwrap();
    assert_eq!((moved.x % 8.0, moved.y % 8.0), (0.0, 0.0));
}

#[test]
fn only_the_selected_solid_has_handles_and_dragging_one_resizes_in_one_step() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    let corner = PLATFORM.point() + PLATFORM.size();
    assert_eq!(
        tool.hit_handle(&session.draft, corner, 6.0),
        None,
        "nothing selected"
    );
    tool.select(Some(0));
    let hit = tool.hit_handle(&session.draft, corner + Vec2::new(2.0, 2.0), 6.0);
    assert_eq!(hit, Some(TerrainHit::Handle(0, Handle::SouthEast)));
    tool.grab(session, &mut history, hit.unwrap(), corner);
    for step in 1..=4 {
        let at = corner + Vec2::new(10.0 * step as f32, 5.0 * step as f32);
        tool.drag(session, at, &plain()).unwrap();
    }
    tool.release(session, &mut history).unwrap();
    assert_eq!(
        session.draft.terrain_rect(0),
        Some(Rect::new(600.0, 380.0, 160.0, 40.0))
    );
    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert_eq!(session.draft.terrain_rect(0), Some(PLATFORM));
}

#[test]
fn erasing_removes_the_topmost_solid_under_the_cursor_and_undo_brings_it_back() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        Vec2::new(300.0, 300.0),
        Vec2::new(360.0, 320.0),
    )
    .unwrap();
    session.dirty = false;
    assert_eq!(
        tool.erase_at(session, &mut history, Vec2::new(10.0, 10.0)),
        Ok(false)
    );
    assert!(!session.dirty && session.draft.layout.terrain.len() == 2);
    assert_eq!(
        tool.erase_at(session, &mut history, Vec2::new(310.0, 310.0)),
        Ok(true)
    );
    assert_eq!(
        session.draft.layout.terrain,
        vec![(600.0, 380.0, 120.0, 20.0)]
    );
    assert!(session.dirty);
    assert_eq!(undo_session(&mut history, session), Ok(true));
    assert_eq!(session.draft.layout.terrain.len(), 2);
}

#[test]
fn deleting_keeps_the_selection_pointing_at_the_same_solid() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = with_platform(session);
    draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        Vec2::new(300.0, 300.0),
        Vec2::new(360.0, 320.0),
    )
    .unwrap();
    assert_eq!(tool.selected(), Some(1));
    tool.erase_at(session, &mut history, PLATFORM.center())
        .unwrap();
    assert_eq!(
        tool.selected(),
        Some(0),
        "the selected solid shifted down one"
    );
    assert_eq!(
        session.draft.terrain_rect(0),
        Some(Rect::new(300.0, 300.0, 60.0, 20.0))
    );
    assert_eq!(tool.delete_selected(session, &mut history), Ok(true));
    assert_eq!(tool.selected(), None);
    assert_eq!(tool.delete_selected(session, &mut history), Ok(false));
    assert!(session.draft.layout.terrain.is_empty());
}

#[test]
fn undo_that_removes_the_selected_solid_leaves_no_dangling_selection() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, mut history) = (TerrainTool::default(), History::default());
    draw(
        &mut tool,
        session,
        &mut history,
        &plain(),
        PLATFORM.point(),
        PLATFORM.point() + PLATFORM.size(),
    )
    .unwrap();
    assert_eq!(tool.selected(), Some(0));
    undo_session(&mut history, session).unwrap();
    tool.sync(&session.draft);
    assert_eq!(tool.selected(), None);

    // A drag in progress on a solid that undo removed is dropped too.
    redo_session(&mut history, session).unwrap();
    tool.grab(
        session,
        &mut history,
        TerrainHit::Body(0),
        PLATFORM.center(),
    );
    history.clear();
    session.draft.layout.terrain.clear();
    tool.sync(&session.draft);
    assert!(!tool.has_gesture());
    assert_eq!(tool.selected(), None);
}

#[test]
fn clearing_the_tool_drops_the_selection_and_any_drag() {
    let mut game = editing_game();
    let session = game.editor_mut().unwrap();
    let (mut tool, _history) = with_platform(session);
    tool.begin_draw(session, PLATFORM.point(), &plain());
    assert!(tool.has_gesture());
    tool.select(Some(0));
    tool.clear();
    assert!(!tool.has_gesture());
    assert_eq!(tool.selected(), None);
}
