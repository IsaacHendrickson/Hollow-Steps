use std::path::Path;

use hollow_steps::game::GameState;

#[test]
fn shipped_rooms_load_and_validate() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // Startup validates room geometry, entities, behaviors, and exit destinations,
    // including saved custom rooms. Authored coordinates and contents may change.
    GameState::try_new(&assets).expect("shipped rooms must load and validate");
}
