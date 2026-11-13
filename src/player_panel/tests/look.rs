//! Tests of the Look section, drawn through the Player panel.
use super::*;
use hollow_steps::player_body::PlayerPose;
use hollow_steps::player_look_editor::PreviewSettings;

/// A private copy of the fixture assets with the wanderer body and its poses added, so the
/// player has a body to edit and nothing is written to the repository.
struct BodyAssets(std::path::PathBuf);

impl BodyAssets {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
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
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-player-panel-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures");
        copy_dir(&fixtures, &path);
        copy_dir(&fixtures.join("player_look"), &path);
        std::fs::remove_dir_all(path.join("player_look")).unwrap();
        Self(path)
    }

    fn editing(&self) -> GameState {
        let mut game = GameState::try_new(&self.0).unwrap();
        game.toggle_editor().unwrap();
        game
    }
}

impl Drop for BodyAssets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn drag(
    ctx: &egui::Context,
    id: egui::Id,
    by: f32,
    editor: &mut EditorSession,
    state: &mut PlayerUi,
) {
    let start = rect_of(ctx, id).center();
    frame(ctx, pointer(start, Some(true)), editor, state);
    let moved = start + egui::vec2(by, 0.0);
    frame(ctx, pointer(moved, None), editor, state);
    frame(ctx, vec![], editor, state);
    frame(ctx, pointer(moved, Some(false)), editor, state);
    frame(ctx, vec![], editor, state);
}

fn part_id(part: &str, key: &'static str, axis: &'static str) -> egui::Id {
    egui::Id::new(("part", part, key, axis))
}

#[test]
fn the_look_section_shows_its_controls_the_body_panel_and_every_part() {
    let assets = BodyAssets::new();
    let mut game = assets.editing();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let text = frame(&ctx, vec![], editor, &mut state);
    for expected in [
        "Look",
        "Matches the saved look.",
        "Show player on canvas",
        "Idle",
        "Run",
        "Jump",
        "Attack",
        "Face left",
        "Show hitbox outline",
        // The Entity panel's body section, drawn for the player.
        "Body visual",
        "Primary",
        "Eye",
        "Damage pose",
        "Poses by state",
        "Idle pose",
        "Run pose above",
        "Parts (shared by every user of this template)",
        "Pose assets",
        "Reset look",
    ] {
        assert!(text.contains(expected), "missing {expected:?}\n{text}");
    }
    let template = editor.content.bodies["wanderer"].clone();
    for part in &template.parts {
        assert!(text.contains(&part.name), "missing part {}", part.name);
        let size = rect_of(&ctx, part_id(&part.name, "size", "x"));
        assert!(
            size.width() > 20.0 && size.right() <= PANEL_WIDTH,
            "{size:?}"
        );
        rect_of(
            &ctx,
            egui::Id::new(("part", part.name.as_str(), "rotation")),
        );
    }
    assert!(!text.contains("The player cannot use this look"), "{text}");
}

#[test]
fn the_preview_controls_set_what_the_canvas_draws_and_a_closed_panel_draws_nothing() {
    let assets = BodyAssets::new();
    let mut game = assets.editing();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    // Shown by default: the player standing at the spawn point, idle, facing right, outlined.
    let settings = state
        .canvas_preview()
        .expect("shown while the panel is open");
    assert_eq!(settings.pose, PlayerPose::Idle);
    assert!(!settings.face_left && settings.show_hitbox);

    click(
        &ctx,
        egui::Id::new(("preview pose", "Jump")),
        editor,
        &mut state,
    );
    click(&ctx, egui::Id::new("preview face left"), editor, &mut state);
    click(&ctx, egui::Id::new("preview hitbox"), editor, &mut state);
    let settings = state.canvas_preview().unwrap();
    assert_eq!(settings.pose, PlayerPose::Jump);
    assert!(settings.face_left && !settings.show_hitbox);
    // What the canvas draws is built from exactly these settings.
    let preview = editor.player_preview(settings);
    assert_eq!(preview.pose, PlayerPose::Jump);
    assert_eq!(
        preview.body.unwrap().pose_name.as_deref(),
        Some(editor.look.edited().pose_name(PlayerPose::Jump))
    );

    for pose in PlayerPose::ALL {
        click(
            &ctx,
            egui::Id::new(("preview pose", pose.label())),
            editor,
            &mut state,
        );
        assert_eq!(state.canvas_preview().unwrap().pose, pose);
    }

    click(&ctx, egui::Id::new("preview show"), editor, &mut state);
    assert!(state.canvas_preview().is_none(), "ticked off");
    click(&ctx, egui::Id::new("preview show"), editor, &mut state);
    assert!(state.canvas_preview().is_some());
    state.open = false;
    assert!(state.canvas_preview().is_none(), "closed panel");
}

#[test]
fn dragging_a_part_edits_the_shared_template_and_marks_the_editor_unsaved() {
    let assets = BodyAssets::new();
    let mut game = assets.editing();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let cloak = |editor: &EditorSession| {
        editor.content.bodies["wanderer"]
            .parts
            .iter()
            .find(|part| part.name == "cloak")
            .unwrap()
            .size
    };
    let before = cloak(editor);
    assert!(!editor.dirty);

    drag(
        &ctx,
        part_id("cloak", "size", "x"),
        30.0,
        editor,
        &mut state,
    );
    assert!(
        cloak(editor).0 > before.0,
        "{:?} vs {before:?}",
        cloak(editor)
    );
    assert_eq!(cloak(editor).1, before.1, "the other axis is untouched");
    assert!(editor.dirty, "the editor's Save writes the template");
    assert!(!editor.look.is_dirty(), "player_body.ron has not changed");
    // Dragging far past the end holds at the range's end instead of making an invalid part.
    drag(
        &ctx,
        part_id("cloak", "size", "x"),
        50_000.0,
        editor,
        &mut state,
    );
    assert!(cloak(editor).0 <= 4.0, "{:?}", cloak(editor));
    editor.content.bodies["wanderer"].validate().unwrap();
}

#[test]
fn the_body_panel_edits_the_players_look_and_reset_look_restores_it() {
    let assets = BodyAssets::new();
    let mut game = assets.editing();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let scale = editor.look.edited().visual.scale;

    drag(&ctx, egui::Id::new("body scale"), 40.0, editor, &mut state);
    assert!(editor.look.edited().visual.scale > scale);
    assert!(editor.look.is_dirty());
    assert_eq!(editor.look.saved().visual.scale, scale);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Colors or poses changed"), "{text}");
    // Only the player's draft changed, not an enemy's body.
    assert!(editor.content.bodies["wanderer"].validate().is_ok());
    assert!(!editor.dirty);

    click(
        &ctx,
        egui::Id::new(("button", "Reset look")),
        editor,
        &mut state,
    );
    assert!(!editor.look.is_dirty());
    assert_eq!(editor.look.edited().visual.scale, scale);
}

#[test]
fn a_look_the_player_cannot_use_is_explained_beside_the_look() {
    let assets = BodyAssets::new();
    let mut game = assets.editing();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    editor
        .look
        .set_pose_name(PlayerPose::Attack, "no_such_pose".into());
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("The player cannot use this look"), "{text}");
    assert!(text.contains("no_such_pose"), "{text}");
    // The preview falls back to the box rather than drawing something wrong.
    let preview = editor.player_preview(PreviewSettings {
        pose: PlayerPose::Attack,
        ..PreviewSettings::default()
    });
    assert!(preview.body.is_none());

    click(
        &ctx,
        egui::Id::new(("button", "Reset look")),
        editor,
        &mut state,
    );
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(!text.contains("The player cannot use this look"), "{text}");
}

#[test]
fn without_a_body_template_the_look_says_so_and_offers_no_parts() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("The player cannot use this look"), "{text}");
    assert!(!text.contains("Parts (shared"), "{text}");
    assert!(state.canvas_preview().is_some(), "the box is still drawn");
}
