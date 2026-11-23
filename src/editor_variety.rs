//! The room editor's edits of an enemy placement's `group` and `chance` (`crate::room::
//! EnemyPlacement`, `crate::variety`): which group of a room the enemy is in, and how likely it
//! is to be in the room in a run. The draft always holds every placement, so Save writes them all
//! and the Test room shows them all; only the game's world loading, for a run with a seed, leaves
//! some out.
//!
//! A grouped placement always has chance 100 (a group keeps exactly one of its members whatever
//! the dice say), so the draft never holds a grouped placement with another chance: each setter
//! refuses, with the draft unchanged, a change that would make one. `RoomDraft::validate_local`
//! also checks it (`validate_layout_and_variety`), for a draft read from a file.
use crate::editor::RoomDraft;
use crate::variety::{check_chance, check_group, FULL_CHANCE};

impl RoomDraft {
    /// The group of enemy placement `index`: `None` when there is no such placement, `Some(None)`
    /// when it is in no group.
    pub fn enemy_group(&self, index: usize) -> Option<Option<u8>> {
        self.enemies.get(index).map(|enemy| enemy.group)
    }

    /// The chance, in percent, of enemy placement `index`; `None` when there is none.
    pub fn enemy_chance(&self, index: usize) -> Option<u8> {
        self.enemies.get(index).map(|enemy| enemy.chance)
    }

    /// Puts enemy placement `index` in `group` (1 to 9), or in none. Refused, with the draft
    /// unchanged, for a group out of range, a missing placement, and a placement whose chance is
    /// not 100.
    pub fn set_enemy_group(&mut self, index: usize, group: Option<u8>) -> Result<(), String> {
        if let Some(group) = group {
            check_group(group)?;
        }
        let enemy = self.enemies.get_mut(index).ok_or("missing enemy")?;
        if group.is_some() && enemy.chance != FULL_CHANCE {
            return Err(format!(
                "this enemy has chance {}%: set Chance to {FULL_CHANCE} before putting it in a \
                 group (a group keeps exactly one of its members)",
                enemy.chance
            ));
        }
        enemy.group = group;
        Ok(())
    }

    /// Sets the chance, 0 to 100 percent, of enemy placement `index`. Refused, with the draft
    /// unchanged, above 100, for a missing placement, and for a placement in a group.
    pub fn set_enemy_chance(&mut self, index: usize, chance: u8) -> Result<(), String> {
        check_chance(chance)?;
        let enemy = self.enemies.get_mut(index).ok_or("missing enemy")?;
        if let Some(group) = enemy.group.filter(|_| chance != FULL_CHANCE) {
            return Err(format!(
                "this enemy is in group {group}, and a grouped enemy always has chance \
                 {FULL_CHANCE}: take it out of the group first"
            ));
        }
        enemy.chance = chance;
        Ok(())
    }

    /// Marks the room as one a run's seed may play mirrored, or clears the mark. The draft and
    /// Save always keep the room as authored; only a run mirrors it, and never the hub.
    pub fn set_flippable(&mut self, flippable: bool) -> Result<(), String> {
        self.layout.flippable = flippable;
        Ok(())
    }

    /// Every placement's group and chance obey their rules; the error names the placement.
    pub(crate) fn validate_variety(&self) -> Result<(), String> {
        self.enemies
            .iter()
            .enumerate()
            .try_for_each(|(index, enemy)| enemy.check_variety(index))
    }

    /// The room's own rules that need no definitions: the layout's geometry first, then the
    /// variety of its enemies. `validate_local` starts with this (one call, so `editor.rs`, which
    /// is over the line limit, does not grow).
    pub(crate) fn validate_layout_and_variety(&self) -> Result<(), String> {
        self.layout.validate_geometry()?;
        self.validate_variety()
    }
}

#[cfg(test)]
mod tests;
