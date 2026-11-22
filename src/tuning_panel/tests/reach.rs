//! The reach line F4 shows under "Saved to" after a Save that changed the player's numbers.
use super::*;

fn save(ctx: &egui::Context, panel: &mut TuningPanel, game: &mut GameState) -> String {
    click(
        ctx,
        rect_of(ctx, egui::Id::new("save button")).center(),
        panel,
        game,
    );
    frame(ctx, vec![], panel, game)
}

#[test]
fn a_save_that_changed_the_jump_shows_the_reach_line_and_an_unchanged_save_does_not() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::JumpSpeed, "500", &mut panel, &mut game);
    let before = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(!before.contains("dash jump"), "{before}");

    let text = save(&ctx, &mut panel, &mut game);
    assert!(text.contains("Saved to"), "{text}");
    assert!(text.contains("jump "), "{text}");
    assert!(text.contains(" px, dash jump "), "{text}");
    assert!(text.contains("; run Check dungeon"), "{text}");

    let text = save(&ctx, &mut panel, &mut game);
    assert!(text.contains("Saved to"), "{text}");
    assert!(!text.contains("dash jump"), "nothing changed: {text}");
}

#[test]
fn an_edit_after_the_save_takes_the_reach_line_away_with_the_saved_message() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::JumpSpeed, "500", &mut panel, &mut game);
    let text = save(&ctx, &mut panel, &mut game);
    assert!(text.contains("dash jump"), "{text}");

    type_into(&ctx, Field::JumpSpeed, "510", &mut panel, &mut game);
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(!text.contains("dash jump"), "{text}");
}

#[test]
fn a_save_that_changed_only_the_double_jump_speed_shows_every_abilitys_numbers() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::DoubleJumpSpeed, "450", &mut panel, &mut game);

    let text = save(&ctx, &mut panel, &mut game);

    assert!(text.contains("Saved to"), "{text}");
    assert!(text.contains(" px, dash jump "), "{text}");
    assert!(text.contains(", double jump "), "{text}");
    assert!(text.contains(" px high / "), "{text}");
    assert!(text.contains(" px far; run Check dungeon"), "{text}");
}
