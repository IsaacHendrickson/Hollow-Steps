//! Macroquad front end for event-driven effects.
//!
//! `Effects` owns the particle system and the sound bank, hands them each frame's located
//! `GameEvent`s, steps the particles with the fixed tick and draws them in world space. The
//! rules live in `hollow_steps::particles` and `hollow_steps::sound`.

mod sound_bank;

use std::path::{Path, PathBuf};

use hollow_steps::events::LocatedEvent;
use hollow_steps::particles::{Particle, ParticleConfig, ParticleSystem};
use hollow_steps::sound::{SoundConfig, SOUND_DIR};
use macroquad::prelude::*;
use sound_bank::SoundBank;

pub struct Effects {
    assets: PathBuf,
    particles: ParticleSystem,
    sound_config: SoundConfig,
    sounds: SoundBank,
}

impl Effects {
    /// Loads the effect settings and sound files from `assets`. Invalid settings fall back
    /// to the built-in defaults, and the reasons come back so they can be shown on screen.
    pub fn load(assets: &Path) -> (Self, Option<String>) {
        let mut errors = Vec::new();
        let particle_config = ParticleConfig::load(assets).unwrap_or_else(|error| {
            errors.push(error);
            ParticleConfig::default()
        });
        let sound_config = SoundConfig::load(assets).unwrap_or_else(|error| {
            errors.push(error);
            SoundConfig::default()
        });
        let effects = Self {
            assets: assets.to_owned(),
            particles: ParticleSystem::new(particle_config),
            sounds: SoundBank::load(&sound_config, &assets.join(SOUND_DIR)),
            sound_config,
        };
        let error = (!errors.is_empty()).then(|| errors.join("\n"));
        (effects, error)
    }

    /// Re-reads the settings and sound files. Nothing changes if the settings fail to load.
    pub fn reload(&mut self) -> Result<(), String> {
        let particle_config = ParticleConfig::load(&self.assets)?;
        let sound_config = SoundConfig::load(&self.assets)?;
        self.particles.set_config(particle_config);
        self.sounds = SoundBank::load(&sound_config, &self.assets.join(SOUND_DIR));
        self.sound_config = sound_config;
        Ok(())
    }

    /// Reacts to the events of one frame, oldest first: spawns particles and starts sounds.
    pub fn handle(&mut self, events: &[LocatedEvent]) {
        for located in events {
            self.particles.handle(located.event, located.at);
        }
        let plays = self
            .sound_config
            .plan(events.iter().map(|located| located.event));
        self.sounds.play(&plays);
    }

    /// Moves the particles on by one fixed tick.
    pub fn update(&mut self, delta_seconds: f32) {
        self.particles.update(delta_seconds);
    }

    /// Drops everything in flight, for when the world changes under it.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// The particle settings in effect, for the HUD's own emitter to share.
    pub fn particle_config(&self) -> &ParticleConfig {
        self.particles.config()
    }

    /// Draws the particles. Call with the world camera set.
    pub fn draw(&self) {
        draw_particles(self.particles.particles());
    }
}

/// Draws `particles` as fading circles at their own positions: world pixels when the world
/// camera is set, screen pixels when the HUD's is.
pub fn draw_particles(particles: &[Particle]) {
    for particle in particles {
        let [red, green, blue, alpha] = particle.color();
        draw_circle(
            particle.position.x,
            particle.position.y,
            particle.size() / 2.0,
            Color::from_rgba(red, green, blue, alpha),
        );
    }
}
