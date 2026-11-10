mod support;

use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::room::{test_rooms, RoomId};
use hollow_steps::room_size::{resize_session, resized, RoomSize};

fn setup() -> (RoomDraft, EnemyDefinitions) {
    let rooms = test_rooms();
    let path = support::fixture_assets().join("rooms").join("first.ron");
    let document = ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    (
        RoomDraft::from_document(document, &rooms[0]),
        EnemyDefinitions::load(&support::fixture_assets()).unwrap(),
    )
}

/// The fixture room has an enemy, two items, an exit and a spawn; tests that target one
/// kind of content clear the others so a different check cannot be what rejects.
fn bare(draft: &mut RoomDraft) {
    draft.enemies.clear();
    draft.layout.items.clear();
    draft.layout.exits.clear();
    draft.layout.spawn = (180.0, 380.0);
}

fn size(draft: &RoomDraft) -> RoomSize {
    RoomSize::of(&draft.layout)
}

#[test]
fn room_size_reads_and_writes_the_layout() {
    let (draft, _) = setup();
    let mut layout = draft.layout.clone();
    let mut s = RoomSize::of(&layout);
    assert_eq!((s.x, s.y, s.width, s.height), layout.bounds);
    assert_eq!(s.floor_y, layout.floor_y);
    s.width += 40.0;
    s.floor_y -= 8.0;
    s.apply_to(&mut layout);
    assert_eq!(layout.bounds.2, draft.layout.bounds.2 + 40.0);
    assert_eq!(layout.floor_y, draft.layout.floor_y - 8.0);
}

#[test]
fn growing_the_room_is_accepted_and_leaves_the_original_draft_alone() {
    let (draft, definitions) = setup();
    let mut s = size(&draft);
    s.width = 1400.0;
    s.height += 100.0;
    let next = resized(&draft, s, &definitions, None).unwrap();
    assert_eq!(next.layout.bounds.2, 1400.0);
    assert_eq!(next.layout.bounds.3, draft.layout.bounds.3 + 100.0);
    assert_eq!(next.layout.items, draft.layout.items);
    assert_eq!(next.enemies.len(), draft.enemies.len());
    assert_ne!(draft.layout.bounds.2, 1400.0);
}

#[test]
fn shrinking_past_an_enemys_patrol_is_rejected() {
    let (draft, definitions) = setup();
    let mut s = size(&draft);
    s.width = 700.0; // the patrol runs to x = 800
    assert!(draft.enemies.iter().any(|e| e.patrol_right > 700.0));
    assert!(resized(&draft, s, &definitions, None).is_err());
}

#[test]
fn shrinking_past_an_item_is_rejected() {
    let (mut draft, definitions) = setup();
    draft.enemies.clear();
    let item_x = draft
        .layout
        .items
        .iter()
        .map(|i| i.position.0)
        .fold(0.0, f32::max);
    let mut s = size(&draft);
    s.width = item_x - 10.0;
    let error = resized(&draft, s, &definitions, None).unwrap_err();
    assert!(error.contains("item"), "{error}");
    // The same shrink is fine once the item is out of the way.
    draft.layout.items.clear();
    assert!(resized(&draft, s, &definitions, None).is_ok());
}

#[test]
fn shrinking_past_the_spawn_is_rejected() {
    let (mut draft, definitions) = setup();
    bare(&mut draft);
    let mut s = size(&draft);
    s.width = 150.0; // the spawn body spans x = 164..196
    let error = resized(&draft, s, &definitions, None).unwrap_err();
    assert!(error.contains("spawn"), "{error}");
}

#[test]
fn lowering_the_ceiling_of_the_floor_past_the_spawn_is_rejected() {
    let (mut draft, definitions) = setup();
    bare(&mut draft);
    let mut s = size(&draft);
    s.floor_y = 300.0; // the spawn body reaches y = 404
    let error = resized(&draft, s, &definitions, None).unwrap_err();
    assert!(error.contains("spawn"), "{error}");
}

#[test]
fn shrinking_the_span_an_exit_sits_on_is_rejected() {
    let (mut draft, definitions) = setup();
    bare(&mut draft);
    // A right-edge exit at offset 364 with length 96 needs a room at least 460 tall.
    draft.layout.exits = test_rooms()[0]
        .exits
        .iter()
        .map(|exit| hollow_steps::editor::ExitPlacement {
            edge: exit.edge,
            offset: exit.offset,
            length: exit.length,
            target_room: exit.target_room,
            spawn: exit.spawn.into(),
        })
        .collect();
    assert_eq!(draft.layout.exits.len(), 1);
    let mut s = size(&draft);
    s.height = 420.0;
    s.floor_y = 420.0;
    let error = resized(&draft, s, &definitions, None).unwrap_err();
    assert!(error.contains("exit"), "{error}");
}

#[test]
fn moving_the_room_past_existing_content_is_rejected() {
    let (mut draft, definitions) = setup();
    bare(&mut draft);
    let mut s = size(&draft);
    s.x = 400.0; // the spawn is at x = 180
    assert!(resized(&draft, s, &definitions, None).is_err());
    s.x = 100.0;
    s.y = 20.0;
    let moved = resized(&draft, s, &definitions, None).unwrap();
    assert_eq!(moved.layout.bounds.0, 100.0);
    assert_eq!(moved.layout.bounds.1, 20.0);
}

#[test]
fn nonsense_sizes_are_rejected() {
    let (draft, definitions) = setup();
    for change in [
        |s: &mut RoomSize| s.width = 10.0,
        |s: &mut RoomSize| s.height = 20.0,
        |s: &mut RoomSize| s.width = f32::NAN,
        |s: &mut RoomSize| s.height = f32::INFINITY,
        |s: &mut RoomSize| s.floor_y = f32::NAN,
        |s: &mut RoomSize| s.x = f32::NEG_INFINITY,
        |s: &mut RoomSize| s.floor_y = s.y + s.height + 1.0,
    ] {
        let mut s = size(&draft);
        change(&mut s);
        assert!(resized(&draft, s, &definitions, None).is_err(), "{s:?}");
    }
}

#[test]
fn terrain_must_stay_inside_the_resized_room() {
    let (mut draft, definitions) = setup();
    bare(&mut draft);
    draft.layout.terrain = vec![(600.0, 300.0, 100.0, 20.0)];
    let mut s = size(&draft);
    s.width = 650.0;
    let error = resized(&draft, s, &definitions, None).unwrap_err();
    assert!(
        error.to_lowercase().contains("terrain") || error.contains("bounds"),
        "{error}"
    );
    s.width = 800.0;
    assert!(resized(&draft, s, &definitions, None).is_ok());
}

fn session_game() -> GameState {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

#[test]
fn resizing_a_session_marks_it_unsaved() {
    let mut game = session_game();
    let session = game.editor_mut().unwrap();
    assert!(!session.dirty);
    let mut s = RoomSize::of(&session.draft.layout);
    s.width += 64.0;
    resize_session(session, s).unwrap();
    assert!(session.dirty);
    assert_eq!(session.draft.layout.bounds.2, s.width);
}

#[test]
fn a_refused_session_resize_changes_nothing() {
    let mut game = session_game();
    let session = game.editor_mut().unwrap();
    let before: RoomLayout = session.draft.layout.clone();
    let enemies = session.draft.enemies.len();
    let mut s = RoomSize::of(&before);
    s.width = 200.0;
    assert!(resize_session(session, s).is_err());
    assert_eq!(session.draft.layout, before);
    assert_eq!(session.draft.enemies.len(), enemies);
    assert!(!session.dirty);
}

#[test]
fn applying_the_current_size_is_not_an_edit() {
    let mut game = session_game();
    let session = game.editor_mut().unwrap();
    let s = RoomSize::of(&session.draft.layout);
    resize_session(session, s).unwrap();
    assert!(!session.dirty);
}

#[test]
fn shrinking_a_room_past_its_boss_is_rejected() {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.edit_room(RoomId::Boss).unwrap();
    let session = game.editor_mut().unwrap();
    assert!(session.draft.boss.is_some());
    let boss_x = session.draft.boss.as_ref().unwrap().position.0;
    let mut s = RoomSize::of(&session.draft.layout);
    s.width = boss_x; // the boss's body would hang over the right wall
    let error = resize_session(session, s).unwrap_err();
    assert!(error.to_lowercase().contains("boss"), "{error}");
    assert!(!session.dirty);
    s.width = 1100.0;
    resize_session(session, s).unwrap();
    assert!(session.dirty);
}
