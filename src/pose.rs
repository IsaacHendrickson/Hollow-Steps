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
        poses.insert(name.to_owned(), Arc::new(pose));
    }
    Ok(poses)
}

#[derive(Clone, Debug)]
pub enum PoseStep {
    Apply { name: Option<String>, seconds: f32 },
    Idle(f32),
    Clear,
    Face(bool),
}
