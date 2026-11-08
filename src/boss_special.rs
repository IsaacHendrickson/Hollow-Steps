use macroquad::math::Vec2;

use crate::coin::Coin;
use crate::enemy::Enemy;
use crate::player::WorldBounds;

pub const SLAM_TELEGRAPH_SECONDS: f32 = 0.5;
pub const SUMMON_WINDUP_SECONDS: f32 = 1.0;
pub const RINGS_FUSE_SECONDS: f32 = 1.0;
pub const DEATH_EFFECT_SECONDS: f32 = 0.25;
pub const SLAM_RADIUS: f32 = 56.0;
pub const RING_RADIUS: f32 = 48.0;
pub const BOSS_MAX_HEALTH: u32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialAttack {
    Slam,
    Summon,
    Rings,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ring {
    position: Vec2,
    exploded: bool,
}

impl Ring {
    pub fn position(&self) -> Vec2 {
        self.position
    }
    pub fn exploded(&self) -> bool {
        self.exploded
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveAttack {
    kind: SpecialAttack,
    elapsed: f32,
    target_x: f32,
}

#[derive(Debug)]
pub struct BossSpecials {
    position: Vec2,
    health: u32,
    active: Option<ActiveAttack>,
    rings: Vec<Ring>,
    summoned_enemies: Vec<Enemy>,
    coins: Vec<Coin>,
    death_effect_remaining: f32,
    defeated: bool,
    random_state: u32,
}

impl BossSpecials {
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            health: BOSS_MAX_HEALTH,
            active: None,
            rings: Vec::new(),
            summoned_enemies: Vec::new(),
            coins: Vec::new(),
            death_effect_remaining: 0.0,
            defeated: false,
            random_state: 0x51a7_2026,
        }
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }
    pub fn health(&self) -> u32 {
        self.health
    }
    pub fn is_defeated(&self) -> bool {
        self.defeated
    }
    pub fn is_telegraphing(&self) -> bool {
        self.active.is_some()
    }
    pub fn active_attack(&self) -> Option<SpecialAttack> {
        self.active.map(|attack| attack.kind)
    }
    pub fn rings(&self) -> &[Ring] {
        &self.rings
    }
    pub fn summoned_enemies(&self) -> &[Enemy] {
        &self.summoned_enemies
    }
    pub fn coins(&self) -> &[Coin] {
        &self.coins
    }
    pub fn is_death_effect_active(&self) -> bool {
        self.death_effect_remaining > 0.0
    }

    pub fn start_attack(
        &mut self,
        kind: SpecialAttack,
        player_position: Vec2,
        bounds: WorldBounds,
    ) {
        if self.defeated || self.active.is_some() {
            return;
        }
        let target_x = player_position.x.clamp(bounds.left, bounds.right);
        self.position.x = target_x;
        if kind == SpecialAttack::Slam {
            self.position.y = bounds.floor_y - 180.0;
        }
        self.active = Some(ActiveAttack {
            kind,
            elapsed: 0.0,
            target_x,
        });
        if kind == SpecialAttack::Rings {
            self.rings = (0..3)
                .map(|_| Ring {
                    position: self.random_arena_position(bounds),
                    exploded: false,
                })
                .collect();
        }
    }

    pub fn damage_boss(&mut self, amount: u32) {
        if self.defeated {
            return;
        }
        self.health = self.health.saturating_sub(amount);
        if self.health == 0 {
            self.defeated = true;
            self.active = None;
            self.death_effect_remaining = DEATH_EFFECT_SECONDS;
        }
    }

    /// Advances the framework and returns one damage event when an attack lands.
    pub fn update(
        &mut self,
        delta_seconds: f32,
        player_position: Vec2,
        bounds: WorldBounds,
    ) -> u32 {
        let dt = delta_seconds.max(0.0);
        if self.defeated {
            if self.death_effect_remaining > 0.0 {
                self.death_effect_remaining = (self.death_effect_remaining - dt).max(0.0);
                if self.death_effect_remaining == 0.0 && self.coins.is_empty() {
                    self.coins = (0..20)
                        .map(|_| Coin {
                            position: self.random_arena_position(bounds),
                        })
                        .collect();
                }
            }
            return 0;
        }

        let Some(attack) = self.active.as_mut() else {
            return 0;
        };
        attack.elapsed += dt;
        let threshold = match attack.kind {
            SpecialAttack::Slam => SLAM_TELEGRAPH_SECONDS,
            SpecialAttack::Summon | SpecialAttack::Rings => SUMMON_WINDUP_SECONDS,
        };
        if attack.elapsed < threshold {
            return 0;
        }

        let kind = attack.kind;
        let target_x = attack.target_x;
        self.active = None;
        match kind {
            SpecialAttack::Slam => {
                self.position.x = target_x;
                self.position.y = bounds.floor_y - 24.0;
                u32::from((player_position.x - self.position.x).abs() <= SLAM_RADIUS)
            }
            SpecialAttack::Summon => {
                self.summoned_enemies.extend([
                    Enemy::new(bounds.left + 120.0, bounds.left + 120.0, 0.0, 0.0, bounds),
                    Enemy::new(bounds.right - 120.0, bounds.right - 120.0, 0.0, 0.0, bounds),
                ]);
                0
            }
            SpecialAttack::Rings => {
                for ring in &mut self.rings {
                    ring.exploded = true;
                }
                u32::from(
                    self.rings
                        .iter()
                        .any(|ring| ring.position.distance(player_position) <= RING_RADIUS),
                )
            }
        }
    }

    fn random_arena_position(&mut self, bounds: WorldBounds) -> Vec2 {
        let x = bounds.left + self.next_random() * (bounds.right - bounds.left);
        let y = 80.0 + self.next_random() * (bounds.floor_y - 80.0);
        Vec2::new(x, y)
    }

    fn next_random(&mut self) -> f32 {
        self.random_state = self
            .random_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.random_state as f32 / u32::MAX as f32
    }
}
