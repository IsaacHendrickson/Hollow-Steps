use std::sync::Arc;

use macroquad::math::Vec2;

use crate::action_runtime::{ActionActor, ActionOutput, ActionRuntime, ActorMotion};
use crate::behavior::Action;
use crate::behavior::BehaviorContext;
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

pub type Ring = crate::behavior::Blast;

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveAttack {
    index: usize,
    action: usize,
    target: Vec2,
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
    runtime: ActionRuntime,
    player_size: Vec2,
    uses_ground_motion: bool,
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
            runtime: ActionRuntime::default(),
            player_size: Vec2::ZERO,
            uses_ground_motion: false,
            summoned_enemies: Vec::new(),
            coins: Vec::new(),
            imported_enemies: 0,
            imported_coins: 0,
            death_effect_remaining: 0.0,
            defeated: false,
            random_state: 0x51a7_2026,
        })
    }

    pub(crate) fn same_definition(&self, other: &Self) -> bool {
        self.definition == other.definition
    }

    pub fn projectile_feedback(&self) -> crate::enemy_attack::AttackFeedback {
        self.active
            .and_then(|a| self.definition.attacks[a.index].actions.get(a.action))
            .and_then(Action::feedback)
            .unwrap_or_else(crate::enemy_attack::default_feedback)
    }
    pub fn warning(&self) -> Option<crate::enemy_attack::AttackWarning> {
        let progress = self.runtime.attack_runtime.as_ref()?.windup_progress()?;
        let attack = self.attack_warning()?;
        let feedback = self.projectile_feedback();
        Some(crate::enemy_attack::AttackWarning {
            attack,
            feedback,
            strength: (1.0 + (std::f32::consts::TAU * feedback.flash_cycles * progress).cos())
                / 2.0,
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
    pub fn landing_preview(&self) -> Option<(f32, f32)> {
        let active = self.active?;
        self.definition.attacks[active.index].actions[active.action..]
            .iter()
            .find_map(|action| match action {
                Action::DamageTarget { radius, .. } => Some((active.target.x, *radius)),
                Action::SpawnShockwave { radius, .. } => Some((self.position.x, *radius)),
                _ => None,
            })
    }
    pub fn active_attack(&self) -> Option<SpecialAttack> {
        match self.active_attack_name() {
            Some("Slam") => Some(SpecialAttack::Slam),
            Some("Summon") => Some(SpecialAttack::Summon),
            Some("Rings") => Some(SpecialAttack::Rings),
            _ => None,
        }
    }
    pub fn attack_hit(&self, player: Vec2, size: Vec2) -> Option<crate::enemy_attack::DamageHit> {
        if self.defeated {
            None
        } else {
            self.runtime.attack_hit(self.position, player, size)
        }
    }
    pub fn active_damage_area(&self) -> Option<crate::enemy_attack::AttackBox> {
        if self.defeated {
            None
        } else {
            self.runtime.attack_runtime.as_ref().and_then(|r| r.active)
        }
    }
    pub fn projectiles(&self) -> &[crate::enemy_attack::Projectile] {
        &self.runtime.projectiles
    }
    pub fn attack_warning(&self) -> Option<crate::enemy_attack::AttackBox> {
        self.runtime
            .attack_runtime
            .as_ref()
            .and_then(|r| r.warning())
    }
    pub fn rings(&self) -> &[Ring] {
        &self.runtime.rings
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
        self.runtime.finish_action();
        self.active = Some(ActiveAttack {
            index,
            action: 0,
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
        self.update_with_player_size(dt, player, Vec2::ZERO, bounds)
    }

    pub fn update_with_player_size(
        &mut self,
        dt: f32,
        player: Vec2,
        player_size: Vec2,
        bounds: WorldBounds,
    ) -> u32 {
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
        self.player_size = player_size;
        self.runtime
            .advance_projectiles(player, player_size, bounds, dt);
        // Nonblocking glides start during action entry and move on the next update.
        let blocking = self.active.is_some_and(|a| {
            matches!(
                self.definition.attacks[a.index].actions.get(a.action),
                Some(Action::JumpToPlayer { .. } | Action::SlamDown { .. } | Action::Hover { .. })
            )
        });
        if !blocking && !self.uses_ground_motion {
            self.glide(f64::from(dt));
        }
        let pending = std::mem::take(&mut self.pending_damage);
        let damage = pending.saturating_add(self.run_actions(f64::from(dt), player, bounds));
        let modern_damage = self
            .attack_hit(player, player_size)
            .map_or(0, |hit| hit.damage.max(0) as u32)
            .max(u32::from(self.runtime.blast_damage(player, player_size)));
        damage.saturating_add(modern_damage)
    }

    fn glide(&mut self, dt: f64) {
        ActionRuntime::advance_glide(
            &mut self.position,
            self.target_position,
            self.move_speed,
            dt,
        );
    }

    fn run_actions(&mut self, mut remaining: f64, player: Vec2, bounds: WorldBounds) -> u32 {
        let mut damage = 0_u32;
        while let Some(mut active) = self.active {
            let definition = self.definition.clone();
            let actions = &definition.attacks[active.index].actions;
            let Some(action) = actions.get(active.action) else {
                self.active = None;
                break;
            };
            let preview = if matches!(action, Action::SpawnRings { .. }) {
                actions[active.action + 1..]
                    .iter()
                    .find(|a| matches!(a, Action::ExplodeRings { .. }))
                    .unwrap_or(action)
            } else if matches!(action, Action::Telegraph { .. }) {
                actions.get(active.action + 1).unwrap_or(action)
            } else {
                action
            };
            if action.uses_ground_motion() {
                self.uses_ground_motion = true;
            } else if !matches!(action, Action::Wait { .. } | Action::Telegraph { .. }) {
                self.uses_ground_motion = false;
            }
            let ground_physics = self.uses_ground_motion;
            let mut runtime = std::mem::take(&mut self.runtime);
            let motion = runtime.motion();
            let size = self.size();
            let health = self.health.min(u32::from(u8::MAX)) as u8;
            let player_size = self.player_size;
            let mut actor = BossActionActor {
                boss: self,
                context: BehaviorContext {
                    player_position: player,
                    health,
                    size,
                    bounds,
                    patrol_left: bounds.left + size.x / 2.0,
                    patrol_right: bounds.right - size.x / 2.0,
                },
                player_size,
                ground_physics,
                motion,
                target: active.target,
                damage: 0,
            };
            let (used, finished) = runtime.step(action, preview, &mut actor, remaining);
            damage = damage.saturating_add(actor.damage);
            self.runtime = runtime;
            remaining = (remaining - used).max(0.0);
            if finished {
                active.action += 1;
                self.runtime.finish_action();
            }
            self.active = Some(active);
            if !finished {
                break;
            }
        }
        if self.uses_ground_motion && !self.runtime.airborne {
            self.runtime.apply_gravity(
                &mut self.position,
                BehaviorContext {
                    player_position: player,
                    health: self.health.min(255) as u8,
                    size: self.definition.size.into(),
                    bounds,
                    patrol_left: bounds.left,
                    patrol_right: bounds.right,
                },
                remaining,
            );
            self.target_position = self.position;
        }
        damage
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

struct BossActionActor<'a> {
    boss: &'a mut BossSpecials,
    context: BehaviorContext,
    player_size: Vec2,
    ground_physics: bool,
    motion: ActorMotion,
    target: Vec2,
    damage: u32,
}
impl ActionActor for BossActionActor<'_> {
    fn position(&self) -> Vec2 {
        self.boss.position
    }
    fn set_position(&mut self, position: Vec2) {
        self.boss.position = position;
    }
    fn context(&self) -> BehaviorContext {
        self.context
    }
    fn player_size(&self) -> Vec2 {
        self.player_size
    }
    fn motion(&self) -> ActorMotion {
        self.motion
    }
    fn set_motion(&mut self, motion: ActorMotion) {
        self.motion = motion;
    }
    fn ground_physics(&self) -> bool {
        self.ground_physics
    }
    fn legacy_target(&self) -> Option<Vec2> {
        Some(self.target)
    }
    fn random_arena_position(&mut self) -> Vec2 {
        self.boss.random_arena_position(self.context.bounds)
    }
    fn emit(&mut self, output: ActionOutput) {
        match output {
            ActionOutput::Hold(position) => self.boss.target_position = position,
            ActionOutput::Damage(damage) => self.damage = self.damage.saturating_add(damage),
            ActionOutput::Glide { target, speed } => {
                self.boss.target_position = target;
                self.boss.move_speed = speed;
            }
            ActionOutput::Spawn(position) => self.boss.summoned_enemies.push(Enemy::new(
                position.x,
                position.x,
                0.0,
                0.0,
                self.context.bounds,
            )),
            ActionOutput::DamageArea { .. } | ActionOutput::Projectile(_) => {}
        }
    }
}
