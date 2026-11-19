//! A live body: point springs, pose playback, hurt flash and the transforms of its parts.
use std::collections::BTreeSet;
use std::sync::Arc;

use macroquad::math::Vec2;

use super::geometry::{resolve_part, PartTransform};
use super::template::{motion_defaults, BodyPart, BodyTemplate, BodyTemplates, PointPositions};
use super::visual::BodyVisual;

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
    pub pose_name: Option<String>,
    pose_points: BTreeSet<String>,
    pose_frequency: Option<f64>,
    action_pose: Option<String>,
    action_frequency: Option<f64>,
    hurt_remaining: f32,
    flash_remaining: f32,
    pose_catalog: crate::pose::Poses,
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
            pose_name: None,
            pose_points: BTreeSet::new(),
            pose_frequency: None,
            action_pose: None,
            action_frequency: None,
            hurt_remaining: 0.0,
            flash_remaining: 0.0,
            pose_catalog: crate::pose::Poses::new(),
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

    pub fn set_pose(&mut self, name: Option<&str>, poses: &crate::pose::Poses) {
        self.action_pose = name.map(str::to_owned);
        self.pose_catalog = poses.clone();
        if self.hurt_remaining > 0.0 {
            return;
        }
        self.apply_pose(name, poses);
    }

    fn apply_pose(&mut self, name: Option<&str>, poses: &crate::pose::Poses) {
        if self.pose_name.as_deref() == name {
            return;
        }
        self.clear_active_pose();
        if let Some((name, pose)) = name.and_then(|n| poses.get(n).map(|p| (n, p))) {
            self.pose_points = pose.resolve(&self.template).into_keys().collect();
            pose.apply(self);
            self.pose_name = Some(name.to_owned());
        }
    }

    pub fn clear_pose(&mut self) {
        self.action_pose = None;
        self.action_frequency = None;
        if self.hurt_remaining <= 0.0 {
            self.clear_active_pose();
        }
    }
    fn clear_active_pose(&mut self) {
        for name in std::mem::take(&mut self.pose_points) {
            self.clear_target(&name).expect("resolved pose point");
        }
        self.pose_name = None;
        self.pose_frequency = None;
    }

    pub fn configure_pose_spring(&mut self, blend_secs: f32, stiffness_multiplier: f32) {
        self.action_frequency = self.action_pose.as_ref().map(|_| {
            f64::from(self.template.pose_motion.settle_factor)
                / f64::from(blend_secs.max(self.template.pose_motion.min_blend_secs))
                * f64::from(stiffness_multiplier).sqrt()
        });
        if self.hurt_remaining <= 0.0 {
            self.pose_frequency = self.action_frequency;
        }
    }

    /// Exact critically damped spring solution, stable even for very short wind-ups.
    fn integrate_pose(&mut self, seconds: f32) {
        let Some(omega) = self.pose_frequency else {
            return;
        };
        let dt = f64::from(seconds);
        let decay = (-omega * dt).exp();
        for name in &self.pose_points {
            let target = self.targets[name].as_dvec2();
            let position = self.points.get_mut(name).expect("pose point");
            let velocity = self.velocities.get_mut(name).expect("pose velocity");
            let error = position.as_dvec2() - target;
            let c = (velocity.as_dvec2() + omega * error) * dt;
            *position = (target + (error + c) * decay).as_vec2();
            *velocity = ((velocity.as_dvec2() - omega * c) * decay).as_vec2();
        }
    }

    pub fn instant_pose_impulse(&mut self) {
        if self.hurt_remaining > 0.0 {
            return;
        }
        self.integrate_pose(self.template.pose_motion.instant_strike_secs);
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
    pub fn hurt(&mut self, poses: &crate::pose::Poses) {
        self.pose_catalog = poses.clone();
        let feedback = self.visual.hurt.clone();
        self.flash_remaining = feedback.flash_secs;
        self.hurt_remaining = feedback.duration_secs;
        if self.hurt_remaining > 0.0 {
            self.apply_pose(feedback.pose.as_deref(), poses);
            self.pose_frequency = self.pose_name.as_ref().map(|_| {
                f64::from(self.template.pose_motion.settle_factor)
                    / f64::from(
                        feedback
                            .blend_secs
                            .max(self.template.pose_motion.min_blend_secs),
                    )
            });
        }
    }
    pub fn flash_strength(&self) -> f32 {
        if self.flash_remaining > 0.0 {
            self.visual.hurt.tint_strength
        } else {
            0.0
        }
    }
    pub fn step(&mut self, delta_seconds: f32) {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        self.flash_remaining = (self.flash_remaining - delta_seconds).max(0.0);
        let hurt_dt = delta_seconds.min(self.hurt_remaining);
        if hurt_dt > 0.0 {
            self.step_motion(hurt_dt);
            self.hurt_remaining = (self.hurt_remaining - hurt_dt).max(0.0);
            if self.hurt_remaining == 0.0 {
                self.apply_pose(
                    self.action_pose.clone().as_deref(),
                    &self.pose_catalog.clone(),
                );
                self.pose_frequency = self.action_frequency;
            }
        }
        self.step_motion(delta_seconds - hurt_dt);
    }
    fn step_motion(&mut self, delta_seconds: f32) {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        self.game_time_secs += f64::from(delta_seconds);
        self.refresh_targets();
        self.integrate_pose(delta_seconds);
        let dt = delta_seconds.min(motion_defaults().max_step_secs);
        for (name, position) in &mut self.points {
            if self.pose_frequency.is_some() && self.pose_points.contains(name) {
                continue;
            }
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
            .filter(|(name, _)| {
                self.template.points.contains_key(*name) && !previous.pose_points.contains(*name)
            })
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
