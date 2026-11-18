//! Rebindable keyboard controls: which keys do each of the player's actions.
//!
//! Pure state and rules, no window. `Keybinds` holds up to two keys per `Action`: a primary key
//! that is always set and an optional alternate. No key is ever bound twice, and only keys from
//! `keys::BINDABLE` can be bound. `store` keeps the bindings in a file in the user's config
//! folder (design note: `docs/keybinds.md`), `input` turns them into a `PlayerInput` from this
//! frame's key states, and `notes` words what a rebind did for the settings screen.

use macroquad::input::KeyCode;

mod input;
mod keys;
mod notes;
mod store;

pub use input::{hint_line, player_input};
pub use keys::{first_pressed, is_bindable, key_from_name, key_name, BINDABLE};
pub use notes::{error_note, key_label, swap_note};
pub use store::{config_dir, default_path, load, remove, save, Platform, FILE_NAME};

/// Something the player does with a key.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    MoveLeft,
    MoveRight,
    Jump,
    Attack,
    Dash,
    Heal,
    /// Picks things up and, held while swinging, strikes upward.
    Up,
    /// Held while swinging in the air, strikes downward.
    Down,
    Inventory,
}

impl Action {
    /// Every action in the order the settings screen lists them.
    pub const ALL: [Action; 9] = [
        Action::MoveLeft,
        Action::MoveRight,
        Action::Jump,
        Action::Attack,
        Action::Dash,
        Action::Heal,
        Action::Up,
        Action::Down,
        Action::Inventory,
    ];

    /// The name shown on the settings screen.
    pub fn label(self) -> &'static str {
        match self {
            Self::MoveLeft => "Move left",
            Self::MoveRight => "Move right",
            Self::Jump => "Jump",
            Self::Attack => "Attack",
            Self::Dash => "Dash",
            Self::Heal => "Heal",
            Self::Up => "Up (pick up)",
            Self::Down => "Down",
            Self::Inventory => "Inventory",
        }
    }

    /// The name in the keybinds file.
    pub fn name(self) -> &'static str {
        match self {
            Self::MoveLeft => "move_left",
            Self::MoveRight => "move_right",
            Self::Jump => "jump",
            Self::Attack => "attack",
            Self::Dash => "dash",
            Self::Heal => "heal",
            Self::Up => "up",
            Self::Down => "down",
            Self::Inventory => "inventory",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.name() == name)
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// One of the two keys an action can have.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Slot {
    #[default]
    Primary,
    Alternate,
}

impl Slot {
    pub const ALL: [Slot; 2] = [Slot::Primary, Slot::Alternate];

    pub fn label(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Alternate => "alternate",
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    /// The slot at `index`, with an index past the end counting as the last slot.
    pub fn from_index(index: usize) -> Self {
        Self::ALL[index.min(Self::ALL.len() - 1)]
    }
}

/// The key an action lost to a rebind, and what that slot holds now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Swap {
    pub action: Action,
    pub slot: Slot,
    /// The key the slot got in exchange: the one the rebound slot used to have, if it had any.
    pub now: Option<KeyCode>,
}

/// What a successful rebind did besides setting the key.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rebound {
    /// Set when the new key was in use elsewhere, so the two slots traded keys.
    pub swapped: Option<Swap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RebindError {
    /// The key is not in `keys::BINDABLE` (Escape, Enter, the F keys, ...).
    NotBindable,
    /// An empty alternate slot was given a key that is another action's primary key: the trade
    /// would leave that action without a primary key.
    PrimaryOf(Action),
    /// The primary key cannot be cleared; replace it instead.
    PrimaryRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keybinds {
    keys: [[Option<KeyCode>; 2]; Action::ALL.len()],
}

/// The shipped controls: action, primary key, alternate key.
const DEFAULTS: [(Action, KeyCode, Option<KeyCode>); Action::ALL.len()] = [
    (Action::MoveLeft, KeyCode::A, Some(KeyCode::Left)),
    (Action::MoveRight, KeyCode::D, Some(KeyCode::Right)),
    (Action::Jump, KeyCode::Space, Some(KeyCode::Z)),
    (Action::Attack, KeyCode::X, None),
    (Action::Dash, KeyCode::LeftShift, Some(KeyCode::RightShift)),
    (Action::Heal, KeyCode::C, None),
    (Action::Up, KeyCode::Up, None),
    (Action::Down, KeyCode::Down, None),
    (Action::Inventory, KeyCode::I, None),
];

impl Default for Keybinds {
    fn default() -> Self {
        let mut keys = [[None; 2]; Action::ALL.len()];
        for (action, primary, alternate) in DEFAULTS {
            keys[action.index()] = [Some(primary), alternate];
        }
        Self { keys }
    }
}

impl Keybinds {
    /// The key in `slot` of `action`, if it has one.
    pub fn key(&self, action: Action, slot: Slot) -> Option<KeyCode> {
        self.keys[action.index()][slot.index()]
    }

    /// The keys of `action`, primary first.
    pub fn keys(&self, action: Action) -> impl Iterator<Item = KeyCode> + '_ {
        self.keys[action.index()].iter().flatten().copied()
    }

    /// True when `test` says any key of `action` is down (or just pressed, for an edge test).
    pub fn triggered(&self, action: Action, test: impl Fn(KeyCode) -> bool) -> bool {
        self.keys(action).any(test)
    }

    /// The action and slot `key` is bound to.
    pub fn action_of(&self, key: KeyCode) -> Option<(Action, Slot)> {
        Action::ALL.into_iter().find_map(|action| {
            Slot::ALL
                .into_iter()
                .find(|&slot| self.key(action, slot) == Some(key))
                .map(|slot| (action, slot))
        })
    }

    pub fn is_default(&self) -> bool {
        *self == Self::default()
    }

    /// Puts every action back on its shipped keys.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Binds `key` to `slot` of `action`. A key that is already in use elsewhere is taken from
    /// there and the two slots trade keys, so nothing is ever bound twice and no action loses
    /// its primary key. Nothing changes when this fails.
    pub fn rebind(
        &mut self,
        action: Action,
        slot: Slot,
        key: KeyCode,
    ) -> Result<Rebound, RebindError> {
        if !is_bindable(key) {
            return Err(RebindError::NotBindable);
        }
        let old = self.key(action, slot);
        if old == Some(key) {
            return Ok(Rebound::default());
        }
        let taken_from = self.action_of(key);
        if let Some((other, other_slot)) = taken_from {
            if old.is_none() && other_slot == Slot::Primary {
                return Err(RebindError::PrimaryOf(other));
            }
            self.keys[other.index()][other_slot.index()] = old;
        }
        self.keys[action.index()][slot.index()] = Some(key);
        Ok(Rebound {
            swapped: taken_from.map(|(other, other_slot)| Swap {
                action: other,
                slot: other_slot,
                now: old,
            }),
        })
    }

    /// Empties the alternate slot of `action`. The primary key cannot be cleared.
    pub fn clear(&mut self, action: Action, slot: Slot) -> Result<(), RebindError> {
        if slot == Slot::Primary {
            return Err(RebindError::PrimaryRequired);
        }
        self.keys[action.index()][slot.index()] = None;
        Ok(())
    }

    /// Checks what every `Keybinds` must satisfy: each action has a primary key, every key is
    /// bindable, and no key is used twice. The error names the action and key at fault.
    pub fn check(&self) -> Result<(), String> {
        let mut seen: Vec<(KeyCode, Action)> = Vec::new();
        for action in Action::ALL {
            if self.key(action, Slot::Primary).is_none() {
                return Err(format!("{} has no primary key", action.label()));
            }
            for key in self.keys(action) {
                let Some(name) = key_name(key) else {
                    return Err(format!("{}: {key:?} cannot be bound", action.label()));
                };
                if let Some((_, other)) = seen.iter().find(|(used, _)| *used == key) {
                    return Err(format!(
                        "{name} is bound to both {} and {}",
                        other.label(),
                        action.label()
                    ));
                }
                seen.push((key, action));
            }
        }
        Ok(())
    }

    /// Sets a whole action from parsed file data (used by `store`, which checks afterwards).
    fn set_action(&mut self, action: Action, keys: [Option<KeyCode>; 2]) {
        self.keys[action.index()] = keys;
    }
}

#[cfg(test)]
mod store_tests;
#[cfg(test)]
mod tests;
