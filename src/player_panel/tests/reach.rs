//! The reach line under "Saved to" in the Player panel; the game decides the text
//! (`GameState::take_reach_note`), the panel only shows it next to the saved message.
use super::*;

const LINE: &str = "jump 183 px, dash jump 296 px; run Check dungeon";

#[test]
fn the_line_is_drawn_under_saved_to_and_goes_with_the_saved_message() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    state.status = Status::Saved("assets/player.ron".into());
    state.reach = Some(LINE.into());
    let text = frame(&ctx, vec![], editor, &mut state);
    let saved = text
        .find("Saved to assets/player.ron")
        .expect("saved message");
    assert!(text[saved..].contains(LINE), "{text}");

    // An edit clears the saved message, and the line with it.
    type_into(&ctx, Field::MoveSpeed, "300", editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(!text.contains(LINE), "{text}");
}

#[test]
fn a_save_with_no_line_and_a_failed_save_show_none() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    state.status = Status::Saved("assets/player.ron".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Saved to assets/player.ron"), "{text}");
    assert!(!text.contains("dash jump"), "{text}");

    state.status = Status::Error("player.ron: the disk is full".into());
    state.reach = Some(LINE.into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("the disk is full"), "{text}");
    assert!(
        !text.contains(LINE),
        "a failed save has no new reach: {text}"
    );
}
