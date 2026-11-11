//! Actions that finish in the tick they start: boss glides, the horizontal damage checks and
//! the summon. None of them use up action time, so each returns `(0.0, true)`.
use super::{ActionActor, ActionOutput, ActionRuntime};
use crate::behavior::{Action, BehaviorContext};
use macroquad::math::Vec2;

impl ActionRuntime {
    pub(super) fn glide_to_player(
        action: &Action,
        position: Vec2,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::GlideToPlayer { speed, above_floor } = *action else {
            unreachable!("glide_to_player runs only for glide_to_player")
        };
        let target = actor.legacy_target().unwrap_or(context.player_position);
        actor.emit(ActionOutput::Glide {
            target: Vec2::new(
                target.x,
                if above_floor == 0.0 {
                    position.y
                } else {
                    context.bounds.floor_y - above_floor
                },
            ),
            speed,
        });
        (0.0, true)
    }

    pub(super) fn glide_to_floor(
        action: &Action,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::GlideToFloor { speed } = *action else {
            unreachable!("glide_to_floor runs only for glide_to_floor")
        };
        let target = actor.legacy_target().unwrap_or(context.player_position);
        actor.emit(ActionOutput::Glide {
            target: Vec2::new(target.x, context.bounds.floor_y - context.size.y / 2.0),
            speed,
        });
        (0.0, true)
    }

    pub(super) fn damage_target(
        action: &Action,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::DamageTarget { radius, damage } = *action else {
            unreachable!("damage_target runs only for damage_target")
        };
        let x = actor.legacy_target().unwrap_or(context.player_position).x;
        Self::horizontal_hit(x, radius, damage, context, actor)
    }

    pub(super) fn spawn_shockwave(
        action: &Action,
        position: Vec2,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::SpawnShockwave { radius, damage } = *action else {
            unreachable!("spawn_shockwave runs only for spawn_shockwave")
        };
        Self::horizontal_hit(position.x, radius, damage, context, actor)
    }

    /// Historical immediate horizontal-radius check; deliberately ignores y and player size.
    fn horizontal_hit(
        x: f32,
        radius: f32,
        damage: u8,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        if (context.player_position.x - x).abs() <= radius {
            actor.emit(ActionOutput::Damage(u32::from(damage)));
        }
        (0.0, true)
    }

    pub(super) fn summon(
        action: &Action,
        context: BehaviorContext,
        actor: &mut impl ActionActor,
    ) -> (f64, bool) {
        let Action::Summon { inset } = *action else {
            unreachable!("summon runs only for summon")
        };
        let inset = inset.clamp(
            crate::enemy::ENEMY_SIZE.x / 2.0,
            (context.bounds.right - context.bounds.left) / 2.0,
        );
        let y = context.bounds.floor_y - crate::enemy::ENEMY_SIZE.y / 2.0;
        actor.emit(ActionOutput::Spawn(Vec2::new(
            context.bounds.left + inset,
            y,
        )));
        actor.emit(ActionOutput::Spawn(Vec2::new(
            context.bounds.right - inset,
            y,
        )));
        (0.0, true)
    }
}
