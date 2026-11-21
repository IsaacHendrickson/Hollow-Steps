use super::ChecksUi;
use hollow_steps::game::GameState;
use hollow_steps::tuning::Field;

fn editing() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game
}

#[test]
fn the_interval_is_the_editors_and_is_measured_again_when_a_number_changes() {
    let mut game = editing();
    let editor = game.editor_mut().unwrap();
    let mut checks = ChecksUi::default();

    let first = checks.interval(editor);
    assert_eq!(first, editor.reach_interval());
    assert_eq!(
        checks.interval(editor),
        first,
        "the same numbers, the same interval"
    );

    let faster = editor.player.edited().player.jump_speed * 1.2;
    editor.player.set_field(Field::JumpSpeed, faster).unwrap();
    let second = checks.interval(editor);
    assert_ne!(second, first, "a higher jump reaches farther");
    assert_eq!(second, editor.reach_interval());
}

#[test]
fn the_toggle_starts_off_and_nothing_is_measured_until_it_is_asked_for() {
    let checks = ChecksUi::default();
    assert!(!checks.reach);
    assert!(checks.measured.is_none());
    assert!(checks.dungeon.is_none());
}
