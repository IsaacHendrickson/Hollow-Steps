//! What the run's screens say and where it goes, with no window: the result screen's text and
//! layout, and the coin captions of the HUD and the inventory. `hud_run` draws them, `main.rs`
//! only translates keys and clicks.
//!
//! Everything is in virtual-screen coordinates and placed with `camera::anchored`.

use macroquad::math::{vec2, Rect, Vec2};

use super::{RunOutcome, RunPhase, RunResult};
use crate::camera::{anchored, Anchor};
use crate::profile::Profile;

/// What the result screen tells the player to do.
pub const RETURN_HINT: &str = "Press Enter or click to return to the hub";

/// The result screen's heading.
pub fn result_title(outcome: RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::Cleared => "Dungeon cleared",
        RunOutcome::Died => "You died",
        RunOutcome::Abandoned => "Run abandoned",
    }
}

/// The lines under the heading: what the run earned and banked, the total banked and, after a
/// clear, the difficulty tier going up. `profile` is the profile after the run was banked.
pub fn result_lines(result: &RunResult, profile: &Profile) -> Vec<String> {
    let mut lines = vec![
        format!("Coins earned: {}", result.coins_earned),
        format!("Coins banked: {}", result.coins_banked),
        format!("Total banked: {}", profile.coins),
    ];
    if result.outcome == RunOutcome::Cleared {
        lines.push(format!(
            "Difficulty tier {} -> {}",
            result.tier_before, result.tier_after
        ));
    }
    lines
}

const PANEL_WIDTH: f32 = 560.0;
const SIDE_PADDING: f32 = 40.0;
// Distances below the panel's top edge; text positions are baselines at the left text edge.
const TITLE_BASELINE: f32 = 70.0;
const FIRST_LINE_BASELINE: f32 = 128.0;
const LINE_STEP: f32 = 34.0;
/// From the last line to the message line, then from it to the hint.
const MESSAGE_GAP: f32 = 46.0;
const HINT_GAP: f32 = 38.0;
const BOTTOM_PADDING: f32 = 30.0;

/// Where everything on the result screen goes.
#[derive(Clone, Debug, PartialEq)]
pub struct ResultLayout {
    pub panel: Rect,
    /// The left edge of the text, and how wide a line may be.
    pub text_x: f32,
    pub text_width: f32,
    /// The heading's baseline (drawn centered in the panel).
    pub title: Vec2,
    pub lines: Vec<Vec2>,
    /// Where a message about the run (a profile that did not save) starts.
    pub message: Vec2,
    pub hint: Vec2,
}

/// The result screen for `lines` lines of text on a `view`-sized screen, centered.
pub fn result_layout(view: Vec2, lines: usize) -> ResultLayout {
    let last = FIRST_LINE_BASELINE + lines.saturating_sub(1) as f32 * LINE_STEP;
    let message = last + MESSAGE_GAP;
    let hint = message + HINT_GAP;
    let size = vec2(PANEL_WIDTH, hint + BOTTOM_PADDING);
    let top_left = anchored(view, Anchor::Center, size, Vec2::ZERO);
    let panel = Rect::new(top_left.x, top_left.y, size.x, size.y);
    let text_x = panel.x + SIDE_PADDING;
    let at = |down: f32| vec2(text_x, panel.y + down);
    ResultLayout {
        panel,
        text_x,
        text_width: PANEL_WIDTH - 2.0 * SIDE_PADDING,
        title: at(TITLE_BASELINE),
        lines: (0..lines)
            .map(|line| at(FIRST_LINE_BASELINE + line as f32 * LINE_STEP))
            .collect(),
        message: at(message),
        hint: at(hint),
    }
}

/// The coin counter beside the health and mana bars: the number, and a small caption under it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Counter {
    pub coins: u32,
    pub caption: Option<String>,
}

/// What the HUD's coin counter shows. With no hub it is the coins collected, as it always was.
/// In the hub it is the banked coins with the tier, in a run the coins of this run.
pub fn hud_counter(phase: Option<RunPhase>, run_coins: u32, profile: &Profile) -> Counter {
    match phase {
        None => Counter {
            coins: run_coins,
            caption: None,
        },
        Some(RunPhase::Hub) => Counter {
            coins: profile.coins,
            caption: Some(format!("banked  |  tier {}", profile.tier)),
        },
        Some(RunPhase::Running | RunPhase::Result(_)) => Counter {
            coins: run_coins,
            caption: Some("this run".to_owned()),
        },
    }
}

/// The inventory's coin line and, when a hub is set, the banked coins and tier beside it.
pub fn inventory_coins(
    phase: Option<RunPhase>,
    run_coins: u32,
    profile: &Profile,
) -> (String, Option<String>) {
    match phase {
        None => (format!("Coins: {run_coins}"), None),
        Some(_) => (
            format!("Coins this run: {run_coins}"),
            Some(format!(
                "Banked: {}  |  Tier {}",
                profile.coins, profile.tier
            )),
        ),
    }
}

#[cfg(test)]
mod tests;
