//! Transition conditions: the `conditions!` table that supplies the `Condition` enum and its
//! editor catalog, and how a condition is evaluated.
use super::validators::nonnegative;
use serde::{Deserialize, Serialize};

macro_rules! conditions {
    ($( $variant:ident => $name:literal ($ty:ty, $label:literal, $default:expr) ),+; $( $unit:ident => $unit_name:literal ),+ $(,)?) => {
        #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
        #[serde(rename_all = "snake_case")]
        pub enum Condition { $($variant($ty),)+ $($unit),+ }
        impl Condition {
            pub const TYPES: &'static [&'static str] = &[$($name,)+ $($unit_name),+];
            pub fn defaults(kind: usize) -> Option<Self> {
                Some(match *Self::TYPES.get(kind)? { $($name => Self::$variant($default),)+ $($unit_name => Self::$unit,)+ _ => return None })
            }
            pub fn kind(&self) -> usize {
                let name = match self { $(Self::$variant(_) => $name,)+ $(Self::$unit => $unit_name),+ };
                Self::TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn parameter(&self) -> Option<(&'static str, String)> {
                match self { $(Self::$variant(value) => Some(($label, value.to_string())),)+ $(Self::$unit => None),+ }
            }
            pub fn set_parameter(&mut self, text: &str) -> Result<(), String> {
                let text = text.trim();
                parse_nonnegative(text)?;
                match self {
                    $(Self::$variant(value) => *value = text.parse::<$ty>().map_err(|_| concat!("expected ", stringify!($ty), " value"))?,)+
                    $(Self::$unit => return Err("condition has no value".into())),+
                }
                Ok(())
            }
        }
    }
}
conditions! {
    TimerElapsed => "timer_elapsed" (f32, "seconds", 1.0),
    PlayerDistanceLessThan => "player_distance_less_than" (f32, "distance", 150.0),
    PlayerDistanceGreaterThan => "player_distance_greater_than" (f32, "distance", 150.0),
    HealthBelow => "health_below" (u8, "health (0..255)", 2);
    ActionFinished => "action_finished",
}

pub fn parse_nonnegative(text: &str) -> Result<f32, String> {
    let value = text
        .trim()
        .parse::<f32>()
        .map_err(|_| "expected a number".to_owned())?;
    nonnegative(value)?;
    Ok(value)
}

#[derive(Clone, Copy, Debug)]
pub struct ConditionContext {
    pub state_elapsed: f64,
    pub player_distance: f64,
    pub health: u8,
    pub action_finished: bool,
}

impl Condition {
    pub fn matches(&self, context: ConditionContext) -> bool {
        match *self {
            Self::TimerElapsed(seconds) => context.state_elapsed >= f64::from(seconds),
            Self::PlayerDistanceLessThan(distance) => context.player_distance < f64::from(distance),
            Self::PlayerDistanceGreaterThan(distance) => {
                context.player_distance > f64::from(distance)
            }
            Self::HealthBelow(health) => context.health < health,
            Self::ActionFinished => context.action_finished,
        }
    }
}
