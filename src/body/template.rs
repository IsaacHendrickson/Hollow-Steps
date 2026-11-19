//! Body template data: points, parts, springs, idle motion, validation and the catalog loader.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, OnceLock};

use macroquad::math::Vec2;
use serde::{Deserialize, Serialize};

use crate::behavior::{read_ron, ron_files};

pub type BodyTemplates = BTreeMap<String, Arc<BodyTemplate>>;
pub type PointPositions = BTreeMap<String, Vec2>;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyPoint {
    pub rest: (f32, f32),
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Follow {
    pub stiffness: f32,
    pub damping: f32,
}

impl Default for Follow {
    fn default() -> Self {
        motion_defaults().follow
    }
}

impl Follow {
    fn validate(self) -> Result<(), String> {
        if !self.stiffness.is_finite() || self.stiffness <= 0.0 {
            return Err("stiffness must be finite and positive".into());
        }
        if !self.damping.is_finite() || self.damping < 0.0 {
            return Err("damping must be finite and nonnegative".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct IdleMotion {
    pub point: String,
    pub amplitude: (f32, f32),
    pub period_secs: f32,
    pub phase_offset_secs: f32,
}

impl IdleMotion {
    pub fn for_point(point: &str) -> Self {
        Self {
            point: point.to_owned(),
            ..motion_defaults().idle.clone()
        }
    }

    pub fn offset_at(&self, game_time_secs: f64) -> Vec2 {
        let cycle = (game_time_secs + f64::from(self.phase_offset_secs))
            .rem_euclid(f64::from(self.period_secs))
            / f64::from(self.period_secs);
        Vec2::from(self.amplitude) * (std::f64::consts::TAU * cycle).sin() as f32
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MotionDefaults {
    follow: Follow,
    pub(super) max_step_secs: f32,
    idle: IdleMotion,
}

pub(super) fn motion_defaults() -> &'static MotionDefaults {
    static DEFAULTS: OnceLock<MotionDefaults> = OnceLock::new();
    DEFAULTS.get_or_init(|| {
        ron::from_str(include_str!("../../data/body_motion_defaults.ron"))
            .expect("valid body motion defaults")
    })
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
pub enum Shape {
    Ellipse,
    Triangle,
    Rect,
    Line,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ColorSlot {
    Primary,
    Secondary,
    Accent,
    Eye,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyPart {
    pub name: String,
    pub shape: Shape,
    pub anchor: Option<String>,
    pub offset: Option<(f32, f32)>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub size: (f32, f32),
    pub rotation: f32,
    pub color_slot: ColorSlot,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyTemplate {
    #[serde(default)]
    pub pose_map: BTreeMap<String, String>,
    #[serde(default)]
    pub pose_motion: crate::pose::PoseMotion,
    pub points: BTreeMap<String, BodyPoint>,
    pub parts: Vec<BodyPart>,
    #[serde(default)]
    pub follow: Follow,
    #[serde(default)]
    pub follow_overrides: BTreeMap<String, Follow>,
    #[serde(default)]
    pub idle: Vec<IdleMotion>,
}

impl BodyTemplate {
    pub fn validate(&self) -> Result<(), String> {
        for semantic in self.pose_map.keys() {
            if !crate::pose::SEMANTIC_POINTS.contains(&semantic.as_str()) {
                return Err(format!("pose_map: unknown semantic point {semantic:?}"));
            }
        }
        self.pose_motion
            .validate()
            .map_err(|e| format!("pose motion: {e}"))?;
        self.follow.validate().map_err(|e| format!("follow: {e}"))?;
        for (name, follow) in &self.follow_overrides {
            if !self.points.contains_key(name) {
                return Err(format!("follow override: unknown point {name:?}"));
            }
            follow
                .validate()
                .map_err(|e| format!("follow override {name:?}: {e}"))?;
        }
        for idle in &self.idle {
            if !self.points.contains_key(&idle.point) {
                return Err(format!("idle: unknown point {:?}", idle.point));
            }
            if !pair_finite(idle.amplitude) || !idle.phase_offset_secs.is_finite() {
                return Err(format!(
                    "idle {:?}: amplitude and phase must be finite",
                    idle.point
                ));
            }
            if !idle.period_secs.is_finite() || idle.period_secs <= 0.0 {
                return Err(format!(
                    "idle {:?}: period_secs must be finite and positive",
                    idle.point
                ));
            }
        }
        for (name, point) in &self.points {
            if name.trim().is_empty() || !pair_finite(point.rest) {
                return Err(format!(
                    "point {name:?}: name must be nonempty and rest position finite"
                ));
            }
        }
        let mut names = BTreeSet::new();
        for part in &self.parts {
            let error = |message| format!("part {:?}: {message}", part.name);
            if part.name.trim().is_empty() || !names.insert(&part.name) {
                return Err(error("name must be nonempty and unique"));
            }
            if !pair_finite(part.size) || part.size.0 <= 0.0 || part.size.1 <= 0.0 {
                return Err(error("size must be finite and positive"));
            }
            if !part.rotation.is_finite() {
                return Err(error("rotation must be finite"));
            }
            let references = if part.shape == Shape::Line {
                if part.anchor.is_some() || part.offset.is_some() {
                    return Err(error("Line uses from/to instead of anchor/offset"));
                }
                vec![
                    part.from
                        .as_deref()
                        .ok_or_else(|| error("missing from point"))?,
                    part.to
                        .as_deref()
                        .ok_or_else(|| error("missing to point"))?,
                ]
            } else {
                if part.from.is_some() || part.to.is_some() {
                    return Err(error("only Line uses from/to"));
                }
                if !part.offset.is_some_and(pair_finite) {
                    return Err(error("offset must be present and finite"));
                }
                vec![part
                    .anchor
                    .as_deref()
                    .ok_or_else(|| error("missing anchor point"))?]
            };
            for name in references {
                if !self.points.contains_key(name) {
                    return Err(error(&format!("unknown point {name:?}")));
                }
            }
        }
        Ok(())
    }

    pub fn rest_positions(&self) -> PointPositions {
        self.points
            .iter()
            .map(|(name, p)| (name.clone(), Vec2::from(p.rest)))
            .collect()
    }
}

pub fn load_body_templates(directory: &Path) -> Result<BodyTemplates, String> {
    let mut templates = BTreeMap::new();
    // Old projects and fixture catalogs without bodies keep the existing box visuals.
    if !directory.exists() {
        return Ok(templates);
    }
    for path in ron_files(directory)? {
        let template: BodyTemplate = read_ron(&path)?;
        template
            .validate()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| format!("{}: invalid body template name", path.display()))?;
        templates.insert(name.to_owned(), Arc::new(template));
    }
    Ok(templates)
}

fn pair_finite(pair: (f32, f32)) -> bool {
    pair.0.is_finite() && pair.1.is_finite()
}
