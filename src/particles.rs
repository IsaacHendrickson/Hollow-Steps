//! Particles: short-lived specks spawned by `GameEvent`s, simulated in world space.
//!
//! Pure state and rules, no drawing. `ParticleSystem` owns the live particles and a seeded
//! random generator, so the same events at the same positions always give the same particles
//! (nothing reads the clock). The front end feeds it the located events each frame, calls
//! `update` with the step time and draws `particles()` through the world camera.
//!
//! Which events spawn which burst is data: each preset in `data/particles.ron` lists its
//! trigger events (an `assets/particles.ron` replaces the whole file). The one exception is
//! `hud_damage`: the health bar's own emitter spawns it where health was lost, so it lists no
//! events and `handle` never makes it.

use std::path::Path;

use macroquad::math::Vec2;
use serde::Deserialize;

use crate::assets::read_override;
use crate::events::GameEvent;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "particles.ron";
/// Seed of the generator, so effects look the same on every run.
pub const FIXED_SEED: u64 = 0x486F_6C6C_6F77_5374;
/// Largest `max_particles` a config may ask for.
pub const MAX_PARTICLE_CAP: u32 = 2048;
/// Most particles one preset may spawn per burst.
pub const MAX_BURST_COUNT: u32 = 64;
/// Longest a particle may live, in seconds.
pub const MAX_LIFETIME_SECS: f32 = 3.0;
/// Shortest a particle may live; keeps the age fraction well defined.
pub const MIN_LIFETIME_SECS: f32 = 0.01;
/// Fastest start speed, in pixels per second.
pub const MAX_SPEED: f32 = 1000.0;
/// Largest particle diameter, in pixels.
pub const MAX_SIZE: f32 = 64.0;
/// Largest gravity either way, in pixels per second squared.
pub const MAX_GRAVITY: f32 = 2000.0;
/// Largest drag, as a share of velocity lost per second.
pub const MAX_DRAG: f32 = 20.0;
/// Largest spawn box half-size or offset, in pixels.
pub const MAX_SPAN_PIXELS: f32 = 256.0;

/// A closed range of values; a particle picks a random one between `min` and `max`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Span {
    pub min: f32,
    pub max: f32,
}

/// A small deterministic random generator (SplitMix64), so no dependency is needed and
/// tests can pin exact sequences.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A number from 0 up to, but never reaching, 1.
    pub fn unit(&mut self) -> f32 {
        // 24 bits fit an f32 mantissa exactly, so the result is never rounded up to 1.
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn between(&mut self, span: Span) -> f32 {
        span.min + (span.max - span.min) * self.unit()
    }
}

/// One burst of particles and how each behaves. See `data/particles.ron` for the units.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preset {
    /// Events that spawn this burst. Empty turns the preset off.
    pub on: Vec<GameEvent>,
    pub count: u32,
    pub lifetime_secs: Span,
    pub speed: Span,
    pub direction_deg: Span,
    /// Half width and height of the box particles start in.
    pub spread: (f32, f32),
    /// Added to the event's position.
    pub offset: (f32, f32),
    pub gravity: f32,
    pub drag: f32,
    /// Diameter at birth and at death.
    pub size: (f32, f32),
    pub color_start: [u8; 4],
    pub color_end: [u8; 4],
}

impl Preset {
    fn validate(&self, name: &str) -> Result<(), String> {
        let check = |what: &str, value: f32, min: f32, max: f32| {
            if value.is_finite() && (min..=max).contains(&value) {
                Ok(())
            } else {
                Err(format!(
                    "particles {name}.{what} must be between {min} and {max}"
                ))
            }
        };
        let check_span = |what: &str, span: Span, min: f32, max: f32| {
            check(&format!("{what}.min"), span.min, min, max)?;
            check(&format!("{what}.max"), span.max, min, max)?;
            if span.min <= span.max {
                Ok(())
            } else {
                Err(format!(
                    "particles {name}.{what}: min must not be above max"
                ))
            }
        };
        if !(1..=MAX_BURST_COUNT).contains(&self.count) {
            return Err(format!(
                "particles {name}.count must be between 1 and {MAX_BURST_COUNT}"
            ));
        }
        for (index, event) in self.on.iter().enumerate() {
            if self.on[..index].contains(event) {
                return Err(format!("particles {name}.on lists {event:?} twice"));
            }
        }
        check_span(
            "lifetime_secs",
            self.lifetime_secs,
            MIN_LIFETIME_SECS,
            MAX_LIFETIME_SECS,
        )?;
        check_span("speed", self.speed, 0.0, MAX_SPEED)?;
        check_span("direction_deg", self.direction_deg, -360.0, 360.0)?;
        check("spread.x", self.spread.0, 0.0, MAX_SPAN_PIXELS)?;
        check("spread.y", self.spread.1, 0.0, MAX_SPAN_PIXELS)?;
        check("offset.x", self.offset.0, -MAX_SPAN_PIXELS, MAX_SPAN_PIXELS)?;
        check("offset.y", self.offset.1, -MAX_SPAN_PIXELS, MAX_SPAN_PIXELS)?;
        check("gravity", self.gravity, -MAX_GRAVITY, MAX_GRAVITY)?;
        check("drag", self.drag, 0.0, MAX_DRAG)?;
        check("size.start", self.size.0, 0.0, MAX_SIZE)?;
        check("size.end", self.size.1, 0.0, MAX_SIZE)
    }
}

/// Particle tunables, from the embedded `data/particles.ron`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleConfig {
    /// Most particles alive at once; a burst that would pass it is cut short.
    pub max_particles: u32,
    pub hit_spark: Preset,
    pub dust: Preset,
    pub dash_trail: Preset,
    pub death_burst: Preset,
    /// The burst thrown off the health bar where it lost health, in screen space. A file
    /// that leaves it out gets the built-in one.
    #[serde(default = "default_hud_damage")]
    pub hud_damage: Preset,
}

/// The `hud_damage` preset of the embedded `data/particles.ron`.
fn default_hud_damage() -> Preset {
    #[derive(Deserialize)]
    struct Shipped {
        hud_damage: Preset,
    }
    ron::from_str::<Shipped>(include_str!("../data/particles.ron"))
        .expect("valid hud_damage default")
        .hud_damage
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/particles.ron")).expect("valid particle defaults")
    }
}

impl ParticleConfig {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let config: Self = ron::from_str(text).map_err(|error| format!("particles: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `particles.ron` in `assets` if there is one (it replaces the defaults whole),
    /// otherwise the embedded `data/particles.ron`.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Ok(Self::default()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !(1..=MAX_PARTICLE_CAP).contains(&self.max_particles) {
            return Err(format!(
                "particles max_particles must be between 1 and {MAX_PARTICLE_CAP}"
            ));
        }
        for (name, preset) in self.presets() {
            preset.validate(name)?;
        }
        self.hud_damage.validate("hud_damage")?;
        if !self.hud_damage.on.is_empty() {
            return Err(
                "particles hud_damage.on must be empty: the HUD spawns this burst \
                        itself, where the health bar lost health"
                    .into(),
            );
        }
        Ok(())
    }

    /// The world presets by name, in the fixed order bursts are spawned. `hud_damage` is not
    /// one of them.
    pub fn presets(&self) -> [(&'static str, &Preset); 4] {
        [
            ("hit_spark", &self.hit_spark),
            ("dust", &self.dust),
            ("dash_trail", &self.dash_trail),
            ("death_burst", &self.death_burst),
        ]
    }
}

/// One live particle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub position: Vec2,
    velocity: Vec2,
    age: f32,
    lifetime: f32,
    gravity: f32,
    drag: f32,
    size: (f32, f32),
    colors: ([u8; 4], [u8; 4]),
}

impl Particle {
    /// How far through its life the particle is, from 0 (new) to 1 (about to be removed).
    pub fn progress(&self) -> f32 {
        (self.age / self.lifetime).clamp(0.0, 1.0)
    }

    /// Diameter in pixels now.
    pub fn size(&self) -> f32 {
        let t = self.progress();
        self.size.0 + (self.size.1 - self.size.0) * t
    }

    /// Red, green, blue and alpha now.
    pub fn color(&self) -> [u8; 4] {
        let t = self.progress();
        let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * t).round() as u8;
        let (start, end) = self.colors;
        [
            mix(start[0], end[0]),
            mix(start[1], end[1]),
            mix(start[2], end[2]),
            mix(start[3], end[3]),
        ]
    }

    pub fn age(&self) -> f32 {
        self.age
    }

    pub fn lifetime(&self) -> f32 {
        self.lifetime
    }

    fn advance(&mut self, delta_seconds: f32) {
        self.age += delta_seconds;
        self.velocity.y += self.gravity * delta_seconds;
        self.velocity *= (-self.drag * delta_seconds).exp();
        self.position += self.velocity * delta_seconds;
    }
}

/// The live particles, the settings they are made from and the generator that varies them.
#[derive(Clone, Debug)]
pub struct ParticleSystem {
    config: ParticleConfig,
    particles: Vec<Particle>,
    rng: Rng,
}

impl ParticleSystem {
    pub fn new(config: ParticleConfig) -> Self {
        Self::with_seed(config, FIXED_SEED)
    }

    pub fn with_seed(config: ParticleConfig, seed: u64) -> Self {
        Self {
            config,
            particles: Vec::new(),
            rng: Rng::new(seed),
        }
    }

    pub fn config(&self) -> &ParticleConfig {
        &self.config
    }

    /// Swaps the settings (after a reload). Particles already alive keep the values they
    /// were born with.
    pub fn set_config(&mut self, config: ParticleConfig) {
        self.config = config;
    }

    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    /// Removes every live particle. The generator carries on where it was.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Reacts to one event: spawns the bursts that list it, at `at` (the event's world
    /// position), and returns how many particles were created. Events with no position
    /// spawn nothing. Entering a room clears everything, since the particles' positions
    /// belong to the room just left.
    pub fn handle(&mut self, event: GameEvent, at: Option<Vec2>) -> usize {
        if event == GameEvent::RoomEntered {
            self.clear();
            return 0;
        }
        let Some(at) = at.filter(|at| at.is_finite()) else {
            return 0;
        };
        let Self {
            config,
            particles,
            rng,
        } = self;
        let cap = config.max_particles as usize;
        let mut spawned = 0;
        for (_, preset) in config.presets() {
            if preset.on.contains(&event) {
                spawned += spawn_burst(particles, rng, cap, preset, at);
            }
        }
        spawned
    }

    /// Spawns the `hud_damage` burst at `at` and returns how many particles were created. This
    /// is for the HUD's own emitter, which lives in screen space: `at` is a screen position.
    /// A position that is not finite spawns nothing.
    pub fn spawn_hud_damage(&mut self, at: Vec2) -> usize {
        if !at.is_finite() {
            return 0;
        }
        let Self {
            config,
            particles,
            rng,
        } = self;
        let cap = config.max_particles as usize;
        spawn_burst(particles, rng, cap, &config.hud_damage, at)
    }

    /// Moves every particle on by `delta_seconds` and removes the ones whose time is up.
    /// Zero, negative and non-finite steps change nothing.
    pub fn update(&mut self, delta_seconds: f32) {
        if !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        for particle in &mut self.particles {
            particle.advance(delta_seconds);
        }
        self.particles
            .retain(|particle| particle.age < particle.lifetime);
    }
}

/// Adds one burst of `preset` around `at`, stopping when `cap` particles are alive.
fn spawn_burst(
    particles: &mut Vec<Particle>,
    rng: &mut Rng,
    cap: usize,
    preset: &Preset,
    at: Vec2,
) -> usize {
    let mut spawned = 0;
    for _ in 0..preset.count {
        if particles.len() >= cap {
            break;
        }
        let angle = rng.between(preset.direction_deg).to_radians();
        let speed = rng.between(preset.speed);
        let jitter = Vec2::new(
            (rng.unit() * 2.0 - 1.0) * preset.spread.0,
            (rng.unit() * 2.0 - 1.0) * preset.spread.1,
        );
        particles.push(Particle {
            position: at + Vec2::new(preset.offset.0, preset.offset.1) + jitter,
            velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
            age: 0.0,
            lifetime: rng.between(preset.lifetime_secs),
            gravity: preset.gravity,
            drag: preset.drag,
            size: preset.size,
            colors: (preset.color_start, preset.color_end),
        });
        spawned += 1;
    }
    spawned
}

#[cfg(test)]
mod tests;
