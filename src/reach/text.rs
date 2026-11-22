//! The reach line the Player panel and F4 show after a save that changed the numbers behind it.
use super::Interval;
use crate::ability::Ability;

/// How the line says a jump with `ability`: a new ability has to choose its words here.
fn phrase(ability: Ability) -> &'static str {
    match ability {
        Ability::Dash => "dash jump",
        Ability::DoubleJump => "double jump",
    }
}

/// `jump 183 px, dash jump 296 px, double jump 270 px high / 240 px far; run Check dungeon`: the
/// longest gap a player with no ability can cross (every buff at its maximum), what each ability
/// adds to it, and what to do about the gates marked against the old numbers. When some ability
/// can be needed by no gap or ledge, the line says why instead (`Interval::errors`, one message
/// each): there is nothing to mark against numbers like these.
pub fn line(interval: &Interval) -> String {
    let errors: Vec<String> = interval
        .errors()
        .into_iter()
        .map(|(_, message)| message)
        .collect();
    if !errors.is_empty() {
        return errors.join("; ");
    }
    let mut parts = vec![format!("jump {:.0} px", interval.without.jump_distance)];
    parts.extend(
        Ability::ALL
            .iter()
            .map(|ability| format!("{} {}", phrase(*ability), interval.figures(*ability))),
    );
    format!("{}; run Check dungeon", parts.join(", "))
}

#[cfg(test)]
mod tests;
