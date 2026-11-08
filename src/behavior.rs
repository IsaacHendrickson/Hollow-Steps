//! Window-free, data-driven enemy state machines. All time is simulation time.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use macroquad::math::Vec2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::player::WorldBounds;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehaviorDefinition {
    pub initial_state: String,
    pub states: Vec<State>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub name: String,
    pub actions: Vec<Action>,
    pub transitions: Vec<Transition>,
}

/// The loader enum and editor catalog are generated together. Add variants here.
macro_rules! actions {
    ($( $variant:ident => $name:literal { $( $field:ident = $default:expr ),+ } ),+ $(,)?) => {
        #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        pub enum Action { $( $variant { $( $field: f32 ),+ } ),+ }
        impl Action {
            pub const TYPES: &'static [&'static str] = &[$($name),+];
            pub fn defaults(kind: usize) -> Option<Self> {
                let name = Self::TYPES.get(kind)?;
                Some(match *name { $( $name => Self::$variant { $($field: $default),+ }, )+ _ => return None })
            }
            pub fn kind(&self) -> usize {
                let name = match self { $(Self::$variant { .. } => $name),+ };
                Self::TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn parameters(&self) -> Vec<(&'static str, String)> {
                match self { $(Self::$variant { $($field),+ } => vec![$((stringify!($field), $field.to_string())),+]),+ }
            }
            pub fn set_parameter(&mut self, index: usize, text: &str) -> Result<(), String> {
                let value = parse_nonnegative(text)?;
                let fields: Vec<&mut f32> = match self { $(Self::$variant { $($field),+ } => vec![$($field),+]),+ };
                let field = fields.into_iter().nth(index).ok_or("missing parameter")?;
                *field = value;
                Ok(())
            }
        }
    }
}
actions! {
    MoveTowardPlayer => "move_toward_player" { speed = 100.0 },
    MoveAwayFromPlayer => "move_away_from_player" { speed = 100.0 },
    Patrol => "patrol" { speed = 80.0 },
    Wait => "wait" { seconds = 1.0 },
    Telegraph => "telegraph" { seconds = 0.5 },
}

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
        .parse::<f32>()
        .map_err(|_| "expected a number".to_owned())?;
    nonnegative(value)?;
    Ok(value)
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub target: String,
    pub condition: Condition,
    /// Inversion allows an exact complement, including distance equality.
    #[serde(default)]
    pub invert: bool,
}

impl BehaviorDefinition {
    pub fn from_ron(source: &str) -> Result<Self, String> {
        let definition: Self = ron::from_str(source).map_err(|error| error.to_string())?;
        definition.validate()?;
        Ok(definition)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut names = BTreeSet::new();
        for state in &self.states {
            if state.name.trim().is_empty() || !names.insert(state.name.as_str()) {
                return Err(format!("empty or duplicate state name {:?}", state.name));
            }
        }
        if !names.contains(self.initial_state.as_str()) {
            return Err(format!("missing initial state {:?}", self.initial_state));
        }
        for state in &self.states {
            for (index, action) in state.actions.iter().enumerate() {
                let value = match action {
                    Action::MoveTowardPlayer { speed }
                    | Action::MoveAwayFromPlayer { speed }
                    | Action::Patrol { speed } => *speed,
                    Action::Wait { seconds } | Action::Telegraph { seconds } => *seconds,
                };
                nonnegative(value)
                    .map_err(|error| format!("state {:?}, action {index}: {error}", state.name))?;
            }
            for transition in &state.transitions {
                if !names.contains(transition.target.as_str()) {
                    return Err(format!(
                        "state {:?}: transition to missing state {:?}",
                        state.name, transition.target
                    ));
                }
                match transition.condition {
                    Condition::TimerElapsed(value)
                    | Condition::PlayerDistanceLessThan(value)
                    | Condition::PlayerDistanceGreaterThan(value) => {
                        nonnegative(value).map_err(|error| {
                            format!("state {:?}, transition condition: {error}", state.name)
                        })?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn nonnegative(value: f32) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!("expected a finite nonnegative value, got {value}"))
    }
}

pub(crate) fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let source =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    ron::from_str(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn ron_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in
        std::fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .path();
        if path.extension().is_some_and(|extension| extension == "ron") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn load_behaviors(
    directory: &Path,
) -> Result<BTreeMap<String, Arc<BehaviorDefinition>>, String> {
    let mut definitions = BTreeMap::new();
    for path in ron_files(directory)? {
        let definition: BehaviorDefinition = read_ron(&path)?;
        definition
            .validate()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{}: invalid behavior filename", path.display()))?;
        definitions.insert(name.to_owned(), Arc::new(definition));
    }
    Ok(definitions)
}

/// Inputs supplied by the owning entity, rather than a renderer or global world.
#[derive(Clone, Copy, Debug)]
pub struct BehaviorContext {
    pub player_position: Vec2,
    pub health: u8,
    pub size: Vec2,
    pub bounds: WorldBounds,
    pub patrol_left: f32,
    pub patrol_right: f32,
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

/// Each instance owns its timers and action cursor; definitions may be shared.
#[derive(Clone, Debug)]
pub struct Behavior {
    definition: Arc<BehaviorDefinition>,
    state: usize,
    action: usize,
    state_elapsed: f64,
    action_elapsed: f64,
    direction: f32,
}

impl Behavior {
    pub fn new(definition: Arc<BehaviorDefinition>) -> Result<Self, String> {
        definition.validate()?;
        let state = definition
            .states
            .iter()
            .position(|state| state.name == definition.initial_state)
            .expect("validated initial state");
        Ok(Self {
            definition,
            state,
            action: 0,
            state_elapsed: 0.0,
            action_elapsed: 0.0,
            direction: 1.0,
        })
    }

    pub fn state_name(&self) -> &str {
        &self.definition.states[self.state].name
    }

    pub fn state_elapsed(&self) -> f64 {
        self.state_elapsed
    }

    pub fn action_finished(&self) -> bool {
        self.action == self.definition.states[self.state].actions.len()
    }

    pub fn is_telegraphing(&self) -> bool {
        matches!(
            self.definition.states[self.state].actions.get(self.action),
            Some(Action::Telegraph { .. })
        )
    }

    fn transition(&mut self, position: Vec2, context: BehaviorContext) -> bool {
        let difference = context.player_position.as_dvec2() - position.as_dvec2();
        let conditions = ConditionContext {
            state_elapsed: self.state_elapsed,
            player_distance: difference.length(),
            health: context.health,
            action_finished: self.action_finished(),
        };
        let target = self.definition.states[self.state]
            .transitions
            .iter()
            .find(|transition| transition.condition.matches(conditions) != transition.invert)
            .map(|transition| transition.target.as_str());
        let Some(target) = target else {
            return false;
        };
        self.state = self
            .definition
            .states
            .iter()
            .position(|state| state.name == target)
            .expect("validated transition");
        self.action = 0;
        self.action_elapsed = 0.0;
        self.state_elapsed = 0.0;
        true
    }

    /// Transitions are checked before and after the action sequence, with at
    /// most one transition per update (including self-transitions).
    pub fn update(&mut self, position: &mut Vec2, context: BehaviorContext, delta_seconds: f32) {
        if !delta_seconds.is_finite()
            || delta_seconds <= 0.0
            || !context.player_position.is_finite()
        {
            return;
        }
        let transitioned = self.transition(*position, context);
        let dt = f64::from(delta_seconds);
        self.state_elapsed += dt;
        let mut remaining = dt;
        while let Some(action) = self.definition.states[self.state].actions.get(self.action) {
            let (used, finished) = match *action {
                Action::Wait { seconds } | Action::Telegraph { seconds } => {
                    let duration = f64::from(seconds);
                    let used = remaining.min((duration - self.action_elapsed).max(0.0));
                    self.action_elapsed += used;
                    (used, self.action_elapsed >= duration)
                }
                Action::MoveTowardPlayer { speed } => {
                    let target = context.player_position.x.clamp(
                        context.bounds.left + context.size.x / 2.0,
                        context.bounds.right - context.size.x / 2.0,
                    );
                    self.move_to(position, target, speed, remaining)
                }
                Action::MoveAwayFromPlayer { speed } => {
                    let difference = position.x - context.player_position.x;
                    if difference != 0.0 {
                        self.direction = difference.signum();
                    }
                    let target = if self.direction > 0.0 {
                        context.bounds.right - context.size.x / 2.0
                    } else {
                        context.bounds.left + context.size.x / 2.0
                    };
                    self.move_to(position, target, speed, remaining)
                }
                Action::Patrol { speed } => {
                    self.patrol(position, context, f64::from(speed) * remaining);
                    (remaining, false)
                }
            };
            remaining = (remaining - used).max(0.0);
            if !finished {
                break;
            }
            self.action += 1;
            self.action_elapsed = 0.0;
        }
        position.y = context.bounds.floor_y - context.size.y / 2.0;
        if !transitioned {
            self.transition(*position, context);
        }
    }

    fn move_to(&mut self, position: &mut Vec2, target: f32, speed: f32, dt: f64) -> (f64, bool) {
        let difference = f64::from(target) - f64::from(position.x);
        if difference == 0.0 {
            return (0.0, true);
        }
        self.direction = difference.signum() as f32;
        if speed == 0.0 {
            return (dt, false);
        }
        let duration = difference.abs() / f64::from(speed);
        if dt >= duration {
            position.x = target;
            (duration, true)
        } else {
            position.x =
                (f64::from(position.x) + difference.signum() * f64::from(speed) * dt) as f32;
            (dt, false)
        }
    }

    fn patrol(&mut self, position: &mut Vec2, context: BehaviorContext, mut travel: f64) {
        // Return from a chase or knockback without teleporting onto the interval.
        let nearest = position.x.clamp(context.patrol_left, context.patrol_right);
        let difference = f64::from(nearest) - f64::from(position.x);
        if difference != 0.0 {
            self.direction = difference.signum() as f32;
            let step = travel.min(difference.abs());
            position.x = (f64::from(position.x) + difference.signum() * step) as f32;
            travel -= step;
        }
        if position.x >= context.patrol_left && position.x <= context.patrol_right {
            let width = f64::from(context.patrol_right) - f64::from(context.patrol_left);
            if width > 0.0 {
                let offset = f64::from(position.x) - f64::from(context.patrol_left);
                let phase = if self.direction > 0.0 {
                    offset
                } else {
                    2.0 * width - offset
                };
                let phase = (phase + travel) % (2.0 * width);
                self.direction = if phase < width { 1.0 } else { -1.0 };
                position.x = (f64::from(context.patrol_left)
                    + if phase < width {
                        phase
                    } else {
                        2.0 * width - phase
                    }) as f32;
            }
        }
    }
}
