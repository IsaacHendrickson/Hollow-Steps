//! Abilities: things the player can do once they have found them (Dash today). They are run
//! state: a run starts without any and loses them when it ends (`Player::reset_for_run`).
//!
//! The game never checks an ability at a door. A room's geometry is what needs one (a gap too
//! wide to jump, a ledge too high); a key is different, because a locked exit refuses entry
//! (`crate::exit_lock`). `Item::Dash` stays the pickup kind in room files, and picking it up
//! grants `Ability::Dash` (`Item::grants`).

/// Something the player can do once granted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ability {
    Dash,
}

impl Ability {
    /// Every ability, so lists built from it (the editor's, later) grow by themselves.
    pub const ALL: &'static [Self] = &[Self::Dash];

    pub fn name(self) -> &'static str {
        match self {
            Self::Dash => "Dash",
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
    fn every_ability_has_a_name() {
        assert_eq!(Ability::Dash.name(), "Dash");
        assert!(Ability::ALL
            .iter()
            .all(|ability| !ability.name().is_empty()));
    }
}
