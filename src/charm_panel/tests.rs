use super::*;
use hollow_steps::charm_editor::CharmDraft;
use hollow_steps::game::GameState;

/// The editor's side panel is this wide; every row has to fit in it.
const PANEL_WIDTH: f32 = 360.0;
/// Tall enough that every row is drawn and clickable without scrolling.
const TALL: f32 = 12_000.0;

fn editing_game() -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.toggle_editor().unwrap();
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
    state: &mut CharmUi,
    player_open: &mut bool,
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
                shown = draw(ui, editor, state, player_open);
            });
        },
    );
    (drawn_text(&output), shown)
}

fn frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut CharmUi,
) -> String {
    frame_shown(ctx, events, editor, state, &mut false).0
}

/// An open panel, drawn a few times so every row has been laid out.
fn open_panel(editor: &mut EditorSession) -> (egui::Context, CharmUi) {
    let ctx = egui::Context::default();
    let mut state = CharmUi {
        open: true,
        ..Default::default()
    };
    for _ in 0..3 {
        frame(&ctx, vec![], editor, &mut state);
    }
    (ctx, state)
}

/// The same, with the editor holding only its first charm, so each probe names one widget.
fn open_with_one_charm(editor: &mut EditorSession) -> (egui::Context, CharmUi) {
    let first = editor.charms.edited()[0].clone();
    editor.charms = CharmDraft::new(vec![first]);
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

fn click(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut CharmUi) {
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

fn cost_id() -> egui::Id {
    egui::Id::new(("field", "mana_cost"))
}

#[test]
fn a_closed_panel_draws_only_its_button_and_the_button_opens_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = CharmUi::default();
    let (text, shown) = frame_shown(&ctx, vec![], editor, &mut state, &mut false);
    assert!(!shown);
    assert!(
        text.contains("Charms") && !text.contains("Save charms"),
        "{text}"
    );
    click(&ctx, button_id("Charms"), editor, &mut state);
    assert!(state.open);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(
        text.contains("Save charms") && text.contains("Close charms"),
        "{text}"
    );
    click(&ctx, button_id("Close charms"), editor, &mut state);
    assert!(!state.open);
}

#[test]
fn opening_charms_closes_the_player_panel() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = CharmUi::default();
    let mut player_open = true;
    for _ in 0..3 {
        let (_, shown) = frame_shown(&ctx, vec![], editor, &mut state, &mut player_open);
        assert!(
            !shown,
            "the Player panel is open: only the Charms button shows"
        );
    }
    let at = rect_of(&ctx, button_id("Charms")).center();
    for events in [pointer(at, Some(true)), pointer(at, Some(false))] {
        frame_shown(&ctx, events, editor, &mut state, &mut player_open);
    }
    assert!(state.open && !player_open);
}

#[test]
fn every_charm_is_listed_with_every_field() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = CharmUi {
        open: true,
        ..Default::default()
    };
    let mut text = String::new();
    for _ in 0..3 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    for charm in editor.charms.edited() {
        assert!(
            text.contains(&charm.name),
            "{} missing in {text}",
            charm.name
        );
        assert!(text.contains(&charm.id), "{} missing", charm.id);
    }
    for field in FIELDS {
        assert!(
            text.contains(field.label),
            "{} missing in {text}",
            field.label
        );
    }
    assert!(
        text.contains("Amount") && text.contains("Max health"),
        "{text}"
    );
    for button in [
        "New charm",
        "Save charms",
        "Undo charm edit",
        "Redo charm edit",
        "Reset all",
    ] {
        assert!(text.contains(button), "{button} missing");
    }
    assert!(text.contains("Matches the saved charms."), "{text}");
}

#[test]
fn dragging_the_mana_cost_edits_the_draft_and_the_drag_is_one_undo_step() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_with_one_charm(editor);
    let saved = editor.charms.edited()[0].mana_cost;
    let start = rect_of(&ctx, cost_id()).center();
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
    assert!(editor.charms.edited()[0].mana_cost > saved);
    assert!(
        editor.charms.is_dirty() && text.contains("Unsaved changes."),
        "{text}"
    );
    assert_eq!(state.history.undo_steps(), 1);
    assert!(state.focused);
    assert!(state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(editor.charms.edited()[0].mana_cost, saved);
    assert!(!editor.charms.is_dirty());
}

#[test]
fn a_typed_cost_above_the_base_max_mana_is_refused_with_the_reason() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_with_one_charm(editor);
    let saved = editor.charms.edited()[0].mana_cost;
    click(&ctx, cost_id(), editor, &mut state);
    frame(
        &ctx,
        vec![egui::Event::Text("250".into())],
        editor,
        &mut state,
    );
    frame(&ctx, press_enter(), editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert_eq!(editor.charms.edited()[0].mana_cost, saved, "{text}");
    assert!(
        text.contains("mana_cost must be between 0 and 100"),
        "{text}"
    );
    // A good number typed afterwards is taken, and the message goes.
    click(&ctx, cost_id(), editor, &mut state);
    frame(
        &ctx,
        vec![egui::Event::Text("40".into())],
        editor,
        &mut state,
    );
    frame(&ctx, press_enter(), editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert_eq!(editor.charms.edited()[0].mana_cost, 40.0, "{text}");
    assert!(!text.contains("mana_cost must be"), "{text}");
}

#[test]
fn a_problem_the_draft_holds_is_written_next_to_its_field() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    editor.charms.set_name(0, "").unwrap();
    editor.charms.set_name(1, "Sharp Charm").unwrap();
    editor.charms.set_color(2, [1, 2, 3, 0]).unwrap();
    let mut text = String::new();
    for _ in 0..2 {
        text = frame(&ctx, vec![], editor, &mut state);
    }
    assert!(
        text.contains("the name must be 1 to 32 characters"),
        "{text}"
    );
    assert!(text.contains("is already called \"Sharp Charm\""), "{text}");
    assert!(text.contains("must not be fully transparent"), "{text}");
}

#[test]
fn new_charm_delete_and_reset_buttons_edit_the_list() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let count = editor.charms.edited().len();
    click(&ctx, button_id("New charm"), editor, &mut state);
    assert_eq!(editor.charms.edited().len(), count + 1);
    assert_eq!(editor.charms.edited()[count].name, "Charm 1");
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Charm 1"), "{text}");
    // Delete the new charm; the saved ones stay.
    click(
        &ctx,
        egui::Id::new(("charm delete", "charm_1")),
        editor,
        &mut state,
    );
    assert_eq!(editor.charms.edited().len(), count);
    click(
        &ctx,
        egui::Id::new(("charm delete", "swift")),
        editor,
        &mut state,
    );
    assert_eq!(editor.charms.deleted_ids(), vec!["swift"]);
    // Reset all brings it back and clears the unsaved note.
    click(&ctx, button_id("Reset all"), editor, &mut state);
    assert!(!editor.charms.is_dirty());
    // Reset charm puts one edited charm back and is off for a clean one.
    editor.charms.set_mana_cost(0, 50.0).unwrap();
    editor.charms.set_mana_cost(2, 50.0).unwrap();
    for _ in 0..2 {
        frame(&ctx, vec![], editor, &mut state);
    }
    click(
        &ctx,
        egui::Id::new(("charm reset", "heart")),
        editor,
        &mut state,
    );
    assert_eq!(editor.charms.edited()[0].mana_cost, 15.0);
    assert_eq!(editor.charms.edited()[2].mana_cost, 50.0);
}

#[test]
fn undo_and_redo_buttons_use_the_panels_own_history() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let count = editor.charms.edited().len();
    click(&ctx, button_id("New charm"), editor, &mut state);
    assert_eq!(editor.charms.edited().len(), count + 1);
    click(&ctx, button_id("Undo charm edit"), editor, &mut state);
    assert_eq!(editor.charms.edited().len(), count);
    click(&ctx, button_id("Redo charm edit"), editor, &mut state);
    assert_eq!(editor.charms.edited().len(), count + 1);
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
    let count = editor.charms.edited().len();
    click(&ctx, button_id("New charm"), editor, &mut state);
    assert!(state.focused);
    assert!(state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(editor.charms.edited().len(), count);
    // A press outside the panel (on the canvas) gives the keys back; the panel's edit stays.
    let outside = egui::pos2(PANEL_WIDTH + 200.0, 50.0);
    frame(&ctx, pointer(outside, Some(true)), editor, &mut state);
    frame(&ctx, pointer(outside, Some(false)), editor, &mut state);
    assert!(!state.focused && !state.apply_history(editor, HistoryKey::Redo));
}

#[test]
fn the_save_button_asks_for_a_save_and_the_status_is_shown() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let at = rect_of(&ctx, button_id("Save charms")).center();
    frame(&ctx, pointer(at, Some(true)), editor, &mut state);
    frame(&ctx, pointer(at, Some(false)), editor, &mut state);
    assert!(std::mem::take(&mut state.save_requested));
    state.status = Status::Error("charm \"x\": broken".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("charm \"x\": broken"), "{text}");
    state.status = Status::Saved("/tmp/charms.ron".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Saved to /tmp/charms.ron"), "{text}");
}
