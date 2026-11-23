//! The Flip row drawn inside the F4 window: what it shows with and without a hub, and that its
//! radio buttons reach `set_flip_choice`.
use super::super::draw_tests::{click, frame, open_panel, rect_of};
use super::super::tests::Scratch;
use super::*;

const BUTTONS: [&str; 3] = ["flip Auto", "flip Never", "flip Always"];

#[test]
fn with_a_hub_the_row_shows_three_radio_buttons_and_a_label() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(text.contains("Flip next run"), "{text}");
    for word in ["Auto", "Never", "Always"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    for name in BUTTONS {
        rect_of(&ctx, name);
    }
    assert!(text.contains("next run is flipped by chance"), "{text}");
    assert_eq!(game.flip_choice(), FlipChoice::Auto);
}

#[test]
fn with_no_hub_there_is_no_flip_row() {
    let scratch = Scratch::new(false);
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(!text.contains("Flip next run"), "{text}");
    for name in BUTTONS {
        assert!(ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(name)))
            .is_none());
    }
}

#[test]
fn clicking_a_radio_button_sets_the_choice_and_the_label_follows() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);

    click(
        &ctx,
        rect_of(&ctx, "flip Always").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert_eq!(game.flip_choice(), FlipChoice::Always);
    assert!(text.contains("The next run is flipped"), "{text}");
    assert!(text.contains("Flip choice set"), "{text}");

    click(
        &ctx,
        rect_of(&ctx, "flip Never").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert_eq!(game.flip_choice(), FlipChoice::Never);
    assert!(text.contains("The next run is not flipped"), "{text}");

    click(
        &ctx,
        rect_of(&ctx, "flip Auto").center(),
        &mut panel,
        &mut game,
    );
    assert_eq!(game.flip_choice(), FlipChoice::Auto);
}

#[test]
fn clicking_the_choice_already_set_changes_nothing() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.set_flip_choice(FlipChoice::Never);
    let (ctx, mut panel, _) = open_panel(&mut game);
    click(
        &ctx,
        rect_of(&ctx, "flip Never").center(),
        &mut panel,
        &mut game,
    );
    assert_eq!(game.flip_choice(), FlipChoice::Never);
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(!text.contains("Flip choice set"), "{text}");
}
