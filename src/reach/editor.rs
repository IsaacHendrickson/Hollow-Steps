//! The reach preview's inputs from the editor session, and the two outlines it draws.
use macroquad::math::{Rect, Vec2};

use super::{max_move_speed_percent_of, Interval};
use crate::editor::Selection;
use crate::editor_session::EditorSession;
use crate::player::PlayerConfig;
use crate::vitals::VitalsConfig;

/// Everything an `Interval` is measured from. Measuring takes a moment, so a window keeps the
/// last interval and measures again only when this has changed.
#[derive(Clone, Debug, PartialEq)]
pub struct ReachKey {
    player: PlayerConfig,
    vitals: VitalsConfig,
    /// The most move speed percent the charms and upgrades give (`max_move_speed_percent_of`).
    percent: f32,
}

impl EditorSession {
    /// What the interval of `reach_interval` depends on: the Player panel's numbers as edited,
    /// and the charms and upgrades as the panels hold them.
    pub fn reach_key(&self) -> ReachKey {
        ReachKey {
            player: self.player.edited().player,
            vitals: *self.player.edited_vitals(),
            percent: max_move_speed_percent_of(self.charms.edited(), &self.upgrades.totals()),
        }
    }

    /// The interval for the numbers as the editor holds them now (unsaved edits included),
    /// measured on the real player.
    pub fn reach_interval(&self) -> Interval {
        Interval::of_charms(
            self.player.edited(),
            self.player.edited_vitals(),
            self.charms.edited(),
            &self.upgrades.totals(),
        )
    }

    /// Where the Reach preview starts: the selected entrance, the room spawn or the arrival of
    /// another room's exit, and the room spawn when something else (or nothing) is selected.
    pub fn reach_origin(&self, selected: Option<Selection>) -> Vec2 {
        let arrival = match selected {
            Some(Selection::Inbound { room, exit }) => self
                .inbound_arrivals()
                .into_iter()
                .find(|inbound| inbound.room == room && inbound.exit == exit)
                .map(|inbound| inbound.position),
            _ => None,
        };
        arrival.unwrap_or_else(|| self.draft.layout.spawn.into())
    }
}

/// The two outlines of the preview, in world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outlines {
    /// What a jump covers without Dash.
    pub without: Rect,
    /// What a jump with Dash covers.
    pub with: Rect,
}

/// The boxes a jump from `origin` (the center of a body of size `body` standing there, at the
/// edge of its platform) sweeps: out to the longest gap on either side, up to the top of the
/// jump. A doorway that lies between the two needs Dash to reach.
pub fn outlines(origin: Vec2, body: Vec2, interval: &Interval) -> Outlines {
    let around = |distance: f32, height: f32| {
        Rect::new(
            origin.x - distance,
            origin.y - body.y / 2.0 - height,
            distance * 2.0,
            height + body.y,
        )
    };
    Outlines {
        without: around(interval.low(), interval.without.jump_height),
        with: around(interval.high(), interval.with.jump_height),
    }
}

/// What each outline says on the canvas: `without Dash 273 px` and `with Dash 485 px`.
pub fn outline_labels(interval: &Interval) -> [String; 2] {
    [
        format!("without Dash {:.0} px", interval.low()),
        format!("with Dash {:.0} px", interval.high()),
    ]
}

#[cfg(test)]
mod tests;
