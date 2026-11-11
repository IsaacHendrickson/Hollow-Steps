//! Pose bookkeeping around every executed action: which pose each attack phase plays, how
//! fast it blends, and the idle time of actions that have no pose.
use super::ActionRuntime;
use crate::behavior::Action;
use crate::pose::PoseStep;
use macroquad::math::Vec2;

/// What the previewed action asks of the pose layer, read once before the action runs.
pub(super) struct PoseCue<'a> {
    bindings: Option<[&'a Option<String>; 3]>,
    blend_secs: f32,
    strike_multiplier: f32,
}

impl ActionRuntime {
    /// A `pose` action: takes no time and holds the named pose (or returns to idle) until the
    /// next pose step.
    pub(super) fn pose_action(&mut self, action: &Action) -> (f64, bool) {
        let Action::Pose {
            pose_name,
            blend_secs,
        } = action
        else {
            unreachable!("pose_action runs only for pose")
        };
        self.pose_steps.push(match pose_name {
            Some(name) => PoseStep::Apply {
                name: Some(name.clone()),
                seconds: 0.0,
                blend_secs: *blend_secs,
                stiffness_multiplier: 1.0,
                instant: false,
            },
            None => PoseStep::Clear,
        });
        (0.0, true)
    }

    pub(super) fn begin_poses<'a>(
        &mut self,
        action: &Action,
        preview: &'a Action,
        position: Vec2,
        player_x: f32,
    ) -> PoseCue<'a> {
        let bindings = preview.pose_binding_refs();
        let (mut blend_secs, strike_multiplier) = preview.pose_timing();
        if let Action::Telegraph { seconds } = *action {
            blend_secs = preview.pose_blend_seconds(seconds);
        }
        if !self.action_started && bindings.is_some() {
            self.pose_steps.push(PoseStep::Face(player_x < position.x));
        }
        PoseCue {
            bindings,
            blend_secs,
            strike_multiplier,
        }
    }

    pub(super) fn record_poses(
        &mut self,
        action: &Action,
        cue: &PoseCue,
        used: f64,
        finished: bool,
    ) {
        let PoseCue {
            bindings,
            blend_secs,
            strike_multiplier,
        } = *cue;
        if let (Some(bindings), Some(runtime)) = (bindings, &self.attack_runtime) {
            for (phase, seconds) in runtime.pose_spans(used) {
                self.pose_steps.push(PoseStep::Apply {
                    name: bindings[phase].clone(),
                    seconds,
                    blend_secs,
                    stiffness_multiplier: if phase == 1 { strike_multiplier } else { 1.0 },
                    instant: phase == 1 && seconds == 0.0 && finished,
                });
            }
            if finished {
                self.pose_steps.push(PoseStep::Clear);
            }
        } else if let (Action::Telegraph { .. }, Some(bindings)) = (action, bindings) {
            self.pose_steps.push(PoseStep::Apply {
                name: bindings[0].clone(),
                seconds: used as f32,
                blend_secs,
                stiffness_multiplier: 1.0,
                instant: false,
            });
        } else {
            self.pose_steps.push(PoseStep::Idle(used as f32));
        }
    }
}
