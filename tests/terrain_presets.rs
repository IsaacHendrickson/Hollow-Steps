//! The terrain look presets in a running game: a room naming a preset the game does not know
//! stops the game from loading, a reload swaps the look without touching anything else, and a
//! `terrain_styles.ron` in the assets folder replaces the built-in presets.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::terrain_style::Look;
use hollow_steps::terrain_styles::RoomStyle;

struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-presets-{}-{}",
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

    fn write(&self, file: &str, source: &str) {
        std::fs::write(self.0.join(file), source).unwrap();
    }

    /// The first room with a full layout and the given `terrain_style` source.
    fn first_room_styled(&self, style: &str) {
        self.write(
            "rooms/first.ron",
            &format!(
                "(enemies: [], layout: Some((\
                    bounds: (0.0, 0.0, 960.0, 460.0), floor_y: 460.0, spawn: (180.0, 436.0), \
                    items: [], \
                    exits: [(edge: Right, offset: 364.0, length: 96.0, target_room: Second, \
                        spawn: (160.0, 376.0))], \
                    terrain: [(300.0, 380.0, 120.0, 20.0)], \
                    terrain_style: {style}, \
                )))"
            ),
        );
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn first_look(game: &GameState) -> Look {
    game.terrain_styles()
        .resolve(&game.room().terrain_style)
        .expect("the current room resolves")
}

#[test]
fn a_room_naming_a_preset_loads_and_resolves_to_it() {
    let assets = Assets::new();
    assets.first_room_styled(r#"(preset: "cave", seed_offset: 3)"#);

    let game = GameState::try_new(&assets.0).unwrap();

    assert_eq!(game.room().terrain_style.preset, "cave");
    let look = first_look(&game);
    let cave = game.terrain_styles().preset("cave").unwrap();
    assert_eq!(look.palette.fill, cave.fill);
    assert_eq!(look.boundary_thickness, cave.boundary_thickness);
    assert_eq!(look.palette.seed_offset, 3);
}

#[test]
fn rooms_without_a_style_get_the_default_look_with_the_boundary() {
    // The fixture rooms were written before terrain styles existed.
    let assets = Assets::new();
    let game = GameState::try_new(&assets.0).unwrap();
    assert_eq!(game.room().terrain_style, RoomStyle::default());
    assert_eq!(first_look(&game), Look::default());
    assert!(first_look(&game).draw_boundary);
}

#[test]
fn an_unknown_preset_stops_the_game_from_loading_and_names_the_room_and_the_known_ones() {
    let assets = Assets::new();
    assets.first_room_styled(r#"(preset: "granite")"#);

    let error = GameState::try_new(&assets.0).unwrap_err();

    assert!(error.contains("first.ron"), "{error}");
    assert!(error.contains("unknown preset"), "{error}");
    assert!(error.contains("granite"), "{error}");
    assert!(
        error.contains("known presets: stone, mossy, cave, ruins"),
        "{error}"
    );
}

#[test]
fn a_bad_override_color_stops_the_game_from_loading() {
    let assets = Assets::new();
    assets.first_room_styled(r#"(fill: (1.5, 0.0, 0.0))"#);

    let error = GameState::try_new(&assets.0).unwrap_err();

    assert!(error.contains("first.ron"), "{error}");
    assert!(error.contains("fill"), "{error}");
}

#[test]
fn reload_swaps_the_look_and_nothing_else() {
    let assets = Assets::new();
    assets.first_room_styled(r#"(preset: "stone")"#);
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.0);
    let before_player = format!("{:?}", game.player());
    let before_bounds = game.room().bounds;
    let before_solids = game.room().terrain.solids().to_vec();
    assert_eq!(game.room().terrain_style.preset, "stone");

    assets.first_room_styled(r#"(preset: "ruins", fill: (0.5, 0.5, 0.5), draw_boundary: false)"#);
    game.reload_rooms().unwrap();

    let style = &game.room().terrain_style;
    assert_eq!(style.preset, "ruins");
    assert!(!first_look(&game).draw_boundary);
    assert_eq!(first_look(&game).palette.fill, (0.5, 0.5, 0.5));
    assert_eq!(format!("{:?}", game.player()), before_player);
    assert_eq!(game.room().bounds, before_bounds);
    assert_eq!(game.room().terrain.solids(), &before_solids[..]);
}

#[test]
fn an_unknown_preset_on_reload_is_an_error_and_changes_nothing() {
    let assets = Assets::new();
    assets.first_room_styled(r#"(preset: "cave")"#);
    let mut game = GameState::try_new(&assets.0).unwrap();
    let original = format!("{game:?}");

    assets.first_room_styled(r#"(preset: "granite")"#);
    let error = game.reload_rooms().unwrap_err();

    assert!(error.contains("first.ron"), "{error}");
    assert!(error.contains("known presets"), "{error}");
    assert_eq!(format!("{game:?}"), original);
    assert_eq!(game.room().terrain_style.preset, "cave");

    // Fixing the file lets the same game reload.
    assets.first_room_styled(r#"(preset: "mossy")"#);
    game.reload_rooms().unwrap();
    assert_eq!(game.room().terrain_style.preset, "mossy");
}

/// A catalog with one preset of its own and one called "stone", which rooms without a style ask for.
const OWN_PRESETS: &str = r#"(
    presets: [
        (
            name: "stone",
            fill: (0.2, 0.2, 0.2), edge: (0.0, 0.0, 0.0),
            highlight: (0.8, 0.8, 0.8), accent: (0.3, 0.6, 0.3),
            moss: true, speckle_density: 1.0, crack_density: 1.0, boundary_thickness: 48.0,
        ),
        (
            name: "granite",
            fill: (0.5, 0.5, 0.5), edge: (0.1, 0.1, 0.1),
            highlight: (0.9, 0.9, 0.9), accent: (0.3, 0.6, 0.3),
            moss: false, speckle_density: 1.0, crack_density: 1.0, boundary_thickness: 40.0,
        ),
    ],
)"#;

#[test]
fn a_terrain_styles_file_in_the_assets_folder_replaces_the_built_in_presets() {
    let assets = Assets::new();
    assets.write("terrain_styles.ron", OWN_PRESETS);
    assets.first_room_styled(r#"(preset: "granite")"#);

    let game = GameState::try_new(&assets.0).unwrap();

    assert_eq!(game.terrain_styles().names(), vec!["stone", "granite"]);
    assert!(game.terrain_styles().preset("cave").is_none());
    assert_eq!(first_look(&game).palette.fill, (0.5, 0.5, 0.5));
    assert_eq!(first_look(&game).boundary_thickness, 40.0);
}

#[test]
fn rooms_must_find_their_preset_in_the_replaced_file() {
    let assets = Assets::new();
    assets.write("terrain_styles.ron", OWN_PRESETS);
    assets.first_room_styled(r#"(preset: "cave")"#);

    let error = GameState::try_new(&assets.0).unwrap_err();

    assert!(error.contains("first.ron"), "{error}");
    assert!(error.contains("unknown preset"), "{error}");
    assert!(error.contains("cave"), "{error}");
    assert!(error.contains("known presets: stone, granite"), "{error}");
}

#[test]
fn reload_picks_up_a_changed_terrain_styles_file_with_the_rooms() {
    let assets = Assets::new();
    assets.first_room_styled(r#"(preset: "stone")"#);
    let mut game = GameState::try_new(&assets.0).unwrap();
    assert!(game.terrain_styles().preset("granite").is_none());

    assets.write("terrain_styles.ron", OWN_PRESETS);
    assets.first_room_styled(r#"(preset: "granite")"#);
    game.reload_rooms().unwrap();

    assert_eq!(game.terrain_styles().names(), vec!["stone", "granite"]);
    assert_eq!(first_look(&game).palette.fill, (0.5, 0.5, 0.5));

    // A room naming a preset the new file lacks is refused, and the game keeps the old ones.
    assets.first_room_styled(r#"(preset: "ruins")"#);
    assert!(game.reload_rooms().is_err());
    assert_eq!(game.terrain_styles().names(), vec!["stone", "granite"]);
    assert_eq!(game.room().terrain_style.preset, "granite");
}

#[test]
fn an_invalid_terrain_styles_file_stops_the_game_and_reload_keeps_the_old_presets() {
    let assets = Assets::new();
    let mut game = GameState::try_new(&assets.0).unwrap();
    let original = format!("{game:?}");
    let known = game
        .terrain_styles()
        .names()
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    for broken in ["not RON", "(presets: [])"] {
        assets.write("terrain_styles.ron", broken);
        let error = game.reload_rooms().unwrap_err();
        assert!(error.contains("terrain_styles.ron"), "{error}");
        assert_eq!(format!("{game:?}"), original);
        assert_eq!(game.terrain_styles().names(), known);
        assert!(GameState::try_new(&assets.0).is_err());
    }
}
