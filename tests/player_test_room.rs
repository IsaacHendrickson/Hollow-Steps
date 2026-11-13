mod support;

use hollow_steps::player_body::PlayerPose;
use hollow_steps::tuning::Field;
use support::player_assets::Assets;

#[test]
fn the_test_room_plays_the_unsaved_player_values_and_writes_nothing() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    session.player.set_field(Field::Width, 40.0).unwrap();
    assert_eq!(game.tuning().player.jump_speed, 480.0, "not in effect yet");

    game.start_test_room().unwrap();
    assert!(game.testing_room());
    assert_eq!(game.player().config().jump_speed, 650.0);
    assert_eq!(game.player().size().x, 40.0);
    assert!(
        !assets.file("player.ron").exists(),
        "a test room writes nothing"
    );

    game.end_test_room().unwrap();
    let session = game.editor().expect("back in the same session");
    assert_eq!(session.player.edited().player.jump_speed, 650.0);
    assert!(session.player.is_dirty(), "still unsaved after the test");
    assert!(!assets.file("player.ron").exists());
}

#[test]
fn the_test_room_draws_the_unsaved_look_and_parts_and_writes_nothing() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    assert!(game.test_player_body().is_none(), "only during a test");
    let session = game.editor_mut().unwrap();
    let mut visual = session.look.edited().visual.clone();
    visual.primary = (0.9, 0.1, 0.1, 1.0);
    session.look.set_visual(visual.clone());
    session
        .look
        .set_pose_name(PlayerPose::Run, "wanderer_jump".into());
    let template = session.content.bodies.get_mut("wanderer").unwrap();
    std::sync::Arc::make_mut(template)
        .parts
        .iter_mut()
        .find(|part| part.name == "cloak")
        .unwrap()
        .size = (0.7, 0.8);

    game.start_test_room().unwrap();
    let body = game.test_player_body().expect("a body for the test");
    assert_eq!(body.look().visual.primary, visual.primary);
    assert_eq!(body.look().run_pose, "wanderer_jump");
    let cloak = body
        .body()
        .template
        .parts
        .iter()
        .find(|part| part.name == "cloak")
        .unwrap();
    assert_eq!(cloak.size, (0.7, 0.8));
    assert!(!assets.file("player_body.ron").exists());
    assert!(!assets
        .file("bodies")
        .join("wanderer.ron")
        .metadata()
        .is_err());
    assert_eq!(
        std::fs::read_to_string(assets.file("bodies").join("wanderer.ron")).unwrap(),
        std::fs::read_to_string(
            support::fixture_assets()
                .join("player_look")
                .join("bodies")
                .join("wanderer.ron")
        )
        .unwrap(),
        "the template on disk is untouched"
    );

    game.end_test_room().unwrap();
    assert!(game.test_player_body().is_none());
    assert!(game.editor().unwrap().look.is_dirty());
}

#[test]
fn a_player_the_rooms_or_the_look_refuse_stops_the_test_and_says_why() {
    let assets = Assets::new(true);
    let mut game = assets.editing();

    // A size that starts inside a solid beside the spawn, in the draft being tested.
    let session = game.editor_mut().unwrap();
    let (x, y) = session.draft.layout.spawn;
    // 4 px from a 32 px player, inside a 64 px one, and well inside the room's bounds.
    let solid = (x + 20.0, y - 10.0, 40.0, 20.0);
    session.draft.layout.terrain.push(solid);
    session.player.set_field(Field::Width, 64.0).unwrap();
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("start inside terrain"), "{error}");
    assert!(!game.testing_room());
    let message = &game.editor().unwrap().message;
    assert!(message.contains("Cannot test the room"), "{message}");

    // A look the game could not draw.
    let session = game.editor_mut().unwrap();
    session.player.reset_all();
    session
        .draft
        .layout
        .terrain
        .retain(|existing| *existing != solid);
    session
        .look
        .set_pose_name(PlayerPose::Jump, "no_such_pose".into());
    let error = game.start_test_room().unwrap_err();
    assert!(error.contains("cannot use this look"), "{error}");
    assert!(error.contains("no_such_pose"), "{error}");
    assert!(!game.testing_room());

    // Put right, the same draft plays.
    game.editor_mut().unwrap().look.reset();
    game.start_test_room().unwrap();
}

#[test]
fn without_a_body_template_the_test_room_keeps_the_box() {
    let assets = Assets::new(false);
    let mut game = assets.editing();
    game.start_test_room().unwrap();
    assert!(game.test_player_body().is_none());
}
