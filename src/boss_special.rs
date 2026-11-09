use std::sync::Arc;

use macroquad::math::Vec2;

use crate::behavior::Action;
use crate::boss_definition::{AttackSelection, BossDefinition};
use crate::coin::Coin;
use crate::enemy::Enemy;
use crate::player::WorldBounds;

pub const SLAM_TELEGRAPH_SECONDS: f32 = 0.5;
pub const SUMMON_WINDUP_SECONDS: f32 = 1.0;
pub const RINGS_FUSE_SECONDS: f32 = 1.0;
pub const DEATH_EFFECT_SECONDS: f32 = 0.25;
pub const SLAM_RADIUS: f32 = 56.0;
pub const RING_RADIUS: f32 = 48.0;
pub const BOSS_MOVE_SPEED: f32 = 1_000.0;

/// Convenience names for the original encounter; execution uses only action data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpecialAttack {
    Slam,
    Summon,
    Rings,
}

impl AsRef<str> for SpecialAttack {
    fn as_ref(&self) -> &str {
        match self {
            Self::Slam => "Slam",
            Self::Summon => "Summon",
            Self::Rings => "Rings",
        }
    }
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
    index: usize,
    action: usize,
    elapsed: f64,
    target: Vec2,
    entered: bool,
}

#[derive(Debug)]
pub struct BossSpecials {
    definition: Arc<BossDefinition>,
    position: Vec2,
    target_position: Vec2,
    move_speed: f32,
    health: u32,
    active: Option<ActiveAttack>,
    pending_damage: u32,
    next_attack: usize,
    attack_timer: f64,
    rings: Vec<Ring>,
    summoned_enemies: Vec<Enemy>,
    coins: Vec<Coin>,
    imported_enemies: usize,
    imported_coins: usize,
    death_effect_remaining: f32,
    defeated: bool,
    random_state: u32,
}

impl BossSpecials {
    pub fn from_definition(
        definition: Arc<BossDefinition>,
        position: Vec2,
    ) -> Result<Self, String> {
        definition.validate()?;
        if !position.is_finite() {
            return Err("boss position must be finite".into());
        }
        Ok(Self {
            health: definition.health,
            definition,
            position,
            target_position: position,
            move_speed: BOSS_MOVE_SPEED,
            active: None,
            pending_damage: 0,
            next_attack: 0,
            attack_timer: 0.0,
            rings: Vec::new(),
            summoned_enemies: Vec::new(),
            coins: Vec::new(),
            imported_enemies: 0,
            imported_coins: 0,
            death_effect_remaining: 0.0,
            defeated: false,
            random_state: 0x51a7_2026,
        })
    }

    pub fn name(&self) -> &str {
        &self.definition.name
    }
    pub fn position(&self) -> Vec2 {
        self.position
    }
    pub fn size(&self) -> Vec2 {
        self.definition.size.into()
    }
    pub fn health(&self) -> u32 {
        self.health
    }
    pub fn max_health(&self) -> u32 {
        self.definition.health
    }
    pub fn contact_damage(&self) -> u8 {
        if self.defeated {
            0
        } else {
            self.definition.contact_damage
        }
    }
    pub fn is_defeated(&self) -> bool {
        self.defeated
    }
    pub fn is_telegraphing(&self) -> bool {
        self.active.is_some()
    }
    pub fn active_attack_name(&self) -> Option<&str> {
        self.active
            .map(|a| self.definition.attacks[a.index].name.as_str())
    }
    pub fn active_attack(&self) -> Option<SpecialAttack> {
        match self.active_attack_name() {
            Some("Slam") => Some(SpecialAttack::Slam),
            Some("Summon") => Some(SpecialAttack::Summon),
            Some("Rings") => Some(SpecialAttack::Rings),
            _ => None,
        }
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
    pub fn attack_timer(&self) -> f32 {
        self.attack_timer as f32
    }
    pub fn set_random_seed(&mut self, seed: u32) {
        self.random_state = seed;
    }

    pub(crate) fn new_enemies(&mut self) -> &[Enemy] {
        let start = self.imported_enemies;
        self.imported_enemies = self.summoned_enemies.len();
        &self.summoned_enemies[start..]
    }
    pub(crate) fn new_coins(&mut self) -> &[Coin] {
        let start = self.imported_coins;
        self.imported_coins = self.coins.len();
        &self.coins[start..]
    }

    /// Called after update, preserving the original encounter's frame-boundary starts.
    pub fn schedule(&mut self, dt: f32, player: Vec2, bounds: WorldBounds) {
        if self.defeated || !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.attack_timer += f64::from(dt);
        let pause = f64::from(self.definition.pause_seconds);
        if self.active.is_none() && self.attack_timer >= pause {
            self.attack_timer = if pause > 0.0 {
                self.attack_timer % pause
            } else {
                0.0
            };
            self.start_next_attack(player, bounds);
        }
    }

    pub fn start_next_attack(&mut self, player: Vec2, bounds: WorldBounds) {
        if self.defeated || self.active.is_some() || !player.is_finite() {
            return;
        }
        let index = match self.definition.selection {
            AttackSelection::Cycle => self.next_attack,
            AttackSelection::Random => {
                // Rejection sampling avoids modulo bias.
                let count = self.definition.attacks.len() as u32;
                let threshold = count.wrapping_neg() % count;
                loop {
                    let value = self.random_u32();
                    if value >= threshold {
                        break (value % count) as usize;
                    }
                }
            }
        };
        self.next_attack = (index + 1) % self.definition.attacks.len();
        self.start_index(index, player, bounds);
    }

    pub fn start_attack(&mut self, name: impl AsRef<str>, player: Vec2, bounds: WorldBounds) {
        if self.defeated || self.active.is_some() || !player.is_finite() {
            return;
        }
        if let Some(index) = self
            .definition
            .attacks
            .iter()
            .position(|a| a.name == name.as_ref())
        {
            self.start_index(index, player, bounds);
        }
    }

    fn start_index(&mut self, index: usize, player: Vec2, bounds: WorldBounds) {
        self.active = Some(ActiveAttack {
            index,
            action: 0,
            elapsed: 0.0,
            entered: false,
            target: Vec2::new(player.x.clamp(bounds.left, bounds.right), player.y),
        });
        // Initialize instantaneous actions/targets without moving or consuming time.
        self.pending_damage = self
            .pending_damage
            .saturating_add(self.run_actions(0.0, player, bounds));
    }

    pub fn damage_boss(&mut self, amount: u32) {
        if self.defeated {
            return;
        }
        self.health = self.health.saturating_sub(amount);
        if self.health == 0 {
            self.defeated = true;
            self.active = None;
            self.pending_damage = 0;
            self.death_effect_remaining = DEATH_EFFECT_SECONDS;
        }
    }

    /// Advances one finite action sequence, returning its damage events.
    pub fn update(&mut self, dt: f32, player: Vec2, bounds: WorldBounds) -> u32 {
        if !dt.is_finite() || dt <= 0.0 || !player.is_finite() {
            return 0;
        }
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
        // Nonblocking glides start during action entry and move on the next update.
        let blocking = self.active.is_some_and(|a| {
            matches!(
                self.definition.attacks[a.index].actions.get(a.action),
                Some(Action::JumpTo { .. } | Action::SlamDown { .. } | Action::Hover { .. })
            )
        });
        if !blocking {
            self.glide(f64::from(dt));
        }
        let pending = std::mem::take(&mut self.pending_damage);
        pending.saturating_add(self.run_actions(f64::from(dt), player, bounds))
    }

    fn glide(&mut self, dt: f64) {
        let offset = self.target_position.as_dvec2() - self.position.as_dvec2();
        let distance = offset.length();
        if distance > 0.0 {
            self.position = (self.position.as_dvec2()
                + offset * (f64::from(self.move_speed) * dt / distance).min(1.0))
            .as_vec2();
        }
    }

    fn run_actions(&mut self, mut remaining: f64, player: Vec2, bounds: WorldBounds) -> u32 {
        let mut damage_event = 0_u32;
        while let Some(mut active) = self.active {
            let Some(action) = self.definition.attacks[active.index]
                .actions
                .get(active.action)
                .cloned()
            else {
                self.active = None;
                break;
            };
            let entering = !active.entered;
            active.entered = true;
            let mut finished = true;
            let mut used = 0.0;
            match action {
                Action::Wait { seconds }
                | Action::Telegraph { seconds }
                | Action::Hover { seconds } => {
                    if matches!(action, Action::Hover { .. }) {
                        self.target_position = self.position;
                    }
                    used = remaining.min((f64::from(seconds) - active.elapsed).max(0.0));
                    active.elapsed += used;
                    finished = active.elapsed >= f64::from(seconds);
                }
                Action::GlideToPlayer { speed, above_floor } => {
                    self.move_speed = speed;
                    self.target_position = Vec2::new(
                        active.target.x,
                        if above_floor == 0.0 {
                            self.position.y
                        } else {
                            bounds.floor_y - above_floor
                        },
                    );
                }
                Action::GlideToFloor { speed } => {
                    self.move_speed = speed;
                    self.target_position =
                        Vec2::new(active.target.x, bounds.floor_y - self.size().y / 2.0);
                }
                Action::JumpTo { speed, .. } | Action::SlamDown { speed } => {
                    if entering {
                        self.target_position = match action {
                            Action::JumpTo { height, .. } => active.target - Vec2::new(0.0, height),
                            _ => Vec2::new(self.position.x, bounds.floor_y - self.size().y / 2.0),
                        };
                        self.move_speed = speed;
                    }
                    let distance = self
                        .position
                        .as_dvec2()
                        .distance(self.target_position.as_dvec2());
                    used = remaining.min(distance / f64::from(speed));
                    // Blocking movement is advanced here, rather than by the background glide.
                    self.glide(used);
                    finished = self.position == self.target_position;
                }
                Action::DamageTarget { radius, damage } => {
                    if (player.x - active.target.x).abs() <= radius {
                        damage_event = damage_event.saturating_add(damage as u32);
                    }
                }
                Action::SpawnShockwave { radius, damage } => {
                    if (player.x - self.position.x).abs() <= radius {
                        damage_event = damage_event.saturating_add(damage as u32);
                    }
                }
                Action::Summon { inset } => {
                    let inset = inset.clamp(
                        crate::enemy::ENEMY_SIZE.x / 2.0,
                        (bounds.right - bounds.left) / 2.0,
                    );
                    self.summoned_enemies.extend([
                        Enemy::new(bounds.left + inset, bounds.left + inset, 0.0, 0.0, bounds),
                        Enemy::new(bounds.right - inset, bounds.right - inset, 0.0, 0.0, bounds),
                    ]);
                }
                Action::SpawnRings { count } => {
                    self.rings = (0..count as usize)
                        .map(|_| Ring {
                            position: self.random_arena_position(bounds),
                            exploded: false,
                        })
                        .collect();
                }
                Action::ExplodeRings { radius, damage } => {
                    if self
                        .rings
                        .iter()
                        .any(|r| !r.exploded && r.position.distance(player) <= radius)
                    {
                        damage_event = damage_event.saturating_add(damage as u32);
                    }
                    for ring in &mut self.rings {
                        ring.exploded = true;
                    }
                }
                _ => unreachable!("validated boss action"),
            }
            remaining = (remaining - used).max(0.0);
            if finished {
                active.action += 1;
                active.elapsed = 0.0;
                active.entered = false;
            }
            self.active = Some(active);
            if !finished {
                break;
            }
        }
        damage_event
    }

    fn random_arena_position(&mut self, bounds: WorldBounds) -> Vec2 {
        Vec2::new(
            bounds.left + self.next_random() * (bounds.right - bounds.left),
            80.0 + self.next_random() * (bounds.floor_y - 80.0),
        )
    }
    fn next_random(&mut self) -> f32 {
        self.random_u32() as f32 / u32::MAX as f32
    }
    fn random_u32(&mut self) -> u32 {
        self.random_state = self
            .random_state
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        self.random_state
    }
}
