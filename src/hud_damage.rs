//! What the health bar does when the player is hurt: it shakes, and throws off particles from
//! the part of the bar that was lost.
//!
//! Pure rules, no drawing. `HudFeedback` owns the bar's shake and a particle emitter of its own
//! (a second `ParticleSystem`, seeded separately from the world's), and both live in
//! virtual-resolution screen space. Nothing here knows about the camera, so the screen shake of
//! `feel.rs` can never move the HUD. The numbers are in `data/hud.ron` (the shake) and in the
//! `hud_damage` preset of `data/particles.ron` (the particles).

use macroquad::math::{vec2, Rect, Vec2};

use crate::events::{GameEvent, HealthLoss, LocatedEvent};
use crate::hud_bars::{fraction, DamageShake, HudConfig};
use crate::particles::{Particle, ParticleConfig, ParticleSystem};

/// Seed of the HUD's particle generator, apart from the world's `particles::FIXED_SEED`.
pub const HUD_SEED: u64 = 0x4855_4444_616D_6167;

/// How much of the horizontal swing the vertical swing has, and how its frequency and phase
/// differ, so the bar rattles instead of sliding on a line.
const Y_SHARE: f32 = 0.5;
const Y_FREQUENCY_RATIO: f32 = 1.7;
const Y_PHASE: f32 = 1.3;

/// The health bar's shake: a sine wave of time whose peak fades linearly to nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BarShake {
    /// Peak distance when this shake began, in pixels; zero when the bar is still.
    amplitude: f32,
    /// Seconds since this shake began.
    elapsed: f32,
}

impl BarShake {
    /// The peak distance for a hit that took `lost` of `max_health`: the share lost times
    /// `amplitude_per_max_health`, kept between `min_amplitude` and `max_amplitude`. A loss
    /// that is zero, negative or not a finite number, or a maximum that is not above zero,
    /// gives zero: nothing to shake about.
    pub fn amplitude_for(shake: &DamageShake, lost: f32, max_health: f32) -> f32 {
        if !lost.is_finite() {
            return 0.0;
        }
        let share = fraction(lost, max_health);
        if share <= 0.0 {
            return 0.0;
        }
        (share * shake.amplitude_per_max_health)
            .max(shake.min_amplitude)
            .min(shake.max_amplitude)
    }

    /// Starts a shake for a hit that took `lost` of `max_health`. A hit that gives no
    /// amplitude changes nothing. A new hit restarts the shake, but never calms a stronger one
    /// that is still going.
    pub fn start(&mut self, shake: &DamageShake, lost: f32, max_health: f32) {
        let amplitude = Self::amplitude_for(shake, lost, max_health);
        if amplitude <= 0.0 {
            return;
        }
        self.amplitude = amplitude.max(self.current_amplitude(shake));
        self.elapsed = 0.0;
    }

    /// Moves the shake on by `delta_seconds`; a zero, negative or non-finite step changes
    /// nothing. The shake ends when `shake.seconds` have passed.
    pub fn update(&mut self, shake: &DamageShake, delta_seconds: f32) {
        if self.amplitude <= 0.0 || !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        self.elapsed += delta_seconds;
        if self.elapsed >= shake.seconds {
            *self = Self::default();
        }
    }

    /// Whether the bar is still shaking.
    pub fn is_active(&self) -> bool {
        self.amplitude > 0.0
    }

    /// The peak distance now: the starting amplitude fading linearly to zero.
    pub fn current_amplitude(&self, shake: &DamageShake) -> f32 {
        if self.amplitude <= 0.0 || shake.seconds <= 0.0 {
            return 0.0;
        }
        self.amplitude * (1.0 - self.elapsed / shake.seconds).clamp(0.0, 1.0)
    }

    /// How far to move the health bar now, in screen pixels. Zero when it is still.
    pub fn offset(&self, shake: &DamageShake) -> Vec2 {
        let amplitude = self.current_amplitude(shake);
        if amplitude <= 0.0 {
            return Vec2::ZERO;
        }
        let phase = std::f32::consts::TAU * shake.frequency_hz * self.elapsed;
        vec2(
            amplitude * phase.sin(),
            amplitude * Y_SHARE * (phase * Y_FREQUENCY_RATIO + Y_PHASE).sin(),
        )
    }
}

/// The part of the health bar `bar` that a hit took: from the health left after it to the
/// health there was before it, as shares of `max_health`. The ends are kept on the bar, so a
/// hit that took the last of the health spans from the left end. `None` when the hit took
/// nothing or `max_health` is not above zero.
pub fn lost_segment(bar: Rect, max_health: f32, loss: HealthLoss) -> Option<Rect> {
    if !loss.lost.is_finite() || loss.lost <= 0.0 || max_health.is_nan() || max_health <= 0.0 {
        return None;
    }
    let after = fraction(loss.remaining, max_health);
    let before = fraction(loss.remaining + loss.lost, max_health);
    Some(Rect::new(
        bar.x + bar.w * after,
        bar.y,
        bar.w * (before - after).max(0.0),
        bar.h,
    ))
}

/// Where the damage particles start: the right end of the lost segment, halfway up the bar.
pub fn burst_point(segment: Rect) -> Vec2 {
    vec2(segment.right(), segment.center().y)
}

/// The health bar's shake and its own particle emitter.
#[derive(Clone, Debug)]
pub struct HudFeedback {
    shake: BarShake,
    particles: ParticleSystem,
}

impl HudFeedback {
    pub fn new(particles: ParticleConfig) -> Self {
        Self::with_seed(particles, HUD_SEED)
    }

    /// Like `new`, with the particle generator started from `seed`.
    pub fn with_seed(particles: ParticleConfig, seed: u64) -> Self {
        Self {
            shake: BarShake::default(),
            particles: ParticleSystem::with_seed(particles, seed),
        }
    }

    /// Swaps the particle settings (after a reload). Particles already alive keep the values
    /// they were born with.
    pub fn set_particle_config(&mut self, particles: ParticleConfig) {
        self.particles.set_config(particles);
    }

    /// Reacts to one frame's events, oldest first. Each `PlayerDamaged` that took health
    /// shakes the bar and spawns a burst at the right end of the lost segment of `bar` (the
    /// health bar's rectangle on screen). Returns how many particles were created. Events that
    /// took nothing, and every other event, do nothing.
    pub fn handle(
        &mut self,
        hud: &HudConfig,
        bar: Rect,
        max_health: f32,
        events: &[LocatedEvent],
    ) -> usize {
        let mut spawned = 0;
        for located in events {
            if located.event != GameEvent::PlayerDamaged {
                continue;
            }
            let Some(loss) = located.loss else { continue };
            let Some(segment) = lost_segment(bar, max_health, loss) else {
                continue;
            };
            self.shake.start(&hud.damage_shake, loss.lost, max_health);
            spawned += self.particles.spawn_hud_damage(burst_point(segment));
        }
        spawned
    }

    /// Moves the shake and the particles on by one step.
    pub fn update(&mut self, hud: &HudConfig, delta_seconds: f32) {
        self.shake.update(&hud.damage_shake, delta_seconds);
        self.particles.update(delta_seconds);
    }

    /// How far the health bar is moved now. Zero when it is still.
    pub fn bar_offset(&self, hud: &HudConfig) -> Vec2 {
        self.shake.offset(&hud.damage_shake)
    }

    pub fn shake_active(&self) -> bool {
        self.shake.is_active()
    }

    /// The live particles, in screen pixels.
    pub fn particles(&self) -> &[Particle] {
        self.particles.particles()
    }

    /// Stops the shake and drops every particle.
    pub fn clear(&mut self) {
        self.shake = BarShake::default();
        self.particles.clear();
    }
}

#[cfg(test)]
mod tests;
