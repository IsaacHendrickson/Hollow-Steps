//! Window-free, data-driven enemy state machines. All time is simulation time.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use macroquad::math::Vec2;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::enemy_attack::{valid_attack, AttackBox, AttackRuntime, DamageHit};
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

/// Shared action library. Boss-only actions are excluded from the enemy editor catalog.
macro_rules! actions {
    ($( $(#[$meta:meta])* $variant:ident => $name:literal { $( $field:ident : $ty:ty => ($label:literal, $validate:ident) ),+ } ),+ $(,)?;
     $( $boss:ident => $boss_name:literal { $( $param:ident : $param_ty:ty => ($param_label:literal, $param_validate:ident) ),+ } ),+ $(,)?) => {
        #[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
        #[serde(rename_all = "snake_case", deny_unknown_fields)]
        pub enum Action { $( $(#[$meta])* $variant { $( $field: $ty ),+ } ),+, $( $boss { $( $param: $param_ty ),+ } ),+ }
        impl Action {
            pub const TYPES: &'static [&'static str] = &[$($name),+];
            pub const ALL_TYPES: &'static [&'static str] = &[$($name),+, $($boss_name),+];
            pub fn all_kind(&self) -> usize {
                let name = match self { $(Self::$variant { .. } => $name,)+ $(Self::$boss { .. } => $boss_name),+ };
                Self::ALL_TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn all_defaults(kind: usize) -> Option<Self> {
                static DEFAULTS: OnceLock<Vec<Action>> = OnceLock::new();
                DEFAULTS.get_or_init(|| {
                    let mut actions: Vec<Action> = ron::from_str(include_str!("../data/action_defaults.ron")).expect("valid action defaults");
                    actions.extend(ron::from_str::<Vec<Action>>(include_str!("../data/boss_action_defaults.ron")).expect("valid boss action defaults"));
                    actions
                }).iter().find(|action| action.all_kind() == kind).cloned()
            }
            pub fn defaults(kind: usize) -> Option<Self> {
                static DEFAULTS: OnceLock<Vec<Action>> = OnceLock::new();
                DEFAULTS.get_or_init(|| ron::from_str(include_str!("../data/action_defaults.ron"))
                    .expect("valid action catalog defaults"))
                    .iter().find(|action| action.kind() == kind).cloned()
            }
            pub fn kind(&self) -> usize {
                let name = match self { $(Self::$variant { .. } => $name,)+ _ => return usize::MAX };
                Self::TYPES.iter().position(|n| *n == name).expect("generated catalog")
            }
            pub fn parameters(&self) -> Vec<(&'static str, String)> {
                match self { $(Self::$variant { $($field),+ } => vec![$((stringify!($field), $field.to_string())),+],)+ $(Self::$boss { $($param),+ } => vec![$((stringify!($param), $param.to_string())),+]),+ }
            }
            pub fn parameter_labels(&self) -> &'static [&'static str] {
                match self { $(Self::$variant { .. } => &[$($label),+],)+ $(Self::$boss { .. } => &[$($param_label),+]),+ }
            }
            pub fn validate(&self) -> Result<(), String> {
                match self { $(Self::$variant { $($field),+ } => { $(
                    $validate(*$field).map_err(|error| format!("{}: {error}", $label))?;
                )+ },)+ $(Self::$boss { $($param),+ } => { $(
                    $param_validate(*$param).map_err(|error| format!("{} ({}): {error}", stringify!($param), $param_label))?;
                )+ }),+ }
                Ok(())
            }
            pub fn set_parameter(&mut self, index: usize, text: &str) -> Result<(), String> {
                match self { $(Self::$variant { $($field),+ } => {
                    let mut fields = [$(stringify!($field)),+].into_iter();
                    let name = fields.nth(index).ok_or("missing parameter")?;
                    match name { $(stringify!($field) => {
                        let value = text.trim().parse::<$ty>()
                            .map_err(|_| format!("{}: expected {} value", $label, stringify!($ty)))?;
                        $validate(value).map_err(|error| format!("{}: {error}", $label))?;
                        *$field = value;
                        Ok(())
                    },)+ _ => unreachable!("generated parameter") }
                },)+ $(Self::$boss { $($param),+ } => {
                    let mut fields = [$(stringify!($param)),+].into_iter();
                    let name = fields.nth(index).ok_or("missing parameter")?;
                    match name { $(stringify!($param) => {
                        let value = text.trim().parse::<$param_ty>()
                            .map_err(|_| format!("{}: expected {} value", $param_label, stringify!($param_ty)))?;
                        $param_validate(value).map_err(|error| format!("{}: {error}", $param_label))?;
                        *$param = value;
                        Ok(())
                    },)+ _ => unreachable!("generated parameter") }
                }),+ }
            }
        }
    }
}
actions! {
    Lunge => "lunge" {
        speed: f32 => ("lunge speed (pixels/second)", nonnegative),
        duration_secs: f32 => ("lunge duration (seconds)", nonnegative),
        attack: AttackBox => ("attack box", valid_attack),
        recovery_secs: f32 => ("recovery duration (seconds)", nonnegative),
        telegraph_secs: f32 => ("wind-up duration (seconds)", nonnegative)
    },
    MoveTowardPlayer => "move_toward_player" { speed: f32 => ("speed toward player (pixels/second)", nonnegative) },
    MoveAwayFromPlayer => "move_away_from_player" { speed: f32 => ("speed away from player (pixels/second)", nonnegative) },
    Patrol => "patrol" { speed: f32 => ("patrol speed (pixels/second)", nonnegative) },
    Wait => "wait" { seconds: f32 => ("wait duration (seconds)", nonnegative) },
    Telegraph => "telegraph" { seconds: f32 => ("telegraph duration (seconds)", nonnegative) },
    #[serde(alias = "jump_to")]
    JumpToPlayer => "jump_to_player" {
        speed: f32 => ("jump speed (pixels/second)", nonnegative),
        dx: f32 => ("target horizontal offset from player (pixels, positive right)", finite),
        dy: f32 => ("target vertical offset from player (pixels, negative above)", finite)
    },
    Hover => "hover" { seconds: f32 => ("hover duration (seconds)", nonnegative) },
    SlamDown => "slam_down" { speed: f32 => ("downward slam speed (pixels/second)", nonnegative) },
    AreaBlast => "area_blast" {
        count: u16 => ("blast count (integer 0..65535)", valid_count),
        radius: f32 => ("blast radius (pixels)", nonnegative),
        fuse_time: f32 => ("blast fuse time (seconds)", nonnegative),
        damage: u8 => ("blast damage (integer 0..255)", valid_damage)
    };
    GlideToPlayer => "glide_to_player" {
        speed: f32 => ("Glide speed (px/s)", positive), above_floor: f32 => ("Height above floor (px)", nonnegative)
    },
    GlideToFloor => "glide_to_floor" { speed: f32 => ("Glide speed (px/s)", positive) },
    DamageTarget => "damage_target" {
        radius: f32 => ("Hit radius (px)", nonnegative), damage: u8 => ("Hit damage (0..255)", valid_damage)
    },
    Summon => "summon" { inset: f32 => ("Distance from arena edges (px)", nonnegative) },
    SpawnRings => "spawn_rings" { count: u16 => ("Ring count (1..256)", boss_count) },
    ExplodeRings => "explode_rings" {
        radius: f32 => ("Explosion radius (px)", nonnegative), damage: u8 => ("Explosion damage (0..255)", valid_damage)
    },
    SpawnShockwave => "spawn_shockwave" {
        radius: f32 => ("Shockwave radius (px)", nonnegative), damage: u8 => ("Shockwave damage (0..255)", valid_damage)
    },
}

pub(crate) fn positive(value: f32) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err("must be finite and positive".into())
    }
}
fn boss_count(value: u16) -> Result<(), String> {
    if (1..=256).contains(&value) {
        Ok(())
    } else {
        Err("must be an integer in 1..256".into())
    }
}

fn finite(value: f32) -> Result<(), String> {
    if value.is_finite() {
        Ok(())
    } else {
        Err("expected a finite value".into())
    }
}

// Integer ranges are enforced by the loader and editor's typed parsing.
fn valid_count(_: u16) -> Result<(), String> {
    Ok(())
}
fn valid_damage(_: u8) -> Result<(), String> {
    Ok(())
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
    /// A stationary starting point for authoring, with no inherited actions or links.
    pub fn blank() -> Self {
        Self {
            initial_state: "idle".into(),
            states: vec![State {
                name: "idle".into(),
                actions: vec![],
                transitions: vec![],
            }],
        }
    }

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
                if action.kind() == usize::MAX {
                    return Err(format!(
                        "state {:?}, action {index}: boss-only action",
                        state.name
                    ));
                }
                action
                    .validate()
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
    action_started: bool,
    jump_target: Option<Vec2>,
    airborne: bool,
    blasts: Vec<Blast>,
    random_state: Option<u32>,
    attack_runtime: Option<AttackRuntime>,
}

/// A stationary warning circle; `exploded` is a damage event for one update.
#[derive(Clone, Debug, PartialEq)]
pub struct Blast {
    pub position: Vec2,
    pub radius: f32,
    pub damage: u8,
    pub exploded: bool,
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
            action_started: false,
            jump_target: None,
            airborne: false,
            blasts: Vec::new(),
            random_state: None,
            attack_runtime: None,
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

    pub fn blasts(&self) -> &[Blast] {
        &self.blasts
    }

    pub fn blast_damage(&self, position: Vec2, size: Vec2) -> u8 {
        self.blasts
            .iter()
            .filter(|blast| {
                let nearest = blast
                    .position
                    .clamp(position - size / 2.0, position + size / 2.0);
                blast.exploded
                    && (nearest.as_dvec2() - blast.position.as_dvec2()).length()
                        <= f64::from(blast.radius)
            })
            .map(|blast| blast.damage)
            .max()
            .unwrap_or(0)
    }

    pub(crate) fn clear_blast_hits(&mut self) {
        self.blasts.retain(|blast| !blast.exploded);
    }

    pub fn attack_hit(&self, origin: Vec2, position: Vec2, size: Vec2) -> Option<DamageHit> {
        self.attack_runtime
            .as_ref()?
            .active
            .filter(|attack| attack.overlaps(origin, position, size))
            .map(AttackBox::hit)
    }

    pub fn attack_warning(&self) -> Option<AttackBox> {
        self.attack_runtime.as_ref()?.warning()
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
        self.action_started = false;
        self.jump_target = None;
        self.attack_runtime = None;
        // Interrupting a fuse cancels its pending attack; keep hits already emitted.
        self.blasts.retain(|blast| blast.exploded);
        true
    }

    /// Transitions are checked before and after the action sequence, with at
    /// most one transition per update (including self-transitions).
    pub fn update(&mut self, position: &mut Vec2, context: BehaviorContext, delta_seconds: f32) {
        self.clear_blast_hits();
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
        while let Some(action) = self.definition.states[self.state]
            .actions
            .get(self.action)
            .cloned()
        {
            if matches!(
                action,
                Action::MoveTowardPlayer { .. }
                    | Action::MoveAwayFromPlayer { .. }
                    | Action::Patrol { .. }
                    | Action::Wait { .. }
                    | Action::Telegraph { .. }
            ) {
                self.airborne = false;
                position.y = context.bounds.floor_y - context.size.y / 2.0;
            }
            let (used, finished) = match action {
                Action::Lunge {
                    speed,
                    duration_secs,
                    attack,
                    recovery_secs,
                    telegraph_secs,
                } => {
                    self.airborne = true;
                    self.attack_runtime
                        .get_or_insert_with(|| {
                            AttackRuntime::lunge(
                                *position,
                                context,
                                speed,
                                duration_secs,
                                attack,
                                recovery_secs,
                                telegraph_secs,
                            )
                        })
                        .step(position, context, remaining)
                }
                Action::Wait { seconds }
                | Action::Telegraph { seconds }
                | Action::Hover { seconds } => {
                    if matches!(action, Action::Hover { .. }) {
                        self.airborne = true;
                    }
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
                Action::JumpToPlayer { dx, dy, speed } => {
                    self.airborne = true;
                    let target = *self.jump_target.get_or_insert_with(|| {
                        Self::clamped_jump_target(
                            context.player_position,
                            context.size,
                            context.bounds,
                            dx,
                            dy,
                        )
                    });
                    Self::move_in_plane(position, target, speed, remaining)
                }
                Action::SlamDown { speed } => {
                    self.airborne = true;
                    Self::move_in_plane(
                        position,
                        Vec2::new(position.x, context.bounds.floor_y - context.size.y / 2.0),
                        speed,
                        remaining,
                    )
                }
                Action::AreaBlast {
                    count,
                    radius,
                    fuse_time,
                    damage,
                } => {
                    if !self.action_started {
                        for _ in 0..count {
                            let x = f64::from(context.bounds.left)
                                + self.next_random(*position)
                                    * (f64::from(context.bounds.right)
                                        - f64::from(context.bounds.left));
                            let y = self.next_random(*position) * f64::from(context.bounds.floor_y);
                            self.blasts.push(Blast {
                                position: Vec2::new(x as f32, y as f32),
                                radius,
                                damage,
                                exploded: false,
                            });
                        }
                    }
                    let duration = f64::from(fuse_time);
                    let used = remaining.min((duration - self.action_elapsed).max(0.0));
                    self.action_elapsed += used;
                    let finished = self.action_elapsed >= duration;
                    if finished {
                        for blast in &mut self.blasts {
                            blast.exploded = true;
                        }
                    }
                    (used, finished)
                }
                _ => unreachable!("validated enemy action"),
            };
            self.action_started = true;
            remaining = (remaining - used).max(0.0);
            if !finished {
                break;
            }
            self.attack_runtime = None;
            self.action += 1;
            self.action_elapsed = 0.0;
            self.action_started = false;
            self.jump_target = None;
        }
        if !self.airborne {
            position.y = context.bounds.floor_y - context.size.y / 2.0;
        }
        if !transitioned {
            self.transition(*position, context);
        }
    }

    pub(crate) fn clamped_jump_target(
        player: Vec2,
        size: Vec2,
        bounds: WorldBounds,
        dx: f32,
        dy: f32,
    ) -> Vec2 {
        let half = size.as_dvec2() / 2.0;
        let target = player.as_dvec2() + macroquad::math::DVec2::new(f64::from(dx), f64::from(dy));
        Vec2::new(
            target.x.clamp(
                f64::from(bounds.left) + half.x,
                f64::from(bounds.right) - half.x,
            ) as f32,
            target.y.clamp(
                half.y.min(f64::from(bounds.floor_y) - half.y),
                f64::from(bounds.floor_y) - half.y,
            ) as f32,
        )
    }

    pub(crate) fn move_in_plane(
        position: &mut Vec2,
        target: Vec2,
        speed: f32,
        dt: f64,
    ) -> (f64, bool) {
        let difference = target.as_dvec2() - position.as_dvec2();
        let distance = difference.length();
        if distance == 0.0 {
            return (0.0, true);
        }
        if speed == 0.0 {
            return (dt, false);
        }
        let duration = distance / f64::from(speed);
        if dt >= duration {
            *position = target;
            (duration, true)
        } else {
            *position = (position.as_dvec2() + difference * (dt / duration)).as_vec2();
            (dt, false)
        }
    }

    fn next_random(&mut self, position: Vec2) -> f64 {
        // Per-instance deterministic PRNG; coefficients are algorithm constants.
        let state = self
            .random_state
            .get_or_insert(position.x.to_bits().rotate_left(16) ^ position.y.to_bits());
        *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        f64::from(*state) / f64::from(u32::MAX)
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
