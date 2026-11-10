//! Data-only body templates, validation, and window-free visual transforms.
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
struct MotionDefaults {
    follow: Follow,
    max_step_secs: f32,
    idle: IdleMotion,
}

fn motion_defaults() -> &'static MotionDefaults {
    static DEFAULTS: OnceLock<MotionDefaults> = OnceLock::new();
    DEFAULTS.get_or_init(|| {
        ron::from_str(include_str!("../data/body_motion_defaults.ron"))
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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BodyVisual {
    pub template: String,
    pub primary: (f32, f32, f32, f32),
    pub secondary: (f32, f32, f32, f32),
    pub accent: (f32, f32, f32, f32),
    pub eye: (f32, f32, f32, f32),
    #[serde(default = "unit_scale")]
    pub scale: f32,
}

fn unit_scale() -> f32 {
    1.0
}
fn pair_finite(pair: (f32, f32)) -> bool {
    pair.0.is_finite() && pair.1.is_finite()
}

impl BodyVisual {
    /// Data-backed authoring defaults; the selected catalog name is always explicit.
    pub fn for_template(template: &str) -> Self {
        let mut visual: Self = ron::from_str(include_str!("../data/body_visual_defaults.ron"))
            .expect("valid body visual defaults");
        visual.template = template.to_owned();
        visual
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.template.trim().is_empty() {
            return Err("missing body template name".into());
        }
        if !self.scale.is_finite() || self.scale <= 0.0 {
            return Err("body scale must be finite and positive".into());
        }
        for slot in [
            ColorSlot::Primary,
            ColorSlot::Secondary,
            ColorSlot::Accent,
            ColorSlot::Eye,
        ] {
            let (r, g, b, a) = self.color(slot);
            if [r, g, b, a]
                .iter()
                .any(|c| !c.is_finite() || !(0.0..=1.0).contains(c))
            {
                return Err(format!(
                    "body {slot:?} color channels must be finite and in 0..=1"
                ));
            }
        }
        Ok(())
    }

    pub fn validate_template(&self, templates: &BodyTemplates) -> Result<(), String> {
        self.validate()?;
        if !templates.contains_key(&self.template) {
            return Err(format!("unknown body template {:?}", self.template));
        }
        Ok(())
    }

    pub fn color(&self, slot: ColorSlot) -> (f32, f32, f32, f32) {
        match slot {
            ColorSlot::Primary => self.primary,
            ColorSlot::Secondary => self.secondary,
            ColorSlot::Accent => self.accent,
            ColorSlot::Eye => self.eye,
        }
    }
}

#[derive(Clone, Debug)]
pub struct BodyInstance {
    pub template: Arc<BodyTemplate>,
    pub visual: BodyVisual,
    /// Unmirrored entity-local box fractions; rendering alone applies facing.
    pub points: PointPositions,
    pub velocities: PointPositions,
    pub targets: PointPositions,
    target_overrides: PointPositions,
    game_time_secs: f64,
}

impl BodyInstance {
    pub fn new(visual: BodyVisual, templates: &BodyTemplates) -> Result<Self, String> {
        visual.validate_template(templates)?;
        let template = templates[&visual.template].clone();
        template.validate()?;
        let mut instance = Self {
            points: template.rest_positions(),
            velocities: template
                .points
                .keys()
                .map(|n| (n.clone(), Vec2::ZERO))
                .collect(),
            targets: template.rest_positions(),
            target_overrides: PointPositions::new(),
            game_time_secs: 0.0,
            template,
            visual,
        };
        instance.refresh_targets();
        instance.snap_to_targets();
        Ok(instance)
    }

    /// Override a point's target in unmirrored local box fractions.
    pub fn set_target(&mut self, point_name: &str, position: Vec2) -> Result<(), String> {
        if !self.template.points.contains_key(point_name) {
            return Err(format!("unknown point {point_name:?}"));
        }
        if !position.is_finite() {
            return Err(format!("target {point_name:?}: position must be finite"));
        }
        self.target_overrides
            .insert(point_name.to_owned(), position);
        self.targets.insert(point_name.to_owned(), position);
        Ok(())
    }

    /// Resume rest + idle at the current game time, without snapping the point.
    pub fn clear_target(&mut self, point_name: &str) -> Result<(), String> {
        if !self.template.points.contains_key(point_name) {
            return Err(format!("unknown point {point_name:?}"));
        }
        self.target_overrides.remove(point_name);
        self.refresh_targets();
        Ok(())
    }

    pub fn snap_to_targets(&mut self) {
        self.points.clone_from(&self.targets);
        self.velocities.values_mut().for_each(|v| *v = Vec2::ZERO);
    }

    fn refresh_targets(&mut self) {
        self.targets = self.template.rest_positions();
        for idle in &self.template.idle {
            *self
                .targets
                .get_mut(&idle.point)
                .expect("validated idle point") += idle.offset_at(self.game_time_secs);
        }
        for (name, target) in &self.target_overrides {
            self.targets.insert(name.clone(), *target);
        }
    }

    /// Explicit game delta only. Idle uses game time; only spring integration is capped.
    pub fn step(&mut self, delta_seconds: f32) {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        self.game_time_secs += f64::from(delta_seconds);
        self.refresh_targets();
        let dt = delta_seconds.min(motion_defaults().max_step_secs);
        for (name, position) in &mut self.points {
            let follow = self
                .template
                .follow_overrides
                .get(name)
                .unwrap_or(&self.template.follow);
            let velocity = self
                .velocities
                .get_mut(name)
                .expect("runtime point velocity");
            let acceleration =
                follow.stiffness * (self.targets[name] - *position) - follow.damping * *velocity;
            *velocity += acceleration * dt;
            *position += *velocity * dt;
        }
    }

    /// Apply a reloaded template while retaining motion for surviving point names.
    pub fn preserve_motion_from(&mut self, previous: &Self) {
        self.game_time_secs = previous.game_time_secs;
        self.target_overrides = previous
            .target_overrides
            .iter()
            .filter(|(name, _)| self.template.points.contains_key(*name))
            .map(|(name, target)| (name.clone(), *target))
            .collect();
        self.refresh_targets();
        self.snap_to_targets();
        for (name, position) in &mut self.points {
            if let Some(previous_position) = previous.points.get(name) {
                *position = *previous_position;
                self.velocities
                    .insert(name.clone(), previous.velocities[name]);
            }
        }
    }

    pub fn part_transform(
        &self,
        part: &BodyPart,
        center: Vec2,
        box_size: Vec2,
        facing_left: bool,
    ) -> Result<PartTransform, String> {
        resolve_part(
            part,
            &self.points,
            center,
            box_size,
            self.visual.scale,
            facing_left,
        )
    }
}

/// Resolved pixel geometry. Rotation is clockwise in the screen's y-down coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct PartTransform {
    pub center: Vec2,
    pub size: Vec2,
    pub rotation: f32,
    pub mirrored: bool,
    pub endpoints: Option<(Vec2, Vec2)>,
}

impl PartTransform {
    /// Transform a shape vertex expressed in fractions of its own size.
    pub fn vertex(&self, local: Vec2) -> Vec2 {
        let mut offset = local * self.size;
        if self.mirrored {
            offset.x = -offset.x;
        }
        self.center + rotate(offset, self.rotation)
    }
}

fn rotate(point: Vec2, degrees: f32) -> Vec2 {
    let (sin, cos) = degrees.to_radians().sin_cos();
    Vec2::new(point.x * cos - point.y * sin, point.x * sin + point.y * cos)
}

pub fn resolve_point(
    name: &str,
    points: &PointPositions,
    center: Vec2,
    box_size: Vec2,
    scale: f32,
    facing_left: bool,
) -> Result<Vec2, String> {
    let point = points
        .get(name)
        .ok_or_else(|| format!("unknown point {name:?}"))?;
    let mut offset = *point * box_size * scale;
    if facing_left {
        offset.x = -offset.x;
    }
    Ok(center + offset)
}

pub fn resolve_part(
    part: &BodyPart,
    points: &PointPositions,
    center: Vec2,
    box_size: Vec2,
    scale: f32,
    facing_left: bool,
) -> Result<PartTransform, String> {
    let rotation = if facing_left {
        -part.rotation
    } else {
        part.rotation
    };
    let (position, endpoints) = if part.shape == Shape::Line {
        let from = resolve_point(
            part.from.as_deref().ok_or("Line missing from point")?,
            points,
            center,
            box_size,
            scale,
            facing_left,
        )?;
        let to = resolve_point(
            part.to.as_deref().ok_or("Line missing to point")?,
            points,
            center,
            box_size,
            scale,
            facing_left,
        )?;
        let midpoint = (from + to) / 2.0;
        (
            midpoint,
            Some((
                midpoint + rotate(from - midpoint, rotation),
                midpoint + rotate(to - midpoint, rotation),
            )),
        )
    } else {
        let anchor = resolve_point(
            part.anchor.as_deref().ok_or("missing anchor point")?,
            points,
            center,
            box_size,
            scale,
            facing_left,
        )?;
        let mut offset = Vec2::from(part.offset.ok_or("missing offset")?) * box_size * scale;
        if facing_left {
            offset.x = -offset.x;
        }
        (anchor + offset, None)
    };
    Ok(PartTransform {
        center: position,
        size: Vec2::from(part.size) * box_size * scale,
        rotation,
        mirrored: facing_left,
        endpoints,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn template() -> BodyTemplate {
        ron::from_str(include_str!("../tests/fixtures/bodies/small.ron")).unwrap()
    }

    fn near(actual: Vec2, expected: Vec2) {
        assert!(
            actual.distance(expected) < 0.0001,
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn resolves_rest_and_current_points_with_box_scale_and_mirroring() {
        let template = template();
        template.validate().unwrap();
        let mut points = template.rest_positions();
        let center = Vec2::new(100.0, 200.0);
        let size = Vec2::new(80.0, 40.0);
        near(
            resolve_point("root", &points, center, size, 2.0, false).unwrap(),
            Vec2::new(116.0, 184.0),
        );
        near(
            resolve_point("root", &points, center, size, 2.0, true).unwrap(),
            Vec2::new(84.0, 184.0),
        );
        points.insert("root".into(), Vec2::new(-0.2, 0.5));
        near(
            resolve_point("root", &points, center, size, 0.5, false).unwrap(),
            Vec2::new(92.0, 210.0),
        );
        assert_eq!(template.points["root"].rest, (0.1, -0.2));
        assert!(resolve_point("missing", &points, center, size, 1.0, false)
            .unwrap_err()
            .contains("unknown point \"missing\""));
    }

    #[test]
    fn transforms_anchor_offset_size_rotation_and_every_vertex_under_reflection() {
        let template = template();
        let points = template.rest_positions();
        let center = Vec2::new(100.0, 200.0);
        for part in &template.parts {
            let right =
                resolve_part(part, &points, center, Vec2::new(80.0, 40.0), 2.0, false).unwrap();
            let left =
                resolve_part(part, &points, center, Vec2::new(80.0, 40.0), 2.0, true).unwrap();
            near(
                left.center,
                Vec2::new(200.0 - right.center.x, right.center.y),
            );
            near(left.size, right.size);
            assert_eq!(left.rotation, -right.rotation);
            for vertex in [
                Vec2::new(-0.5, -0.5),
                Vec2::new(0.5, 0.0),
                Vec2::new(-0.5, 0.5),
            ] {
                let a = right.vertex(vertex);
                near(left.vertex(vertex), Vec2::new(200.0 - a.x, a.y));
            }
        }
        let rect = resolve_part(
            &template.parts[1],
            &points,
            center,
            Vec2::new(80.0, 40.0),
            2.0,
            false,
        )
        .unwrap();
        near(rect.center, Vec2::new(124.0, 192.0));
        near(rect.size, Vec2::new(32.0, 32.0));
        near(rect.vertex(Vec2::new(0.5, 0.0)), Vec2::new(124.0, 208.0));
    }

    #[test]
    fn line_connects_current_points_and_handles_coincident_endpoints_and_rotation() {
        let template = template();
        let mut points = template.rest_positions();
        let mut line = template.parts[0].clone();
        let a = resolve_part(
            &line,
            &points,
            Vec2::ZERO,
            Vec2::new(100.0, 100.0),
            1.0,
            false,
        )
        .unwrap();
        let (from, to) = a.endpoints.unwrap();
        near(from, Vec2::new(10.0, -20.0));
        near(to, Vec2::new(40.0, 30.0));
        line.rotation = 90.0;
        let right =
            resolve_part(&line, &points, Vec2::ZERO, Vec2::splat(100.0), 1.0, false).unwrap();
        let left = resolve_part(&line, &points, Vec2::ZERO, Vec2::splat(100.0), 1.0, true).unwrap();
        let (r1, r2) = right.endpoints.unwrap();
        let (l1, l2) = left.endpoints.unwrap();
        near(l1, Vec2::new(-r1.x, r1.y));
        near(l2, Vec2::new(-r2.x, r2.y));
        points.insert("tip".into(), points["root"]);
        let same =
            resolve_part(&line, &points, Vec2::ZERO, Vec2::splat(100.0), 1.0, false).unwrap();
        let (from, to) = same.endpoints.unwrap();
        near(from, to);
    }

    #[test]
    fn rejects_unknown_anchor_and_both_line_references_and_unknown_color_slot() {
        for (index, field) in [(1, "anchor"), (0, "from"), (0, "to")] {
            let mut template = template();
            match field {
                "anchor" => template.parts[index].anchor = Some("absent".into()),
                "from" => template.parts[index].from = Some("absent".into()),
                _ => template.parts[index].to = Some("absent".into()),
            }
            let error = template.validate().unwrap_err();
            assert!(
                error.contains(&template.parts[index].name)
                    && error.contains("unknown point \"absent\""),
                "{error}"
            );
        }
        let source = include_str!("../tests/fixtures/bodies/small.ron")
            .replace("color_slot: eye", "color_slot: unknown");
        let error = ron::from_str::<BodyTemplate>(&source)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("unknown") && error.contains("primary") && error.contains("eye"),
            "{error}"
        );
    }

    #[test]
    fn rejects_nonfinite_geometry_invalid_sizes_and_mixed_line_fields() {
        let mut bad = template();
        bad.parts[0].anchor = Some("root".into());
        assert!(bad
            .validate()
            .unwrap_err()
            .contains("instead of anchor/offset"));
        for value in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            let mut bad = template();
            bad.parts[1].size.0 = value;
            assert!(bad.validate().unwrap_err().contains("size"));
        }
        let mut bad = template();
        bad.parts[1].rotation = f32::NAN;
        assert!(bad.validate().unwrap_err().contains("rotation"));
        let mut bad = template();
        bad.parts[1].offset = Some((f32::INFINITY, 0.0));
        assert!(bad.validate().unwrap_err().contains("offset"));
        let mut bad = template();
        bad.points.get_mut("root").unwrap().rest.0 = f32::NAN;
        assert!(bad.validate().unwrap_err().contains("root"));
    }
}
