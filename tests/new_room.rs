//! A new room starts empty: it takes the room it is made from only for its size, floor, spawn,
//! background and look, never for its solids.
mod support;

use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEdge, RoomId};

fn editing() -> GameState {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

/// First's layout with two solids and a look of its own.
fn busy_layout(game: &mut GameState) -> RoomLayout {
    let layout = &mut game.editor_mut().unwrap().draft.layout;
    layout.terrain = vec![(300.0, 380.0, 120.0, 24.0), (600.0, 300.0, 80.0, 24.0)];
    layout.terrain_style.seed_offset = 7;
    layout.terrain_style.draw_boundary = false;
    layout.clone()
}

#[test]
fn a_new_room_draft_has_no_solids_items_exits_enemies_boss_or_name() {
    let mut game = editing();
    let mut template = busy_layout(&mut game);
    template.name = Some("Hall".into());
    assert!(!template.terrain.is_empty());
    let draft = RoomDraft::new_room(&template);
    assert!(draft.layout.terrain.is_empty());
    assert!(draft.layout.items.is_empty() && draft.layout.exits.is_empty());
    assert!(draft.enemies.is_empty() && draft.boss.is_none());
    assert_eq!(draft.layout.name, None);
}

#[test]
fn a_new_room_keeps_the_size_floor_spawn_background_and_look() {
    let mut game = editing();
    let template = busy_layout(&mut game);
    let draft = RoomDraft::new_room(&template);
    assert_eq!(draft.layout.bounds, template.bounds);
    assert_eq!(draft.layout.floor_y, template.floor_y);
    assert_eq!(draft.layout.spawn, template.spawn);
    assert_eq!(draft.layout.background, template.background);
    assert_eq!(draft.layout.terrain_style, template.terrain_style);
    draft
        .validate_local(&game.editor().unwrap().definitions)
        .expect("an empty room with the template's spawn is valid");
}

#[test]
fn the_new_room_command_does_not_copy_the_current_rooms_solids() {
    let mut game = editing();
    busy_layout(&mut game);
    let editor = game.editor_mut().unwrap();
    editor.new_room();
    assert!(editor.draft.layout.terrain.is_empty());
    assert!(editor.new && editor.dirty);
    // The room it was made from is untouched on disk and in the catalog.
    let first = editor
        .catalog()
        .into_iter()
        .find(|(id, _)| *id == RoomId::First)
        .unwrap()
        .1;
    assert!(
        first.terrain.is_empty(),
        "the unsaved solids were only in the draft"
    );
}

#[test]
fn a_named_new_room_has_no_solids_either() {
    let mut game = editing();
    busy_layout(&mut game);
    let editor = game.editor_mut().unwrap();
    editor.new_room_named("Cellar").unwrap();
    assert!(editor.draft.layout.terrain.is_empty());
    assert_eq!(editor.draft.layout.name.as_deref(), Some("Cellar"));
}

#[test]
fn a_new_room_made_by_an_exit_link_has_no_solids() {
    let mut game = editing();
    busy_layout(&mut game);
    let editor = game.editor_mut().unwrap();
    editor.begin_link_new_exit(RoomEdge::Right).unwrap();
    editor.link_new_room().unwrap();
    editor.link_name_room("Cellar").unwrap();
    assert_eq!(editor.id, RoomId::Custom(0));
    assert!(editor.draft.layout.terrain.is_empty());
    // A, parked, still has its own solids.
    let parked = editor
        .parked
        .iter()
        .find(|w| w.id == RoomId::First)
        .unwrap();
    assert_eq!(parked.draft.layout.terrain.len(), 2);
}

/// A copy of the fixture assets that is removed again when it goes out of scope.
struct TempAssets(std::path::PathBuf);
impl TempAssets {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("hollow-newroom-{}", std::process::id()));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
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
fn a_saved_new_room_loads_without_solids_and_the_first_room_keeps_its_own() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    busy_layout(&mut game);
    game.save_editor().unwrap();
    game.editor_mut().unwrap().new_room();
    game.save_editor().unwrap();

    let mut restarted = GameState::try_new(&temp.0).unwrap();
    restarted.edit_room(RoomId::First).unwrap();
    assert_eq!(
        restarted.editor().unwrap().draft.layout.terrain.len(),
        2,
        "the first room keeps the solids it was given"
    );
    restarted.edit_room(RoomId::Custom(0)).unwrap();
    assert!(
        restarted.editor().unwrap().draft.layout.terrain.is_empty(),
        "the new room starts without solids"
    );
}
