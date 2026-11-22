//! `DamageResolver::gather_scaled`: what can hurt the player at a difficulty tier
//! (`crate::difficulty`). Enemy contact, enemy attacks, projectiles and blasts, and the boss's
//! contact and attacks are multiplied; a hazard's hit is not. Layered on while gathering, so
//! nothing authored changes.
use macroquad::math::Vec2;

use super::DamageResolver;
use crate::difficulty::TierScale;
use crate::enemy::Enemy;
use crate::room::Room;

impl DamageResolver {
    /// `gather` with the enemy and boss damage of `scale` applied. Hazards are listed first in
    /// `attacks` and keep their damage and knockback; the knockback of every hit is unchanged.
    /// With `TierScale::NEUTRAL` this is exactly `gather`.
    pub fn gather_scaled(
        room: &Room,
        player: Vec2,
        velocity: Vec2,
        size: Vec2,
        boss_action: u32,
        scale: &TierScale,
    ) -> Self {
        let boss = room.boss_special.as_ref();
        let enemy_hit = |mut hit: crate::enemy_attack::DamageHit| {
            hit.damage = scale.enemy_hit_damage(hit.damage);
            hit
        };
        let boss_hit = |mut hit: crate::enemy_attack::DamageHit| {
            hit.damage = scale.boss_hit_damage(hit.damage);
            hit
        };
        Self {
            enemy_contact: room
                .enemies
                .iter()
                .filter(|enemy| enemy.touches(player, size))
                .map(Enemy::contact_damage)
                .max()
                .map_or(0, |damage| scaled_u16(scale.enemy_damage(damage.into()))),
            enemy_action: room
                .enemies
                .iter()
                .map(|enemy| enemy.action_damage(player, size))
                .max()
                .map_or(0, |damage| scaled_u16(scale.enemy_damage(damage.into()))),
            boss_contact: boss
                .filter(|boss| {
                    let separation = (boss.position() - player).abs();
                    let reach = (boss.size() + size) / 2.0;
                    separation.x <= reach.x && separation.y <= reach.y
                })
                .map_or(0, |boss| {
                    scaled_u16(scale.boss_damage(boss.contact_damage().into()))
                }),
            boss_action: scale.boss_damage(boss_action),
            attacks: room
                .hazards
                .iter()
                .filter_map(|hazard| hazard.hit(player, size, velocity))
                .chain(
                    room.enemies
                        .iter()
                        .filter_map(|enemy| enemy.attack_hit(player, size))
                        .map(enemy_hit),
                )
                .chain(
                    boss.and_then(|boss| boss.attack_hit(player, size))
                        .map(boss_hit),
                )
                .collect(),
        }
    }
}

/// A scaled damage as the resolver's `u16` fields hold it (the player clamps again later).
fn scaled_u16(damage: u32) -> u16 {
    damage.min(u32::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests;
