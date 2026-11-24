//! The Preview generated dungeon window: its button, the seed box and Roll next, the plan it
//! draws (rooms by piece name, a pickup list, a key number on a locked exit) and the reason it
//! gives when there is none. The pool is three pieces built here: Start (with a Key slot),
//! Middle and Boss.
use super::map::{laid_out, press, probed, run, text, SIZE};
use super::*;
use crate::editor_panel::map_window::draw_map_canvas;
use crate::editor_panel::test_support::drawn_text;
use hollow_steps::dungeon_map::{EditorMap, EditorMapRoom, MapLink};
use hollow_steps::dungeon_preview::{next_seed, Outcome};
use hollow_steps::editor::RoomLayout;
use hollow_steps::room::RoomId;
use hollow_steps::run::RunConfig;
use hollow_steps::seed::format_seed;
use hollow_steps::variety::generator::CountRange;
use hollow_steps::variety::GeneratorRules;

const HUB: RoomId = RoomId::Custom(0);
const START: RoomId = RoomId::Custom(1);
const MIDDLE: RoomId = RoomId::Custom(2);
const BOSS: RoomId = RoomId::Custom(3);

fn layout(name: &str, role: Option<&str>, sides: &[&str], slot: bool) -> RoomLayout {
    let exits: Vec<String> = sides
        .iter()
        .map(|side| {
            format!(
                "(edge: {side}, offset: 364.0, length: 96.0, target_room: Custom(0), \
                 spawn: (480.0, 436.0))"
            )
        })
        .collect();
    let piece = role.map_or("None".to_owned(), |role| {
        format!("Some((role: {role}, weight: 1))")
    });
    let slots = if slot {
        "(position: (180.0, 440.0), kind: Key)"
    } else {
        ""
    };
    let text = format!(
        "(name: Some({name:?}), bounds: (0.0, 0.0, 960.0, 540.0), floor_y: 460.0, \
         spawn: (480.0, 436.0), items: [], exits: [{}], terrain: [(0.0, 460.0, 960.0, 80.0)], \
         piece: {piece}, slots: [{slots}])",
        exits.join(", ")
    );
    ron::from_str(&text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn hub() -> RoomLayout {
    let mut hub = layout("Hub", None, &["Right"], false);
    hub.exits[0].target_room = RoomId::First;
    hub
}

fn rules(keys: i32) -> GeneratorRules {
    GeneratorRules {
        enabled: true,
        rooms: CountRange { min: 3, max: 3 },
        spine: CountRange { min: 3, max: 3 },
        keys: CountRange {
            min: keys,
            max: keys,
        },
        mirror_percent: 0.0,
        attempts: 100,
    }
}

/// Gives the session the three pieces as saved rooms, the hub, and the rules.
fn with_pool(editor: &mut EditorSession, keys: i32) {
    let gone = [HUB, START, MIDDLE, BOSS];
    editor.rooms.retain(|(id, _)| !gone.contains(id));
    editor.rooms.extend([
        (HUB, hub()),
        (
            START,
            layout("Gate Hall", Some("Start"), &["Right"], keys > 0),
        ),
        (
            MIDDLE,
            layout("Long Corridor", Some("Middle"), &["Left", "Right"], false),
        ),
        (BOSS, layout("Warden's Pit", Some("Boss"), &["Left"], false)),
    ]);
    editor.run =
        RunConfig::from_ron("(hub_room: Custom(0), start_room: First, final_boss_room: Boss)")
            .unwrap();
    editor.generator = rules(keys);
}

fn open(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) {
    press(
        ctx,
        button(ctx, "Preview generated dungeon").center(),
        editor,
        state,
    );
    assert!(state.checks.preview_window, "the button opens the window");
    for _ in 0..3 {
        run(ctx, vec![], editor, state);
    }
}

#[test]
fn the_window_is_closed_until_the_button_is_pressed_and_the_button_closes_it_again() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    assert!(!state.checks.preview_window);
    assert!(!text(&ctx, editor, &mut state).contains("Read-only"));

    open(&ctx, editor, &mut state);
    let shown = text(&ctx, editor, &mut state);
    assert!(shown.contains("Read-only"), "{shown}");
    assert!(shown.contains("Roll next"), "{shown}");

    press(
        &ctx,
        button(&ctx, "Preview generated dungeon").center(),
        editor,
        &mut state,
    );
    assert!(!state.checks.preview_window);
    assert!(!text(&ctx, editor, &mut state).contains("Read-only"));
}

#[test]
fn with_no_pieces_it_says_why_there_is_no_dungeon() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    let shown = text(&ctx, editor, &mut state);
    assert!(shown.contains("No dungeon:"), "{shown}");
    assert!(
        matches!(state.checks.preview.outcome(), Some(Outcome::Failed(_))),
        "{:?}",
        state.checks.preview.outcome()
    );
}

#[test]
fn it_draws_the_plan_by_piece_names_with_the_seed_and_the_pickups() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_pool(editor, 1);
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    let shown = text(&ctx, editor, &mut state);

    assert!(
        matches!(state.checks.preview.outcome(), Some(Outcome::Plan(_))),
        "{:?}",
        state.checks.preview.outcome()
    );
    let seed = state.checks.preview.seed().unwrap();
    assert!(
        shown.contains(&format!("Seed {}", format_seed(seed))),
        "{shown}"
    );
    for name in ["Hub", "Gate Hall", "Long Corridor", "Warden's Pit"] {
        assert!(shown.contains(name), "{name} is missing: {shown}");
    }
    assert!(shown.contains("Keys and abilities"), "{shown}");
    assert!(shown.contains("Key 1 in "), "the key is listed: {shown}");
    assert!(probed(&ctx, "map window canvas").height() > 0.0);
}

#[test]
fn a_dungeon_with_no_key_says_so() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_pool(editor, 0);
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    let shown = text(&ctx, editor, &mut state);
    assert!(
        shown.contains("None: this dungeon has no key or ability."),
        "{shown}"
    );
    assert!(!shown.contains("Key 1 in"), "{shown}");
}

#[test]
fn roll_next_changes_the_box_and_the_dungeon_follows() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_pool(editor, 0);
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    let before = state.checks.preview.seed().unwrap();

    press(&ctx, button(&ctx, "Roll next").center(), editor, &mut state);

    let after = state.checks.preview.seed().unwrap();
    assert_eq!(after, next_seed(before));
    let shown = text(&ctx, editor, &mut state);
    assert!(
        shown.contains(&format!("Seed {}", format_seed(after))),
        "{shown}"
    );
    assert!(
        !shown.contains(&format!("Seed {}:", format_seed(before))),
        "the old plan is gone: {shown}"
    );
}

#[test]
fn typing_in_the_seed_box_reaches_the_model_and_a_bad_seed_says_why() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_pool(editor, 0);
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    let field = probed(&ctx, ("preview", "seed"));
    let digits = state.checks.preview.text().to_owned();
    assert_eq!(digits.len(), 16);

    // Click into the box, go to its end and type a seventeenth digit.
    press(&ctx, field.center(), editor, &mut state);
    run(
        &ctx,
        vec![
            egui::Event::Key {
                key: egui::Key::End,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            },
            egui::Event::Text("f".to_owned()),
        ],
        editor,
        &mut state,
    );
    assert_eq!(state.checks.preview.text(), format!("{digits}f"));
    let shown = text(&ctx, editor, &mut state);
    assert!(shown.contains("Not a seed:"), "{shown}");
    assert!(shown.contains("at most 16"), "{shown}");
    assert!(
        matches!(state.checks.preview.outcome(), Some(Outcome::BadSeed(_))),
        "no plan is drawn for it"
    );

    // Typing a good seed in its place brings the plan back.
    state.checks.preview.set_seed(0x2a);
    let shown = text(&ctx, editor, &mut state);
    assert!(shown.contains("Seed 000000000000002a"), "{shown}");
}

#[test]
fn it_plans_from_an_unsaved_draft_and_changes_nothing_in_the_session() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_pool(editor, 0);
    // The Boss piece is only the draft of the room being edited, not a saved room.
    editor.rooms.retain(|(id, _)| *id != BOSS);
    editor.id = BOSS;
    editor.draft.layout = layout("Warden's Pit", Some("Boss"), &["Left"], false);
    let before = (editor.dirty, editor.catalog());
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    press(&ctx, button(&ctx, "Roll next").center(), editor, &mut state);

    let shown = text(&ctx, editor, &mut state);
    assert!(shown.contains("Warden's Pit"), "{shown}");
    assert!(
        matches!(state.checks.preview.outcome(), Some(Outcome::Plan(_))),
        "{:?}",
        state.checks.preview.outcome()
    );
    assert_eq!((editor.dirty, editor.catalog()), before);
}

#[test]
fn the_seed_and_the_plan_stay_when_the_window_is_closed_and_opened_again() {
    let mut game = editor();
    let editor = game.editor_mut().unwrap();
    with_pool(editor, 0);
    let mut state = EditorUi::default();
    let ctx = laid_out(editor, &mut state);
    open(&ctx, editor, &mut state);
    press(&ctx, button(&ctx, "Roll next").center(), editor, &mut state);
    let rolled = state.checks.preview.text().to_owned();
    for _ in 0..2 {
        press(
            &ctx,
            button(&ctx, "Preview generated dungeon").center(),
            editor,
            &mut state,
        );
    }
    assert!(state.checks.preview_window);
    assert_eq!(state.checks.preview.text(), rolled);
}

// ---- the drawing it shares with the Dungeon map window -------------------------------------

fn lock_map(lock: Option<u8>) -> EditorMap {
    let room = |id, cell| EditorMapRoom {
        id,
        name: format!("{id:?}"),
        cell,
        current: false,
        conflict: false,
        no_entrance: false,
    };
    EditorMap {
        rooms: vec![room(START, (0, 0)), room(MIDDLE, (1, 0))],
        links: vec![MapLink {
            from: START,
            to: MIDDLE,
            from_cell: (0, 0),
            to_cell: (1, 0),
            one_way: false,
            lock,
        }],
    }
}

fn draw_canvas(ctx: &egui::Context, map: &EditorMap) -> String {
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, SIZE)),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                draw_map_canvas(ui, map, egui::vec2(400.0, 200.0), false);
            });
        },
    );
    drawn_text(&output)
}

#[test]
fn a_locked_exit_carries_its_key_number_between_the_two_rooms() {
    let ctx = egui::Context::default();
    for _ in 0..2 {
        draw_canvas(&ctx, &lock_map(Some(3)));
    }
    let shown = draw_canvas(&ctx, &lock_map(Some(3)));
    assert!(shown.contains("Key 3"), "{shown}");
    let tag = probed(&ctx, ("map lock", START, MIDDLE));
    let (from, to) = (
        probed(&ctx, ("map room", START)),
        probed(&ctx, ("map room", MIDDLE)),
    );
    assert!(tag.center().x > from.center().x && tag.center().x < to.center().x);
    assert!(
        !tag.intersects(from) || !tag.intersects(to),
        "the tag does not sit on both rooms"
    );
}

#[test]
fn an_exit_with_no_lock_has_no_tag() {
    let ctx = egui::Context::default();
    for _ in 0..2 {
        draw_canvas(&ctx, &lock_map(None));
    }
    let shown = draw_canvas(&ctx, &lock_map(None));
    assert!(!shown.contains("Key"), "{shown}");
    let tag: Option<egui::Rect> =
        ctx.data(|data| data.get_temp(egui::Id::new(("map lock", START, MIDDLE))));
    assert_eq!(tag, None);
}
