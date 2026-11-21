use super::*;
use hollow_steps::charms::Effect;
use hollow_steps::upgrade_editor::UpgradeDraft;
use hollow_steps::upgrades::UpgradeCatalog;

/// The editor's side panel is this wide; every row has to fit in it.
const PANEL_WIDTH: f32 = 360.0;
/// Tall enough that every row is drawn and clickable without scrolling.
const TALL: f32 = 12_000.0;

/// Three upgrades written for the tests (never the shipped numbers, which the owner tunes).
const SAMPLE: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "More health.",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "Faster.",
     effect: MoveSpeedPercent(5.0), costs: [40, 80, 140]),
    (id: "keen_edge", name: "Keen Edge", description: "Sharper.",
     effect: SwordDamage(3), costs: [50, 100, 180]),
])"#;

fn sample() -> UpgradeDraft {
    UpgradeDraft::from_catalog(&UpgradeCatalog::from_ron(SAMPLE).unwrap())
}

/// A game with its editor open, and the sample list in the editor.
fn editing_game() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    game.editor_mut().unwrap().upgrades = sample();
    game
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

/// One egui pass over the panel; returns what was drawn and whether the panel was shown.
fn frame_shown(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
    others_open: (&mut bool, &mut bool),
) -> (String, bool) {
    let mut shown = false;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(PANEL_WIDTH, TALL),
            )),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                shown = draw(ui, editor, state, others_open.0, others_open.1);
            });
        },
    );
    (drawn_text(&output), shown)
}

fn frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut UpgradeUi,
) -> String {
    frame_shown(ctx, events, editor, state, (&mut false, &mut false)).0
}

/// An open panel, drawn a few times so every row has been laid out.
fn open_panel(editor: &mut EditorSession) -> (egui::Context, UpgradeUi) {
    let ctx = egui::Context::default();
    let mut state = UpgradeUi {
        open: true,
        ..Default::default()
    };
    for _ in 0..3 {
        frame(&ctx, vec![], editor, &mut state);
    }
    (ctx, state)
}

/// The same, with the editor holding only its first upgrade, so each probe names one widget.
fn open_with_one_upgrade(editor: &mut EditorSession) -> (egui::Context, UpgradeUi) {
    let first = editor.upgrades.edited()[0].clone();
    editor.upgrades = UpgradeDraft::new(vec![first]);
    open_panel(editor)
}

fn rect_of(ctx: &egui::Context, id: egui::Id) -> egui::Rect {
    ctx.data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"))
}

fn button_id(name: &str) -> egui::Id {
    egui::Id::new(("button", name))
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

fn click(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut UpgradeUi) {
    let at = rect_of(ctx, id).center();
    frame(ctx, pointer(at, Some(true)), editor, state);
    frame(ctx, pointer(at, Some(false)), editor, state);
    frame(ctx, vec![], editor, state);
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

/// The drag box of the price of `level` (1 is the first).
fn price_id(level: usize) -> egui::Id {
    egui::Id::new(("field", format!("costs.{level}").as_str()))
}

fn id_of(key: &'static str) -> egui::Id {
    egui::Id::new(("field", key))
}

#[test]
fn a_closed_panel_draws_only_its_button_and_the_button_opens_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = UpgradeUi::default();
    let (text, shown) = frame_shown(&ctx, vec![], editor, &mut state, (&mut false, &mut false));
    assert!(!shown);
    assert!(
        text.contains("Upgrades") && !text.contains("Save upgrades"),
        "{text}"
    );
    click(&ctx, button_id("Upgrades"), editor, &mut state);
    assert!(state.open);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(
        text.contains("Save upgrades") && text.contains("Close upgrades"),
        "{text}"
    );
    click(&ctx, button_id("Close upgrades"), editor, &mut state);
    assert!(!state.open);
}

#[test]
fn only_one_of_the_three_panels_is_open_and_the_last_one_opened_wins() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    // The Charms or the Player panel is open: the Upgrades panel is not, only its button shows.
    for (mut charms, mut player) in [(true, false), (false, true)] {
        let ctx = egui::Context::default();
        let mut state = UpgradeUi {
            open: true,
            ..Default::default()
        };
        for _ in 0..3 {
            let (_, shown) =
                frame_shown(&ctx, vec![], editor, &mut state, (&mut charms, &mut player));
            assert!(!shown, "another panel is open");
        }
        assert!(!state.open);
        // Pressing the button opens it and closes the other one.
        let at = rect_of(&ctx, button_id("Upgrades")).center();
        for events in [pointer(at, Some(true)), pointer(at, Some(false))] {
            frame_shown(&ctx, events, editor, &mut state, (&mut charms, &mut player));
        }
        assert!(state.open && !charms && !player);
    }
}

#[test]
fn every_upgrade_is_listed_with_every_field() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = UpgradeUi {
        open: true,
        ..Default::default()
    };
    let mut text = String::new();
    for _ in 0..3 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    for upgrade in editor.upgrades.edited() {
        assert!(
            text.contains(&upgrade.name),
            "{} missing in {text}",
            upgrade.name
        );
        assert!(text.contains(&upgrade.id), "{} missing", upgrade.id);
    }
    for field in FIELDS {
        assert!(
            text.contains(field.label),
            "{} missing in {text}",
            field.label
        );
    }
    assert!(
        text.contains("Per level") && text.contains("Max health"),
        "{text}"
    );
    for level in 1..=3 {
        assert!(text.contains(&format!("Level {level}")), "{text}");
    }
    assert!(
        !text.contains("Level 4"),
        "only the levels there are: {text}"
    );
    for button in [
        "New upgrade",
        "Save upgrades",
        "Undo upgrade edit",
        "Redo upgrade edit",
        "Reset all",
        "Add level",
        "Remove level",
        "Delete upgrade",
        "Reset upgrade",
    ] {
        assert!(text.contains(button), "{button} missing");
    }
    assert!(text.contains("Matches the saved upgrades."), "{text}");
    assert!(
        text.contains("All upgrades at maximum: +30 max health, +15% move speed, +9 sword damage"),
        "{text}"
    );
}

#[test]
fn dragging_a_price_edits_the_draft_and_the_drag_is_one_undo_step() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_with_one_upgrade(editor);
    let saved = editor.upgrades.edited()[0].costs[0];
    let start = rect_of(&ctx, price_id(1)).center();
    frame(&ctx, pointer(start, Some(true)), editor, &mut state);
    for dx in [10.0, 20.0, 30.0] {
        frame(
            &ctx,
            pointer(start + egui::vec2(dx, 0.0), None),
            editor,
            &mut state,
        );
    }
    assert!(state.history.gesture_open());
    frame(
        &ctx,
        pointer(start + egui::vec2(30.0, 0.0), Some(false)),
        editor,
        &mut state,
    );
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(editor.upgrades.edited()[0].costs[0] > saved);
    assert!(
        editor.upgrades.is_dirty() && text.contains("Unsaved changes."),
        "{text}"
    );
    assert_eq!(state.history.undo_steps(), 1, "the whole drag is one step");
    assert!(state.focused);
    assert!(state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(editor.upgrades.edited()[0].costs[0], saved);
    assert!(!editor.upgrades.is_dirty());
    assert!(state.apply_history(editor, HistoryKey::Redo));
    assert!(editor.upgrades.edited()[0].costs[0] > saved);
}

#[test]
fn dragging_the_amount_edits_the_effect_in_the_range_of_its_kind() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_with_one_upgrade(editor);
    let start = rect_of(&ctx, id_of("effect.MaxHealth")).center();
    frame(&ctx, pointer(start, Some(true)), editor, &mut state);
    frame(
        &ctx,
        pointer(start + egui::vec2(15.0, 0.0), None),
        editor,
        &mut state,
    );
    frame(
        &ctx,
        pointer(start + egui::vec2(15.0, 0.0), Some(false)),
        editor,
        &mut state,
    );
    frame(&ctx, vec![], editor, &mut state);
    let Effect::MaxHealth(points) = editor.upgrades.edited()[0].effect else {
        panic!("still a health upgrade");
    };
    assert!(points > 10, "{points}");
    assert_eq!(state.history.undo_steps(), 1);
}

#[test]
fn a_typed_price_outside_the_range_is_refused_with_the_reason() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_with_one_upgrade(editor);
    let saved = editor.upgrades.edited()[0].costs[0];
    click(&ctx, price_id(1), editor, &mut state);
    frame(
        &ctx,
        vec![egui::Event::Text("0".into())],
        editor,
        &mut state,
    );
    frame(&ctx, press_enter(), editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert_eq!(editor.upgrades.edited()[0].costs[0], saved, "{text}");
    assert!(
        text.contains("costs.1 must be between 1 and 1000000 coins"),
        "{text}"
    );
    // A good number typed afterwards is taken, and the message goes.
    click(&ctx, price_id(1), editor, &mut state);
    frame(
        &ctx,
        vec![egui::Event::Text("45".into())],
        editor,
        &mut state,
    );
    frame(&ctx, press_enter(), editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert_eq!(editor.upgrades.edited()[0].costs[0], 45, "{text}");
    assert!(!text.contains("must be between"), "{text}");
}

#[test]
fn a_problem_the_draft_holds_is_written_next_to_its_field() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    editor.upgrades.set_name(0, "").unwrap();
    editor.upgrades.set_name(1, "Keen Edge").unwrap();
    editor.upgrades.set_description(2, "two\nlines").unwrap();
    let mut upgrades = editor.upgrades.edited().to_vec();
    upgrades[1].costs.clear();
    editor.upgrades.set_edited(upgrades);
    let mut text = String::new();
    for _ in 0..2 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    assert!(
        text.contains("the name must be 1 to 32 characters"),
        "{text}"
    );
    assert!(text.contains("is already called \"Keen Edge\""), "{text}");
    assert!(
        text.contains("the description must be one line of at most 200"),
        "{text}"
    );
    assert!(
        text.contains("an upgrade needs 1 to 10 levels, not 0"),
        "{text}"
    );
}

#[test]
fn drawing_the_panel_changes_nothing_even_at_the_edges_of_every_range() {
    // A list at the ends of the price and amount ranges is shown as it is: the drag boxes must not
    // clamp or round it, or opening the panel would make the draft dirty without any edit.
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let mut upgrades = editor.upgrades.edited().to_vec();
    upgrades[0].costs = vec![1, 1_000_000];
    upgrades[1].effect = Effect::MoveSpeedPercent(200.0);
    upgrades[2].effect = Effect::SwordDamage(1000);
    editor.upgrades = UpgradeDraft::new(upgrades);
    let (ctx, mut state) = open_panel(editor);
    for _ in 0..3 {
        frame(&ctx, vec![], editor, &mut state);
    }
    assert!(!editor.upgrades.is_dirty());
    assert_eq!(state.history.undo_steps(), 0);
}

#[test]
fn new_delete_and_reset_buttons_edit_the_list() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let count = editor.upgrades.edited().len();
    click(&ctx, button_id("New upgrade"), editor, &mut state);
    assert_eq!(editor.upgrades.edited().len(), count + 1);
    assert_eq!(editor.upgrades.edited()[count].name, "Upgrade 1");
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Upgrade 1"), "{text}");
    // Delete the new upgrade; the saved ones stay.
    click(
        &ctx,
        egui::Id::new(("upgrade delete", "upgrade_1")),
        editor,
        &mut state,
    );
    assert_eq!(editor.upgrades.edited().len(), count);
    click(
        &ctx,
        egui::Id::new(("upgrade delete", "swift_feet")),
        editor,
        &mut state,
    );
    assert_eq!(editor.upgrades.deleted_ids(), vec!["swift_feet"]);
    // Reset all brings it back and clears the unsaved note.
    click(&ctx, button_id("Reset all"), editor, &mut state);
    assert!(!editor.upgrades.is_dirty());
    // Reset upgrade puts one edited upgrade back and is off for a clean one.
    editor.upgrades.set_cost(0, 0, 99.0).unwrap();
    editor.upgrades.set_cost(2, 0, 99.0).unwrap();
    for _ in 0..2 {
        frame(&ctx, vec![], editor, &mut state);
    }
    click(
        &ctx,
        egui::Id::new(("upgrade reset", "vitality")),
        editor,
        &mut state,
    );
    assert_eq!(editor.upgrades.edited()[0].costs[0], 30);
    assert_eq!(editor.upgrades.edited()[2].costs[0], 99);
}

#[test]
fn add_level_and_remove_level_change_the_number_of_prices_inside_one_to_ten() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_with_one_upgrade(editor);
    let add = egui::Id::new(("upgrade add level", "vitality"));
    let remove = egui::Id::new(("upgrade remove level", "vitality"));
    click(&ctx, add, editor, &mut state);
    assert_eq!(editor.upgrades.edited()[0].costs, [30, 60, 100, 100]);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Level 4"), "{text}");
    assert_eq!(state.history.undo_steps(), 1);
    // Up to the ten the table has; a click on the disabled button does nothing.
    for _ in 0..6 {
        click(&ctx, add, editor, &mut state);
    }
    assert_eq!(editor.upgrades.edited()[0].costs.len(), 10);
    click(&ctx, add, editor, &mut state);
    assert_eq!(editor.upgrades.edited()[0].costs.len(), 10);
    // And down to one.
    for _ in 0..12 {
        click(&ctx, remove, editor, &mut state);
    }
    assert_eq!(editor.upgrades.edited()[0].costs, [30]);
    assert!(editor.upgrades.problems().is_empty());
}

#[test]
fn undo_and_redo_buttons_use_the_panels_own_history() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let count = editor.upgrades.edited().len();
    click(&ctx, button_id("New upgrade"), editor, &mut state);
    assert_eq!(editor.upgrades.edited().len(), count + 1);
    click(&ctx, button_id("Undo upgrade edit"), editor, &mut state);
    assert_eq!(editor.upgrades.edited().len(), count);
    click(&ctx, button_id("Redo upgrade edit"), editor, &mut state);
    assert_eq!(editor.upgrades.edited().len(), count + 1);
}

#[test]
fn the_undo_keys_belong_to_the_panel_only_after_a_press_in_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    assert!(
        !state.apply_history(editor, HistoryKey::Undo),
        "nothing pressed in the panel yet"
    );
    let count = editor.upgrades.edited().len();
    click(&ctx, button_id("New upgrade"), editor, &mut state);
    assert!(state.focused);
    assert!(state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(editor.upgrades.edited().len(), count);
    assert!(
        state.apply_history(editor, HistoryKey::Redo),
        "Ctrl+Y is the panel's too"
    );
    assert_eq!(editor.upgrades.edited().len(), count + 1);
    // A press outside the panel (on the canvas) gives the keys back; the panel's edit stays.
    let outside = egui::pos2(PANEL_WIDTH + 200.0, 50.0);
    frame(&ctx, pointer(outside, Some(true)), editor, &mut state);
    frame(&ctx, pointer(outside, Some(false)), editor, &mut state);
    assert!(!state.focused && !state.apply_history(editor, HistoryKey::Redo));
}

#[test]
fn closing_the_panel_gives_the_undo_keys_back_until_the_next_press_inside_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    click(&ctx, button_id("New upgrade"), editor, &mut state);
    assert!(state.focused);
    click(&ctx, button_id("Close upgrades"), editor, &mut state);
    assert!(!state.open && !state.focused);
    assert!(
        !state.apply_history(editor, HistoryKey::Undo),
        "closed: not the panel's keys"
    );
    // Opened again, the keys stay the room's until something in the panel is pressed.
    click(&ctx, button_id("Upgrades"), editor, &mut state);
    assert!(state.open && !state.apply_history(editor, HistoryKey::Undo));
    click(&ctx, button_id("Reset all"), editor, &mut state);
    assert!(
        state.apply_history(editor, HistoryKey::Undo),
        "the press made it the panel's again"
    );
}

#[test]
fn undo_steps_survive_closing_and_reopening_the_panel() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let count = editor.upgrades.edited().len();
    click(&ctx, button_id("New upgrade"), editor, &mut state);
    click(&ctx, button_id("Close upgrades"), editor, &mut state);
    click(&ctx, button_id("Upgrades"), editor, &mut state);
    click(&ctx, button_id("Undo upgrade edit"), editor, &mut state);
    assert_eq!(editor.upgrades.edited().len(), count);
}

#[test]
fn the_save_button_asks_for_a_save_and_the_status_is_shown() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let at = rect_of(&ctx, button_id("Save upgrades")).center();
    frame(&ctx, pointer(at, Some(true)), editor, &mut state);
    frame(&ctx, pointer(at, Some(false)), editor, &mut state);
    assert!(std::mem::take(&mut state.save_requested));
    state.status = Status::Error("upgrade \"x\": broken".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("upgrade \"x\": broken"), "{text}");
    state.status = Status::Saved("/tmp/upgrades.ron".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Saved to /tmp/upgrades.ron"), "{text}");
}

#[test]
fn finish_frame_takes_the_edited_list_into_effect_and_saves_when_asked() {
    let assets =
        std::env::temp_dir().join(format!("hollow-steps-upgrade-panel-{}", std::process::id()));
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    std::fs::create_dir_all(&assets).unwrap();
    copy_dir(&fixtures, &assets);
    std::fs::write(assets.join("upgrades.ron"), SAMPLE).unwrap();
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
    let mut state = UpgradeUi::default();
    game.editor_mut()
        .unwrap()
        .upgrades
        .set_cost(0, 0, 77.0)
        .unwrap();
    state.finish_frame(&mut game);
    assert_eq!(
        game.upgrade_catalog().upgrades()[0].costs[0],
        77,
        "in effect at once"
    );
    assert_eq!(
        std::fs::read_to_string(assets.join("upgrades.ron")).unwrap(),
        SAMPLE,
        "nothing written until Save"
    );
    state.save_requested = true;
    state.finish_frame(&mut game);
    assert!(
        matches!(&state.status, Status::Saved(path) if path.ends_with("upgrades.ron")),
        "{:?}",
        state.status
    );
    assert!(std::fs::read_to_string(assets.join("upgrades.ron"))
        .unwrap()
        .contains("77"));
    assert!(!game.upgrades_unsaved());
    // An invalid list is an error in the status and writes nothing.
    game.editor_mut().unwrap().upgrades.set_name(0, "").unwrap();
    let written = std::fs::read_to_string(assets.join("upgrades.ron")).unwrap();
    state.save_requested = true;
    state.finish_frame(&mut game);
    assert!(matches!(&state.status, Status::Error(error) if error.contains("name")));
    assert_eq!(
        std::fs::read_to_string(assets.join("upgrades.ron")).unwrap(),
        written
    );
    std::fs::remove_dir_all(&assets).unwrap();
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}
