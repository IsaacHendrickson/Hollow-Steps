//! Optional enemy placements: the first thing a run's seed decides (`docs/variety.md`).
//!
//! An enemy placement can have a `group` (1 to 9) and a `chance` (0 to 100 percent, 100 when the
//! file says nothing). When the game builds rooms for a run, every **ungrouped** enemy is in
//! the room with probability `chance`, and from each **group** of a room exactly one placement
//! stays. Both draws come from `seed::stream(seed, "enemies", key)`, so the same seed always
//! gives the same enemies, and anything that rebuilds the world in the middle of a run (F5, an
//! editor Save) puts back the same ones. The min tier is applied first; then this.
//!
//! Only the game's world loading calls `thin` (`game::tier_link::Layer`), with the seed of a run.
//! The editor, `load_rooms`, `load_extra_rooms`, Save, the hub and a test room never do, so
//! outside a run nothing is left out and Save keeps every placement in the file.
use std::collections::{BTreeSet, HashMap};

use serde::de::Error as _;
use serde::{Deserialize, Deserializer};

use crate::enemy::Enemy;
use crate::room::{EnemyPlacement, RoomId};
use crate::seed::{fnv1a, stream};

/// The highest group number; groups are 1 to this.
pub const MAX_GROUP: u8 = 9;
/// A placement that is always there: the chance when a file says nothing.
pub const FULL_CHANCE: u8 = 100;
/// What the draws are for, the label of every stream in this module.
const LABEL: &str = "enemies";

pub(crate) fn full_chance() -> u8 {
    FULL_CHANCE
}

pub(crate) fn is_full_chance(chance: &u8) -> bool {
    *chance == FULL_CHANCE
}

/// A group number is 1 to `MAX_GROUP`.
pub fn check_group(group: u8) -> Result<(), String> {
    if !(1..=MAX_GROUP).contains(&group) {
        return Err(format!(
            "group {group} is not a group number, use 1 to {MAX_GROUP}"
        ));
    }
    Ok(())
}

/// A chance is 0 to `FULL_CHANCE` percent.
pub fn check_chance(chance: u8) -> Result<(), String> {
    if chance > FULL_CHANCE {
        return Err(format!(
            "chance {chance} is above {FULL_CHANCE}, the most a percent can be"
        ));
    }
    Ok(())
}

/// Reads a placement's `group`, refusing a number outside 1 to `MAX_GROUP`.
pub(crate) fn read_group<'de, D: Deserializer<'de>>(reader: D) -> Result<Option<u8>, D::Error> {
    let group = Option::<u8>::deserialize(reader)?;
    if let Some(group) = group {
        check_group(group).map_err(D::Error::custom)?;
    }
    Ok(group)
}

/// Reads a placement's `chance`, refusing one above `FULL_CHANCE`.
pub(crate) fn read_chance<'de, D: Deserializer<'de>>(reader: D) -> Result<u8, D::Error> {
    let chance = u8::deserialize(reader)?;
    check_chance(chance).map_err(D::Error::custom)?;
    Ok(chance)
}

impl EnemyPlacement {
    /// The rules of a placement's `group` and `chance`, with the error naming the placement
    /// (`index` is its place in the room's list): each in range, and a grouped placement has
    /// chance 100, because a group keeps exactly one of its members whatever the dice say.
    pub fn check_variety(&self, index: usize) -> Result<(), String> {
        let name = format!("enemy {index} ({:?})", self.definition);
        if let Some(group) = self.group {
            check_group(group).map_err(|error| format!("{name}: {error}"))?;
        }
        check_chance(self.chance).map_err(|error| format!("{name}: {error}"))?;
        if let Some(group) = self.group.filter(|_| self.chance != FULL_CHANCE) {
            return Err(format!(
                "{name} is in group {group} with chance {}: a grouped placement must have \
                 chance {FULL_CHANCE}, because a group keeps exactly one of its members",
                self.chance
            ));
        }
        Ok(())
    }
}

/// Numbers every enemy of a freshly built room by what it is: its definition's name, its patrol
/// and how many placements before it are the same, so two identical enemies still differ. It is
/// done over every placement, before the tier leaves any out, so the picks a seed makes do not
/// move with the tier. The same placements give the same numbers wherever the room is rebuilt.
pub(crate) fn mark_identities(enemies: &mut [Enemy]) {
    let mut seen: HashMap<(String, u32, u32), u32> = HashMap::new();
    for enemy in enemies {
        let (name, left, right) = enemy.placement_key();
        let count = seen
            .entry((name.to_owned(), left.to_bits(), right.to_bits()))
            .or_insert(0);
        let text = format!(
            "{name}:{:08x}:{:08x}:{count}",
            left.to_bits(),
            right.to_bits()
        );
        *count += 1;
        enemy.set_identity(fnv1a(text.as_bytes()));
    }
}

/// The key of the draw that picks which member of `group` stays in `room`.
fn group_key(room: RoomId, group: u8) -> u64 {
    fnv1a(format!("group:{}:{group}", room.index()).as_bytes())
}

/// The key of the draw that decides whether the enemy with `identity` is in `room`.
fn chance_key(room: RoomId, identity: u64) -> u64 {
    fnv1a(format!("chance:{}:{identity:016x}", room.index()).as_bytes())
}

/// Leaves out of `enemies` (the placements of `room` that the tier left in, in file order, after
/// `mark_identities`) what `seed` decides: one member stays of each group, and an ungrouped enemy
/// stays when its roll is under its chance. Chance 0 never stays and chance 100 always does; the
/// rest keep their order. A grouped enemy ignores its chance.
pub(crate) fn thin(enemies: &mut Vec<Enemy>, room: RoomId, seed: u64) {
    let mut stays = vec![true; enemies.len()];
    let groups: BTreeSet<u8> = enemies.iter().filter_map(Enemy::group).collect();
    for group in groups {
        let members: Vec<usize> = (0..enemies.len())
            .filter(|index| enemies[*index].group() == Some(group))
            .collect();
        let pick = stream(seed, LABEL, group_key(room, group)).next_u64() % members.len() as u64;
        for (place, index) in members.into_iter().enumerate() {
            stays[index] = place as u64 == pick;
        }
    }
    for (index, enemy) in enemies.iter().enumerate() {
        if enemy.group().is_none() && enemy.chance() < FULL_CHANCE {
            let roll = stream(seed, LABEL, chance_key(room, enemy.identity())).next_u64() % 100;
            stays[index] = roll < u64::from(enemy.chance());
        }
    }
    let mut stays = stays.into_iter();
    enemies.retain(|_| stays.next().unwrap_or(true));
}

pub mod flip;
pub use flip::{decide_flip, FlipChoice, VarietyRules};

#[cfg(test)]
mod tests;
