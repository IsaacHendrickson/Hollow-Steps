mod support;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::body::{load_body_templates, ColorSlot};
use hollow_steps::game::GameState;
use hollow_steps::player_body::{PlayerBody, PlayerLook, PlayerPose, FILE_NAME};
use hollow_steps::player_look_editor::{PlayerLookDraft, PreviewSettings};
use hollow_steps::tuning::Field;
use macroquad::math::Vec2;

/// A private copy of the fixture assets, with the wanderer body and its poses added, so Save
/// never touches the repository and the player has a body to edit.
struct Assets(PathBuf);

fn copy_dir(from: &Path, to: &Path) {
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

impl Assets {
    /// `with_body: false` is the plain fixtures, which have no player body.
    fn new(with_body: bool) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-player-look-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let fixtures = support::fixture_assets();
        copy_dir(&fixtures, &path);
        // The overlay folder is not an asset directory of its own.
        std::fs::remove_dir_all(path.join("player_look")).unwrap();
        if with_body {
            copy_dir(&fixtures.join("player_look"), &path);
        }
        Self(path)
    }

    /// A game with its editor open.
    fn editing(&self) -> GameState {
        let mut game = GameState::try_new(&self.0).unwrap();
        game.toggle_editor().unwrap();
        game
    }

    fn look_file(&self) -> PathBuf {
        self.0.join(FILE_NAME)
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

/// One way to break a look, applied to the draft.
type Edit = Box<dyn Fn(&mut PlayerLookDraft)>;

fn shipped_look() -> PlayerLook {
    PlayerLook::parse(include_str!("../data/player_body.ron")).unwrap()
}

#[test]
fn a_look_written_by_the_editor_parses_back_to_the_same_look() {
    let mut look = shipped_look();
    look.visual.primary = (0.9, 0.1, 0.2, 1.0);
    look.visual.scale = 1.25;
    look.jump_pose = "wanderer_run".into();
    look.run_speed = 80.0;
    let text = look.to_ron().unwrap();
    assert_eq!(PlayerLook::parse(&text).unwrap(), look);
}

#[test]
fn a_look_that_the_game_would_refuse_is_not_written() {
    let mut look = shipped_look();
    look.visual.eye = (1.0, 2.0, 0.0, 1.0);
    assert!(look.to_ron().unwrap_err().contains("0..=1"));
    let mut look = shipped_look();
    look.run_speed = 0.0;
    assert!(look.to_ron().is_err());
}

#[test]
fn a_draft_is_clean_until_the_look_differs_and_reset_brings_it_back() {
    let mut draft = PlayerLookDraft::new(shipped_look());
    assert!(!draft.is_dirty());
    let mut visual = draft.edited().visual.clone();
    visual.accent = (0.1, 0.2, 0.3, 1.0);
    draft.set_visual(visual);
    assert!(draft.is_dirty());
    assert_eq!(draft.saved(), &shipped_look());
    draft.reset();
    assert!(!draft.is_dirty());

    draft.set_pose_name(PlayerPose::Run, "wanderer_idle".into());
    draft.set_run_speed(60.0);
    assert_eq!(draft.edited().run_pose, "wanderer_idle");
    assert_eq!(draft.edited().run_speed, 60.0);
    assert!(draft.is_dirty());
    draft.reset();
    assert_eq!(draft.edited(), &shipped_look());
}

#[test]
fn a_color_edited_in_the_session_is_saved_and_the_game_loads_it() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    assert!(!session.look.is_dirty());
    assert!(session
        .look
        .problem(&session.content.bodies, &session.content.poses)
        .is_none());
    let mut visual = session.look.edited().visual.clone();
    visual.primary = (0.9, 0.2, 0.2, 1.0);
    visual.secondary = (0.0, 0.0, 0.0, 1.0);
    session.look.set_visual(visual.clone());
    assert!(session.look.is_dirty());
    assert!(!assets.look_file().exists());

    let written = game.save_player().unwrap();
    assert!(written.contains(&assets.look_file()), "{written:?}");
    assert!(!game.editor().unwrap().look.is_dirty());
    // What the game loads next start is the edited look, with its body.
    let loaded = PlayerLook::load(&assets.0).unwrap();
    assert_eq!(loaded.visual, visual);
    let body = PlayerBody::load(&assets.0).unwrap().expect("a body");
    assert_eq!(body.look().visual.color(ColorSlot::Primary), visual.primary);
    assert!(!assets.0.join("player_body.ron.tmp").exists());
}

#[test]
fn a_part_edited_in_the_session_is_saved_with_the_editor_and_loads_back() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let template = session.content.bodies.get_mut("wanderer").unwrap();
    let edited = std::sync::Arc::make_mut(template);
    let cloak = edited.parts.iter_mut().find(|p| p.name == "cloak").unwrap();
    cloak.size = (0.7, 0.8);
    cloak.color_slot = ColorSlot::Accent;
    edited.validate().unwrap();
    // The Look section sets this when a part changes.
    session.dirty = true;

    game.save_editor().unwrap();
    let saved = load_body_templates(&assets.0.join("bodies")).unwrap();
    let cloak = saved["wanderer"]
        .parts
        .iter()
        .find(|p| p.name == "cloak")
        .unwrap();
    assert_eq!(cloak.size, (0.7, 0.8));
    assert_eq!(cloak.color_slot, ColorSlot::Accent);
    // Other parts and the other templates are as they were.
    assert_eq!(saved["wanderer"].parts.len(), 13);
    assert!(saved.contains_key("small"));
}

#[test]
fn an_invalid_look_is_refused_and_nothing_is_written() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    game.set_tuning_field(Field::MoveSpeed, 250.0).unwrap();

    let cases: [(&str, Edit); 5] = [
        (
            "unknown body template",
            Box::new(|draft| {
                let mut visual = draft.edited().visual.clone();
                visual.template = "nothing".into();
                draft.set_visual(visual);
            }),
        ),
        (
            "unknown pose",
            Box::new(|draft| draft.set_pose_name(PlayerPose::Jump, "no_such_pose".into())),
        ),
        ("run_speed", Box::new(|draft| draft.set_run_speed(-1.0))),
        (
            "0..=1",
            Box::new(|draft| {
                let mut visual = draft.edited().visual.clone();
                visual.primary = (3.0, 0.0, 0.0, 1.0);
                draft.set_visual(visual);
            }),
        ),
        (
            "scale",
            Box::new(|draft| {
                let mut visual = draft.edited().visual.clone();
                visual.scale = 0.0;
                draft.set_visual(visual);
            }),
        ),
    ];
    for (expected, edit) in cases {
        edit(&mut game.editor_mut().unwrap().look);
        let session = game.editor().unwrap();
        let problem = session
            .look
            .problem(&session.content.bodies, &session.content.poses)
            .unwrap_or_else(|| panic!("{expected} was accepted"));
        assert!(problem.contains(expected), "{expected}: {problem}");
        let error = game.save_player().unwrap_err();
        assert!(error.contains(expected), "{expected}: {error}");
        assert!(!assets.look_file().exists(), "{expected} wrote a file");
        assert!(
            !assets.0.join("player.ron").exists(),
            "{expected} wrote player.ron beside it"
        );
        game.editor_mut().unwrap().look.reset();
    }
    // With the look put right the same save goes through, stats and look together.
    let written = game.save_player().unwrap();
    assert_eq!(
        written,
        vec![assets.0.join("player.ron")],
        "a clean look is not rewritten"
    );
}

#[test]
fn a_look_that_names_a_pose_not_on_disk_yet_waits_for_the_editor_save() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    session.content.create_pose("wanderer_dash").unwrap();
    session
        .look
        .set_pose_name(PlayerPose::Run, "wanderer_dash".into());
    // The look is fine as far as the editor's own content goes ...
    assert!(session
        .look
        .problem(&session.content.bodies, &session.content.poses)
        .is_none());
    // ... but the game could not start from a file that names a pose that is not saved.
    let error = game.save_player().unwrap_err();
    assert!(
        error.contains("wanderer_dash") && error.contains("not saved yet"),
        "{error}"
    );
    assert!(!assets.look_file().exists());

    game.editor_mut().unwrap().dirty = true;
    game.save_editor().unwrap();
    game.save_player().unwrap();
    assert_eq!(
        PlayerLook::load(&assets.0).unwrap().run_pose,
        "wanderer_dash"
    );
    assert!(PlayerBody::load(&assets.0).unwrap().is_some());
}

#[test]
fn the_preview_stands_at_the_spawn_point_in_the_chosen_pose_and_size() {
    let assets = Assets::new(true);
    let mut game = assets.editing();
    let session = game.editor_mut().unwrap();
    let settings = |pose, face_left, show_hitbox| PreviewSettings {
        pose,
        face_left,
        show_hitbox,
    };

    let idle = session.player_preview(settings(PlayerPose::Idle, false, true));
    let spawn: Vec2 = session.draft.layout.spawn.into();
    assert_eq!(idle.center, spawn);
    assert_eq!(idle.size, session.player.edited().player.size);
    assert!(idle.show_hitbox && !idle.face_left);
    let idle_body = idle.body.as_ref().expect("the wanderer body");
    assert_eq!(idle_body.pose_name.as_deref(), Some("wanderer_idle"));

    let transforms = |preview: &hollow_steps::player_look_editor::PlayerPreview| {
        let body = preview.body.as_ref().unwrap();
        body.template
            .parts
            .iter()
            .map(|part| body.part_transform(part, preview.center, preview.size, false))
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    };
    let mut seen = vec![transforms(&idle)];
    for pose in [PlayerPose::Run, PlayerPose::Jump, PlayerPose::Attack] {
        let preview = session.player_preview(settings(pose, true, false));
        assert_eq!(preview.pose, pose);
        assert!(preview.face_left && !preview.show_hitbox);
        let body = preview.body.as_ref().unwrap();
        assert_eq!(
            body.pose_name.as_deref(),
            Some(session.look.edited().pose_name(pose))
        );
        let drawn = transforms(&preview);
        assert!(
            !seen.contains(&drawn),
            "{pose:?} is drawn like an earlier pose"
        );
        seen.push(drawn);
    }

    // The preview follows the unsaved size and the unsaved colors.
    session.player.set_field(Field::Width, 20.0).unwrap();
    session.player.set_field(Field::Height, 60.0).unwrap();
    let mut visual = session.look.edited().visual.clone();
    visual.primary = (0.0, 1.0, 0.0, 1.0);
    session.look.set_visual(visual);
    let small = session.player_preview(PreviewSettings::default());
    assert_eq!(small.size, Vec2::new(20.0, 60.0));
    assert_eq!(small.body.unwrap().visual.primary, (0.0, 1.0, 0.0, 1.0));
}

#[test]
fn without_a_body_template_the_preview_is_a_box_and_the_look_says_why() {
    let assets = Assets::new(false);
    let game = assets.editing();
    let session = game.editor().unwrap();
    let problem = session
        .look
        .problem(&session.content.bodies, &session.content.poses)
        .expect("these assets have no wanderer");
    assert!(
        problem.contains("wanderer") || problem.contains("pose"),
        "{problem}"
    );
    let preview = session.player_preview(PreviewSettings::default());
    assert!(preview.body.is_none());
    assert_eq!(preview.size, session.player.edited().player.size);
}
