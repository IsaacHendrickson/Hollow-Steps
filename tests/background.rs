//! Background layers through the public room API: loading, validation, the editor round
//! trip and the game's room loader.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::background::{BackgroundLayer, Shape};
use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEnemies, RoomId};

fn layout(name: &str) -> RoomLayout {
    let path = support::fixture_assets().join(name);
    ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn background_layout() -> RoomLayout {
    layout("background/background_layout.ron")
}

fn draft(layout: RoomLayout) -> RoomDraft {
    RoomDraft {
        enemies: Vec::new(),
        boss: None,
        layout,
    }
}

fn definitions() -> EnemyDefinitions {
    EnemyDefinitions::load(&support::fixture_assets()).expect("fixtures load")
}

/// A copy of the behavioral fixtures that a test may rewrite.
struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-background-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let fixtures = support::fixture_assets();
        for directory in ["behaviors", "entities", "rooms", "bosses"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(fixtures.join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn write_first_room(&self, layout: &RoomLayout) {
        let document = RoomEnemies {
            enemies: Vec::new(),
            boss: None,
            layout: Some(layout.clone()),
        };
        std::fs::write(
            self.0.join("rooms").join("first.ron"),
            ron::to_string(&document).unwrap(),
        )
        .unwrap();
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_fixture_has_three_layers_back_to_front_and_validates() {
    let layout = background_layout();
    layout.validate_geometry().unwrap();
    let depths: Vec<f32> = layout.background.iter().map(|layer| layer.depth).collect();
    assert_eq!(depths, [0.0, 0.3, 0.8]);
    assert!(matches!(
        layout.background[0].shapes[0],
        Shape::Circle { .. }
    ));
    assert!(matches!(
        layout.background[1].shapes[0],
        Shape::Triangle { .. }
    ));
    assert!(matches!(layout.background[2].shapes[0], Shape::Rect { .. }));
    assert_eq!(layout.background[2].color, (36, 40, 72, 200));
}

#[test]
fn old_room_files_load_without_a_background_and_saving_adds_none() {
    let legacy = layout("terrain/legacy_layout.ron");
    assert!(legacy.background.is_empty());
    legacy.validate_geometry().unwrap();
    assert!(!ron::to_string(&legacy).unwrap().contains("background"));
}

#[test]
fn building_a_room_carries_the_layers_and_the_editor_save_keeps_them() {
    let definitions = definitions();
    let layout = background_layout();
    let room = draft(layout.clone())
        .build(RoomId::Custom(0), &definitions)
        .unwrap();
    assert_eq!(room.background, layout.background);

    // What the editor reads back from a live room still has the layers...
    assert_eq!(RoomLayout::from_room(&room).background, layout.background);

    // ...and so does the file it writes.
    let text = draft(layout.clone()).serialize(&definitions, &[]).unwrap();
    let saved: RoomEnemies = ron::from_str(&text).unwrap();
    assert_eq!(saved.layout.unwrap().background, layout.background);
}

#[test]
fn an_invalid_layer_is_a_validation_error_naming_the_layer_and_blocks_the_build() {
    let mut layout = background_layout();
    layout.background[1].depth = 1.5;
    let error = layout.validate_geometry().unwrap_err();
    assert!(error.contains("background layer 1"), "{error}");
    assert!(error.contains("depth"), "{error}");
    assert!(draft(layout)
        .build(RoomId::Custom(0), &definitions())
        .is_err());

    let mut layout = background_layout();
    layout.background[0].shapes.clear();
    assert!(layout
        .validate_geometry()
        .unwrap_err()
        .contains("background layer 0"));
}

#[test]
fn a_bad_layer_in_a_room_file_is_a_load_error_naming_the_file() {
    let assets = Assets::new();
    let mut layout = background_layout();
    layout.background.push(BackgroundLayer {
        depth: -1.0,
        color: (0, 0, 0, 255),
        shapes: vec![Shape::Circle {
            x: 0.0,
            y: 0.0,
            radius: 5.0,
        }],
    });
    assets.write_first_room(&layout);
    let Err(error) = GameState::try_new(&assets.0) else {
        panic!("a bad layer should not load");
    };
    assert!(error.contains("first.ron"), "{error}");
    assert!(error.contains("background layer 3"), "{error}");
}

#[test]
fn the_game_loads_a_rooms_layers_and_a_reload_picks_up_a_change() {
    let assets = Assets::new();
    let layout = background_layout();
    assets.write_first_room(&layout);
    let mut game = GameState::try_new(&assets.0).expect("fixtures load");
    assert_eq!(game.room().background, layout.background);

    // A reload re-reads the layers of a room that is already loaded.
    let mut changed = layout.clone();
    changed.background.truncate(1);
    assets.write_first_room(&changed);
    game.reload_rooms().unwrap();
    assert_eq!(game.room().background.len(), 1);
}
