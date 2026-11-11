//! Semantic visual targets. No pose data is used by gameplay geometry.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use crate::body::{BodyInstance, BodyTemplate, PointPositions};

pub const SEMANTIC_POINTS: &[&str] = &[
    "head", "torso", "hand_l", "hand_r", "foot_l", "foot_r", "tail",
];
pub type Poses = BTreeMap<String, Arc<Pose>>;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Pose {
    pub offsets: BTreeMap<String, (f32, f32)>,
}

impl Pose {
    pub fn validate(&self) -> Result<(), String> {
        for (name, (x, y)) in &self.offsets {
            if !SEMANTIC_POINTS.contains(&name.as_str()) {
                return Err(format!("unknown semantic point {name:?}"));
            }
            if !x.is_finite() || !y.is_finite() {
                return Err(format!("pose point {name:?}: offsets must be finite"));
            }
        }
        Ok(())
    }

    /// Local, unmirrored targets; the existing body transform flips with facing.
    pub fn resolve(&self, template: &BodyTemplate) -> PointPositions {
        self.offsets
            .iter()
            .filter_map(|(semantic, offset)| {
                let name = template.pose_map.get(semantic)?;
                let point = template.points.get(name)?;
                Some((
                    name.clone(),
                    macroquad::math::Vec2::from(point.rest) + macroquad::math::Vec2::from(*offset),
                ))
            })
            .collect()
    }

    pub fn apply(&self, body: &mut BodyInstance) {
        for (name, target) in self.resolve(&body.template) {
            body.set_target(&name, target).expect("resolved pose point");
        }
    }
}

pub fn load_poses(directory: &Path) -> Result<Poses, String> {
    let mut poses = Poses::new();
    if !directory.exists() {
        return Ok(poses);
    }
    for path in crate::behavior::ron_files(directory)? {
        let pose: Pose = crate::behavior::read_ron(&path)?;
        pose.validate()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or("invalid pose filename")?;
        validate_name(name).map_err(|e| format!("{}: {e}", path.display()))?;
        poses.insert(name.to_owned(), Arc::new(pose));
    }
    Ok(poses)
}

#[derive(Clone, Debug)]
pub enum PoseStep {
    Apply {
        name: Option<String>,
        seconds: f32,
        blend_secs: f32,
        stiffness_multiplier: f32,
        instant: bool,
    },
    Idle(f32),
    Clear,
    Face(bool),
}

#[derive(Clone, Copy, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PoseMotion {
    pub settle_factor: f32,
    pub min_blend_secs: f32,
    /// Visual spring impulse for actions whose strike has zero simulation duration.
    pub instant_strike_secs: f32,
}
#[derive(serde::Deserialize)]
struct Defaults {
    pose_blend_ratio: f32,
    strike_stiffness_multiplier: f32,
    motion: PoseMotion,
}
fn defaults() -> Defaults {
    ron::from_str(include_str!("../data/pose_defaults.ron")).expect("valid pose defaults")
}
impl Default for PoseMotion {
    fn default() -> Self {
        defaults().motion
    }
}
impl PoseMotion {
    pub fn validate(self) -> Result<(), String> {
        for value in [
            self.settle_factor,
            self.min_blend_secs,
            self.instant_strike_secs,
        ] {
            crate::behavior::positive(value)?;
        }
        Ok(())
    }
}
pub fn default_blend_ratio() -> f32 {
    defaults().pose_blend_ratio
}
pub fn default_strike_multiplier() -> f32 {
    defaults().strike_stiffness_multiplier
}
pub(crate) fn valid_blend_ratio(value: f32) -> Result<(), String> {
    crate::behavior::positive(value)?;
    if value > 1.0 {
        return Err("pose blend ratio must be at most 1".into());
    }
    Ok(())
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct HurtFeedback {
    pub pose: Option<String>,
    pub duration_secs: f32,
    pub blend_secs: f32,
    pub flash_secs: f32,
    pub tint: (f32, f32, f32),
    pub tint_strength: f32,
}
impl Default for HurtFeedback {
    fn default() -> Self {
        ron::from_str(include_str!("../data/hurt_defaults.ron")).expect("valid hurt defaults")
    }
}
impl HurtFeedback {
    pub fn validate(&self) -> Result<(), String> {
        crate::behavior::nonnegative(self.duration_secs)?;
        crate::behavior::positive(self.blend_secs)?;
        crate::behavior::nonnegative(self.flash_secs)?;
        for value in [self.tint.0, self.tint.1, self.tint.2, self.tint_strength] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err("hurt tint must be in 0..1".into());
            }
        }
        Ok(())
    }
}

pub fn validate_behavior(
    behavior: &crate::behavior::BehaviorDefinition,
    poses: &Poses,
) -> Result<(), String> {
    for state in &behavior.states {
        for (index, action) in state.actions.iter().enumerate() {
            if let Some(bindings) = action.pose_bindings() {
                for (field, name) in ["telegraph_pose", "active_pose", "recovery_pose"]
                    .into_iter()
                    .zip(bindings)
                {
                    if let Some(name) = name {
                        if !poses.contains_key(&name) {
                            return Err(format!(
                                "state {:?}, action {index}, {field}: unknown pose {name:?}",
                                state.name
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
pub fn validate_entity(
    entity: &crate::enemy::EnemyDefinition,
    behavior: &crate::behavior::BehaviorDefinition,
    poses: &Poses,
) -> Result<(), String> {
    let has_binding = behavior
        .states
        .iter()
        .flat_map(|s| &s.actions)
        .filter_map(crate::behavior::Action::pose_bindings)
        .flatten()
        .any(|n| n.is_some());
    if entity.body.is_none() && has_binding {
        return Err(format!(
            "entity {:?}: pose binding requires a body template",
            entity.name
        ));
    }
    if let Some(body) = &entity.body {
        if let Some(name) = &body.hurt.pose {
            if !poses.contains_key(name) {
                return Err(format!(
                    "entity {:?}, hurt pose: unknown pose {name:?}",
                    entity.name
                ));
            }
        }
    }
    Ok(())
}

/// A boss's hurt pose must exist in the catalog.
pub fn validate_boss(
    boss: &crate::boss_definition::BossDefinition,
    poses: &Poses,
) -> Result<(), String> {
    if let Some(name) = boss.body.as_ref().and_then(|body| body.hurt.pose.as_ref()) {
        if !poses.contains_key(name) {
            return Err(format!(
                "boss {:?}, hurt pose: unknown pose {name:?}",
                boss.name
            ));
        }
    }
    Ok(())
}

pub fn is_pose_parameter(field: &str) -> bool {
    matches!(field, "telegraph_pose" | "active_pose" | "recovery_pose")
}

pub fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err("pose name must use lowercase letters, digits, underscores or hyphens".into());
    }
    Ok(())
}
