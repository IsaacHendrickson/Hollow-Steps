//! Replaying a script on the real `Player`, on a room's own terrain, hazards and exits.
//!
//! The room is a `RoomLayout`: its terrain (with the barriers and walls of its puzzles, as they
//! stand before play), its floor and walls (`world_bounds`), its hazards and its exits. A hazard
//! the body touches ends the replay (the player is hit). A locked exit only counts when the script
//! holds its key.
use std::borrow::Cow;

use hollow_steps::ability::Ability;
use hollow_steps::editor::{ExitPlacement, RoomLayout};
use hollow_steps::hazard::Hazard;
use hollow_steps::item::Item;
use hollow_steps::player::Player;
use hollow_steps::puzzle::RoomPuzzles;
use hollow_steps::terrain::Terrain;
use hollow_steps::timestep::FIXED_STEP_SECONDS;
use macroquad::math::{Rect, Vec2};

use super::script::{Loadout, PathScript, Tick};

/// Where the player stands when the replay begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Arrival {
    /// The room's spawn point.
    Spawn,
    /// A player center: the arrival point of an exit of another room (`Arrival::of`).
    At(Vec2),
}

impl Arrival {
    /// The arrival point `exit` (of another room) gives in the room it leads to.
    pub fn of(exit: &ExitPlacement) -> Self {
        Self::At(exit.spawn.into())
    }

    fn point(self, room: &RoomLayout) -> Vec2 {
        match self {
            Self::Spawn => room.spawn.into(),
            Self::At(point) => point,
        }
    }

    pub fn describe(self) -> String {
        match self {
            Self::Spawn => "the spawn".to_owned(),
            Self::At(point) => format!("({:.0}, {:.0})", point.x, point.y),
        }
    }
}

/// What the script is meant to reach.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    /// The trigger area of exit number `index` of the room: reached on the first tick the body
    /// touches it (a door also needs the body's center within the door's width; pressing Up is
    /// not modelled).
    Exit(usize),
    /// A rectangle in room coordinates: reached when the body's center is inside it at the end of
    /// the script (the body has come to rest there, or is still there).
    Area(Rect),
}

impl Target {
    pub fn area(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::Area(Rect::new(x, y, w, h))
    }

    pub fn describe(self, room: &RoomLayout) -> String {
        match self {
            Self::Exit(index) => match room.exits.get(index) {
                Some(exit) => {
                    let zone = exit.exit().zone(room.rect());
                    format!("exit {index} (trigger area {})", rect_text(zone))
                }
                None => format!("exit {index} (the room has {} exits)", room.exits.len()),
            },
            Self::Area(rect) => format!("area {}", rect_text(rect)),
        }
    }
}

pub fn rect_text(rect: Rect) -> String {
    format!(
        "x {:.0}..{:.0}, y {:.0}..{:.0}",
        rect.x,
        rect.right(),
        rect.y,
        rect.bottom()
    )
}

/// The player after one tick.
#[derive(Clone, Debug, PartialEq)]
pub struct Sample {
    /// 0 is the arrival, settled; tick 1 is the first of the script.
    pub tick: usize,
    pub position: Vec2,
    pub velocity: Vec2,
    pub grounded: bool,
    /// The hazard (its index in the room) the body touches, if any.
    pub hazard: Option<usize>,
}

/// How a replay ended, judged against a target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Verdict {
    Reached { tick: usize },
    Died { tick: usize, hazard: usize },
    Missed,
}

/// Every tick of a script, up to the end or the first hazard touched.
#[derive(Clone, Debug)]
pub struct Replay {
    pub samples: Vec<Sample>,
    /// How many ticks the script has (more than the samples hold when a hazard ended it).
    pub planned: usize,
    pub body: Vec2,
    /// The solids the player collided with, then the floor, for the trace.
    pub solids: Vec<Rect>,
    pub floor: Rect,
}

impl Replay {
    pub fn last(&self) -> &Sample {
        self.samples.last().expect("a replay holds the arrival")
    }

    /// The samples of every `n`th tick (the arrival included).
    pub fn every(&self, n: usize) -> impl Iterator<Item = &Sample> {
        self.samples
            .iter()
            .filter(move |sample| sample.tick % n.max(1) == 0)
    }

    fn body_at(&self, position: Vec2) -> Rect {
        Rect::new(
            position.x - self.body.x / 2.0,
            position.y - self.body.y / 2.0,
            self.body.x,
            self.body.y,
        )
    }

    pub fn verdict(&self, room: &RoomLayout, target: Target, holds: &Loadout) -> Verdict {
        let died = self
            .samples
            .iter()
            .find_map(|s| s.hazard.map(|h| (s.tick, h)));
        match target {
            Target::Exit(index) => {
                let Some(exit) = room.exits.get(index) else {
                    return Verdict::Missed;
                };
                let reached = self.samples.iter().find(|sample| {
                    exit_touched(
                        room,
                        exit,
                        self.body_at(sample.position),
                        sample.position,
                        holds,
                    )
                });
                match (reached, died) {
                    (Some(sample), Some((tick, _))) if sample.tick < tick => {
                        Verdict::Reached { tick: sample.tick }
                    }
                    (Some(sample), None) => Verdict::Reached { tick: sample.tick },
                    (_, Some((tick, hazard))) => Verdict::Died { tick, hazard },
                    (None, None) => Verdict::Missed,
                }
            }
            Target::Area(rect) => match died {
                Some((tick, hazard)) => Verdict::Died { tick, hazard },
                None if rect.contains(self.last().position) => Verdict::Reached {
                    tick: self.last().tick,
                },
                None => Verdict::Missed,
            },
        }
    }
}

/// The game's rule for walking into an exit (`Room::exit_at`, `Room::door_at`), with the lock.
fn exit_touched(
    room: &RoomLayout,
    exit: &ExitPlacement,
    body: Rect,
    center: Vec2,
    holds: &Loadout,
) -> bool {
    if exit.locked_by.is_some_and(|key| !holds.keys.contains(&key)) {
        return false;
    }
    let zone = exit.exit().zone(room.rect());
    zone.overlaps(&body) && (exit.door.is_none() || (zone.x..=zone.right()).contains(&center.x))
}

fn item_of(ability: Ability) -> Item {
    match ability {
        Ability::Dash => Item::Dash,
        Ability::DoubleJump => Item::DoubleJump,
    }
}

/// Plays `script` from `arrival` and records every tick.
pub fn replay(room: &RoomLayout, arrival: Arrival, script: &PathScript) -> Replay {
    let puzzles = RoomPuzzles::from_layout(room);
    let terrain = Terrain::from_tuples(&room.terrain);
    let solid: Cow<Terrain> = puzzles.collision(&terrain);
    let hazards: Vec<Hazard> = room.hazards.iter().map(Hazard::from).collect();
    let bounds = room.world_bounds();

    let mut player = Player::new(Vec2::ZERO, script.config);
    for ability in &script.holds.abilities {
        player.collect_item(item_of(*ability));
    }
    player.place_at(arrival.point(room), bounds);
    let size = player.size();
    let touched = |player: &Player| {
        hazards
            .iter()
            .position(|h| h.touches(player.position(), size))
    };
    let sample = |player: &Player, tick: usize| Sample {
        tick,
        position: player.position(),
        velocity: player.velocity(),
        grounded: player.is_grounded(),
        hazard: touched(player),
    };
    let step = |player: &mut Player, tick: Tick| {
        player.update_with_terrain(tick.input(), FIXED_STEP_SECONDS, bounds, &solid);
        player.take_events();
    };

    // Settle onto whatever lies under the arrival point, as the game does on arriving.
    step(
        &mut player,
        Tick {
            axis: 0.0,
            jump_pressed: false,
            jump_held: false,
            dash_pressed: false,
        },
    );
    let mut samples = vec![sample(&player, 0)];
    let ticks = script.ticks();
    for (index, tick) in ticks.iter().enumerate() {
        if samples.last().is_some_and(|last| last.hazard.is_some()) {
            break;
        }
        step(&mut player, *tick);
        samples.push(sample(&player, index + 1));
    }
    Replay {
        samples,
        planned: ticks.len(),
        body: size,
        solids: solid.solids().to_vec(),
        floor: Rect::new(
            bounds.left,
            bounds.floor_y,
            bounds.right - bounds.left,
            1.0e6,
        ),
    }
}
