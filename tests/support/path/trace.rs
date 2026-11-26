//! The text a failed proof prints: what was tried, and a compact trace of the replay.
use hollow_steps::editor::RoomLayout;
use macroquad::math::{Rect, Vec2};

use super::replay::{Arrival, Replay, Sample, Target, Verdict};
use super::script::PathScript;

/// How many ticks apart the trace's rows are.
pub const TRACE_EVERY: usize = 10;

/// The terrain edge nearest to the body: which solid, and how far apart they are on each axis
/// (0 on an axis where they overlap or touch).
fn nearest_edge(replay: &Replay, position: Vec2) -> String {
    let body = Rect::new(
        position.x - replay.body.x / 2.0,
        position.y - replay.body.y / 2.0,
        replay.body.x,
        replay.body.y,
    );
    let gap = |solid: &Rect| {
        let dx = (solid.x - body.right())
            .max(body.x - solid.right())
            .max(0.0);
        let dy = (solid.y - body.bottom())
            .max(body.y - solid.bottom())
            .max(0.0);
        (dx, dy)
    };
    let named = replay
        .solids
        .iter()
        .enumerate()
        .map(|(index, solid)| (format!("terrain #{index}"), gap(solid)))
        .chain(std::iter::once(("floor".to_owned(), gap(&replay.floor))));
    named
        .min_by(|a, b| a.1 .0.hypot(a.1 .1).total_cmp(&b.1 .0.hypot(b.1 .1)))
        .map_or_else(
            || "none".to_owned(),
            |(name, (dx, dy))| format!("{name} dx {dx:.0} dy {dy:.0}"),
        )
}

fn row(replay: &Replay, sample: &Sample) -> String {
    let hazard = sample
        .hazard
        .map_or("-".to_owned(), |index| format!("hazard #{index}"));
    format!(
        "{:>5} {:>8.1} {:>8.1} {:>8.1} {:>8.1}  {:<8} {:<28} {hazard}",
        sample.tick,
        sample.position.x,
        sample.position.y,
        sample.velocity.x,
        sample.velocity.y,
        if sample.grounded { "grounded" } else { "air" },
        nearest_edge(replay, sample.position),
    )
}

/// What a failed proof says: the headline, what was tried, a row every tenth tick and the last.
pub fn report(
    headline: &str,
    room: &RoomLayout,
    arrival: Arrival,
    script: &PathScript,
    target: Target,
    replay: &Replay,
    verdict: Verdict,
) -> String {
    let held: Vec<String> = script
        .holds
        .abilities
        .iter()
        .map(|ability| ability.name().to_owned())
        .chain(script.holds.keys.iter().map(|key| format!("Key {key}")))
        .collect();
    let mut lines = vec![
        headline.to_owned(),
        format!(
            "from {}, holding {}, to {}; the script is {} ticks",
            arrival.describe(),
            if held.is_empty() { "nothing".to_owned() } else { held.join(", ") },
            target.describe(room),
            replay.planned
        ),
        match verdict {
            Verdict::Reached { tick } => format!("reached it at tick {tick}"),
            Verdict::Died { tick, hazard } => format!(
                "hit hazard #{hazard} at tick {tick}, so the replay stopped ({} of {} ticks played)",
                replay.last().tick,
                replay.planned
            ),
            Verdict::Missed => {
                let last = replay.last();
                format!(
                    "ended at ({:.1}, {:.1}) after {} ticks, {}",
                    last.position.x,
                    last.position.y,
                    last.tick,
                    if last.grounded { "on the ground" } else { "in the air" }
                )
            }
        },
        format!(
            "{:>5} {:>8} {:>8} {:>8} {:>8}  {:<8} {:<28} hazard",
            "tick", "x", "y", "vx", "vy", "state", "nearest terrain edge"
        ),
    ];
    let last = replay.last();
    lines.extend(
        replay
            .every(TRACE_EVERY)
            .filter(|sample| sample.tick != last.tick)
            .map(|sample| row(replay, sample)),
    );
    lines.push(row(replay, last));
    lines.join("\n")
}
