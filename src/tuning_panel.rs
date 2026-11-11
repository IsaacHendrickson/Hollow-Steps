//! The F4 player-tuning window: a drag box for every number in `player.ron`.
//!
//! Presentation only. The ranges, validation and file writing live in `hollow_steps::tuning`
//! and `GameState`; this module turns widget changes into `GameState::set_tuning` calls, so
//! a change is in effect on the next tick, and shows why a typed value was refused.
use std::cell::RefCell;

use egui_macroquad::egui;
use hollow_steps::game::GameState;
use hollow_steps::tuning::{Field, FieldSpec};

/// What the footer says about the last Save or refused edit.
#[derive(Clone, Debug, Default, PartialEq)]
enum Status {
    #[default]
    Quiet,
    Saved(String),
    Error(String),
}

/// State of the F4 window; the values themselves live in `GameState`.
#[derive(Debug, Default)]
pub struct TuningPanel {
    open: bool,
    status: Status,
}

/// Pixels of label column, so the drag boxes line up.
const LABEL_WIDTH: f32 = 190.0;

impl TuningPanel {
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Draws the window while it is open and applies any edit or Save to `game`.
    pub fn draw(&mut self, ctx: &egui::Context, game: &mut GameState) {
        if !self.open {
            return;
        }
        let Self { open, status } = self;
        let list_height = (ctx.screen_rect().height() - 190.0).max(120.0);
        egui::Window::new("Player tuning (F4)")
            .open(open)
            .default_pos([16.0, 16.0])
            .resizable(false)
            .show(ctx, |ui| {
                ui.spacing_mut().interact_size.y = 20.0;
                egui::ScrollArea::vertical()
                    .id_salt("tuning fields")
                    .max_height(list_height)
                    .auto_shrink([true, true])
                    .show(ui, |ui| {
                        let mut group = "";
                        for field in Field::ALL {
                            if field.spec().group != group {
                                group = field.spec().group;
                                ui.add_space(4.0);
                                ui.strong(group);
                            }
                            field_row(ui, game, status, field);
                        }
                    });
                ui.separator();
                footer(ui, game, status);
            });
    }
}

fn field_row(ui: &mut egui::Ui, game: &mut GameState, status: &mut Status, field: Field) {
    let spec = field.spec();
    let mut value = field.get(game.tuning());
    let rejected = RefCell::new(None);
    ui.horizontal(|ui| {
        ui.add_sized([LABEL_WIDTH, 20.0], egui::Label::new(spec.label));
        let mut drag = egui::DragValue::new(&mut value)
            .range(spec.min..=spec.max)
            .speed(spec.step)
            .fixed_decimals(spec.decimals)
            // Judge a typed number when it is entered, not on every key press.
            .update_while_editing(false)
            .custom_parser(|text| parse_entry(spec, text, &rejected));
        if !spec.unit.is_empty() {
            drag = drag.suffix(format!(" {}", spec.unit));
        }
        let response = ui.add(drag).on_hover_text(format!(
            "{}: {} to {}{}",
            spec.key,
            spec.min,
            spec.max,
            if spec.unit.is_empty() {
                String::new()
            } else {
                format!(" {}", spec.unit)
            }
        ));
        #[cfg(test)]
        ui.data_mut(|data| data.insert_temp(egui::Id::new(("field", spec.key)), response.rect));
        if response.changed() {
            let applied = game
                .tuning()
                .with_field(field, value)
                .and_then(|changed| game.set_tuning(changed));
            *status = match applied {
                Ok(()) => Status::Quiet,
                Err(error) => Status::Error(error),
            };
        }
    });
    if let Some(message) = rejected.into_inner() {
        *status = Status::Error(message);
    }
}

fn footer(ui: &mut egui::Ui, game: &mut GameState, status: &mut Status) {
    let save = ui.button("Save");
    #[cfg(test)]
    ui.data_mut(|data| data.insert_temp(egui::Id::new("save button"), save.rect));
    if save.clicked() {
        *status = match game.save_tuning() {
            Ok(path) => Status::Saved(path.display().to_string()),
            Err(error) => Status::Error(error),
        };
    }
    ui.label(if game.tuning_unsaved() {
        "Unsaved changes. F5 reloads the saved values."
    } else {
        "Matches the saved values."
    });
    match status {
        Status::Quiet => {}
        Status::Saved(path) => {
            ui.label(format!("Saved to {path}"));
        }
        Status::Error(message) => {
            ui.colored_label(egui::Color32::LIGHT_RED, message.as_str());
        }
    }
    ui.small("Changes apply at once. Save writes player.ron in the assets folder.");
}

/// Reads typed text for one field. Anything outside the field's range, or not a number, is
/// refused: the drag box keeps its value and `rejected` gets the reason for the footer.
fn parse_entry(spec: &FieldSpec, text: &str, rejected: &RefCell<Option<String>>) -> Option<f64> {
    let text = text.trim();
    let number = if spec.unit.is_empty() {
        text
    } else {
        text.strip_suffix(spec.unit).unwrap_or(text).trim()
    };
    let result = number
        .replace(',', ".")
        .parse::<f64>()
        .map_err(|_| format!("{}: \"{text}\" is not a number", spec.key))
        .and_then(|value| spec.check(value as f32))
        .map(f64::from);
    match result {
        Ok(value) => Some(value),
        Err(message) => {
            *rejected.borrow_mut() = Some(message);
            None
        }
    }
}

/// egui gets the wheel delta at 28 points per step, the same as the room editor, since the
/// integration's own scaling is too small to scroll the field list.
pub fn scroll_from_wheel(ctx: &egui::Context, wheel: (f32, f32)) {
    let delta = egui::vec2(wheel.0.clamp(-3.0, 3.0), wheel.1.clamp(-3.0, 3.0)) * 28.0;
    ctx.input_mut(|input| {
        input.raw_scroll_delta = delta;
        input.smooth_scroll_delta = delta;
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use hollow_steps::tuning::{TuningConfig, FILE_NAME};
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
                    std::fs::copy(entry.path(), path.join(directory).join(entry.file_name()))
                        .unwrap();
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
        for _ in 0..3 {
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
        for _ in 0..3 {
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
        assert_eq!(parse_entry(health, "100", &rejected), None);
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
}
