//! The only test file that reads the real `assets/` folder. It loads and validates what ships,
//! one test per area, and asserts nothing about its content: no numbers, names, layouts or
//! coordinates, because the owner changes those in the editor. Anything that checks what an
//! enemy, room or body does lives on fixtures (`tests/fixtures/`, `tests/support/`).
use std::path::{Path, PathBuf};

use hollow_steps::game::GameState;

fn assets() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets")
}

#[test]
fn shipped_rooms_load_and_validate() {
    // Startup validates room geometry, entities, behaviors, charms, and exit destinations,
    // including saved custom rooms.
    GameState::try_new(&assets()).expect("shipped rooms must load and validate");
}

#[test]
fn shipped_behavior_presets_load_and_validate() {
    hollow_steps::preset::load_presets(&assets().join("presets"))
        .expect("shipped behavior presets must load and validate");
}

#[test]
fn shipped_player_body_loads_and_validates() {
    // The wanderer template, its four poses and the look must load and fit together.
    hollow_steps::player_body::PlayerBody::load(&assets())
        .expect("shipped player body must load and validate")
        .expect("the shipped assets include a player body");
}

#[test]
fn shipped_boss_body_loads_and_validates() {
    // Every boss definition, with its body template, hurt pose and colors, must load and fit
    // together.
    hollow_steps::boss_definition::BossDefinitions::load(&assets())
        .expect("shipped bosses must load and validate");
}

#[test]
fn shipped_body_templates_and_poses_load_and_fit_the_enemies() {
    let assets = assets();
    // Loading validates every template and every pose; loading the enemies checks each one's
    // body, flap poses and behavior against them.
    hollow_steps::body::load_body_templates(&assets.join("bodies"))
        .expect("shipped body templates must load and validate");
    hollow_steps::pose::load_poses(&assets.join("poses"))
        .expect("shipped poses must load and validate");
    hollow_steps::enemy::EnemyDefinitions::load(&assets)
        .expect("shipped enemies must load and validate");
}

#[test]
fn shipped_charms_load_and_validate() {
    let assets = assets();
    // The cost of every charm has to fit the shipped max mana.
    let vitals = hollow_steps::vitals::VitalsConfig::load(&assets)
        .expect("shipped vitals must load and validate");
    hollow_steps::charms::CharmCatalog::load(&assets, vitals.max_mana)
        .expect("shipped charms must load and validate");
}

#[test]
fn shipped_upgrades_load_and_validate() {
    // Load only: which upgrades there are and what they cost is the owner's to change.
    hollow_steps::upgrades::UpgradeCatalog::load(&assets())
        .expect("shipped upgrades.ron must load and validate");
}

#[test]
fn shipped_terrain_presets_and_room_palettes_load_and_validate() {
    let assets = assets();
    hollow_steps::terrain_styles::TerrainStyles::load(&assets)
        .expect("shipped terrain presets must load and validate");
    // Every room has to name a preset the game knows (and may override colors) for the game to
    // start.
    let game = GameState::try_new(&assets).expect("shipped rooms must load and validate");
    game.room().terrain_style.validate().unwrap();
    let rooms = hollow_steps::room::load_rooms(&assets).expect("shipped rooms load");
    for room in &rooms {
        game.terrain_styles()
            .resolve(&room.terrain_style)
            .unwrap_or_else(|error| panic!("{:?}: {error}", room.id));
    }
}

#[test]
fn shipped_run_rules_and_hub_room_load_and_validate() {
    let assets = assets();
    let config = hollow_steps::run::RunConfig::load(&assets).expect("shipped run.ron must load");
    // Startup loads every room and checks the run rules against them: the hub, the start room and
    // the final boss room must all exist.
    let game = GameState::try_new(&assets).expect("shipped rooms and run rules must fit together");
    if let Some(hub) = config.hub_room {
        assert_eq!(game.room().id, hub, "the game starts in the hub");
    }
}

#[test]
fn shipped_difficulty_loads_and_validates() {
    let assets = assets();
    assert!(
        assets.join(hollow_steps::difficulty::FILE_NAME).is_file(),
        "assets/difficulty.ron ships with the game"
    );
    hollow_steps::difficulty::Difficulty::load(&assets).expect("shipped difficulty.ron must load");
}

#[test]
fn shipped_variety_loads_and_validates() {
    // Only that the file ships and passes the same checks as at start-up; it asserts no numbers.
    let assets = assets();
    assert!(
        assets
            .join(hollow_steps::variety::flip::FILE_NAME)
            .is_file(),
        "assets/variety.ron ships with the game"
    );
    hollow_steps::variety::VarietyRules::load(&assets).expect("shipped variety.ron must load");
}

#[test]
fn shipped_dungeon_has_no_problems() {
    // Opens the editor on the shipped rooms (custom rooms included) and runs Check dungeon, the
    // same check as its button, under the shipped run rules. It asserts nothing about content.
    let mut game = GameState::try_new(&assets()).expect("shipped rooms must load");
    game.toggle_editor().expect("the editor must open");
    let problems = game.editor().expect("editor is open").check_dungeon();
    let text: Vec<_> = problems.iter().map(|problem| problem.text()).collect();
    assert!(
        problems.is_empty(),
        "Check dungeon found:\n{}",
        text.join("\n")
    );
}
