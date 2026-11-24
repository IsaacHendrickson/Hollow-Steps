//! The Generate row drawn inside the F4 window: what it shows with and without a hub, and that its
//! radio buttons reach `set_generate_choice`.
use super::super::draw_tests::{click, frame, open_panel, rect_of};
use super::super::tests::Scratch;
use super::*;

const BUTTONS: [&str; 3] = ["generate Auto", "generate Never", "generate Always"];

#[test]
fn with_a_hub_the_row_shows_three_radio_buttons_and_a_label() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(text.contains("Generate next run"), "{text}");
    for word in ["Auto", "Never", "Always"] {
        assert!(text.contains(word), "{word}: {text}");
    }
    for name in BUTTONS {
        rect_of(&ctx, name);
    }
    assert!(text.contains("generator.enabled is off"), "{text}");
    assert_eq!(game.generate_choice(), GenerateChoice::Auto);
}

#[test]
fn with_no_hub_there_is_no_generate_row() {
    let scratch = Scratch::new(false);
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(!text.contains("Generate next run"), "{text}");
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
        rect_of(&ctx, "generate Always").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert_eq!(game.generate_choice(), GenerateChoice::Always);
    assert!(text.contains("generated, or does not start"), "{text}");
    assert!(text.contains("Generate choice set"), "{text}");

    click(
        &ctx,
        rect_of(&ctx, "generate Never").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert_eq!(game.generate_choice(), GenerateChoice::Never);
    assert!(
        text.contains("The next run plays the fixed dungeon"),
        "{text}"
    );

    click(
        &ctx,
        rect_of(&ctx, "generate Auto").center(),
        &mut panel,
        &mut game,
    );
    assert_eq!(game.generate_choice(), GenerateChoice::Auto);
}

#[test]
fn clicking_the_generate_choice_already_set_changes_nothing() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.set_generate_choice(GenerateChoice::Never);
    let (ctx, mut panel, _) = open_panel(&mut game);
    click(
        &ctx,
        rect_of(&ctx, "generate Never").center(),
        &mut panel,
        &mut game,
    );
    assert_eq!(game.generate_choice(), GenerateChoice::Never);
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(!text.contains("Generate choice set"), "{text}");
}

#[test]
fn the_generate_row_leaves_the_flip_rows_alone() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    click(
        &ctx,
        rect_of(&ctx, "generate Always").center(),
        &mut panel,
        &mut game,
    );
    assert_eq!(game.generate_choice(), GenerateChoice::Always);
    assert_eq!(game.flip_choice(), hollow_steps::variety::FlipChoice::Auto);
    assert_eq!(
        game.room_flip_choice(),
        hollow_steps::variety::FlipChoice::Auto
    );
}
