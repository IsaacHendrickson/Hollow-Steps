//! The RON definition of an enemy type: name, health, size, damage, behavior, body and
//! how long it takes to die.

use serde::{Deserialize, Serialize};

use crate::body::BodyVisual;

/// How long a defeated enemy lingers (flashing and fading) when its definition says nothing.
pub const DEFAULT_DEATH_SECS: f32 = 0.4;
/// The longest death an entity definition may ask for, in seconds.
pub const MAX_DEATH_SECS: f32 = 5.0;

fn default_death_secs() -> f32 {
    DEFAULT_DEATH_SECS
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnemyDefinition {
    pub name: String,
    pub health: u8,
    pub size: (f32, f32),
    pub contact_damage: u8,
    pub behavior: String,
    #[serde(default)]
    pub body: Option<BodyVisual>,
    /// Seconds a defeated enemy stays, harmless and fading, before it is removed. `0` removes
    /// it the moment its health runs out.
    #[serde(default = "default_death_secs")]
    pub death_secs: f32,
}

impl EnemyDefinition {
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("entity name must not be empty".into());
        }
        if self.health == 0 {
            return Err(format!("entity {:?}: health must be positive", self.name));
        }
        if !self.size.0.is_finite()
            || !self.size.1.is_finite()
            || self.size.0 <= 0.0
            || self.size.1 <= 0.0
        {
            return Err(format!(
                "entity {:?}: size must be finite and positive",
                self.name
            ));
        }
        if self.behavior.trim().is_empty() {
            return Err(format!("entity {:?}: missing behavior name", self.name));
        }
        if !self.death_secs.is_finite() || !(0.0..=MAX_DEATH_SECS).contains(&self.death_secs) {
            return Err(format!(
                "entity {:?}: death_secs must be between 0 and {MAX_DEATH_SECS}",
                self.name
            ));
        }
        if let Some(body) = &self.body {
            body.validate()
                .map_err(|e| format!("entity {:?}: {e}", self.name))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition(death_secs: f32) -> EnemyDefinition {
        EnemyDefinition {
            name: "walker".into(),
            health: 3,
            size: (36.0, 36.0),
            contact_damage: 1,
            behavior: "patrol".into(),
            body: None,
            death_secs,
        }
    }

    #[test]
    fn a_definition_without_death_secs_gets_the_default() {
        let parsed: EnemyDefinition = ron::from_str(
            r#"(name: "walker", health: 3, size: (36.0, 36.0), contact_damage: 1, behavior: "patrol")"#,
        )
        .unwrap();
        assert_eq!(parsed.death_secs, DEFAULT_DEATH_SECS);
        assert!(parsed.validate().is_ok());
    }

    #[test]
    fn death_secs_round_trips_through_ron() {
        let text = ron::to_string(&definition(1.25)).unwrap();
        let parsed: EnemyDefinition = ron::from_str(&text).unwrap();
        assert_eq!(parsed.death_secs, 1.25);
    }

    #[test]
    fn death_secs_must_be_finite_and_within_range_but_zero_is_allowed() {
        for ok in [0.0, 0.4, MAX_DEATH_SECS] {
            assert!(definition(ok).validate().is_ok(), "{ok}");
        }
        for bad in [-0.1, MAX_DEATH_SECS + 0.01, f32::NAN, f32::INFINITY] {
            let error = definition(bad).validate().unwrap_err();
            assert!(error.contains("death_secs"), "{error}");
        }
    }
}
