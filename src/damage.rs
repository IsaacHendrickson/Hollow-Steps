//! Pure resolution of everything that can hurt the player in one tick.
//!
//! `DamageResolver::gather` reads the sources from a room, and `resolve` turns
//! them into the single hit to apply. The rules: the strongest source sets the
//! damage, and an attack hit (which carries knockback) is used only when its
//! damage is at least that strongest value. With several such hits the highest
//! wins, and the later one wins a tie (enemies are listed before the boss).

use macroquad::math::Vec2;

use crate::enemy::Enemy;
use crate::enemy_attack::DamageHit;
use crate::room::Room;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DamageResolver {
    /// Contact damage of the strongest enemy touching the player.
    pub enemy_contact: u16,
    /// Strongest blast or attack damage landing from any enemy (`Enemy::action_damage`).
    pub enemy_action: u16,
    /// Boss body contact damage (zero once it is defeated).
    pub boss_contact: u16,
    /// Damage events from the boss's action sequence this tick; capped at 65535.
    pub boss_action: u32,
    /// Attack and projectile hits that carry knockback: enemies' first, then the boss's.
    pub attacks: Vec<DamageHit>,
}

impl DamageResolver {
    pub fn gather(room: &Room, player: Vec2, size: Vec2, boss_action: u32) -> Self {
        let boss = room.boss_special.as_ref();
        Self {
            enemy_contact: room
                .enemies
                .iter()
                .filter(|enemy| enemy.touches(player, size))
                .map(Enemy::contact_damage)
                .max()
                .unwrap_or(0),
            enemy_action: room
                .enemies
                .iter()
                .map(|enemy| enemy.action_damage(player, size))
                .max()
                .unwrap_or(0),
            boss_contact: boss
                .filter(|boss| {
                    let separation = (boss.position() - player).abs();
                    let reach = (boss.size() + size) / 2.0;
                    separation.x <= reach.x && separation.y <= reach.y
                })
                .map_or(0, |boss| boss.contact_damage()),
            boss_action,
            attacks: room
                .enemies
                .iter()
                .filter_map(|enemy| enemy.attack_hit(player, size))
                .chain(boss.and_then(|boss| boss.attack_hit(player, size)))
                .collect(),
        }
    }

    pub fn resolve(&self) -> DamageHit {
        let base_damage = self
            .enemy_contact
            .max(self.enemy_action)
            .max(self.boss_contact)
            .max(self.boss_action.min(u32::from(u16::MAX)) as u16);
        self.attacks
            .iter()
            .copied()
            .filter(|hit| hit.damage >= i32::from(base_damage))
            .max_by_key(|hit| hit.damage)
            .unwrap_or(DamageHit {
                damage: i32::from(base_damage),
                knockback: Vec2::ZERO,
                knockback_secs: 0.0,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::test_rooms;

    const SIZE: Vec2 = Vec2::new(32.0, 48.0);

    fn hit(damage: i32, knockback_x: f32) -> DamageHit {
        DamageHit {
            damage,
            knockback: Vec2::new(knockback_x, -50.0),
            knockback_secs: 0.25,
        }
    }

    fn plain(damage: i32) -> DamageHit {
        DamageHit {
            damage,
            knockback: Vec2::ZERO,
            knockback_secs: 0.0,
        }
    }

    #[test]
    fn no_sources_deal_no_damage_and_no_knockback() {
        assert_eq!(DamageResolver::default().resolve(), plain(0));
    }

    #[test]
    fn each_source_alone_sets_the_damage_without_knockback() {
        let sources = [
            DamageResolver {
                enemy_contact: 2,
                ..Default::default()
            },
            DamageResolver {
                enemy_action: 2,
                ..Default::default()
            },
            DamageResolver {
                boss_contact: 2,
                ..Default::default()
            },
            DamageResolver {
                boss_action: 2,
                ..Default::default()
            },
        ];
        for source in sources {
            assert_eq!(source.resolve(), plain(2), "{source:?}");
        }
    }

    #[test]
    fn the_strongest_source_wins_whatever_its_kind() {
        let mix = |enemy_contact, enemy_action, boss_contact, boss_action| DamageResolver {
            enemy_contact,
            enemy_action,
            boss_contact,
            boss_action,
            attacks: Vec::new(),
        };
        assert_eq!(mix(1, 2, 3, 2).resolve().damage, 3);
        assert_eq!(mix(4, 2, 3, 2).resolve().damage, 4);
        assert_eq!(mix(1, 5, 3, 2).resolve().damage, 5);
        assert_eq!(mix(1, 2, 3, 6).resolve().damage, 6);
        assert_eq!(mix(0, 0, 0, 0).resolve().damage, 0);
    }

    #[test]
    fn boss_action_damage_is_capped_at_65535() {
        let sources = DamageResolver {
            boss_action: 100_000,
            ..Default::default()
        };
        assert_eq!(sources.resolve().damage, 65535);
    }

    #[test]
    fn an_attack_hit_at_least_as_strong_as_the_rest_keeps_its_knockback() {
        let sources = DamageResolver {
            enemy_contact: 1,
            enemy_action: 2,
            attacks: vec![hit(2, 100.0)],
            ..Default::default()
        };
        assert_eq!(sources.resolve(), hit(2, 100.0));
        let stronger = DamageResolver {
            attacks: vec![hit(3, 100.0)],
            ..sources
        };
        assert_eq!(stronger.resolve(), hit(3, 100.0));
    }

    #[test]
    fn a_weaker_attack_hit_is_dropped_and_the_strongest_source_applies_without_knockback() {
        let sources = DamageResolver {
            boss_action: 3,
            attacks: vec![hit(2, 100.0)],
            ..Default::default()
        };
        assert_eq!(sources.resolve(), plain(3));
    }

    #[test]
    fn the_highest_attack_hit_wins_and_a_tie_goes_to_the_later_hit() {
        let sources = DamageResolver {
            attacks: vec![hit(1, 1.0), hit(3, 2.0), hit(2, 3.0)],
            ..Default::default()
        };
        assert_eq!(sources.resolve(), hit(3, 2.0));
        let tie = DamageResolver {
            attacks: vec![hit(3, 1.0), hit(3, 2.0)],
            ..Default::default()
        };
        assert_eq!(tie.resolve(), hit(3, 2.0));
    }

    #[test]
    fn extreme_attack_hits_are_kept_for_the_player_to_clamp() {
        let sources = DamageResolver {
            enemy_action: 255,
            attacks: vec![hit(300, 1.0), hit(-4, 2.0)],
            ..Default::default()
        };
        assert_eq!(sources.resolve(), hit(300, 1.0));
        let negative_only = DamageResolver {
            attacks: vec![hit(-4, 2.0)],
            ..Default::default()
        };
        assert_eq!(negative_only.resolve(), plain(0));
    }

    #[test]
    fn a_zero_damage_hit_passes_through_when_nothing_else_hurts() {
        let sources = DamageResolver {
            attacks: vec![hit(0, 5.0)],
            ..Default::default()
        };
        assert_eq!(sources.resolve(), hit(0, 5.0));
    }

    #[test]
    fn gather_reads_enemy_contact_only_while_touching() {
        let [room, _, _] = test_rooms();
        let enemy = &room.enemies[0];
        let touching = DamageResolver::gather(&room, enemy.position(), SIZE, 0);
        assert_eq!(touching.enemy_contact, enemy.contact_damage());
        assert!(touching.enemy_contact > 0);
        let apart = DamageResolver::gather(&room, Vec2::new(-5_000.0, -5_000.0), SIZE, 0);
        assert_eq!(apart.enemy_contact, 0);
        assert_eq!(apart.enemy_action, 0);
        assert!(apart.attacks.is_empty());
    }

    #[test]
    fn gather_passes_boss_action_through_and_reads_boss_contact_until_defeated() {
        let [_, _, mut room] = test_rooms();
        let mut definition = crate::boss_definition::BossDefinition::from_ron(include_str!(
            "../tests/fixtures/bosses/ground_pound.ron"
        ))
        .unwrap();
        definition.pause_seconds = 1_000.0;
        room.boss_special = Some(
            crate::boss_special::BossSpecials::from_definition(
                std::sync::Arc::new(definition),
                Vec2::new(560.0, 436.0),
            )
            .unwrap(),
        );
        let on_boss = Vec2::new(560.0, 436.0);
        let touching = DamageResolver::gather(&room, on_boss, SIZE, 7);
        assert_eq!(touching.boss_contact, 20);
        assert_eq!(touching.boss_action, 7);
        let apart = DamageResolver::gather(&room, Vec2::new(100.0, 436.0), SIZE, 0);
        assert_eq!(apart.boss_contact, 0);
        room.boss_special.as_mut().unwrap().damage_boss(100);
        assert_eq!(
            DamageResolver::gather(&room, on_boss, SIZE, 0).boss_contact,
            0
        );
        room.boss_special = None;
        assert_eq!(
            DamageResolver::gather(&room, on_boss, SIZE, 0).boss_contact,
            0
        );
    }
}
