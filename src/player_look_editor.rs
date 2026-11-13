//! The player's look as the editor edits it: a draft of `player_body.ron` (colors, scale, hurt
//! feedback, which pose plays for each state) with saved and edited copies, checked against the
//! body templates and poses the editor holds, plus the picture of the player the editor canvas
//! draws at the room's spawn point. Pure rules; no window.
//!
//! What each file is saved by:
//! - `player_body.ron` (this draft): the Player panel's Save, through a staged write;
//! - the body template (`bodies/wanderer.ron`: parts, pose map, spring and idle values) and the
//!   poses (`poses/`): the editor's own Save (`GameState::save_editor`), because they are
//!   ordinary body and pose assets shared with every enemy that uses them.
//!
//! `player_body.ron` must not name a template or pose that is not on disk, or the game could
//! not start with it; `check_savable` refuses until the editor has saved them.

use std::path::{Path, PathBuf};

use macroquad::math::Vec2;

use crate::body::{BodyInstance, BodyTemplates, BodyVisual};
use crate::editor::EditorSession;
use crate::player_body::{PlayerBody, PlayerLook, PlayerPose};
use crate::pose::Poses;

/// The player's look as saved and as edited.
#[derive(Clone, Debug, PartialEq)]
pub struct PlayerLookDraft {
    saved: PlayerLook,
    edited: PlayerLook,
}

impl PlayerLookDraft {
    pub fn new(look: PlayerLook) -> Self {
        Self {
            saved: look.clone(),
            edited: look,
        }
    }

    /// The look in effect: `assets/player_body.ron`, or the shipped default.
    pub fn load(assets: &Path) -> Result<Self, String> {
        PlayerLook::load(assets).map(Self::new)
    }

    pub fn saved(&self) -> &PlayerLook {
        &self.saved
    }

    pub fn edited(&self) -> &PlayerLook {
        &self.edited
    }

    pub fn is_dirty(&self) -> bool {
        self.saved != self.edited
    }

    /// Takes the colors, scale and hurt feedback the body panel edited. Whether they are
    /// acceptable is `problem`'s to say; an unacceptable look can be held, but not saved.
    pub fn set_visual(&mut self, visual: BodyVisual) {
        self.edited.visual = visual;
    }

    pub fn set_pose_name(&mut self, pose: PlayerPose, name: String) {
        self.edited.set_pose_name(pose, name);
    }

    pub fn set_run_speed(&mut self, run_speed: f32) {
        self.edited.run_speed = run_speed;
    }

    pub fn reset(&mut self) {
        self.edited = self.saved.clone();
    }

    /// Why the edited look cannot be used with these templates and poses, if it cannot: the
    /// same checks the game makes when it loads the player's body.
    pub fn problem(&self, templates: &BodyTemplates, poses: &Poses) -> Option<String> {
        PlayerBody::new(self.edited.clone(), templates, poses.clone()).err()
    }

    /// `problem`, and also that the template and the four poses are files in the assets folder.
    pub fn check_savable(
        &self,
        assets: &Path,
        templates: &BodyTemplates,
        poses: &Poses,
    ) -> Result<(), String> {
        if let Some(problem) = self.problem(templates, poses) {
            return Err(problem);
        }
        let on_disk = |directory: &str, name: &str| {
            assets.join(directory).join(format!("{name}.ron")).is_file()
        };
        if !on_disk("bodies", &self.edited.visual.template) {
            return Err(format!(
                "body template {:?} is not saved yet: Save in the room tools first",
                self.edited.visual.template
            ));
        }
        for pose in PlayerPose::ALL {
            let name = self.edited.pose_name(pose);
            if !on_disk("poses", name) {
                return Err(format!(
                    "pose {name:?} is not saved yet: Save in the room tools first"
                ));
            }
        }
        Ok(())
    }

    /// Writes `player_body.ron`. An unacceptable look writes nothing and stays unsaved.
    pub fn save(
        &mut self,
        assets: &Path,
        templates: &BodyTemplates,
        poses: &Poses,
    ) -> Result<PathBuf, String> {
        self.check_savable(assets, templates, poses)?;
        let path = self.edited.save(assets)?;
        self.saved = self.edited.clone();
        Ok(path)
    }

    /// A body at rest in `pose`, as the game would draw it standing still in that state.
    pub fn preview_body(
        &self,
        templates: &BodyTemplates,
        poses: &Poses,
        pose: PlayerPose,
    ) -> Result<BodyInstance, String> {
        let name = self.edited.pose_name(pose);
        if !poses.contains_key(name) {
            return Err(format!("unknown pose {name:?}"));
        }
        let mut body = BodyInstance::new(self.edited.visual.clone(), templates)?;
        body.set_pose(Some(name), poses);
        body.snap_to_targets();
        Ok(body)
    }
}

/// What the canvas shows of the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewSettings {
    pub pose: PlayerPose,
    pub face_left: bool,
    pub show_hitbox: bool,
}

impl Default for PreviewSettings {
    fn default() -> Self {
        Self {
            pose: PlayerPose::Idle,
            face_left: false,
            show_hitbox: true,
        }
    }
}

/// The player as drawn on the editor canvas: at the room's spawn point, at the size the Player
/// panel currently has.
#[derive(Clone, Debug)]
pub struct PlayerPreview {
    pub center: Vec2,
    pub size: Vec2,
    pub pose: PlayerPose,
    /// `None` when the look cannot be drawn (no such template or pose): the plain box is drawn.
    pub body: Option<BodyInstance>,
    pub face_left: bool,
    pub show_hitbox: bool,
}

impl EditorSession {
    /// The player at this room's spawn point, from the editor's unsaved look, size and body.
    pub fn player_preview(&self, settings: PreviewSettings) -> PlayerPreview {
        PlayerPreview {
            center: self.draft.layout.spawn.into(),
            size: self.player.edited().player.size,
            pose: settings.pose,
            body: self
                .look
                .preview_body(&self.content.bodies, &self.content.poses, settings.pose)
                .ok(),
            face_left: settings.face_left,
            show_hitbox: settings.show_hitbox,
        }
    }
}
