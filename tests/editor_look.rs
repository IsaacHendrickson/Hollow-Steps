//! Editing a room's terrain look in the room editor, below the panel: the draft edits, the
//! unsaved flag, undo and redo, the test room, the saved file and the cached canvas preview.
mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::editor::{EditorSession, RoomDraft};
use hollow_steps::editor_history::{redo_session, tracked, undo_session, History};
use hollow_steps::editor_look::{LookColor, LookPreview};
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEnemies, RoomId};
use hollow_steps::terrain_style::{build_room, Look, RoomInputs};
use hollow_steps::terrain_styles::{RoomStyle, TerrainStyles};
use macroquad::math::Rect;

fn editing_game() -> GameState {
    let mut game = GameState::try_new(&support::fixture_assets()).unwrap();
    game.toggle_editor().unwrap();
    game
}

fn styles() -> TerrainStyles {
    TerrainStyles::default()
}

fn draft() -> RoomDraft {
    let mut game = editing_game();
    game.editor_mut().unwrap().draft.clone()
}

fn style(draft: &RoomDraft) -> &RoomStyle {
    &draft.layout.terrain_style
}

// ---- the draft edits ---------------------------------------------------------------------

#[test]
fn a_room_starts_with_the_default_look_and_the_session_knows_the_presets() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    assert_eq!(*style(&editor.draft), RoomStyle::default());
    assert_eq!(editor.look(), Look::default());
    assert_eq!(editor.styles, styles());
    assert_eq!(editor.styles.names(), ["stone", "mossy", "cave", "ruins"]);
    assert!(!editor.dirty);
}

#[test]
fn choosing_a_preset_clears_color_and_moss_overrides_and_keeps_the_seed_and_boundary() {
    let styles = styles();
    let mut draft = draft();
    draft
        .set_look_color(&styles, LookColor::Fill, (0.9, 0.1, 0.1))
        .unwrap();
    draft
        .set_look_color(&styles, LookColor::Accent, (0.1, 0.9, 0.1))
        .unwrap();
    draft.set_look_moss(&styles, false).unwrap();
    draft.set_seed_offset(7);
    draft.set_draw_boundary(false);
    assert!(style(&draft).fill.is_some() && style(&draft).moss.is_some());

    draft.set_look_preset(&styles, "cave").unwrap();

    assert_eq!(
        *style(&draft),
        RoomStyle {
            seed_offset: 7,
            draw_boundary: false,
            ..RoomStyle::named("cave")
        }
    );
    // The preset's colors apply again.
    let cave = styles.preset("cave").unwrap();
    let look = draft.look(&styles);
    assert_eq!(look.palette.fill, cave.fill);
    assert_eq!(look.palette.accent, cave.accent);
    assert_eq!(look.palette.moss, cave.moss);
    assert_eq!(look.palette.seed_offset, 7);
    assert!(!look.draw_boundary);
}

#[test]
fn an_unknown_preset_is_refused_with_the_known_ones_and_changes_nothing() {
    let mut draft = draft();
    draft.set_look_preset(&styles(), "mossy").unwrap();
    let before = draft.clone();

    let error = draft.set_look_preset(&styles(), "granite").unwrap_err();

    assert!(error.contains("granite"), "{error}");
    assert!(
        error.contains("known presets: stone, mossy, cave, ruins"),
        "{error}"
    );
    assert_eq!(draft, before);
}

#[test]
fn a_color_override_beats_the_preset_and_reset_goes_back_to_it() {
    let styles = styles();
    let mut draft = draft();
    draft.set_look_preset(&styles, "ruins").unwrap();
    let ruins = styles.preset("ruins").unwrap();

    draft
        .set_look_color(&styles, LookColor::Edge, (0.5, 0.25, 0.125))
        .unwrap();

    assert_eq!(style(&draft).edge, Some((0.5, 0.25, 0.125)));
    assert_eq!(draft.look(&styles).palette.edge, (0.5, 0.25, 0.125));
    // The other three colors stay the preset's.
    for other in [LookColor::Fill, LookColor::Highlight, LookColor::Accent] {
        assert_eq!(other.overridden(style(&draft)), None, "{other:?}");
    }
    assert_eq!(draft.look(&styles).palette.fill, ruins.fill);

    draft.reset_look_color(LookColor::Edge);

    assert_eq!(style(&draft).edge, None);
    assert_eq!(draft.look(&styles).palette.edge, ruins.edge);
    assert_eq!(*style(&draft), RoomStyle::named("ruins"));
}

#[test]
fn a_color_that_is_the_presets_own_is_no_override() {
    let styles = styles();
    let mut draft = draft();
    draft.set_look_preset(&styles, "cave").unwrap();
    for color in LookColor::ALL {
        let own = color.of_preset(styles.preset("cave").unwrap());
        draft
            .set_look_color(&styles, color, (0.5, 0.5, 0.5))
            .unwrap();
        assert_eq!(color.overridden(style(&draft)), Some((0.5, 0.5, 0.5)));
        draft.set_look_color(&styles, color, own).unwrap();
        assert_eq!(color.overridden(style(&draft)), None, "{color:?}");
    }
    assert_eq!(*style(&draft), RoomStyle::named("cave"));
}

#[test]
fn colors_that_are_not_finite_or_not_between_zero_and_one_are_refused() {
    let styles = styles();
    let mut draft = draft();
    draft
        .set_look_color(&styles, LookColor::Fill, (0.3, 0.3, 0.3))
        .unwrap();
    let before = draft.clone();
    for bad in [
        (f32::NAN, 0.0, 0.0),
        (0.0, f32::INFINITY, 0.0),
        (0.0, 0.0, f32::NEG_INFINITY),
        (1.5, 0.0, 0.0),
        (0.0, -0.1, 0.0),
    ] {
        let error = draft
            .set_look_color(&styles, LookColor::Fill, bad)
            .unwrap_err();
        assert!(error.contains("fill"), "{bad:?}: {error}");
        assert_eq!(draft, before, "{bad:?}");
    }
    // The ends of the range are fine.
    draft
        .set_look_color(&styles, LookColor::Edge, (0.0, 1.0, 0.0))
        .unwrap();
}

#[test]
fn moss_follows_the_preset_until_the_room_says_otherwise() {
    let styles = styles();
    let mut draft = draft();
    draft.set_look_preset(&styles, "ruins").unwrap();
    assert!(!draft.look(&styles).palette.moss, "ruins has no moss");

    draft.set_look_moss(&styles, true).unwrap();
    assert_eq!(style(&draft).moss, Some(true));
    assert!(draft.look(&styles).palette.moss);

    // Back to what the preset says: no override is kept.
    draft.set_look_moss(&styles, false).unwrap();
    assert_eq!(style(&draft).moss, None);
    assert_eq!(*style(&draft), RoomStyle::named("ruins"));

    // A preset with moss: switching it off is the override.
    draft.set_look_preset(&styles, "stone").unwrap();
    draft.set_look_moss(&styles, false).unwrap();
    assert_eq!(style(&draft).moss, Some(false));
}

#[test]
fn the_boundary_switch_and_the_seed_offset_are_plain_edits() {
    let styles = styles();
    let mut draft = draft();
    assert!(draft.look(&styles).draw_boundary);
    draft.set_draw_boundary(false);
    assert!(!draft.look(&styles).draw_boundary);
    draft.set_draw_boundary(true);
    assert_eq!(*style(&draft), RoomStyle::default());

    draft.set_seed_offset(41);
    assert_eq!(draft.look(&styles).palette.seed_offset, 41);
}

#[test]
fn reroll_moves_to_the_next_seed_which_draws_a_different_look_and_wraps() {
    let styles = styles();
    let mut draft = draft();
    draft
        .add_terrain(Rect::new(600.0, 380.0, 120.0, 20.0))
        .unwrap();
    let list = |draft: &RoomDraft| {
        let room = draft
            .build(
                RoomId::First,
                &hollow_steps::enemy::EnemyDefinitions::load(&support::fixture_assets()).unwrap(),
            )
            .unwrap();
        build_room(
            &RoomInputs {
                room: room.id,
                bounds: room.bounds,
                floor_y: room.floor_y,
                exits: &room.exits,
                solids: room.terrain.solids(),
            },
            &draft.look(&styles),
        )
    };
    let first = list(&draft);

    draft.reroll_seed();

    assert_eq!(style(&draft).seed_offset, 1);
    assert_ne!(
        list(&draft),
        first,
        "a new seed shuffles the speckles and moss"
    );
    draft.set_seed_offset(0);
    assert_eq!(list(&draft), first, "the same seed is the same look");

    draft.set_seed_offset(u32::MAX);
    draft.reroll_seed();
    assert_eq!(style(&draft).seed_offset, 0);
}

// ---- the session: unsaved flag, undo and redo ---------------------------------------------

#[test]
fn a_look_edit_marks_the_session_unsaved_and_an_edit_that_changes_nothing_does_not() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();

    editor
        .edit_look(|d, s| d.set_look_preset(s, "stone"))
        .unwrap();
    editor
        .edit_look(|d, _| {
            d.set_seed_offset(0);
            Ok(())
        })
        .unwrap();
    editor.edit_look(|d, s| d.set_look_moss(s, true)).unwrap();
    assert!(!editor.dirty, "setting what is already set is not an edit");

    editor
        .edit_look(|d, s| d.set_look_preset(s, "mossy"))
        .unwrap();
    assert!(editor.dirty);
}

#[test]
fn a_refused_look_edit_is_an_error_and_does_not_mark_the_session_unsaved() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let before = editor.draft.clone();

    assert!(editor
        .edit_look(|d, s| d.set_look_preset(s, "granite"))
        .is_err());
    assert!(editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Fill, (2.0, 0.0, 0.0)))
        .is_err());

    assert!(!editor.dirty);
    assert_eq!(editor.draft, before);
}

fn step(
    history: &mut History,
    editor: &mut EditorSession,
    edit: impl FnOnce(&mut RoomDraft, &TerrainStyles) -> Result<(), String>,
) {
    tracked(history, editor, |e| e.edit_look(edit)).unwrap();
}

#[test]
fn look_edits_undo_and_redo_one_step_each() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    let original = editor.draft.clone();

    step(&mut history, editor, |d, s| d.set_look_preset(s, "cave"));
    let cave = editor.draft.clone();
    step(&mut history, editor, |d, s| {
        d.set_look_color(s, LookColor::Fill, (0.9, 0.1, 0.1))
    });
    let red = editor.draft.clone();
    step(&mut history, editor, |d, _| {
        d.set_draw_boundary(false);
        Ok(())
    });
    step(&mut history, editor, |d, _| {
        d.reroll_seed();
        Ok(())
    });
    let rerolled = editor.draft.clone();
    // Nothing changes: no step.
    step(&mut history, editor, |d, s| {
        d.set_look_preset(s, "cave")
            .map(|_| ())
            .and_then(|_| d.set_look_color(s, LookColor::Fill, (0.9, 0.1, 0.1)))
    });
    assert_eq!(editor.draft.layout.terrain_style.preset, "cave");

    // The last two steps (boundary, reroll) come off first; `cave` was no step to repeat.
    assert!(undo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft.layout.terrain_style.seed_offset, 0);
    assert!(!editor.draft.layout.terrain_style.draw_boundary);
    assert!(undo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft, red);
    assert!(undo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft, cave);
    assert!(undo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft, original);
    assert!(!undo_session(&mut history, editor).unwrap(), "nothing left");
    assert!(editor.dirty, "undo is an unsaved change too");

    assert!(redo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft, cave);
    for _ in 0..3 {
        assert!(redo_session(&mut history, editor).unwrap());
    }
    assert_eq!(editor.draft, rerolled);
}

#[test]
fn a_whole_gesture_of_color_changes_is_one_undo_step() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let mut history = History::default();
    let original = editor.draft.clone();

    history.begin_gesture(&editor.draft);
    for i in 1..=10 {
        let shade = i as f32 / 10.0;
        editor
            .edit_look(|d, s| d.set_look_color(s, LookColor::Fill, (shade, 0.2, 0.2)))
            .unwrap();
    }
    history.end_gesture(&editor.draft);

    assert_eq!(
        editor.draft.layout.terrain_style.fill,
        Some((1.0, 0.2, 0.2))
    );
    assert!(undo_session(&mut history, editor).unwrap());
    assert_eq!(editor.draft, original, "one undo takes back the whole drag");
    assert!(!history.can_undo());
    assert!(redo_session(&mut history, editor).unwrap());
    assert_eq!(
        editor.draft.layout.terrain_style.fill,
        Some((1.0, 0.2, 0.2))
    );
}

// ---- test room, saved file, preview -------------------------------------------------------

#[test]
fn the_test_room_plays_with_the_drafts_unsaved_look() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    editor
        .edit_look(|d, s| d.set_look_preset(s, "ruins"))
        .unwrap();
    editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Highlight, (0.2, 0.4, 0.6)))
        .unwrap();
    editor
        .edit_look(|d, _| {
            d.set_seed_offset(9);
            d.set_draw_boundary(false);
            Ok(())
        })
        .unwrap();
    let style = editor.draft.layout.terrain_style.clone();
    let expected = editor.look();

    game.start_test_room().unwrap();

    assert!(game.testing_room());
    assert_eq!(game.room().terrain_style, style);
    assert_eq!(
        game.terrain_styles()
            .resolve(&game.room().terrain_style)
            .unwrap(),
        expected,
        "the played room resolves to the look the editor shows"
    );
}

#[test]
fn building_the_draft_carries_the_style() {
    let styles = styles();
    let mut draft = draft();
    draft.set_look_preset(&styles, "mossy").unwrap();
    draft
        .set_look_color(&styles, LookColor::Fill, (0.1, 0.2, 0.3))
        .unwrap();
    draft.set_seed_offset(3);
    let room = draft
        .build(
            RoomId::Custom(0),
            &hollow_steps::enemy::EnemyDefinitions::load(&support::fixture_assets()).unwrap(),
        )
        .unwrap();
    assert_eq!(room.terrain_style, draft.layout.terrain_style);
}

struct TempAssets(PathBuf);

impl TempAssets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-look-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["rooms", "entities", "behaviors", "bosses"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }
}

impl Drop for TempAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn a_saved_look_is_in_the_room_file_and_comes_back_when_the_game_reloads() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    editor
        .edit_look(|d, s| d.set_look_preset(s, "cave"))
        .unwrap();
    editor
        .edit_look(|d, s| d.set_look_color(s, LookColor::Accent, (0.25, 0.5, 0.75)))
        .unwrap();
    editor.edit_look(|d, s| d.set_look_moss(s, false)).unwrap();
    editor
        .edit_look(|d, _| {
            d.set_seed_offset(12);
            d.set_draw_boundary(false);
            Ok(())
        })
        .unwrap();
    let style = editor.draft.layout.terrain_style.clone();

    game.save_editor().unwrap();

    assert!(!game.editor().unwrap().dirty);
    assert_eq!(
        game.room().terrain_style,
        style,
        "the running game has the saved look without a reload"
    );
    let text = std::fs::read_to_string(temp.0.join("rooms/first.ron")).unwrap();
    let saved: RoomEnemies = ron::from_str(&text).unwrap();
    assert_eq!(saved.layout.unwrap().terrain_style, style);
    assert!(text.contains("\"cave\""), "{text}");
    // Only what the room changes is written.
    assert!(text.contains("accent:"), "{text}");
    assert!(
        !text.contains("fill:") && !text.contains("highlight:"),
        "{text}"
    );

    let reloaded = GameState::try_new(&temp.0).unwrap();
    assert_eq!(reloaded.room().terrain_style, style);
    let look = reloaded.terrain_styles().resolve(&style).unwrap();
    assert_eq!(look.palette.accent, (0.25, 0.5, 0.75));
    assert!(!look.palette.moss);
    assert_eq!(look.palette.seed_offset, 12);
    assert!(!look.draw_boundary);
}

#[test]
fn leaving_a_room_at_its_default_look_writes_no_style() {
    let temp = TempAssets::new();
    let mut game = GameState::try_new(&temp.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    editor
        .edit_look(|d, s| d.set_look_preset(s, "cave"))
        .unwrap();
    editor
        .edit_look(|d, s| d.set_look_preset(s, "stone"))
        .unwrap();
    editor.dirty = true;

    game.save_editor().unwrap();

    let text = std::fs::read_to_string(temp.0.join("rooms/first.ron")).unwrap();
    assert!(!text.contains("terrain_style"), "{text}");
}

fn layout_with_terrain() -> hollow_steps::editor::RoomLayout {
    let mut draft = draft();
    draft
        .add_terrain(Rect::new(600.0, 380.0, 120.0, 20.0))
        .unwrap();
    draft
        .add_terrain(Rect::new(200.0, 300.0, 100.0, 20.0))
        .unwrap();
    draft.layout
}

#[test]
fn the_preview_is_the_list_the_game_draws_and_is_rebuilt_only_when_something_changes() {
    let styles = styles();
    let mut preview = LookPreview::default();
    let mut layout = layout_with_terrain();
    layout.terrain_style = RoomStyle::named("cave");

    let first = preview
        .shapes(RoomId::First, &layout, &styles)
        .shapes
        .to_vec();
    assert!(!first.is_empty());
    assert_eq!(preview.builds(), 1);
    for _ in 0..5 {
        assert_eq!(
            preview.shapes(RoomId::First, &layout, &styles).shapes,
            first
        );
    }
    assert_eq!(preview.builds(), 1, "unchanged frames reuse the list");

    // Things that are not part of the picture do not rebuild it.
    layout.spawn.0 += 4.0;
    layout.items.clear();
    preview.shapes(RoomId::First, &layout, &styles);
    assert_eq!(preview.builds(), 1);

    // The same inputs through the game's own entry point give the same list.
    let room = {
        let [mut room, ..] = hollow_steps::room::test_rooms();
        layout.apply(&mut room).unwrap();
        room
    };
    let game_list = build_room(
        &RoomInputs {
            room: RoomId::First,
            bounds: room.bounds,
            floor_y: room.floor_y,
            exits: &room.exits,
            solids: room.terrain.solids(),
        },
        &styles.resolve(&room.terrain_style).unwrap(),
    );
    assert_eq!(first, game_list);

    // A look change rebuilds it...
    layout.terrain_style = RoomStyle::named("ruins");
    let changed = preview
        .shapes(RoomId::First, &layout, &styles)
        .shapes
        .to_vec();
    assert_ne!(changed, first);
    assert_eq!(preview.builds(), 2);
    // ...and so does moving a solid, or another room id (the seed includes it).
    layout.terrain[0].0 += 10.0;
    preview.shapes(RoomId::First, &layout, &styles);
    assert_eq!(preview.builds(), 3);
    preview.shapes(RoomId::Second, &layout, &styles);
    assert_eq!(preview.builds(), 4);
}

#[test]
fn the_preview_follows_the_boundary_switch_and_survives_an_unknown_preset() {
    let styles = styles();
    let mut preview = LookPreview::default();
    let mut layout = layout_with_terrain();

    let with = preview.shapes(RoomId::First, &layout, &styles);
    assert!(with.boundary);
    let count = with.shapes.len();
    layout.terrain_style.draw_boundary = false;
    let without = preview.shapes(RoomId::First, &layout, &styles);
    assert!(!without.boundary);
    assert!(without.shapes.len() < count);

    // The editor never offers a preset that is not in the catalog, but a file edited under it
    // must not stop the canvas: it draws the default look.
    layout.terrain_style = RoomStyle::named("granite");
    let fallback = preview
        .shapes(RoomId::First, &layout, &styles)
        .shapes
        .to_vec();
    layout.terrain_style = RoomStyle::named("stone");
    assert_eq!(
        preview.shapes(RoomId::First, &layout, &styles).shapes,
        fallback
    );
}
