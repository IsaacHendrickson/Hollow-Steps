mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{save_room, Placement, RoomDraft, RoomLayout, Selection};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::item::Item;
use hollow_steps::player::PlayerInput;
use hollow_steps::room::{test_rooms, RoomEnemies, RoomId};
use macroquad::math::Vec2;

fn assets() -> PathBuf {
    support::fixture_assets()
}
fn setup() -> (RoomDraft, EnemyDefinitions, Vec<(RoomId, RoomLayout)>) {
    let rooms = test_rooms();
    let catalog = rooms
        .iter()
        .map(|room| (room.id, RoomLayout::from_room(room)))
        .collect();
    let document =
        ron::from_str(&std::fs::read_to_string(assets().join("rooms/first.ron")).unwrap()).unwrap();
    (
        RoomDraft::from_document(document, &rooms[0]),
        EnemyDefinitions::load(&assets()).unwrap(),
        catalog,
    )
}

#[test]
fn place_move_delete_are_pure_and_preserve_enemy_patrol_width() {
    let (mut draft, definitions, _) = setup();
    let original = draft.enemies.len();
    let enemy = draft
        .place(
            Placement::Enemy("patrol".into()),
            Vec2::new(200.0, 50.0),
            &definitions,
        )
        .unwrap();
    assert_eq!(enemy, Selection::Enemy(original));
    draft.enemies[original].patrol_right = 280.0;
    draft
        .move_to(enemy, Vec2::new(300.0, 100.0), &definitions)
        .unwrap();
    assert_eq!(draft.enemies[original].patrol_left, 300.0);
    assert_eq!(draft.enemies[original].patrol_right, 380.0);
    let item = draft
        .place(
            Placement::Item(Item::Dash),
            Vec2::new(400.0, 200.0),
            &definitions,
        )
        .unwrap();
    draft
        .move_to(item, Vec2::new(450.0, 180.0), &definitions)
        .unwrap();
    assert_eq!(draft.layout.items.last().unwrap().position, (450.0, 180.0));
    draft.delete(item).unwrap();
    draft.delete(enemy).unwrap();
    assert_eq!(draft.enemies.len(), original);
    assert_eq!(draft.layout.items.len(), 2);
}

#[test]
fn invalid_operations_are_atomic_and_body_boundaries_are_inclusive() {
    let (mut draft, definitions, _) = setup();
    let item = draft
        .place(
            Placement::Item(Item::Dash),
            Vec2::new(8.0, 450.0),
            &definitions,
        )
        .unwrap();
    let before = format!("{draft:?}");
    assert!(draft
        .move_to(item, Vec2::new(7.9, 450.0), &definitions)
        .is_err());
    assert!(draft
        .place(
            Placement::Enemy("missing".into()),
            Vec2::new(100.0, 100.0),
            &definitions
        )
        .is_err());
    assert!(draft
        .place(
            Placement::Item(Item::Dash),
            Vec2::splat(f32::NAN),
            &definitions
        )
        .is_err());
    assert!(draft
        .move_to(Selection::Enemy(999), Vec2::ZERO, &definitions)
        .is_err());
    assert!(draft.delete(Selection::Spawn).is_err());
    assert_eq!(format!("{draft:?}"), before);
}

#[test]
fn move_spawn_exit_and_destination_and_validate_graph() {
    let (mut draft, definitions, mut catalog) = setup();
    draft
        .move_to(Selection::Spawn, Vec2::new(250.0, 436.0), &definitions)
        .unwrap();
    draft
        .move_to(Selection::Exit(0), Vec2::new(944.0, 350.0), &definitions)
        .unwrap();
    assert_eq!(draft.layout.exits[0].offset, 302.0);
    draft
        .move_to(
            Selection::Destination(0),
            Vec2::new(200.0, 376.0),
            &definitions,
        )
        .unwrap();
    catalog[0].1 = draft.layout.clone();
    draft.validate(&definitions, &catalog).unwrap();
    draft.layout.exits[0].target_room = RoomId::Custom(100);
    assert!(draft
        .serialize(&definitions, &catalog)
        .unwrap_err()
        .contains("missing room"));
    draft.layout.exits[0].target_room = RoomId::Second;
    draft.layout.exits[0].spawn = (1000.0, 0.0);
    assert!(draft.validate(&definitions, &catalog).is_err());
    draft.layout.exits[0].spawn = (90.0, 376.0);
    assert!(draft.validate(&definitions, &catalog).is_err());
}

#[test]
fn serialize_round_trips_all_authored_data_and_rejects_nonfinite_geometry() {
    let (draft, definitions, catalog) = setup();
    let source = draft.serialize(&definitions, &catalog).unwrap();
    let document: RoomEnemies = ron::from_str(&source).unwrap();
    let roundtrip = RoomDraft::from_document(document, &test_rooms()[0]);
    assert_eq!(roundtrip.serialize(&definitions, &catalog).unwrap(), source);
    let built = roundtrip.build(RoomId::First, &definitions).unwrap();
    assert_eq!(built.enemies.len(), 1);
    assert_eq!(built.exits[0].target_room, RoomId::Second);
    assert_eq!(built.spawn, Vec2::new(180.0, 436.0));
    let mut bad = draft;
    bad.layout.bounds.2 = f32::INFINITY;
    assert!(bad.serialize(&definitions, &catalog).is_err());
}

struct TempAssets(PathBuf);
impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-editor-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors"] {
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
fn failed_save_preserves_file_and_new_room_never_overwrites_existing_file() {
    let temp = TempAssets::new();
    let path = temp.0.join("rooms/first.ron");
    let before = std::fs::read(&path).unwrap();
    let (mut draft, definitions, catalog) = setup();
    draft.enemies[0].definition = "missing".into();
    assert!(save_room(&path, &draft, &definitions, &catalog, false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(!path.with_extension("ron.tmp").exists());
    draft.enemies[0].definition = "chase".into();
    assert!(save_room(&path, &draft, &definitions, &catalog, true).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    save_room(&path, &draft, &definitions, &catalog, false).unwrap();
    let game = GameState::try_new(&temp.0).unwrap();
    assert_eq!(game.room().enemies.len(), draft.enemies.len());
    assert_eq!(game.items().len(), draft.layout.items.len());
}

#[test]
fn editor_pauses_simulation_and_new_rooms_survive_restart_and_allow_sparse_ids() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    let player = format!("{:?}", game.player());
    game.toggle_editor().unwrap();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(format!("{:?}", game.player()), player);
    let editor = game.editor_mut().unwrap();
    editor.new_room();
    assert!(editor.draft.enemies.is_empty());
    assert!(editor.draft.layout.items.is_empty());
    assert!(editor.draft.layout.exits.is_empty());
    game.save_editor().unwrap();
    assert_eq!(game.room().id, RoomId::Custom(0));
    game.toggle_editor().unwrap();
    let mut restarted = GameState::try_new(&temp.0).unwrap();
    restarted.edit_room(RoomId::Custom(0)).unwrap();
    assert_eq!(restarted.room().id, RoomId::Custom(0));
    // Room IDs are file IDs, not offsets into an array.
    std::fs::rename(
        temp.0.join("rooms/room_0.ron"),
        temp.0.join("rooms/room_7.ron"),
    )
    .unwrap();
    let mut sparse = GameState::try_new(&temp.0).unwrap();
    sparse.edit_room(RoomId::Custom(7)).unwrap();
    sparse.toggle_editor().unwrap();
    sparse.update(PlayerInput::default(), 0.1);
    assert_eq!(sparse.room().id, RoomId::Custom(7));
}

#[test]
fn entering_editor_uses_authored_items_after_session_pickup() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        120.0 / 220.0,
    );
    game.update(
        PlayerInput {
            pickup_pressed: true,
            ..PlayerInput::default()
        },
        0.0,
    );
    assert_eq!(game.items().len(), 1);
    game.toggle_editor().unwrap();
    assert_eq!(game.editor().unwrap().draft.layout.items.len(), 2);
    game.toggle_editor().unwrap();
    assert_eq!(game.items().len(), 1);
}

#[test]
fn edited_room_layout_is_loaded_by_the_game_and_save_checks_inbound_exits() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    editor.draft.layout.spawn = (240.0, 436.0);
    editor.draft.layout.items.clear();
    game.save_editor().unwrap();
    let restarted = GameState::try_new(&temp.0).unwrap();
    assert_eq!(restarted.player().position(), Vec2::new(240.0, 436.0));
    assert!(restarted.items().is_empty());
    let before = std::fs::read(temp.0.join("rooms/first.ron")).unwrap();
    game.editor_mut().unwrap().draft.layout.bounds.2 = 700.0;
    assert!(game.save_editor().is_err());
    assert_eq!(
        std::fs::read(temp.0.join("rooms/first.ron")).unwrap(),
        before
    );
}

#[test]
fn new_room_can_be_connected_and_entered_through_saved_exit() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    game.editor_mut().unwrap().new_room();
    game.save_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let selected = editor.draft.add_exit(
        hollow_steps::room::RoomEdge::Left,
        RoomId::First,
        Vec2::new(180.0, 436.0),
    );
    assert_eq!(selected, Selection::Exit(0));
    editor
        .draft
        .move_to(selected, Vec2::new(16.0, 412.0), &editor.definitions)
        .unwrap();
    game.save_editor().unwrap();
    game.edit_room(RoomId::First).unwrap();
    game.editor_mut()
        .unwrap()
        .draft
        .set_exit_target(0, RoomId::Custom(0), Vec2::new(180.0, 436.0))
        .unwrap();
    game.save_editor().unwrap();
    let mut restarted = GameState::try_new(&temp.0).unwrap();
    restarted.update(
        PlayerInput {
            move_axis: 1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(restarted.room().id, RoomId::Custom(0));
    restarted.update(
        PlayerInput {
            move_axis: -1.0,
            ..PlayerInput::default()
        },
        10.0,
    );
    assert_eq!(restarted.room().id, RoomId::First);
}

#[test]
fn filesystem_save_failure_keeps_previous_file_and_does_not_remove_existing_temp() {
    let temp = TempAssets::new();
    let path = temp.0.join("rooms/first.ron");
    let temporary = path.with_extension("ron.tmp");
    std::fs::write(&temporary, "another writer").unwrap();
    let before = std::fs::read(&path).unwrap();
    let (draft, definitions, catalog) = setup();
    assert!(save_room(&path, &draft, &definitions, &catalog, false).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(
        std::fs::read_to_string(&temporary).unwrap(),
        "another writer"
    );
}

#[test]
fn patrol_handles_validate_walls_size_order_and_finite_positions_without_mutating_on_failure() {
    let (mut draft, definitions, _) = setup();
    let size = definitions.size(&draft.enemies[0].definition).unwrap();
    let left = draft.layout.bounds.0 + size.x / 2.0;
    let right = draft.layout.rect().right() - size.x / 2.0;
    draft
        .move_to(Selection::PatrolLeft(0), Vec2::new(left, 0.0), &definitions)
        .unwrap();
    draft
        .move_to(
            Selection::PatrolRight(0),
            Vec2::new(right, 0.0),
            &definitions,
        )
        .unwrap();
    assert_eq!(draft.enemies[0].patrol_left, left);
    assert_eq!(draft.enemies[0].patrol_right, right);
    let before = draft.enemies.clone();
    for (selection, x) in [
        (Selection::PatrolLeft(0), left - 0.1),
        (Selection::PatrolRight(0), right + 0.1),
        (Selection::PatrolLeft(0), right + 1.0),
        (Selection::PatrolRight(0), left - 1.0),
        (Selection::PatrolRight(0), f32::NAN),
    ] {
        assert!(draft
            .move_to(selection, Vec2::new(x, 0.0), &definitions)
            .is_err());
        assert_eq!(draft.enemies, before);
    }
    draft
        .move_to(
            Selection::PatrolLeft(0),
            Vec2::new(right, 0.0),
            &definitions,
        )
        .unwrap();
    assert_eq!(draft.enemies[0].patrol_left, draft.enemies[0].patrol_right);
}

#[test]
fn deleting_exit_or_arrival_removes_both_and_no_exits_only_warns() {
    let (mut draft, definitions, _) = setup();
    let count = draft.layout.exits.len();
    assert!(draft.warnings().is_empty());
    draft.delete(Selection::Destination(0)).unwrap();
    assert_eq!(draft.layout.exits.len(), count - 1);
    while !draft.layout.exits.is_empty() {
        draft.delete(Selection::Exit(0)).unwrap();
    }
    assert!(draft.delete(Selection::Exit(0)).is_err());
    assert_eq!(draft.warnings(), ["Warning: room has no exits"]);
    draft.validate_local(&definitions).unwrap();
}

#[test]
fn failed_editor_write_preserves_session_and_can_retry() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let path = editor.content.entity_path("patrol").unwrap();
    editor.content.entities.get_mut(&path).unwrap().name = "renamed".into();
    editor.dirty = true;
    let original = std::fs::read(&path).unwrap();
    let room_before = game.room().enemies.len();
    let staging = path.with_extension("ron.tmp");
    std::fs::create_dir(&staging).unwrap();
    assert!(game.save_editor().is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(game.room().enemies.len(), room_before);
    let editor = game.editor().unwrap();
    assert!(editor.dirty);
    assert_eq!(editor.content.entities[&path].name, "renamed");
    assert_eq!(editor.content.entity_path("patrol"), Some(path.clone()));
    std::fs::remove_dir(staging).unwrap();
    game.save_editor().unwrap();
    assert!(!game.editor().unwrap().dirty);
    assert_eq!(
        game.editor().unwrap().content.entity_path("renamed"),
        Some(path)
    );
}

#[test]
fn item_catalog_contains_each_serializable_item_once() {
    for item in Item::ALL {
        let source = ron::to_string(item).unwrap();
        assert_eq!(ron::from_str::<Item>(&source).unwrap(), *item);
        assert_eq!(Item::ALL.iter().filter(|i| *i == item).count(), 1);
        assert!(!item.name().is_empty());
    }
}

#[test]
fn patrol_handles_cannot_cross_inside_the_room() {
    let (mut draft, definitions, _) = setup();
    draft.enemies[0].patrol_left = 200.0;
    draft.enemies[0].patrol_right = 300.0;
    assert!(draft
        .move_to(
            Selection::PatrolLeft(0),
            Vec2::new(301.0, 0.0),
            &definitions
        )
        .is_err());
    assert!(draft
        .move_to(
            Selection::PatrolRight(0),
            Vec2::new(199.0, 0.0),
            &definitions
        )
        .is_err());
    assert!(draft
        .move_to(
            Selection::PatrolLeft(usize::MAX),
            Vec2::new(250.0, 0.0),
            &definitions
        )
        .is_err());
    assert_eq!(draft.enemies[0].patrol_left, 200.0);
    assert_eq!(draft.enemies[0].patrol_right, 300.0);
}
