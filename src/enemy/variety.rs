//! An enemy's optional-placement data (`crate::variety`): the `group` and `chance` of the
//! placement it came from, and the number that tells it apart from every other enemy of its
//! room. Only the game's world loading acts on them, once, when a run's rooms are built.
use super::Enemy;
use crate::room::EnemyPlacement;

impl Enemy {
    /// The group (1 to 9) the placement is in, if it is in one: one member of each group of a
    /// room stays in a run.
    pub fn group(&self) -> Option<u8> {
        self.group
    }

    /// The chance, in percent, that the placement is in the room in a run (100 = always).
    pub fn chance(&self) -> u8 {
        self.chance
    }

    /// The enemy marked as in `group`, or with `chance` percent of being in the room.
    pub fn with_variety(mut self, group: Option<u8>, chance: u8) -> Self {
        self.group = group;
        self.chance = chance;
        self
    }

    /// The enemy with everything its placement says about when it is in a room: the min tier,
    /// the group and the chance.
    pub fn with_placement(self, placement: &EnemyPlacement) -> Self {
        self.with_min_tier(placement.min_tier)
            .with_variety(placement.group, placement.chance)
    }

    /// What tells this enemy from the others of its room, set when a run's rooms are built
    /// (`variety::mark_identities`); 0 until then.
    pub(crate) fn identity(&self) -> u64 {
        self.identity
    }

    pub(crate) fn set_identity(&mut self, identity: u64) {
        self.identity = identity;
    }
}
