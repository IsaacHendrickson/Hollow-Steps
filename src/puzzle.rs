//! Puzzle pieces of a room: breakable walls, switches and barriers. Here: how a room file stores
//! them (`BreakablePlacement`, `SwitchPlacement`, `BarrierPlacement`, in `RoomLayout`'s
//! `breakables`, `switches` and `barriers`), how the layout checks them, the live pieces of a
//! room being played (`RoomPuzzles`), what a nail swing does to them and which of them are solid.
//! The run-long memory of what was broken or switched on is `game::world_puzzles`; the editor's
//! add, move, edit and delete operations are in `editor_puzzle`; `docs/puzzles.md`.
//!
//! The live room keeps the pieces in their own lists and never merges them into `Room::terrain`,
//! so `RoomLayout::from_room` gives back what the file said whatever happened during play. The
//! player collides with `RoomPuzzles::collision`: the terrain plus the pieces still standing.
use std::borrow::Cow;

use macroquad::math::Rect;
use serde::{Deserialize, Serialize};

use crate::editor::RoomLayout;
use crate::terrain::Terrain;

/// The smallest width and height of any piece, in pixels.
pub const MIN_PIECE_SIZE: f32 = 8.0;
/// A breakable wall takes from 1 to this many nail hits.
pub const MAX_BREAKABLE_HITS: u8 = 9;
/// The lowest and highest switch number a room file can use.
pub const FIRST_SWITCH: u8 = 1;
pub const LAST_SWITCH: u8 = 8;

/// Overlaps shallower than this count as touching (the same as `terrain`'s and `hazard`'s).
const TOUCH_EPSILON: f32 = 0.001;
/// The player's body, which the room's spawn and every exit's arrival must keep clear of pieces.
const PLAYER_BODY: (f32, f32) = (32.0, 48.0);

type Numbers = (f32, f32, f32, f32);

/// One breakable wall in a room file: a solid rectangle that goes away once the nail has hit it
/// `hits` times (one hit per swing).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BreakablePlacement {
    /// `(x, y, width, height)` in room coordinates, like a solid.
    pub rect: Numbers,
    /// How many hits break it, 1 to `MAX_BREAKABLE_HITS`.
    pub hits: u8,
}

/// One switch in a room file: a small rectangle the nail turns on for the rest of the run.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SwitchPlacement {
    pub rect: Numbers,
    /// Its number, `FIRST_SWITCH` to `LAST_SWITCH`, unique in the room.
    pub id: u8,
}

/// One barrier in a room file: a solid rectangle that goes away while its switch is on.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BarrierPlacement {
    pub rect: Numbers,
    /// The number of the switch that opens it. That switch must be in the same room, because
    /// the dungeon checker does not model areas inside a room.
    pub switch: u8,
}

impl BreakablePlacement {
    pub fn rect(&self) -> Rect {
        rect_of(self.rect)
    }
}

impl SwitchPlacement {
    pub fn rect(&self) -> Rect {
        rect_of(self.rect)
    }
}

impl BarrierPlacement {
    pub fn rect(&self) -> Rect {
        rect_of(self.rect)
    }
}

fn rect_of((x, y, w, h): Numbers) -> Rect {
    Rect::new(x, y, w, h)
}

fn numbers_of(rect: Rect) -> Numbers {
    (rect.x, rect.y, rect.w, rect.h)
}

/// The rectangle of a player body centered at `center`.
fn body_at(center: (f32, f32)) -> Rect {
    Rect::new(
        center.0 - PLAYER_BODY.0 / 2.0,
        center.1 - PLAYER_BODY.1 / 2.0,
        PLAYER_BODY.0,
        PLAYER_BODY.1,
    )
}

/// The rectangles share more than an edge.
pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.right() - TOUCH_EPSILON
        && a.right() > b.x + TOUCH_EPSILON
        && a.y < b.bottom() - TOUCH_EPSILON
        && a.bottom() > b.y + TOUCH_EPSILON
}

/// A breakable wall of the room being played: what the file said, and what play has done to it.
#[derive(Clone, Debug, PartialEq)]
pub struct Breakable {
    pub rect: Rect,
    /// The hits the file asks for.
    pub hits: u8,
    /// The hits it still takes; 0 once it is broken.
    pub remaining: u8,
    /// The swing that hit it last: one swing is one hit however many ticks its box overlaps.
    last_swing: Option<u64>,
}

/// A switch of the room being played.
#[derive(Clone, Debug, PartialEq)]
pub struct Switch {
    pub rect: Rect,
    pub id: u8,
    pub on: bool,
}

/// A barrier of the room being played. It is open exactly while its switch is on, so it keeps no
/// state of its own.
#[derive(Clone, Debug, PartialEq)]
pub struct Barrier {
    pub rect: Rect,
    pub switch: u8,
}

/// What one swing changed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Struck {
    /// The rectangles of the walls that broke.
    pub broken: Vec<Rect>,
    /// The rectangle and number of each switch that went from off to on.
    pub switched_on: Vec<(Rect, u8)>,
}

/// The puzzle pieces of the room being played, in file order (a piece's place in its list is its
/// place in the file).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RoomPuzzles {
    breakables: Vec<Breakable>,
    switches: Vec<Switch>,
    barriers: Vec<Barrier>,
}

impl RoomPuzzles {
    /// Every piece as its layout places it: walls whole, switches off.
    pub fn from_layout(layout: &RoomLayout) -> Self {
        Self {
            breakables: layout
                .breakables
                .iter()
                .map(|piece| Breakable {
                    rect: piece.rect(),
                    hits: piece.hits,
                    remaining: piece.hits,
                    last_swing: None,
                })
                .collect(),
            switches: layout
                .switches
                .iter()
                .map(|piece| Switch {
                    rect: piece.rect(),
                    id: piece.id,
                    on: false,
                })
                .collect(),
            barriers: layout
                .barriers
                .iter()
                .map(|piece| Barrier {
                    rect: piece.rect(),
                    switch: piece.switch,
                })
                .collect(),
        }
    }

    pub fn breakables(&self) -> &[Breakable] {
        &self.breakables
    }

    pub fn switches(&self) -> &[Switch] {
        &self.switches
    }

    pub fn barriers(&self) -> &[Barrier] {
        &self.barriers
    }

    /// What the file said about the walls, whatever has been broken.
    pub fn breakable_placements(&self) -> Vec<BreakablePlacement> {
        self.breakables
            .iter()
            .map(|piece| BreakablePlacement {
                rect: numbers_of(piece.rect),
                hits: piece.hits,
            })
            .collect()
    }

    /// What the file said about the switches, whichever are on.
    pub fn switch_placements(&self) -> Vec<SwitchPlacement> {
        self.switches
            .iter()
            .map(|piece| SwitchPlacement {
                rect: numbers_of(piece.rect),
                id: piece.id,
            })
            .collect()
    }

    pub fn barrier_placements(&self) -> Vec<BarrierPlacement> {
        self.barriers
            .iter()
            .map(|piece| BarrierPlacement {
                rect: numbers_of(piece.rect),
                switch: piece.switch,
            })
            .collect()
    }

    /// The barrier has gone away: a switch with its number is on.
    pub fn is_open(&self, barrier: &Barrier) -> bool {
        self.switches
            .iter()
            .any(|switch| switch.id == barrier.switch && switch.on)
    }

    /// The pieces that block the player now: walls not yet broken, barriers whose switch is off.
    pub fn standing_solids(&self) -> impl Iterator<Item = Rect> + '_ {
        let walls = self
            .breakables
            .iter()
            .filter(|wall| wall.remaining > 0)
            .map(|wall| wall.rect);
        let barriers = self
            .barriers
            .iter()
            .filter(|barrier| !self.is_open(barrier))
            .map(|barrier| barrier.rect);
        walls.chain(barriers)
    }

    /// What the player collides with: `terrain` and the pieces still standing. Borrows `terrain`
    /// when nothing stands, so a room without puzzles costs nothing.
    pub fn collision<'a>(&self, terrain: &'a Terrain) -> Cow<'a, Terrain> {
        let mut standing = self.standing_solids().peekable();
        if standing.peek().is_none() {
            return Cow::Borrowed(terrain);
        }
        let solids = terrain.solids().iter().copied().chain(standing).collect();
        Cow::Owned(Terrain::new(solids))
    }

    /// Swing number `swing` is active with `hitbox`. Each wall it overlaps takes one hit, once
    /// per swing however long the box stays there; each switch it overlaps turns on. The nail is
    /// not stopped by solids, so a swing reaches a piece behind a thin wall.
    pub fn strike(&mut self, hitbox: Rect, swing: u64) -> Struck {
        let mut struck = Struck::default();
        for wall in &mut self.breakables {
            if wall.remaining > 0 && wall.last_swing != Some(swing) && overlaps(hitbox, wall.rect) {
                wall.last_swing = Some(swing);
                wall.remaining -= 1;
                if wall.remaining == 0 {
                    struck.broken.push(wall.rect);
                }
            }
        }
        for switch in &mut self.switches {
            if !switch.on && overlaps(hitbox, switch.rect) {
                switch.on = true;
                struck.switched_on.push((switch.rect, switch.id));
            }
        }
        struck
    }

    /// Breaks the wall placed exactly at `rect`. False when there is none, or it is broken.
    pub(crate) fn break_at(&mut self, rect: Rect) -> bool {
        self.breakables
            .iter_mut()
            .find(|wall| wall.rect == rect && wall.remaining > 0)
            .map(|wall| wall.remaining = 0)
            .is_some()
    }

    /// Turns on a switch that is off and placed exactly at `rect`. False when there is none.
    pub(crate) fn switch_on_at(&mut self, rect: Rect) -> bool {
        self.switches
            .iter_mut()
            .find(|switch| switch.rect == rect && !switch.on)
            .map(|switch| switch.on = true)
            .is_some()
    }
}

/// Checks the rectangle of piece `index` of a kind called `label`: finite, at least the minimum
/// size, fully inside `room`.
fn check_rect(label: &str, index: usize, rect: Numbers, room: Rect) -> Result<(), String> {
    let (x, y, w, h) = rect;
    if ![x, y, w, h, x + w, y + h].iter().all(|n| n.is_finite()) {
        return Err(format!("{label} {index} must have finite numbers"));
    }
    if w < MIN_PIECE_SIZE || h < MIN_PIECE_SIZE {
        return Err(format!(
            "{label} {index} must be at least {MIN_PIECE_SIZE} px wide and tall"
        ));
    }
    if x < room.x || y < room.y || x + w > room.right() || y + h > room.bottom() {
        return Err(format!("{label} {index} must be inside the room bounds"));
    }
    Ok(())
}

impl RoomLayout {
    /// Every piece is valid, none overlaps the spawn point (the player would start the room
    /// inside it), the switch numbers are unique, and every barrier names a switch of this room.
    pub(crate) fn validate_puzzles(&self) -> Result<(), String> {
        let room = self.rect();
        let spawn = body_at(self.spawn);
        let clear = |label: &str, index: usize, rect: Numbers| {
            if overlaps(rect_of(rect), spawn) {
                return Err(format!("spawn must not be inside {label} {index}"));
            }
            Ok(())
        };
        for (index, wall) in self.breakables.iter().enumerate() {
            const LABEL: &str = "breakable wall";
            check_rect(LABEL, index, wall.rect, room)?;
            if !(1..=MAX_BREAKABLE_HITS).contains(&wall.hits) {
                return Err(format!(
                    "{LABEL} {index} hits must be between 1 and {MAX_BREAKABLE_HITS}"
                ));
            }
            clear(LABEL, index, wall.rect)?;
        }
        for (index, switch) in self.switches.iter().enumerate() {
            check_rect("switch", index, switch.rect, room)?;
            check_switch_number("switch", index, switch.id)?;
            if let Some(first) = self.switches[..index]
                .iter()
                .position(|other| other.id == switch.id)
            {
                return Err(format!(
                    "switch {index} repeats switch number {} of switch {first}",
                    switch.id
                ));
            }
            clear("switch", index, switch.rect)?;
        }
        for (index, barrier) in self.barriers.iter().enumerate() {
            check_rect("barrier", index, barrier.rect, room)?;
            check_switch_number("barrier", index, barrier.switch)?;
            if !self
                .switches
                .iter()
                .any(|switch| switch.id == barrier.switch)
            {
                return Err(format!(
                    "barrier {index} needs switch {}, which this room does not have",
                    barrier.switch
                ));
            }
            clear("barrier", index, barrier.rect)?;
        }
        Ok(())
    }

    /// A player arriving at `arrival` (a body's center, from an exit of another room) must not
    /// land inside a piece of this room.
    pub(crate) fn validate_arrival(&self, arrival: (f32, f32)) -> Result<(), String> {
        let body = body_at(arrival);
        let lists: [(&str, Vec<Rect>); 3] = [
            (
                "breakable wall",
                self.breakables.iter().map(|p| p.rect()).collect(),
            ),
            ("switch", self.switches.iter().map(|p| p.rect()).collect()),
            ("barrier", self.barriers.iter().map(|p| p.rect()).collect()),
        ];
        for (label, rects) in lists {
            if let Some(index) = rects.iter().position(|rect| overlaps(*rect, body)) {
                return Err(format!(
                    "exit arrival must not be inside {label} {index} of the destination room"
                ));
            }
        }
        Ok(())
    }

    /// Takes every puzzle piece out of the layout (a new room starts without its template's).
    pub(crate) fn clear_puzzles(&mut self) {
        self.breakables.clear();
        self.switches.clear();
        self.barriers.clear();
    }
}

/// `Ok` for a switch number from `FIRST_SWITCH` to `LAST_SWITCH`; `label` and `index` name the
/// piece that uses it.
fn check_switch_number(label: &str, index: usize, id: u8) -> Result<(), String> {
    if (FIRST_SWITCH..=LAST_SWITCH).contains(&id) {
        Ok(())
    } else {
        Err(format!(
            "{label} {index}: switch {id} does not exist: switches are {FIRST_SWITCH} to {LAST_SWITCH}"
        ))
    }
}

#[cfg(test)]
mod tests;
