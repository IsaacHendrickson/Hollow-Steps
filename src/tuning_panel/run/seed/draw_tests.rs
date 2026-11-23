//! The Seed row drawn inside the F4 window: what it shows with and without a hub, and that its
//! checkbox and text box reach `set_fixed_seed`.
use super::super::draw_tests::{click, frame, open_panel, rect_of};
use super::super::tests::Scratch;
use super::*;

fn type_text(
    ctx: &egui::Context,
    text: &str,
    panel: &mut crate::tuning_panel::TuningPanel,
    game: &mut GameState,
) -> String {
    click(ctx, rect_of(ctx, "seed text box").center(), panel, game);
    // Clicking in the box can select what is in it, so the typing starts at the end.
    let end = [true, false].map(|pressed| egui::Event::Key {
        key: egui::Key::End,
        physical_key: Some(egui::Key::End),
        pressed,
        repeat: false,
        modifiers: Default::default(),
    });
    frame(ctx, end.to_vec(), panel, game);
    frame(ctx, vec![egui::Event::Text(text.into())], panel, game);
    frame(ctx, vec![], panel, game)
}

#[test]
fn with_a_hub_the_row_shows_the_checkbox_the_text_box_and_the_label() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(text.contains("Fixed seed"), "{text}");
    assert!(text.contains("No run has started"), "{text}");
    rect_of(&ctx, "fixed seed checkbox");
    rect_of(&ctx, "seed text box");
    assert_eq!(game.fixed_seed(), None);
}

#[test]
fn with_no_hub_there_is_no_seed_row() {
    let scratch = Scratch::new(false);
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(!text.contains("Fixed seed"), "{text}");
    for name in ["fixed seed checkbox", "seed text box"] {
        assert!(ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(name)))
            .is_none());
    }
}

#[test]
fn typing_a_seed_and_ticking_the_box_fixes_it_and_unticking_frees_it() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_text(&ctx, "1f3a", &mut panel, &mut game);
    assert_eq!(game.fixed_seed(), None, "typing alone fixes nothing");

    click(
        &ctx,
        rect_of(&ctx, "fixed seed checkbox").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert_eq!(game.fixed_seed(), Some(0x1F3A));
    assert!(text.contains("Fixed seed on"), "{text}");

    click(
        &ctx,
        rect_of(&ctx, "fixed seed checkbox").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert_eq!(game.fixed_seed(), None);
    assert!(text.contains("Fixed seed off"), "{text}");
}

#[test]
fn ticking_the_box_with_nothing_typed_is_refused_and_says_why() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);

    click(
        &ctx,
        rect_of(&ctx, "fixed seed checkbox").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);

    assert_eq!(game.fixed_seed(), None, "the box stays unticked");
    assert!(text.contains("empty"), "{text}");
    assert!(!text.contains("Fixed seed on"), "{text}");
}

#[test]
fn typing_while_fixed_changes_the_seed_and_a_bad_text_keeps_the_old_one() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_text(&ctx, "ab", &mut panel, &mut game);
    click(
        &ctx,
        rect_of(&ctx, "fixed seed checkbox").center(),
        &mut panel,
        &mut game,
    );
    assert_eq!(game.fixed_seed(), Some(0xAB));

    // "abc" is still a seed.
    let text = type_text(&ctx, "c", &mut panel, &mut game);
    assert_eq!(game.fixed_seed(), Some(0xABC));
    assert!(text.contains("Fixed seed changed"), "{text}");

    // "abcg" is not: the old seed stays and the reason shows.
    let text = type_text(&ctx, "g", &mut panel, &mut game);
    assert_eq!(game.fixed_seed(), Some(0xABC), "{text}");
    assert!(text.contains("'g'") && text.contains("hex digit"), "{text}");
}

#[test]
fn the_label_shows_the_seed_of_the_run_that_started() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_text(&ctx, "ab", &mut panel, &mut game);
    click(
        &ctx,
        rect_of(&ctx, "fixed seed checkbox").center(),
        &mut panel,
        &mut game,
    );

    game.update(
        hollow_steps::player::PlayerInput {
            move_axis: 1.0,
            ..Default::default()
        },
        10.0,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(
        text.contains("Seed of this run: 00000000000000ab"),
        "{text}"
    );
}
