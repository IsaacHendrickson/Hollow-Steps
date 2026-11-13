mod support;

use hollow_steps::editor_guard::{
    perform, save_all, unsaved_items, GuardChoice, GuardStep, GuardedAction, UnsavedGuard,
};
use hollow_steps::player_body::{PlayerLook, PlayerPose};
use hollow_steps::room::RoomId;
use hollow_steps::tuning::Field;
use support::player_assets::Assets;

#[test]
fn a_dirty_player_makes_a_clean_room_ask_and_the_dialog_lists_player() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let mut guard = UnsavedGuard::default();

    // Nothing unsaved: the close goes straight through and lists nothing.
    assert!(unsaved_items(game.editor().unwrap()).is_empty());
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Perform(GuardedAction::Close)
    );
    assert!(guard.unsaved().is_empty());

    // A stat edit alone is unsaved work.
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    assert!(!session.dirty);
    assert_eq!(
        guard.request_for(GuardedAction::Close, game.editor()),
        GuardStep::Wait
    );
    assert!(guard.is_open());
    assert_eq!(guard.unsaved(), ["Player"]);
    guard.choose(GuardChoice::Cancel);

    // So is a look edit alone.
    let session = game.editor_mut().unwrap();
    session.player.reset_all();
    session.look.set_run_speed(80.0);
    assert_eq!(unsaved_items(game.editor().unwrap()), ["Player"]);
    assert_eq!(
        guard.request_for(GuardedAction::NewRoom, game.editor()),
        GuardStep::Wait
    );
    assert_eq!(guard.unsaved(), ["Player"]);
    guard.choose(GuardChoice::Cancel);

    // With the room dirty too, both are listed.
    let session = game.editor_mut().unwrap();
    session.dirty = true;
    assert_eq!(
        unsaved_items(game.editor().unwrap()),
        ["Room and definitions", "Player"]
    );
    guard.request_for(GuardedAction::Close, game.editor());
    assert_eq!(guard.unsaved(), ["Room and definitions", "Player"]);
    guard.choose(GuardChoice::Cancel);

    // A request without a list does not show an old one.
    guard.request(GuardedAction::Close, true);
    assert!(guard.unsaved().is_empty());
}

#[test]
fn save_writes_the_player_and_only_what_is_dirty() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    session.look.set_run_speed(80.0);

    save_all(&mut game).unwrap();
    assert!(assets.file("player.ron").exists());
    let look = PlayerLook::load(&assets.0).unwrap();
    assert_eq!(look.run_speed, 80.0);
    let session = game.editor().unwrap();
    assert!(!session.player.is_dirty() && !session.look.is_dirty());
    assert!(
        !session.message.starts_with("Saved"),
        "the clean room was not saved: {}",
        session.message
    );
    assert!(unsaved_items(session).is_empty());

    // A dirty room is saved too.
    let session = game.editor_mut().unwrap();
    session.dirty = true;
    session.player.set_field(Field::DashSpeed, 900.0).unwrap();
    save_all(&mut game).unwrap();
    let session = game.editor().unwrap();
    assert!(session.message.starts_with("Saved"), "{}", session.message);
    assert!(!session.dirty && !session.player.is_dirty());
}

#[test]
fn a_player_that_cannot_be_saved_fails_the_save_and_writes_no_player_file() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    session
        .look
        .set_pose_name(PlayerPose::Idle, "no_such_pose".into());
    let error = save_all(&mut game).unwrap_err();
    assert!(error.contains("no_such_pose"), "{error}");
    assert!(!assets.file("player.ron").exists());
    assert!(!assets.file("player_body.ron").exists());
    assert!(game.editor().unwrap().player.is_dirty(), "still unsaved");
}

#[test]
fn discard_gives_up_the_players_unsaved_edits_and_save_keeps_them() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    session.look.set_run_speed(80.0);
    game.sync_player_from_editor();
    assert_eq!(game.player().config().jump_speed, 650.0, "in effect");

    // Discard: the editor closes and the saved values are back in play.
    perform(&mut game, &GuardedAction::Close).unwrap();
    assert!(game.editor().is_none());
    assert_eq!(game.tuning().player.jump_speed, 480.0);
    assert_eq!(game.player().config().jump_speed, 480.0);
    assert!(!game.player_draft().is_dirty());
    assert!(!assets.file("player.ron").exists());

    // Opening another session starts clean, not from the discarded edits.
    game.toggle_editor().unwrap();
    let session = game.editor().unwrap();
    assert!(!session.player.is_dirty() && !session.look.is_dirty());

    // Discard on the way to another room does the same.
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::DashSpeed, 900.0).unwrap();
    perform(&mut game, &GuardedAction::OpenRoom(RoomId::Second)).unwrap();
    assert!(!game.editor().unwrap().player.is_dirty());
    assert_ne!(game.tuning().player.dash_speed, 900.0);

    // Save then close: what was saved stays.
    let session = game.editor_mut().unwrap();
    session.player.set_field(Field::JumpSpeed, 650.0).unwrap();
    save_all(&mut game).unwrap();
    perform(&mut game, &GuardedAction::Close).unwrap();
    assert_eq!(game.tuning().player.jump_speed, 650.0);
    assert_eq!(game.player().config().jump_speed, 650.0);
}
