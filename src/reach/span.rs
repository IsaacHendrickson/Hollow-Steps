//! What each ability adds to a jump, as lengths that need it: `Interval::gap` and
//! `Interval::height`, and the reasons an ability can be needed by no length at all.
use super::Interval;
use crate::ability::Ability;

/// The lengths that need one ability: longer than `low` (the best a player with no ability and
/// every buff at its maximum manages), not longer than `high` (the best with the ability at base
/// stats). Empty when `low` is not below `high`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Span {
    pub low: f32,
    pub high: f32,
}

impl Span {
    /// True when some length needs the ability.
    pub fn usable(&self) -> bool {
        self.low < self.high
    }

    /// True when a length this long needs the ability and can be managed with it. Never true
    /// for a number that is not a number.
    pub fn contains(&self, length: f32) -> bool {
        length > self.low && length <= self.high
    }
}

/// What an ability is for: a gap that is longer than a jump, a ledge that is higher, or both. A
/// new ability has to say here, so its empty intervals are reported and a dash is not blamed for
/// adding no height.
fn adds(ability: Ability) -> (bool, bool) {
    match ability {
        Ability::Dash => (true, false),
        Ability::DoubleJump => (true, true),
    }
}

/// What the player adjusts to make the ability reach further (for the hint in a message).
fn lever(ability: Ability) -> &'static str {
    match ability {
        Ability::Dash => "dash",
        Ability::DoubleJump => "double jump speed",
    }
}

impl Interval {
    /// The gap lengths, in pixels, that need `ability`.
    pub fn gap(&self, ability: Ability) -> Span {
        let high = match ability {
            Ability::Dash => self.with.dash_jump_distance,
            Ability::DoubleJump => self.with.double_jump_distance,
        };
        Span {
            low: self.without.jump_distance,
            high,
        }
    }

    /// The ledge heights, in pixels above where a jump takes off, that need `ability`. Empty for an
    /// ability that adds no height (a dash only stops the fall for a moment).
    pub fn height(&self, ability: Ability) -> Span {
        let high = match ability {
            Ability::Dash => self.with.jump_height,
            Ability::DoubleJump => self.with.double_jump_height,
        };
        Span {
            low: self.without.jump_height,
            high,
        }
    }

    /// Why `ability` can be needed by no gap, or no ledge, that it is for: one message for each
    /// empty interval, naming both numbers. Empty when every interval it is for is usable.
    pub fn problems(&self, ability: Ability) -> Vec<String> {
        let (gaps, heights) = adds(ability);
        let (name, lever) = (ability.name(), lever(ability));
        let mut found = Vec::new();
        let gap = self.gap(ability);
        if gaps && !gap.usable() {
            found.push(format!(
                "no gap can need {name}: without it the best jump with every buff crosses {:.0} px, \
                 but with {name} a jump crosses only {:.0} px; lower the move speed buffs or raise \
                 the {lever}",
                gap.low, gap.high
            ));
        }
        let height = self.height(ability);
        if heights && !height.usable() {
            found.push(format!(
                "no ledge can need {name}: without it the best jump rises {:.0} px, but with \
                 {name} a jump rises only {:.0} px; raise the {lever}",
                height.low, height.high
            ));
        }
        found
    }

    /// The abilities with no problem (some length each is for needs it), in `Ability::ALL` order.
    /// These are the ones the preview draws an outline for.
    pub fn usable_abilities(&self) -> Vec<Ability> {
        Ability::ALL
            .iter()
            .copied()
            .filter(|ability| self.problems(*ability).is_empty())
            .collect()
    }

    /// How far and how high a jump with `ability` goes, in whole pixels, as the preview and the
    /// reach line say it: `296 px` for an ability that adds only distance, `270 px high / 240 px
    /// far` for one that adds height too.
    pub fn figures(&self, ability: Ability) -> String {
        let far = self.gap(ability).high;
        let high = self.height(ability).high;
        match adds(ability) {
            (_, false) => format!("{far:.0} px"),
            (false, true) => format!("{high:.0} px high"),
            (true, true) => format!("{high:.0} px high / {far:.0} px far"),
        }
    }

    /// Every ability's problems, in `Ability::ALL` order; empty when each ability can be needed.
    pub fn errors(&self) -> Vec<(Ability, String)> {
        Ability::ALL
            .iter()
            .flat_map(|ability| {
                self.problems(*ability)
                    .into_iter()
                    .map(|message| (*ability, message))
            })
            .collect()
    }
}
