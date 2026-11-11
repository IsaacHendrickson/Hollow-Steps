//! How a boss drives its optional body: building it, stepping it, the hurt flash, facing and
//! the death fade. The hitbox is always the definition's `size`; the body is only a picture.

use std::sync::Arc;

use super::{BossSpecials, DEATH_EFFECT_SECONDS};
use crate::behavior::Action;
use crate::body::{BodyInstance, BodyTemplates, BodyVisual};
use crate::enemy_attack::AttackFeedback;
use crate::pose::{PoseStep, Poses};

/// A boss's body and the pose catalog its hurt feedback draws on.
#[derive(Debug)]
pub(super) struct BossBody {
    instance: BodyInstance,
    poses: Arc<Poses>,
    /// Seconds the pose steps already advanced the body this update.
    stepped: f32,
}

impl BossBody {
    pub(super) fn new(
        visual: BodyVisual,
        templates: &BodyTemplates,
        poses: Arc<Poses>,
    ) -> Result<Self, String> {
        Ok(Self {
            instance: BodyInstance::new(visual, templates)?,
            poses,
            stepped: 0.0,
        })
    }
}

impl BossSpecials {
    /// The boss's body, or `None` while it is drawn as a plain box.
    pub fn body(&self) -> Option<&BodyInstance> {
        self.body.as_ref().map(|body| &body.instance)
    }

    pub fn facing_left(&self) -> bool {
        self.facing_left
    }

    /// Drawing opacity: `1` while alive, fading to `0` as the death effect plays out.
    pub fn opacity(&self) -> f32 {
        if self.defeated {
            (self.death_effect_remaining / DEATH_EFFECT_SECONDS).clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    /// The warning tint (feedback and strength) the body shows: an attack's wind-up, or a
    /// plain `telegraph` action pulsing with the feedback of the action that follows it.
    pub fn body_warning(&self) -> Option<(AttackFeedback, f32)> {
        if let Some(warning) = self.warning() {
            return Some((warning.feedback, warning.strength));
        }
        let active = self.active?;
        let actions = &self.definition.attacks[active.index].actions;
        let Action::Telegraph { seconds } = actions.get(active.action)? else {
            return None;
        };
        if *seconds <= 0.0 {
            return None;
        }
        let feedback = actions
            .get(active.action + 1)
            .and_then(Action::feedback)
            .unwrap_or_else(crate::enemy_attack::default_feedback);
        let progress = (self.runtime.action_elapsed / f64::from(*seconds)) as f32;
        Some((
            feedback,
            crate::enemy_attack::warning_strength(feedback, progress),
        ))
    }

    pub(super) fn hurt_body(&mut self) {
        if let Some(body) = &mut self.body {
            body.instance.hurt(&body.poses);
        }
    }

    /// Plays the pose steps the boss's actions recorded, the way an enemy does. A boss without
    /// a body ignores everything but a facing step.
    pub(super) fn apply_pose_steps(&mut self, steps: Vec<PoseStep>) {
        for step in steps {
            if let PoseStep::Face(left) = step {
                self.facing_left = left;
                continue;
            }
            let Some(body) = &mut self.body else {
                continue;
            };
            match step {
                PoseStep::Face(_) => {}
                PoseStep::Apply {
                    name,
                    seconds,
                    blend_secs,
                    stiffness_multiplier,
                    instant,
                } => {
                    body.instance.set_pose(name.as_deref(), &body.poses);
                    body.instance
                        .configure_pose_spring(blend_secs, stiffness_multiplier);
                    if instant {
                        body.instance.instant_pose_impulse();
                    }
                    body.instance.step(seconds);
                    body.stepped += seconds;
                }
                PoseStep::Clear => body.instance.clear_pose(),
                PoseStep::Idle(seconds) => {
                    body.instance.step(seconds);
                    body.stepped += seconds;
                }
            }
        }
    }

    /// Faces the direction of travel while moving and the player while idle, drops a finished
    /// pose, then advances the body by whatever part of `dt` the pose steps did not cover.
    pub(super) fn update_look(&mut self, dt: f32, previous_x: f32, player_x: f32) {
        if self.position.x != previous_x {
            self.facing_left = self.position.x < previous_x;
        } else if self.active.is_none() && player_x != self.position.x {
            self.facing_left = player_x < self.position.x;
        }
        // Glides outlast the attack that started them, so a pose set by a `pose` action holds
        // until the boss is idle and has arrived; then it returns to the idle pose.
        let settled = self.active.is_none() && self.position == self.target_position;
        if let Some(body) = &mut self.body {
            if settled {
                body.instance.clear_pose();
            }
            let remainder = (dt - std::mem::take(&mut body.stepped)).max(0.0);
            body.instance.step(remainder);
        }
    }
}
