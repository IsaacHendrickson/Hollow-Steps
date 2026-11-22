//! The Difficulty section drawn inside the F4 window: what it shows and that its boxes and its
//! Save button reach `set_percent`, `set_cap` and `save`.
use super::tests::Scratch;
use super::*;
use crate::tuning_panel::TuningPanel;
use hollow_steps::profile::Profile;

const TALL: f32 = 2_400.0;
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

fn rect_of(ctx: &egui::Context, id: impl std::hash::Hash) -> egui::Rect {
    ctx.data(|data| data.get_temp(egui::Id::new(id)))
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

/// Clicks the box at `rect`, types `text` and presses Enter, then draws once more.
fn type_in(
    ctx: &egui::Context,
    rect: egui::Rect,
    text: &str,
    panel: &mut TuningPanel,
    game: &mut GameState,
) -> String {
    click(ctx, rect.center(), panel, game);
    frame(ctx, vec![egui::Event::Text(text.into())], panel, game);
    frame(ctx, press_enter(), panel, game);
    frame(ctx, vec![], panel, game)
}

fn box_of(ctx: &egui::Context, field: Percent) -> egui::Rect {
    rect_of(ctx, ("difficulty box", field.label()))
}

#[test]
fn the_section_shows_the_six_boxes_the_readout_and_the_save_button() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (ctx, _panel, text) = open_panel(&mut game);

    assert!(text.contains("Difficulty\n"), "{text}");
    for field in Percent::ALL {
        assert!(text.contains(field.label()), "{}", field.label());
        box_of(&ctx, field);
    }
    assert!(text.contains("Tier cap"));
    rect_of(&ctx, "difficulty cap box");
    rect_of(&ctx, "save difficulty button");
    assert!(text.contains("Save difficulty"));
    assert!(text.contains("Matches the saved rules."), "{text}");
    assert!(text.contains("Tier 0: every number as authored"), "{text}");
}

#[test]
fn the_boxes_show_the_rules_in_effect() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (_ctx, _panel, text) = open_panel(&mut game);
    assert!(text.contains("10.0 %"), "{text}");
    assert!(text.contains("50.0 %"), "the boss damage percent: {text}");
    assert!(text.contains("20"), "the cap");
}

#[test]
fn the_readout_follows_the_debug_tier() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    game.set_profile(Profile {
        tier: 3,
        ..Profile::default()
    })
    .unwrap();
    let (_ctx, _panel, text) = open_panel(&mut game);
    assert!(text.contains("Tier 3\n"), "{text}");
    assert!(
        text.contains("Enemies: health x1.30, damage x1.60"),
        "{text}"
    );
    assert!(
        text.contains("Bosses: health x1.60, damage x2.50"),
        "{text}"
    );
    assert!(text.contains("Coins: x1.30"), "{text}");
}

#[test]
fn typing_a_percent_sets_it_for_the_next_run_and_shows_it_unsaved() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);

    let text = type_in(
        &ctx,
        box_of(&ctx, Percent::EnemyHealth),
        "45",
        &mut panel,
        &mut game,
    );

    assert_eq!(game.difficulty().enemy_health_percent, 45.0);
    assert!(text.contains("Unsaved changes"), "{text}");
    assert_eq!(
        Difficulty::load(&scratch.0).unwrap().enemy_health_percent,
        10.0,
        "the file is not written until Save"
    );
}

#[test]
fn a_typed_percent_past_the_limit_is_held_at_the_limit() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_in(
        &ctx,
        box_of(&ctx, Percent::Coins),
        "500",
        &mut panel,
        &mut game,
    );
    assert_eq!(game.difficulty().coin_percent, 100.0);
}

#[test]
fn typing_the_cap_sets_it_and_holds_it_inside_1_to_99() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);

    type_in(
        &ctx,
        rect_of(&ctx, "difficulty cap box"),
        "7",
        &mut panel,
        &mut game,
    );
    assert_eq!(game.difficulty().max_effective_tier, 7);

    type_in(
        &ctx,
        rect_of(&ctx, "difficulty cap box"),
        "500",
        &mut panel,
        &mut game,
    );
    assert_eq!(game.difficulty().max_effective_tier, 99);

    type_in(
        &ctx,
        rect_of(&ctx, "difficulty cap box"),
        "0",
        &mut panel,
        &mut game,
    );
    assert_eq!(game.difficulty().max_effective_tier, 1);
}

#[test]
fn clicking_save_writes_the_file_and_says_where() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_in(
        &ctx,
        box_of(&ctx, Percent::BossHealth),
        "35",
        &mut panel,
        &mut game,
    );

    click(
        &ctx,
        rect_of(&ctx, "save difficulty button").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);

    assert_eq!(
        Difficulty::load(&scratch.0).unwrap().boss_health_percent,
        35.0
    );
    assert!(text.contains("Saved to"), "{text}");
    assert!(text.contains("difficulty.ron"), "{text}");
    assert!(text.contains("Matches the saved rules."), "{text}");
}

#[test]
fn a_failed_save_is_shown_with_the_file_name_and_the_rules_stay_unsaved() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_in(
        &ctx,
        box_of(&ctx, Percent::Coins),
        "35",
        &mut panel,
        &mut game,
    );
    std::fs::remove_file(scratch.file()).unwrap();
    std::fs::create_dir(scratch.file()).unwrap();

    click(
        &ctx,
        rect_of(&ctx, "save difficulty button").center(),
        &mut panel,
        &mut game,
    );
    let text = frame(&ctx, vec![], &mut panel, &mut game);

    assert!(!text.contains("Saved to"), "{text}");
    assert!(text.contains("difficulty.ron"), "{text}");
    assert!(text.contains("Unsaved changes"), "{text}");
    assert_eq!(game.difficulty().coin_percent, 35.0);
}

#[test]
fn an_edit_never_changes_the_profile() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    game.set_profile(Profile {
        tier: 4,
        coins: 9,
        ..Profile::default()
    })
    .unwrap();
    let (ctx, mut panel, _) = open_panel(&mut game);
    type_in(
        &ctx,
        box_of(&ctx, Percent::EnemyDamage),
        "60",
        &mut panel,
        &mut game,
    );
    assert_eq!((game.profile().tier, game.profile().coins), (4, 9));
}
