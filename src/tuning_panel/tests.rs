use super::*;
use hollow_steps::tuning::{TuningConfig, FILE_NAME};
use hollow_steps::vitals::{VitalsConfig, FILE_NAME as VITALS_FILE_NAME};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

/// A private copy of the fixture assets, so Save never touches the repository.
struct Assets(PathBuf);

impl Assets {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-tuning-panel-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        for directory in ["behaviors", "entities", "rooms", "bosses"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
            for entry in std::fs::read_dir(fixtures.join(directory)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(directory).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn game(&self) -> GameState {
        GameState::try_new(&self.0).unwrap()
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

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

/// The game window's height; the field list has to scroll in it.
const GAME_HEIGHT: f32 = 540.0;
/// Tall enough that every row is drawn and clickable without scrolling.
const TALL: f32 = 1_400.0;
/// Frames egui needs to size the scrolling list before every row is drawn; a longer list
/// needs more (three were enough for 15 fields, 22 need five, the 33 with the vitals nine).
const SETTLE_FRAMES: usize = 10;

/// One egui pass over the window in a 960-wide screen of the given height.
fn frame_in(
    height: f32,
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    panel: &mut TuningPanel,
    game: &mut GameState,
) -> String {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(960.0, height),
            )),
            events,
            ..Default::default()
        },
        |ctx| panel.draw(ctx, game),
    );
    drawn_text(&output)
}

fn frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    panel: &mut TuningPanel,
    game: &mut GameState,
) -> String {
    frame_in(TALL, ctx, events, panel, game)
}

fn open_panel(game: &mut GameState) -> (egui::Context, TuningPanel) {
    let ctx = egui::Context::default();
    let mut panel = TuningPanel::default();
    panel.toggle();
    for _ in 0..SETTLE_FRAMES {
        frame(&ctx, vec![], &mut panel, game);
    }
    (ctx, panel)
}

fn rect_of(ctx: &egui::Context, id: egui::Id) -> egui::Rect {
    ctx.data(|data| data.get_temp(id))
        .expect("widget was not drawn")
}

fn field_rect(ctx: &egui::Context, field: Field) -> egui::Rect {
    rect_of(ctx, egui::Id::new(("field", field.spec().key)))
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

/// Clicks a drag box, types `text` over its value and presses Enter.
fn type_into(
    ctx: &egui::Context,
    field: Field,
    text: &str,
    panel: &mut TuningPanel,
    game: &mut GameState,
) {
    click(ctx, field_rect(ctx, field).center(), panel, game);
    frame(ctx, vec![egui::Event::Text(text.into())], panel, game);
    frame(ctx, press_enter(), panel, game);
    frame(ctx, vec![], panel, game);
}

#[test]
fn a_closed_window_draws_nothing_and_toggle_opens_and_closes_it() {
    let assets = Assets::new();
    let mut game = assets.game();
    let ctx = egui::Context::default();
    let mut panel = TuningPanel::default();
    assert!(frame(&ctx, vec![], &mut panel, &mut game).is_empty());
    panel.toggle();
    // egui lays a new window out invisibly on its first frame.
    frame(&ctx, vec![], &mut panel, &mut game);
    assert!(frame(&ctx, vec![], &mut panel, &mut game).contains("Player tuning"));
    panel.toggle();
    assert!(frame(&ctx, vec![], &mut panel, &mut game).is_empty());
}

#[test]
fn the_window_shows_every_group_field_and_the_save_button() {
    let assets = Assets::new();
    let mut game = assets.game();
    let ctx = egui::Context::default();
    let mut panel = TuningPanel::default();
    panel.toggle();
    let mut text = String::new();
    for _ in 0..SETTLE_FRAMES {
        text = frame(&ctx, vec![], &mut panel, &mut game);
    }
    for field in Field::ALL {
        let spec = field.spec();
        assert!(text.contains(spec.label), "missing {}\n{text}", spec.label);
        assert!(text.contains(spec.group), "missing {}", spec.group);
        field_rect(&ctx, field);
    }
    assert!(text.contains("Save"));
    assert!(text.contains("Matches the saved values."));
    assert!(!text.contains("Unsaved"));
    rect_of(&ctx, egui::Id::new("save button"));
}

#[test]
fn dragging_a_value_changes_the_game_at_once_and_stops_at_the_range_end() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    let before = game.tuning().player.move_speed;
    let start = field_rect(&ctx, Field::MoveSpeed).center();

    frame(&ctx, pointer(start, Some(true)), &mut panel, &mut game);
    let moved = start + egui::vec2(30.0, 0.0);
    frame(&ctx, pointer(moved, None), &mut panel, &mut game);
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(game.tuning().player.move_speed > before);
    assert_eq!(
        game.player().config().move_speed,
        game.tuning().player.move_speed,
        "the player has the new value already"
    );
    assert!(game.tuning_unsaved());
    assert!(text.contains("Unsaved changes"), "{text}");

    // Dragging far past the end holds at the maximum instead of being refused.
    frame(
        &ctx,
        pointer(start + egui::vec2(50_000.0, 0.0), None),
        &mut panel,
        &mut game,
    );
    frame(&ctx, pointer(start, Some(false)), &mut panel, &mut game);
    assert_eq!(game.tuning().player.move_speed, Field::MoveSpeed.spec().max);
    game.tuning().validate().unwrap();
}

#[test]
fn typing_a_number_in_range_sets_it_and_clears_the_footer_message() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::MoveSpeed, "99999", &mut panel, &mut game);
    assert!(matches!(panel.status, Status::Error(_)));
    type_into(&ctx, Field::MoveSpeed, "300", &mut panel, &mut game);
    assert_eq!(game.tuning().player.move_speed, 300.0);
    assert_eq!(game.player().config().move_speed, 300.0);
    assert_eq!(panel.status, Status::Quiet);
}

#[test]
fn typing_an_out_of_range_or_non_numeric_value_is_blocked_with_a_message() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    let before = *game.tuning();
    for (field, text) in [
        (Field::MoveSpeed, "99999"),
        (Field::Gravity, "-4"),
        (Field::DashDuration, "0"),
        (Field::EnemyDefaultHealth, "0"),
        (Field::JumpSpeed, "fast"),
    ] {
        type_into(&ctx, field, text, &mut panel, &mut game);
        let Status::Error(message) = panel.status.clone() else {
            panic!("{text} for {field:?} was not refused: {:?}", panel.status);
        };
        assert!(message.contains(field.spec().key), "{message}");
        assert_eq!(*game.tuning(), before, "{text} changed the game");
        let shown = frame(&ctx, vec![], &mut panel, &mut game);
        assert!(shown.contains(&message), "{shown}");
    }
    assert!(!game.tuning_unsaved());
}

#[test]
fn in_the_game_window_the_list_scrolls_so_the_last_field_can_be_reached() {
    let assets = Assets::new();
    let mut game = assets.game();
    let ctx = egui::Context::default();
    let mut panel = TuningPanel::default();
    panel.toggle();
    let draw = |ctx: &egui::Context, events, panel: &mut TuningPanel, game: &mut GameState| {
        frame_in(GAME_HEIGHT, ctx, events, panel, game)
    };
    for _ in 0..3 {
        draw(&ctx, vec![], &mut panel, &mut game);
    }
    assert!(
        ctx.used_rect().max.y <= GAME_HEIGHT,
        "the window must fit the screen: {:?}",
        ctx.used_rect()
    );
    let save = rect_of(&ctx, egui::Id::new("save button"));
    assert!(save.max.y <= GAME_HEIGHT, "Save stays in view: {save:?}");
    let last = Field::EnemyKnockbackSeconds;
    // The footer sits under the list, so a row that overlaps it is scrolled out of view.
    assert!(
        field_rect(&ctx, last).max.y > save.min.y,
        "starts out of view: {:?} vs {save:?}",
        field_rect(&ctx, last)
    );

    let over_list = field_rect(&ctx, Field::MoveSpeed).center();
    for _ in 0..60 {
        let wheel = egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: egui::vec2(0.0, -3.0),
            modifiers: Default::default(),
        };
        draw(
            &ctx,
            vec![egui::Event::PointerMoved(over_list), wheel],
            &mut panel,
            &mut game,
        );
    }
    draw(&ctx, vec![], &mut panel, &mut game);
    assert!(
        field_rect(&ctx, last).max.y <= save.min.y,
        "scrolled into view: {:?} vs {save:?}",
        field_rect(&ctx, last)
    );
}

#[test]
fn parse_entry_accepts_numbers_with_unit_or_comma_and_reports_the_reason_otherwise() {
    let spec = Field::MoveSpeed.spec();
    let rejected = RefCell::new(None);
    assert_eq!(parse_entry(spec, " 300 ", &rejected), Some(300.0));
    assert_eq!(parse_entry(spec, "300 px/s", &rejected), Some(300.0));
    assert_eq!(parse_entry(spec, "300.5", &rejected), Some(300.5));
    let seconds = Field::CoyoteSeconds.spec();
    assert_eq!(parse_entry(seconds, "0,25", &rejected), Some(0.25));
    assert_eq!(*rejected.borrow(), None);
    for text in ["", "abc", "NaN", "inf", "-1", "1501", "1e9"] {
        assert_eq!(parse_entry(spec, text, &rejected), None, "{text:?}");
        let message = rejected.borrow_mut().take().expect(text);
        assert!(message.contains("player.move_speed"), "{message}");
    }
    let health = Field::EnemyDefaultHealth.spec();
    assert_eq!(parse_entry(health, "4.4", &rejected), Some(4.0));
    assert_eq!(parse_entry(health, "1000", &rejected), None);
}

#[test]
fn save_writes_player_ron_that_loads_back_and_clears_the_unsaved_note() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::DashCooldown, "1.25", &mut panel, &mut game);
    assert!(game.tuning_unsaved());
    assert!(!assets.0.join(FILE_NAME).exists());

    click(
        &ctx,
        rect_of(&ctx, egui::Id::new("save button")).center(),
        &mut panel,
        &mut game,
    );
    assert!(
        matches!(panel.status, Status::Saved(_)),
        "{:?}",
        panel.status
    );
    assert!(!game.tuning_unsaved());
    let saved = TuningConfig::load(&assets.0).unwrap();
    assert_eq!(saved.player.dash_cooldown, 1.25);
    assert_eq!(saved, *game.tuning());
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(text.contains("Saved to"), "{text}");
    assert!(text.contains("Matches the saved values."), "{text}");
    assert!(!assets.0.join("player.ron.tmp").exists());
}

#[test]
fn a_failed_save_shows_the_error_and_keeps_the_unsaved_note() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::Gravity, "2000", &mut panel, &mut game);
    std::fs::create_dir(assets.0.join("player.ron.tmp")).unwrap();
    click(
        &ctx,
        rect_of(&ctx, egui::Id::new("save button")).center(),
        &mut panel,
        &mut game,
    );
    let Status::Error(message) = &panel.status else {
        panic!("save should have failed: {:?}", panel.status);
    };
    assert!(message.contains("player.ron"), "{message}");
    assert!(game.tuning_unsaved());
    assert!(!assets.0.join(FILE_NAME).exists());
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(text.contains("Unsaved changes"), "{text}");
    std::fs::remove_dir(assets.0.join("player.ron.tmp")).unwrap();
}

#[test]
fn an_edit_after_a_save_drops_the_saved_message() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    click(
        &ctx,
        rect_of(&ctx, egui::Id::new("save button")).center(),
        &mut panel,
        &mut game,
    );
    assert!(matches!(panel.status, Status::Saved(_)));
    type_into(&ctx, Field::JumpSpeed, "500", &mut panel, &mut game);
    assert_eq!(panel.status, Status::Quiet);
    assert!(game.tuning_unsaved());
}

#[test]
fn the_wheel_scrolls_at_the_same_rate_as_the_room_editor() {
    let ctx = egui::Context::default();
    let mut seen = egui::Vec2::ZERO;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        scroll_from_wheel(ctx, (0.0, 1.0));
        seen = ctx.input(|input| input.raw_scroll_delta);
    });
    assert_eq!(seen, egui::vec2(0.0, 28.0));
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        scroll_from_wheel(ctx, (100.0, -100.0));
        seen = ctx.input(|input| input.raw_scroll_delta);
    });
    assert_eq!(seen, egui::vec2(84.0, -84.0), "clamped to three steps");
}

fn vitals_rect(ctx: &egui::Context, field: VitalsField) -> egui::Rect {
    rect_of(ctx, egui::Id::new(("field", field.spec().key)))
}

/// Clicks the drag box of a vitals field, types `text` over its value and presses Enter.
fn type_into_vitals(
    ctx: &egui::Context,
    field: VitalsField,
    text: &str,
    panel: &mut TuningPanel,
    game: &mut GameState,
) {
    click(ctx, vitals_rect(ctx, field).center(), panel, game);
    frame(ctx, vec![egui::Event::Text(text.into())], panel, game);
    frame(ctx, press_enter(), panel, game);
    frame(ctx, vec![], panel, game);
}

fn click_save(ctx: &egui::Context, panel: &mut TuningPanel, game: &mut GameState) {
    click(
        ctx,
        rect_of(ctx, egui::Id::new("save button")).center(),
        panel,
        game,
    );
}

#[test]
fn the_window_lists_every_vitals_group_and_field() {
    let assets = Assets::new();
    let mut game = assets.game();
    let ctx = egui::Context::default();
    let mut panel = TuningPanel::default();
    panel.toggle();
    let mut text = String::new();
    for _ in 0..SETTLE_FRAMES {
        text = frame(&ctx, vec![], &mut panel, &mut game);
    }
    for field in VitalsField::ALL {
        let spec = field.spec();
        assert!(text.contains(spec.label), "missing {}\n{text}", spec.label);
        assert!(
            text.contains(&format!("Vitals: {}", spec.group)),
            "missing {}",
            spec.group
        );
        vitals_rect(&ctx, field);
    }
}

#[test]
fn typing_a_vitals_value_applies_at_once_and_save_writes_vitals_ron_too() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into_vitals(
        &ctx,
        VitalsField::HealMoveScale,
        "0.25",
        &mut panel,
        &mut game,
    );
    assert_eq!(game.vitals_config().heal_move_scale, 0.25);
    assert_eq!(panel.status, Status::Quiet);
    assert!(game.vitals_unsaved());
    assert!(!game.tuning_unsaved(), "player.ron values did not change");
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(text.contains("Unsaved changes"), "{text}");
    assert!(!assets.0.join(VITALS_FILE_NAME).exists());

    click_save(&ctx, &mut panel, &mut game);
    let Status::Saved(paths) = &panel.status else {
        panic!("save failed: {:?}", panel.status);
    };
    assert!(
        paths.contains("player.ron") && paths.contains("vitals.ron"),
        "{paths}"
    );
    assert!(!game.vitals_unsaved());
    assert_eq!(
        VitalsConfig::load(&assets.0).unwrap(),
        *game.vitals_config()
    );
    assert_eq!(VitalsConfig::load(&assets.0).unwrap().heal_move_scale, 0.25);
    assert!(!assets.0.join("vitals.ron.tmp").exists());
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(text.contains("Matches the saved values."), "{text}");
}

#[test]
fn save_leaves_vitals_ron_alone_when_only_player_values_changed() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into(&ctx, Field::Gravity, "2000", &mut panel, &mut game);
    click_save(&ctx, &mut panel, &mut game);
    assert!(matches!(&panel.status, Status::Saved(paths) if !paths.contains("vitals")));
    assert!(assets.0.join(FILE_NAME).exists());
    assert!(!assets.0.join(VITALS_FILE_NAME).exists());
}

#[test]
fn a_refused_vitals_value_shows_its_reason_and_changes_nothing() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    let before = *game.vitals_config();
    for (field, text) in [
        (VitalsField::RegainFraction, "1.5"),
        (VitalsField::TrailDrain, "0"),
        (VitalsField::HealMoveScale, "slow"),
        // In its own range, but more than the mana cap: the whole file would be invalid.
        (VitalsField::ManaStart, "101"),
    ] {
        type_into_vitals(&ctx, field, text, &mut panel, &mut game);
        let Status::Error(message) = panel.status.clone() else {
            panic!("{text} for {field:?} was not refused: {:?}", panel.status);
        };
        assert!(message.contains(field.spec().key), "{message}");
        assert_eq!(*game.vitals_config(), before, "{text} changed the game");
        let shown = frame(&ctx, vec![], &mut panel, &mut game);
        assert!(shown.contains(&message), "{shown}");
    }
    assert!(!game.vitals_unsaved());
}

#[test]
fn a_failed_vitals_save_names_the_file_and_keeps_the_unsaved_note() {
    let assets = Assets::new();
    let mut game = assets.game();
    let (ctx, mut panel) = open_panel(&mut game);
    type_into_vitals(&ctx, VitalsField::TrailDrain, "25", &mut panel, &mut game);
    std::fs::create_dir(assets.0.join("vitals.ron.tmp")).unwrap();
    click_save(&ctx, &mut panel, &mut game);
    let Status::Error(message) = &panel.status else {
        panic!("save should have failed: {:?}", panel.status);
    };
    assert!(message.contains("vitals.ron"), "{message}");
    assert!(game.vitals_unsaved());
    assert!(!assets.0.join(VITALS_FILE_NAME).exists());
    let text = frame(&ctx, vec![], &mut panel, &mut game);
    assert!(text.contains("Unsaved changes"), "{text}");
    std::fs::remove_dir(assets.0.join("vitals.ron.tmp")).unwrap();
}
