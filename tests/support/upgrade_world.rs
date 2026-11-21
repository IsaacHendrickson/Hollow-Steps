//! A private world with a hub and a known upgrade list, for tests of the Upgrades panel's draft
//! in a game. Permanent upgrades only count in a game with a hub, so `run.ron` and a hub room are
//! added to the private copy, and `upgrades.ron` is written from `SAMPLE` so the shipped numbers
//! (which the owner tunes) never decide a result.
use std::collections::BTreeMap;

use hollow_steps::game::GameState;
use hollow_steps::profile::Profile;

use super::player_assets::Assets;

/// Three upgrades written for the tests.
pub const SAMPLE: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "More health.",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "Faster.",
     effect: MoveSpeedPercent(5.0), costs: [40, 80, 140]),
    (id: "keen_edge", name: "Keen Edge", description: "Sharper.",
     effect: SwordDamage(3), costs: [50, 100, 180]),
])"#;

const HUB_FILE: &str = r#"(enemies: [], layout: Some((
    bounds: (0.0, 0.0, 960.0, 460.0),
    floor_y: 460.0,
    spawn: (180.0, 436.0),
    items: [],
    exits: [(edge: Right, offset: 364.0, length: 96.0, target_room: First, spawn: (180.0, 436.0))],
)))"#;

const RUN_FILE: &str = "(hub_room: Custom(0), start_room: First, final_boss_room: Boss, \
                        coin_keep_percent: 50.0)";

/// A private copy of the fixtures with a hub, `run.ron` and `upgrades.ron` (`SAMPLE`).
pub fn hub_assets() -> Assets {
    let assets = Assets::new(false);
    std::fs::write(assets.file("rooms").join("room_0.ron"), HUB_FILE).unwrap();
    std::fs::write(assets.file("run.ron"), RUN_FILE).unwrap();
    std::fs::write(assets.file("upgrades.ron"), SAMPLE).unwrap();
    assets
}

pub fn upgrades_text(assets: &Assets) -> String {
    std::fs::read_to_string(assets.file("upgrades.ron")).unwrap()
}

/// The game of `assets` in the hub, with the profile holding `levels` (upgrade id, level), saved
/// in the private folder, and its editor open.
pub fn editing_hub(assets: &Assets, levels: &[(&str, u32)]) -> GameState {
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.use_profile_file(assets.file("profile.ron")).unwrap();
    let upgrades: BTreeMap<String, u32> = levels
        .iter()
        .map(|(id, level)| ((*id).to_owned(), *level))
        .collect();
    game.set_profile(Profile {
        upgrades,
        ..Profile::default()
    })
    .unwrap();
    game.toggle_editor().unwrap();
    game
}
