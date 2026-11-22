//! Abilities: things the player can do once they have found them (Dash, Double Jump). They are
//! run state: a run starts without any and loses them when it ends (`Player::reset_for_run`).
//!
//! The game never checks an ability at a door. A room's geometry is what needs one (a gap too
//! wide to jump, a ledge too high); a key is different, because a locked exit refuses entry
//! (`crate::exit_lock`). `Item::Dash` and `Item::DoubleJump` stay the pickup kinds in room files,
//! and picking one up grants its ability (`Item::grants`).

use serde::{Deserialize, Serialize};

/// Something the player can do once granted. It is written by name in room files (the gates of
/// `crate::gates`), so a name that does not exist is a load error.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ability {
    Dash,
    /// One extra jump in the air, refilled on landing (`Player`'s `air_jump`).
    DoubleJump,
}

impl Ability {
    /// Every ability, so lists built from it (the editor's, later) grow by themselves.
    pub const ALL: &'static [Self] = &[Self::Dash, Self::DoubleJump];

    pub fn name(self) -> &'static str {
        match self {
            Self::Dash => "Dash",
            Self::DoubleJump => "Double Jump",
        }
    }
}

/// The abilities the player has. Granting one twice changes nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AbilitySet {
    granted: Vec<Ability>,
}

impl AbilitySet {
    pub fn has(&self, ability: Ability) -> bool {
        self.granted.contains(&ability)
    }

    pub fn grant(&mut self, ability: Ability) {
        if !self.has(ability) {
            self.granted.push(ability);
        }
    }

    /// Takes every ability away (a run ending).
    pub fn clear(&mut self) {
        self.granted.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.granted.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_set_has_nothing() {
        let set = AbilitySet::default();
        assert!(set.is_empty());
        for ability in Ability::ALL {
            assert!(!set.has(*ability), "{}", ability.name());
        }
    }

    #[test]
    fn grant_adds_once_and_clear_removes_everything() {
        let mut set = AbilitySet::default();
        set.grant(Ability::Dash);
        set.grant(Ability::Dash);
        assert!(set.has(Ability::Dash));
        assert_eq!(set, {
            let mut once = AbilitySet::default();
            once.grant(Ability::Dash);
            once
        });

        set.clear();
        assert!(!set.has(Ability::Dash));
        assert!(set.is_empty());
        set.clear();
        assert!(set.is_empty(), "clearing an empty set is fine");
    }

    #[test]
    fn double_jump_is_listed_named_and_granted_like_any_ability() {
        assert!(Ability::ALL.contains(&Ability::DoubleJump));
        assert_eq!(Ability::DoubleJump.name(), "Double Jump");
        let mut set = AbilitySet::default();
        set.grant(Ability::DoubleJump);
        assert!(set.has(Ability::DoubleJump) && !set.has(Ability::Dash));
        set.clear();
        assert!(set.is_empty());
        // Written by variant name in room files (the gates of `crate::gates`).
        let text = ron::to_string(&Ability::DoubleJump).unwrap();
        assert_eq!(text, "DoubleJump");
        assert_eq!(
            ron::from_str::<Ability>(&text).unwrap(),
            Ability::DoubleJump
        );
    }

    #[test]
    fn every_ability_has_a_name() {
        assert_eq!(Ability::Dash.name(), "Dash");
        assert!(Ability::ALL
            .iter()
            .all(|ability| !ability.name().is_empty()));
    }
}
