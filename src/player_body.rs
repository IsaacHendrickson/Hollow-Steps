//! The player's body: which pose plays for what the player is doing, and the body that plays it.
//!
//! Window-free. The look (colors and pose names) is the embedded `data/player_body.ron`; an
//! `assets/player_body.ron` replaces it whole. The body template and poses are ordinary
//! assets (`assets/bodies/`, `assets/poses/`), so the same body and pose code that draws
//! enemies draws the player. Gameplay never reads this: the player's hitbox is unchanged.

use std::path::Path;

use macroquad::math::Vec2;
use serde::Deserialize;

use crate::body::{load_body_templates, BodyInstance, BodyTemplates, BodyVisual};
use crate::pose::{load_poses, validate_name, Poses};

/// Name of the override file inside the assets folder.
pub const FILE_NAME: &str = "player_body.ron";

const DEFAULTS: &str = include_str!("../data/player_body.ron");

/// What the player is doing, as far as the body is concerned. Earlier entries win.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayerPose {
    Attack,
    Jump,
    Run,
    Idle,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlayerLook {
    pub visual: BodyVisual,
    pub idle_pose: String,
    pub run_pose: String,
    pub jump_pose: String,
    pub attack_pose: String,
    /// Horizontal speed (pixels per second) above which the run pose plays.
    pub run_speed: f32,
}

impl PlayerLook {
    /// The embedded defaults, or `assets/player_body.ron` when that file exists.
    pub fn load(assets: &Path) -> Result<Self, String> {
        let (text, source) = match crate::assets::read_override(assets, FILE_NAME)? {
            Some(text) => (text, assets.join(FILE_NAME).display().to_string()),
            None => (DEFAULTS.to_owned(), "data/player_body.ron".to_owned()),
        };
        Self::parse(&text).map_err(|error| format!("{source}: {error}"))
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let look: Self = ron::from_str(text).map_err(|error| error.to_string())?;
        look.validate()?;
        Ok(look)
    }

    pub fn validate(&self) -> Result<(), String> {
        self.visual.validate()?;
        for name in self.pose_names() {
            validate_name(name).map_err(|error| format!("pose {name:?}: {error}"))?;
        }
        if !self.run_speed.is_finite() || self.run_speed <= 0.0 {
            return Err("run_speed must be finite and positive".into());
        }
        Ok(())
    }

    fn pose_names(&self) -> [&str; 4] {
        [
            &self.idle_pose,
            &self.run_pose,
            &self.jump_pose,
            &self.attack_pose,
        ]
    }

    /// The pose for the player's state: a swing beats being airborne, which beats running.
    pub fn pose_for(&self, grounded: bool, velocity_x: f32, attacking: bool) -> PlayerPose {
        if attacking {
            PlayerPose::Attack
        } else if !grounded {
            PlayerPose::Jump
        } else if velocity_x.is_finite() && velocity_x.abs() > self.run_speed {
            PlayerPose::Run
        } else {
            PlayerPose::Idle
        }
    }

    pub fn pose_name(&self, pose: PlayerPose) -> &str {
        match pose {
            PlayerPose::Attack => &self.attack_pose,
            PlayerPose::Jump => &self.jump_pose,
            PlayerPose::Run => &self.run_pose,
            PlayerPose::Idle => &self.idle_pose,
        }
    }
}

/// The player's body and the poses it plays.
#[derive(Clone, Debug)]
pub struct PlayerBody {
    look: PlayerLook,
    poses: Poses,
    body: BodyInstance,
}

impl PlayerBody {
    /// Loads the look, body template and poses from the assets folder. `Ok(None)` when the
    /// folder has no template of the look's name (old projects and test fixtures), which
    /// keeps the plain box; a template that is there but broken, or a pose that is missing,
    /// is an error.
    pub fn load(assets: &Path) -> Result<Option<Self>, String> {
        let look = PlayerLook::load(assets)?;
        let templates = load_body_templates(&assets.join("bodies"))?;
        if !templates.contains_key(&look.visual.template) {
            return Ok(None);
        }
        let poses = load_poses(&assets.join("poses"))?;
        Self::new(look, &templates, poses).map(Some)
    }

    pub fn new(look: PlayerLook, templates: &BodyTemplates, poses: Poses) -> Result<Self, String> {
        look.validate()?;
        for name in look.pose_names() {
            if !poses.contains_key(name) {
                return Err(format!("player body: unknown pose {name:?}"));
            }
        }
        let body = BodyInstance::new(look.visual.clone(), templates)?;
        Ok(Self { look, poses, body })
    }

    pub fn look(&self) -> &PlayerLook {
        &self.look
    }

    pub fn body(&self) -> &BodyInstance {
        &self.body
    }

    /// The pose that is playing now (what the last `update` chose).
    pub fn current_pose(&self) -> Option<&str> {
        self.body.pose_name.as_deref()
    }

    /// Advances the body by one tick for the player's current state.
    pub fn update(&mut self, grounded: bool, velocity: Vec2, attacking: bool, delta_seconds: f32) {
        let pose = self.look.pose_for(grounded, velocity.x, attacking);
        self.body
            .set_pose(Some(self.look.pose_name(pose)), &self.poses);
        self.body.step(delta_seconds);
    }
}
