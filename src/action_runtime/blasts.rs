//! Timed ground blasts: the boss `legacy_area_blast` fuse and the paired `spawn_rings` /
//! `explode_rings` actions that plant rings first and detonate them later.
use super::{ActionActor, ActionOutput, ActionRuntime};
use crate::behavior::{Action, BehaviorContext, Blast};

impl ActionRuntime {
    pub(super) fn legacy_area_blast(
        &mut self,
        action: &Action,
        context: BehaviorContext,
        remaining: f64,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::AreaBlast {
            count,
            radius,
            fuse_time,
            damage,
        } = *action
        else {
            unreachable!("legacy_area_blast runs only for legacy_area_blast")
        };
        let legacy_boss = actor.legacy_target().is_some();
        let blasts = if legacy_boss {
            &mut self.rings
        } else {
            &mut self.blasts
        };
        if !self.action_started {
            if legacy_boss {
                blasts.clear();
            }
            for _ in 0..count {
                blasts.push(Blast {
                    position: actor.random_arena_position(),
                    radius,
                    damage,
                    exploded: false,
                });
            }
        }
        let duration = f64::from(fuse_time);
        let used = remaining.min((duration - self.action_elapsed).max(0.0));
        self.action_elapsed += used;
        let finished = self.action_elapsed >= duration;
        if finished {
            for blast in blasts.iter_mut() {
                blast.exploded = true;
            }
            if legacy_boss
                && blasts
                    .iter()
                    .any(|b| b.position.distance(context.player_position) <= radius)
            {
                actor.emit(ActionOutput::Damage(u32::from(damage)));
            }
        }
        (used, finished)
    }

    /// Plants the rings; the radius comes from the `explode_rings` action that follows.
    pub(super) fn spawn_rings(
        &mut self,
        action: &Action,
        preview: &Action,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::SpawnRings { count } = *action else {
            unreachable!("spawn_rings runs only for spawn_rings")
        };
        let radius = match preview {
            Action::ExplodeRings { radius, .. } => *radius,
            _ => 0.0,
        };
        self.rings = (0..count)
            .map(|_| Blast {
                position: actor.random_arena_position(),
                radius,
                damage: 0,
                exploded: false,
            })
            .collect();
        (0.0, true)
    }

    pub(super) fn explode_rings(
        &mut self,
        action: &Action,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::ExplodeRings { radius, damage } = *action else {
            unreachable!("explode_rings runs only for explode_rings")
        };
        if self
            .rings
            .iter()
            .any(|r| !r.exploded && r.position.distance(context.player_position) <= radius)
        {
            actor.emit(ActionOutput::Damage(u32::from(damage)));
        }
        for ring in &mut self.rings {
            ring.radius = radius;
            ring.damage = damage;
            ring.exploded = true;
        }
        (0.0, true)
    }
}
