use super::*;
use hollow_steps::game::GameState;

/// The editor's side panel is this wide; every row has to fit in it.
const PANEL_WIDTH: f32 = 360.0;
/// Tall enough that every row is drawn and clickable without scrolling.
const TALL: f32 = 1_800.0;

/// The fixture game with its editor open.
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

/// One egui pass over the panel in the editor's narrow side panel. Also returns whether the
/// panel was shown.
fn frame_shown(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut PlayerUi,
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
                shown = draw(ui, editor, state);
            });
        },
    );
    (drawn_text(&output), shown)
}

fn frame(
    ctx: &egui::Context,
    events: Vec<egui::Event>,
    editor: &mut EditorSession,
    state: &mut PlayerUi,
) -> String {
    frame_shown(ctx, events, editor, state).0
}

/// An open panel, drawn a few times so every row has been laid out.
fn open_panel(editor: &mut EditorSession) -> (egui::Context, PlayerUi) {
    let ctx = egui::Context::default();
    let mut state = PlayerUi {
        open: true,
        ..Default::default()
    };
    for _ in 0..3 {
        frame(&ctx, vec![], editor, &mut state);
    }
    (ctx, state)
}

fn rect_of(ctx: &egui::Context, id: egui::Id) -> egui::Rect {
    ctx.data(|data| data.get_temp(id))
        .unwrap_or_else(|| panic!("nothing drawn for {id:?}"))
}

fn field_id(field: Field) -> egui::Id {
    egui::Id::new(("field", field.spec().key))
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

fn click(ctx: &egui::Context, id: egui::Id, editor: &mut EditorSession, state: &mut PlayerUi) {
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

/// Clicks a drag box, types `text` over its value and presses Enter.
fn type_into(
    ctx: &egui::Context,
    field: Field,
    text: &str,
    editor: &mut EditorSession,
    state: &mut PlayerUi,
) {
    click(ctx, field_id(field), editor, state);
    frame(ctx, vec![egui::Event::Text(text.into())], editor, state);
    frame(ctx, press_enter(), editor, state);
    frame(ctx, vec![], editor, state);
}

#[test]
fn every_field_is_in_exactly_one_section() {
    let listed: Vec<Field> = SECTIONS
        .iter()
        .flat_map(|(_, fields)| fields.iter().copied())
        .collect();
    for field in Field::ALL {
        let count = listed.iter().filter(|listed| **listed == field).count();
        assert_eq!(count, 1, "{field:?} is in {count} sections");
    }
    assert_eq!(listed.len(), Field::ALL.len());
    for (name, fields) in SECTIONS {
        assert!(!fields.is_empty(), "{name} is empty");
    }
}

#[test]
fn a_closed_panel_draws_only_its_button_and_the_button_opens_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let ctx = egui::Context::default();
    let mut state = PlayerUi::default();
    let (text, shown) = frame_shown(&ctx, vec![], editor, &mut state);
    assert!(!shown);
    assert!(text.contains("Player"), "{text}");
    assert!(!text.contains("Movement"), "{text}");
    assert!(!text.contains("Save player"), "{text}");
    let no_field: Option<egui::Rect> = ctx.data(|data| data.get_temp(field_id(Field::JumpSpeed)));
    assert!(no_field.is_none());

    click(
        &ctx,
        egui::Id::new(("button", "Player")),
        editor,
        &mut state,
    );
    let (text, shown) = frame_shown(&ctx, vec![], editor, &mut state);
    assert!(state.open && shown);
    assert!(text.contains("Movement"), "{text}");
    assert!(text.contains("Close player"), "{text}");

    click(
        &ctx,
        egui::Id::new(("button", "Close player")),
        editor,
        &mut state,
    );
    assert!(!state.open);
}

#[test]
fn the_open_panel_shows_every_section_field_and_button_inside_the_narrow_panel() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let text = frame(&ctx, vec![], editor, &mut state);
    for (name, fields) in SECTIONS {
        assert!(text.contains(name), "missing section {name}\n{text}");
        // `section` records the header's id (not a rectangle) under the section's name.
        let header: Option<egui::Id> =
            ctx.data(|data| data.get_temp(egui::Id::new(("section", name))));
        assert!(header.is_some(), "section {name} was not drawn");
        for &field in fields {
            let spec = field.spec();
            assert!(text.contains(spec.label), "missing {}\n{text}", spec.label);
            let rect = rect_of(&ctx, field_id(field));
            assert!(rect.width() > 20.0, "{} is squeezed: {rect:?}", spec.key);
            assert!(rect.right() <= PANEL_WIDTH, "{} sticks out", spec.key);
            rect_of(&ctx, egui::Id::new(("reset", spec.key)));
        }
    }
    for expected in ["Save player", "Reset all", "Matches the saved values."] {
        assert!(text.contains(expected), "missing {expected}\n{text}");
    }
    assert!(!text.contains("Unsaved"), "{text}");
    // The unit is drawn with the number, so the author sees what the number means.
    assert!(text.contains(" px/s"), "{text}");
}

#[test]
fn dragging_a_field_edits_the_draft_and_its_reset_restores_the_saved_value() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let before = Field::MoveSpeed.get(editor.player.edited());
    let start = rect_of(&ctx, field_id(Field::MoveSpeed)).center();

    frame(&ctx, pointer(start, Some(true)), editor, &mut state);
    let moved = start + egui::vec2(30.0, 0.0);
    frame(&ctx, pointer(moved, None), editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    frame(&ctx, pointer(moved, Some(false)), editor, &mut state);
    assert!(Field::MoveSpeed.get(editor.player.edited()) > before);
    assert!(editor.player.is_dirty());
    assert!(editor.player.is_field_dirty(Field::MoveSpeed));
    assert!(!editor.player.is_field_dirty(Field::JumpSpeed));
    assert!(text.contains("Unsaved changes."), "{text}");

    click(
        &ctx,
        egui::Id::new(("reset", Field::MoveSpeed.spec().key)),
        editor,
        &mut state,
    );
    assert!(!editor.player.is_dirty());
    assert_eq!(Field::MoveSpeed.get(editor.player.edited()), before);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Matches the saved values."), "{text}");
}

#[test]
fn reset_all_puts_every_edited_field_back() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    editor.player.set_field(Field::JumpSpeed, 520.0).unwrap();
    editor.player.set_field(Field::DashSpeed, 900.0).unwrap();
    editor.player.set_field(Field::Height, 40.0).unwrap();
    assert!(editor.player.is_dirty());
    click(
        &ctx,
        egui::Id::new(("button", "Reset all")),
        editor,
        &mut state,
    );
    assert!(!editor.player.is_dirty());
}

#[test]
fn a_typed_number_in_range_is_taken_and_one_out_of_range_is_refused_with_a_message() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let before = *editor.player.edited();

    for (field, text) in [
        (Field::MoveSpeed, "99999"),
        (Field::Gravity, "-4"),
        (Field::DashDuration, "0"),
        (Field::JumpSpeed, "fast"),
    ] {
        type_into(&ctx, field, text, editor, &mut state);
        let Some((refused, message)) = state.refused.clone() else {
            panic!("{text} for {field:?} was not refused");
        };
        assert_eq!(refused, field);
        assert!(message.contains(field.spec().key), "{message}");
        assert_eq!(*editor.player.edited(), before, "{text} changed the draft");
        assert!(!editor.player.is_dirty());
        let shown = frame(&ctx, vec![], editor, &mut state);
        assert!(shown.contains(&message), "{shown}");
    }

    type_into(&ctx, Field::MoveSpeed, "300", editor, &mut state);
    assert_eq!(editor.player.edited().player.move_speed, 300.0);
    assert!(state.refused.is_none());
    assert_eq!(state.status, Status::Quiet);
}

#[test]
fn a_rule_that_spans_fields_shows_its_message_by_each_field_it_names() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    editor
        .player
        .set_field(Field::HeldJumpGravityMultiplier, 0.0)
        .unwrap();
    let message = editor.player.problems(&[])[0].message.clone();
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains(&message), "{text}");
    // The same sentence is drawn once under each of the three fields that cause it.
    assert_eq!(text.matches(message.as_str()).count(), 3, "{text}");
}

#[test]
fn a_size_that_starts_inside_a_room_solid_is_shown_with_the_room_name() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    // A solid close beside the spawn, in the draft being edited (nothing is saved).
    let (x, y) = editor.draft.layout.spawn;
    editor
        .draft
        .layout
        .terrain
        .push((x + 20.0, y - 100.0, 40.0, 200.0));
    editor.player.set_field(Field::Width, 64.0).unwrap();
    let problems = editor.player.problems(&editor.spawn_checks());
    assert_eq!(problems.len(), 1, "{problems:?}");
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains(&problems[0].message), "{text}");
    // It is the editing panel's job to show it; Save is still the game's to refuse.
    assert!(editor.player.validate(&editor.spawn_checks()).is_err());
}

#[test]
fn the_save_button_asks_for_a_save_and_changes_nothing_itself() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    editor.player.set_field(Field::MoveSpeed, 300.0).unwrap();
    let draft = editor.player;
    assert!(!state.save_requested);
    click(
        &ctx,
        egui::Id::new(("button", "Save player")),
        editor,
        &mut state,
    );
    assert!(state.save_requested);
    assert_eq!(editor.player, draft);
}

#[test]
fn an_error_from_the_last_save_is_drawn_and_an_edit_clears_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    state.status = Status::Error("player.ron: the disk is full".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("the disk is full"), "{text}");
    type_into(&ctx, Field::MoveSpeed, "300", editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(!text.contains("the disk is full"), "{text}");

    state.status = Status::Saved("assets/player.ron".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Saved to assets/player.ron"), "{text}");
}
