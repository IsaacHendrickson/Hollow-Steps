//! Macroquad front end for event-driven effects.
//!
//! `Effects` owns the particle system, hands it each frame's located `GameEvent`s, steps it
//! with the fixed tick and draws it in world space. The rules live in `hollow_steps::particles`.

use std::path::{Path, PathBuf};

use hollow_steps::events::LocatedEvent;
use hollow_steps::particles::{ParticleConfig, ParticleSystem};
use macroquad::prelude::*;

pub struct Effects {
    assets: PathBuf,
    particles: ParticleSystem,
}

impl Effects {
    /// Loads the effect settings from `assets`. Invalid settings fall back to the built-in
    /// defaults, and the reason comes back so it can be shown on screen.
    pub fn load(assets: &Path) -> (Self, Option<String>) {
        let (config, error) = match ParticleConfig::load(assets) {
            Ok(config) => (config, None),
            Err(error) => (ParticleConfig::default(), Some(error)),
        };
        let effects = Self {
            assets: assets.to_owned(),
            particles: ParticleSystem::new(config),
        };
        (effects, error)
    }

    /// Re-reads the settings. Nothing changes if they fail to load.
    pub fn reload(&mut self) -> Result<(), String> {
        let config = ParticleConfig::load(&self.assets)?;
        self.particles.set_config(config);
        Ok(())
    }

    /// Reacts to the events of one frame, oldest first.
    pub fn handle(&mut self, events: &[LocatedEvent]) {
        for located in events {
            self.particles.handle(located.event, located.at);
        }
    }

    /// Moves the particles on by one fixed tick.
    pub fn update(&mut self, delta_seconds: f32) {
        self.particles.update(delta_seconds);
    }

    /// Drops everything in flight, for when the world changes under it.
    pub fn clear(&mut self) {
        self.particles.clear();
    }

    /// Draws the particles. Call with the world camera set.
    pub fn draw(&self) {
        for particle in self.particles.particles() {
            let [red, green, blue, alpha] = particle.color();
            draw_circle(
                particle.position.x,
                particle.position.y,
                particle.size() / 2.0,
                Color::from_rgba(red, green, blue, alpha),
            );
        }
    }
}
