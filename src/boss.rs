use macroquad::math::{Rect, Vec2};

pub const BOSS_PAUSE: f32 = 0.5;
pub const WARDEN_MAX_HEALTH: u8 = 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossState {
    Idle,
    Telegraphing,
    Attacking,
    Dead,
}

/// One reusable attack: the preview and active hitbox share a local rectangle.
#[derive(Clone, Debug)]
pub struct BossAttack {
    pub name: &'static str,
    pub area: Rect,
    pub telegraph_seconds: f32,
    pub active_seconds: f32,
    pub cooldown_seconds: f32,
}

/// Window-free, stationary boss rules. Content is supplied entirely by attacks.
#[derive(Clone, Debug)]
pub struct Boss {
    position: Vec2,
    size: Vec2,
    health: u8,
    max_health: u8,
    attacks: Vec<BossAttack>,
    attack_index: usize,
    state: BossState,
    remaining: f64,
    cycle_seconds: f64,
    last_swing_id: Option<u64>,
}

impl Boss {
    pub fn new(position: Vec2, size: Vec2, health: u8, attacks: Vec<BossAttack>) -> Self {
        assert!(position.is_finite() && size.is_finite() && size.x > 0.0 && size.y > 0.0);
        assert!(health > 0 && !attacks.is_empty());
        for attack in &attacks {
            assert!(attack.area.x.is_finite() && attack.area.y.is_finite());
            assert!(attack.area.w.is_finite() && attack.area.w > 0.0);
            assert!(attack.area.h.is_finite() && attack.area.h > 0.0);
            assert!(attack.telegraph_seconds.is_finite() && attack.telegraph_seconds > 0.0);
            assert!(attack.active_seconds.is_finite() && attack.active_seconds > 0.0);
            assert!(attack.cooldown_seconds.is_finite() && attack.cooldown_seconds >= 0.0);
        }
        let cycle_seconds = attacks
            .iter()
            .map(|attack| {
                f64::from(attack.telegraph_seconds)
                    + f64::from(attack.active_seconds)
                    + f64::from(attack.cooldown_seconds)
                    + f64::from(BOSS_PAUSE)
            })
            .sum();
        Self {
            position,
            size,
            health,
            max_health: health,
            attacks,
            attack_index: 0,
            state: BossState::Idle,
            remaining: f64::from(BOSS_PAUSE),
            cycle_seconds,
            last_swing_id: None,
        }
    }

    pub fn ash_warden(position: Vec2) -> Self {
        Self::new(
            position,
            Vec2::new(64.0, 80.0),
            WARDEN_MAX_HEALTH,
            vec![
                BossAttack {
                    name: "Low sweep",
                    area: Rect::new(-150.0, 12.0, 300.0, 28.0),
                    telegraph_seconds: 0.75,
                    active_seconds: 0.25,
                    cooldown_seconds: 0.5,
                },
                BossAttack {
                    name: "Crown flare",
                    area: Rect::new(-120.0, -120.0, 240.0, 100.0),
                    telegraph_seconds: 0.75,
                    active_seconds: 0.5,
                    cooldown_seconds: 0.75,
                },
            ],
        )
    }

    pub fn position(&self) -> Vec2 {
        self.position
    }

    pub fn size(&self) -> Vec2 {
        self.size
    }

    pub fn health(&self) -> u8 {
        self.health
    }

    pub fn max_health(&self) -> u8 {
        self.max_health
    }

    pub fn state(&self) -> BossState {
        self.state
    }

    pub fn is_dead(&self) -> bool {
        self.state == BossState::Dead
    }

    pub fn current_attack(&self) -> &BossAttack {
        &self.attacks[self.attack_index]
    }

    pub fn update(&mut self, delta_seconds: f32) {
        if self.is_dead() || !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        let mut dt = f64::from(delta_seconds);
        while dt >= self.remaining {
            dt -= self.remaining;
            match self.state {
                BossState::Idle => {
                    self.state = BossState::Telegraphing;
                    self.remaining = f64::from(self.current_attack().telegraph_seconds);
                    // A full cycle returns to this exact attack/phase. Bound work
                    // even for very large finite input without losing the remainder.
                    dt %= self.cycle_seconds;
                }
                BossState::Telegraphing => {
                    self.state = BossState::Attacking;
                    self.remaining = f64::from(self.current_attack().active_seconds);
                }
                BossState::Attacking => {
                    self.remaining =
                        f64::from(self.current_attack().cooldown_seconds) + f64::from(BOSS_PAUSE);
                    self.attack_index = (self.attack_index + 1) % self.attacks.len();
                    self.state = BossState::Idle;
                }
                BossState::Dead => return,
            }
        }
        self.remaining -= dt;
    }

    /// Returns true only for the first hit from this player swing.
    pub fn hit(&mut self, swing_id: u64) -> bool {
        if self.is_dead() || self.last_swing_id == Some(swing_id) {
            return false;
        }
        self.last_swing_id = Some(swing_id);
        self.health -= 1;
        if self.health == 0 {
            self.state = BossState::Dead;
            self.remaining = 0.0;
        }
        true
    }

    pub fn touches(&self, position: Vec2, size: Vec2) -> bool {
        let separation = (self.position - position).abs();
        let reach = (self.size + size) / 2.0;
        !self.is_dead() && separation.x <= reach.x && separation.y <= reach.y
    }

    /// Preview during telegraphing; damaging during attacking; absent otherwise.
    pub fn attack_area(&self) -> Option<Rect> {
        matches!(self.state, BossState::Telegraphing | BossState::Attacking).then(|| {
            let local = self.current_attack().area;
            Rect::new(
                self.position.x + local.x,
                self.position.y + local.y,
                local.w,
                local.h,
            )
        })
    }

    pub fn damages(&self, position: Vec2, size: Vec2) -> bool {
        self.touches(position, size)
            || (self.state == BossState::Attacking
                && self.attack_area().is_some_and(|area| {
                    area.overlaps(&Rect::new(
                        position.x - size.x / 2.0,
                        position.y - size.y / 2.0,
                        size.x,
                        size.y,
                    ))
                }))
    }
}
