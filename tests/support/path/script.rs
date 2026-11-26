//! A path script: a short list of steps, built with readable helpers.
//!
//! ```text
//! PathScript::new([run_right(160), jump(120), run_right(40), dash(), run_right(120), wait(30)])
//!     .with_ability(Ability::Dash)
//! ```
//!
//! A heading (left, right or none) is held from the step that sets it until another sets it:
//! `run_left` and `run_right` set it, `wait` clears it, and `jump` and `dash` keep it, so a jump
//! out of a run is a running jump. A jump keeps its key down for `hold` ticks while the script
//! goes on, so a dash can be pressed in the middle of a held jump.
use hollow_steps::ability::Ability;
use hollow_steps::player::{PlayerConfig, PlayerInput};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Heading {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Hold a direction for this many ticks.
    Run(Heading, usize),
    /// Hold no direction for this many ticks.
    Wait(usize),
    /// Press the jump key on one tick and keep it down for `hold` ticks (the press tick
    /// included; at least one) while the script goes on. Takes one tick of the script.
    Jump { hold: usize },
    /// Press the dash key on one tick.
    Dash,
}

pub fn run_left(ticks: usize) -> Step {
    Step::Run(Heading::Left, ticks)
}

pub fn run_right(ticks: usize) -> Step {
    Step::Run(Heading::Right, ticks)
}

pub fn wait(ticks: usize) -> Step {
    Step::Wait(ticks)
}

pub fn jump(hold: usize) -> Step {
    Step::Jump { hold }
}

pub fn dash() -> Step {
    Step::Dash
}

/// What the player holds when the replay starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Loadout {
    pub abilities: Vec<Ability>,
    pub keys: Vec<u8>,
}

/// What a proof takes away: an ability (a gate) or a key (a locked exit).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lack {
    Ability(Ability),
    Key(u8),
}

impl From<Ability> for Lack {
    fn from(ability: Ability) -> Self {
        Self::Ability(ability)
    }
}

impl std::fmt::Display for Lack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ability(ability) => write!(f, "{}", ability.name()),
            Self::Key(key) => write!(f, "Key {key}"),
        }
    }
}

/// The keys of one tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tick {
    pub axis: f32,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub dash_pressed: bool,
}

impl Tick {
    pub fn input(self) -> PlayerInput {
        PlayerInput {
            move_axis: self.axis,
            jump_pressed: self.jump_pressed,
            jump_held: self.jump_held,
            dash_pressed: self.dash_pressed,
            ..PlayerInput::default()
        }
    }
}

/// Steps, what the player holds and the numbers it plays at (the shipped tuning by default).
#[derive(Clone, Debug)]
pub struct PathScript {
    pub steps: Vec<Step>,
    pub holds: Loadout,
    pub config: PlayerConfig,
}

impl PathScript {
    pub fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            holds: Loadout::default(),
            config: PlayerConfig::default(),
        }
    }

    pub fn with_ability(mut self, ability: Ability) -> Self {
        if !self.holds.abilities.contains(&ability) {
            self.holds.abilities.push(ability);
        }
        self
    }

    pub fn with_key(mut self, key: u8) -> Self {
        if !self.holds.keys.contains(&key) {
            self.holds.keys.push(key);
        }
        self
    }

    pub fn with_config(mut self, config: PlayerConfig) -> Self {
        self.config = config;
        self
    }

    /// The same script holding one thing less.
    pub fn without(&self, lack: Lack) -> Self {
        let mut script = self.clone();
        match lack {
            Lack::Ability(ability) => script.holds.abilities.retain(|held| *held != ability),
            Lack::Key(key) => script.holds.keys.retain(|held| *held != key),
        }
        script
    }

    /// The keys of every tick, in order.
    pub fn ticks(&self) -> Vec<Tick> {
        let (mut axis, mut held_for) = (0.0, 0_usize);
        let mut ticks = Vec::new();
        let mut push = |axis: f32, pressed: bool, dash: bool, held_for: &mut usize| {
            ticks.push(Tick {
                axis,
                jump_pressed: pressed,
                jump_held: *held_for > 0,
                dash_pressed: dash,
            });
            *held_for = held_for.saturating_sub(1);
        };
        for step in &self.steps {
            match *step {
                Step::Run(heading, count) => {
                    axis = if heading == Heading::Right { 1.0 } else { -1.0 };
                    (0..count).for_each(|_| push(axis, false, false, &mut held_for));
                }
                Step::Wait(count) => {
                    axis = 0.0;
                    (0..count).for_each(|_| push(axis, false, false, &mut held_for));
                }
                Step::Jump { hold } => {
                    held_for = hold.max(1);
                    push(axis, true, false, &mut held_for);
                }
                Step::Dash => push(axis, false, true, &mut held_for),
            }
        }
        ticks
    }

    /// How many ticks the script lasts.
    pub fn length(&self) -> usize {
        self.ticks().len()
    }
}
