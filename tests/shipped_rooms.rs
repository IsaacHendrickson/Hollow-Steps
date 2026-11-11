use std::path::Path;

use hollow_steps::game::GameState;

#[test]
fn shipped_rooms_load_and_validate() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // Startup validates room geometry, entities, behaviors, and exit destinations,
    // including saved custom rooms. Authored coordinates and contents may change.
    GameState::try_new(&assets).expect("shipped rooms must load and validate");
    hollow_steps::preset::load_presets(&assets.join("presets"))
        .expect("shipped behavior presets must load and validate");
}

#[test]
fn shipped_player_body_loads_and_validates() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // The wanderer template, its four poses and the look must load and fit together.
    let body = hollow_steps::player_body::PlayerBody::load(&assets)
        .expect("shipped player body must load and validate");
    assert!(body.is_some(), "the wanderer body template is shipped");
}
