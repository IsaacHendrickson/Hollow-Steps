//! The Run (debug) section drawn inside the F4 window: what it shows with and without a hub, and
//! that its widgets reach `set_tier` and `reset_profile`.
use super::tests::{saved, Scratch};
use super::*;
use crate::tuning_panel::TuningPanel;

const TALL: f32 = 1_800.0;
/// Frames egui needs to size the scrolling list before every row is drawn.
const SETTLE_FRAMES: usize = 16;

fn drawn_text(output: &egui::FullOutput) -> String {
    fn collect(shape: &egui::epaint::Shape, out: &mut String) {
        match shape {
            egui::epaint::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let mut text = String::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

fn frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    panel: &mut TuningPanel,
    game: &mut GameState,
) -> String {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, TALL),
            )),
            events,
            ..Default::default()
        },
        |ctx| panel.draw(ctx, game),
    );
    drawn_text(&output)
}

fn open_panel(game: &mut GameState) -> (egui::Context, TuningPanel, String) {
    let ctx = egui::Context::default();
    let mut panel = TuningPanel::default();
    panel.toggle();
    let mut text = String::new();
    for _ in 0..SETTLE_FRAMES {
        text = frame(&ctx, vec![], &mut panel, game);
    }
    (ctx, panel, text)
}

fn rect_of(ctx: &egui::Context, name: &str) -> egui::Rect {
    ctx.data(|data| data.get_temp(egui::Id::new(name)))
        .expect("widget was not drawn")
}

fn pointer(at: egui::Pos2, pressed: Option<bool>) -> Vec<egui::Event> {
    let mut events = vec![egui::Event::PointerMoved(at)];
    if let Some(pressed) = pressed {
        events.push(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        });
    }
    events
}

fn click(ctx: &egui::Context, at: egui::Pos2, panel: &mut TuningPanel, game: &mut GameState) {
    frame(ctx, pointer(at, Some(true)), panel, game);
    frame(ctx, pointer(at, Some(false)), panel, game);
    frame(ctx, vec![], panel, game);
}

fn press_enter() -> Vec<egui::Event> {
    [true, false]
        .into_iter()
        .map(|pressed| egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: Some(egui::Key::Enter),
            pressed,
            repeat: false,
            modifiers: Default::default(),
        })
        .collect()
}

#[test]
fn with_a_hub_the_section_shows_the_tier_box_the_banked_numbers_and_the_reset_button() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.set_profile(Profile {
        coins: 12,
        tier: 3,
        cleared: 2,
        ..Profile::default()
    })
    .unwrap();

    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(text.contains("Run (debug)"), "{text}");
    assert!(text.contains("Difficulty tier"));
    assert!(text.contains("Reset profile"));
    assert!(text.contains("Banked coins 12"));
    assert!(text.contains("dungeons cleared 2"));
    rect_of(&ctx, "tier box");
    rect_of(&ctx, "reset profile button");
}

#[test]
fn with_no_hub_the_section_only_says_there_are_no_runs() {
    let scratch = Scratch::new(false);
    let mut game = scratch.game();

    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(text.contains("Run (debug)"));
    assert!(text.contains("no hub room"), "{text}");
    assert!(!text.contains("Reset profile"));
    assert!(ctx
        .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("tier box")))
        .is_none());
}

#[test]
fn clicking_reset_profile_zeroes_and_saves_the_profile_and_says_so() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.set_profile(Profile {
        coins: 50,
        tier: 4,
        cleared: 4,
        ..Profile::default()
    })
    .unwrap();
    let (ctx, mut panel, _) = open_panel(&mut game);

    click(
        &ctx,
        rect_of(&ctx, "reset profile button").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);

    assert_eq!(*game.profile(), Profile::default());
    assert_eq!(saved(&scratch), Profile::default());
    assert!(text.contains("Profile reset"), "{text}");
}

#[test]
fn typing_a_tier_in_the_box_sets_and_saves_it() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);

    click(
        &ctx,
        rect_of(&ctx, "tier box").center(),
        &mut panel,
        &mut game,
    );
    frame(
        &ctx,
        vec![egui::Event::Text("12".into())],
        &mut panel,
        &mut game,
    );
    frame(&ctx, press_enter(), &mut panel, &mut game);
    let text = frame(&ctx, vec![], &mut panel, &mut game);

    assert_eq!(game.profile().tier, 12);
    assert_eq!(saved(&scratch).tier, 12);
    assert!(text.contains("Tier saved"), "{text}");
}

#[test]
fn a_typed_tier_past_the_limit_is_held_at_the_limit() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);

    click(
        &ctx,
        rect_of(&ctx, "tier box").center(),
        &mut panel,
        &mut game,
    );
    frame(
        &ctx,
        vec![egui::Event::Text("500".into())],
        &mut panel,
        &mut game,
    );
    frame(&ctx, press_enter(), &mut panel, &mut game);
    frame(&ctx, vec![], &mut panel, &mut game);

    assert_eq!(game.profile().tier, MAX_TIER);
}

#[test]
fn a_failed_save_is_shown_under_the_buttons() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    std::fs::create_dir(scratch.0.join("profile.ron.tmp")).unwrap();
    let (ctx, mut panel, _) = open_panel(&mut game);

    click(
        &ctx,
        rect_of(&ctx, "reset profile button").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);

    assert!(!text.contains("Profile reset"), "{text}");
    assert!(
        text.contains("profile.ron"),
        "the error names the file\n{text}"
    );
    assert_eq!(game.profile().coins, 0);
}
