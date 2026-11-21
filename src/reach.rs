//! The reach envelope: how high and how far the player can jump, with and without Dash.
//!
//! Every number here is measured by stepping the real [`Player`] at the fixed step, never
//! computed from a formula that could drift from the physics. The test world is a flat platform
//! ending at an edge over a deep pit. The player walks off the edge and jumps at each tick of the
//! coyote window, jump held and moving on; the best takeoff is kept. A gap between two platforms
//! at the same height is crossed when the body comes down on the far one, so the longest such gap
//! is where the body's leading edge is when it comes back to platform height.
//!
//! These are physical maxima of one technique with perfect timing, so a room that wants a human
//! margin leaves some. With Dash the technique is: jump at the best tick, dash once at the best
//! tick of the flight. A dash only stops the fall for a moment, so it adds distance, never height.
//! A start of the dash on the ground, or a second dash, is not tried: the number stays reachable.
//!
//! Two scenarios make an [`Interval`]: `without` is the best a player can do with no ability and
//! every buff at its maximum (a conservative upper bound), `with` is base stats and Dash. A gap
//! that needs Dash must be longer than the first and not longer than the second; when the first
//! is not below the second the interval is empty and the numbers (or the buffs) must change.

use macroquad::math::{Rect, Vec2};

use crate::charms::{effective_with, Charm, CharmCatalog, Effect};
use crate::item::Item;
use crate::player::{Player, PlayerConfig, PlayerInput, WorldBounds};
use crate::terrain::Terrain;
use crate::timestep::FIXED_STEP_SECONDS;
use crate::tuning::TuningConfig;
use crate::upgrades::MaxTotals;
use crate::vitals::VitalsConfig;

/// Height of the platforms' top in the test world.
const TOP: f32 = 600.0;
/// Where the takeoff platform ends; the far platform starts `gap` pixels later.
const EDGE: f32 = 0.0;
/// How far below the platforms the pit floor lies, so a fall never reaches it in time.
const PIT_DEPTH: f32 = 4_000.0;
/// Ticks of walking before the body's back edge leaves the platform.
const RUN_UP_TICKS: usize = 40;
/// Longest flight, in ticks (5 s). A jump that stays up longer is not a platformer jump: a config
/// that never comes back down in that time measures nothing (and costs little).
const MAX_TICKS: usize = 600;
/// The dash key is tried in about this many ticks of a flight at most (every tick of a normal one).
const DASH_SAMPLES: usize = 200;
/// A body this far above the platform top has left it.
const RISE_EPSILON: f32 = 0.01;
/// The slack `terrain` allows before a body counts as overlapping a solid.
const TOUCH_EPSILON: f32 = 0.001;

/// What a player with some numbers can do, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reach {
    /// How far a jump with the button held lifts the body above where it left the ground.
    pub jump_height: f32,
    /// The longest gap between two platforms at the same height that a jump crosses.
    pub jump_distance: f32,
    /// The same gap with one dash during the jump (never below `jump_distance`).
    pub dash_jump_distance: f32,
}

impl Reach {
    /// The reach of a player with exactly these numbers, measured on the flat test world.
    pub fn of(config: &PlayerConfig) -> Self {
        Self::in_world(config, &World::new(None, &[]))
    }

    fn in_world(config: &PlayerConfig, world: &World) -> Self {
        Self {
            jump_height: jump_height(world, config),
            jump_distance: distance(config, &landings(world, config, false)),
            dash_jump_distance: distance(config, &landings(world, config, true)),
        }
    }
}

/// The two scenarios and the gap lengths between them that need Dash.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    /// No ability, every buff at its maximum: the best a player gets without Dash.
    pub without: Reach,
    /// Base stats and Dash.
    pub with: Reach,
}

impl Interval {
    /// The scenarios for the player numbers in `base`, with the move speed raised by every charm
    /// in `charms` and by the upgrades' `upgrades` totals (see [`max_move_speed_percent`]).
    pub fn new(
        base: &TuningConfig,
        vitals: &VitalsConfig,
        charms: &CharmCatalog,
        upgrades: &MaxTotals,
    ) -> Self {
        Self::of_charms(base, vitals, charms.charms(), upgrades)
    }

    /// `new` for a plain list of charms (the editor's list, which may not be valid yet).
    pub fn of_charms(
        base: &TuningConfig,
        vitals: &VitalsConfig,
        charms: &[Charm],
        upgrades: &MaxTotals,
    ) -> Self {
        let percent = max_move_speed_percent_of(charms, upgrades);
        // The game's own layering, so the buff cannot drift from what a player really gets.
        let (buffed, _) = effective_with(base, vitals, &[], &[Effect::MoveSpeedPercent(percent)]);
        Self::from_configs(&base.player, &buffed.player)
    }

    /// The scenarios for a base config and a config with the buffs applied.
    pub fn from_configs(base: &PlayerConfig, buffed: &PlayerConfig) -> Self {
        Self {
            without: Reach::of(buffed),
            with: Reach::of(base),
        }
    }

    /// A gap that needs Dash is longer than this.
    pub fn low(&self) -> f32 {
        self.without.jump_distance
    }

    /// A gap that needs Dash is not longer than this.
    pub fn high(&self) -> f32 {
        self.with.dash_jump_distance
    }

    /// True when some gap length needs Dash: the first distance is below the second.
    pub fn usable(&self) -> bool {
        self.low() < self.high()
    }

    /// Why no gap can need Dash, naming both distances; `None` when the interval is usable.
    pub fn error(&self) -> Option<String> {
        (!self.usable()).then(|| {
            format!(
                "no gap can need Dash: without it the best jump with every buff crosses {:.0} px, \
                 but with Dash a jump crosses only {:.0} px; lower the move speed buffs or raise \
                 the dash",
                self.low(),
                self.high()
            )
        })
    }

    /// True when a gap this long needs Dash and can be crossed with it.
    pub fn needs_dash(&self, gap: f32) -> bool {
        gap > self.low() && gap <= self.high()
    }
}

/// The most move speed percent a player can have: every charm in the catalog (as if all were
/// worn at once) plus every upgrade at its last level. Only gains count.
pub fn max_move_speed_percent(charms: &CharmCatalog, upgrades: &MaxTotals) -> f32 {
    max_move_speed_percent_of(charms.charms(), upgrades)
}

/// `max_move_speed_percent` for a plain list of charms.
pub fn max_move_speed_percent_of(charms: &[Charm], upgrades: &MaxTotals) -> f32 {
    let from_charms: f32 = charms
        .iter()
        .filter_map(|charm| match charm.effect {
            Effect::MoveSpeedPercent(percent) if percent.is_finite() => Some(percent.max(0.0)),
            _ => None,
        })
        .sum();
    from_charms + upgrades.move_speed_percent.max(0.0)
}

/// Two flat platforms at the same height. Without a `gap` there is only the takeoff platform, so
/// a body that leaves it falls; `extra` solids are added as they are.
struct World {
    terrain: Terrain,
    bounds: WorldBounds,
}

impl World {
    fn new(gap: Option<f32>, extra: &[Rect]) -> Self {
        let mut solids = vec![Rect::new(-10_000.0, TOP, 10_000.0 + EDGE, PIT_DEPTH)];
        if let Some(gap) = gap {
            solids.push(Rect::new(EDGE + gap, TOP, 10_000.0, PIT_DEPTH));
        }
        solids.extend_from_slice(extra);
        Self {
            terrain: Terrain::new(solids),
            bounds: WorldBounds::new(-20_000.0, 20_000.0, TOP + PIT_DEPTH).with_top(TOP - 20_000.0),
        }
    }
}

/// The keys held for one tick.
#[derive(Clone, Copy, Debug)]
struct Keys {
    axis: f32,
    jump: bool,
    hold: bool,
    dash: bool,
}

impl Keys {
    const IDLE: Self = Self {
        axis: 0.0,
        jump: false,
        hold: false,
        dash: false,
    };
    /// Toward the gap.
    const WALK: Self = Self {
        axis: 1.0,
        ..Self::IDLE
    };
    /// The tick the jump button goes down; it is held from then on.
    const TAKEOFF: Self = Self {
        jump: true,
        hold: true,
        ..Self::WALK
    };
    const FLY: Self = Self {
        hold: true,
        ..Self::WALK
    };
}

/// Where a crossing attempt came back to platform height, and whether it landed there.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Landing {
    /// The body's center when it came back down to platform height.
    x: f32,
    /// True when the body is standing on a solid then (on the far platform of a gap world).
    landed: bool,
}

/// A player partway through a jump attempt: the state a sample branches from.
#[derive(Clone)]
struct Attempt {
    player: Player,
    tick: usize,
    risen: bool,
}

impl Attempt {
    /// A player standing on the takeoff platform at `x`. It holds Dash but only uses it when a
    /// sample presses the key.
    fn new(world: &World, config: &PlayerConfig, x: f32) -> Self {
        let mut player = Player::new(Vec2::ZERO, *config);
        player.collect_item(Item::Dash);
        player.place_at(Vec2::new(x, TOP - config.size.y / 2.0), world.bounds);
        let mut attempt = Self {
            player,
            tick: 0,
            risen: false,
        };
        attempt.advance(world, Keys::IDLE); // settles onto the platform
        attempt
    }

    fn advance(&mut self, world: &World, keys: Keys) {
        let input = PlayerInput {
            move_axis: keys.axis,
            jump_pressed: keys.jump,
            jump_held: keys.hold,
            dash_pressed: keys.dash,
            ..PlayerInput::default()
        };
        self.player
            .update_with_terrain(input, FIXED_STEP_SECONDS, world.bounds, &world.terrain);
        self.player.take_events();
        self.tick += 1;
    }

    /// Call after every tick of a jump: `Some` on the first tick the body is back at platform
    /// height after having been above it.
    fn crossing(&mut self) -> Option<Landing> {
        let bottom = self.player.position().y + self.player.size().y / 2.0;
        if bottom < TOP - RISE_EPSILON {
            self.risen = true;
            return None;
        }
        let grounded = self.player.is_grounded();
        (self.risen && (bottom > TOP + TOUCH_EPSILON || grounded)).then(|| Landing {
            x: self.player.position().x,
            landed: grounded,
        })
    }
}

/// Runs `attempt` on, holding right and jump, until it comes back to platform height.
fn finish(world: &World, attempt: &mut Attempt, first: Keys) -> Option<Landing> {
    let mut keys = first;
    while attempt.tick < MAX_TICKS {
        attempt.advance(world, keys);
        if let Some(landing) = attempt.crossing() {
            return Some(landing);
        }
        keys = Keys::FLY;
    }
    None
}

/// The state just before the tick in which the body's back edge leaves the platform, walking
/// toward the gap at full speed and aligned so that edge leaves exactly at the platform's edge (the
/// most generous start). `None` when the player never leaves (no move speed).
fn walk_off(world: &World, config: &PlayerConfig) -> Option<Attempt> {
    let step = config.move_speed * FIXED_STEP_SECONDS;
    if !step.is_finite() || step <= 0.0 {
        return None;
    }
    // Start at least a body width back, a whole number of ticks from the edge.
    let ticks = ((config.size.x / step).ceil().min(20_000.0) as usize).max(RUN_UP_TICKS);
    let x = (EDGE + config.size.x / 2.0 - ticks as f32 * step).max(-9_000.0);
    let mut attempt = Attempt::new(world, config, x);
    if !attempt.player.is_grounded() {
        return None;
    }
    for _ in 0..ticks + 8 {
        let mut before = attempt.clone();
        attempt.advance(world, Keys::WALK);
        if !attempt.player.is_grounded() {
            // The flight budget counts from here, not from the start of the walk.
            before.tick = 0;
            return Some(before);
        }
    }
    None
}

/// Every attempt worth measuring: the jump button goes down in each tick from the last one on the
/// platform through the coyote window (a tick too late makes no jump and lands nowhere), and with
/// `dash` the dash key also goes down in each tick of the flight (see `DASH_SAMPLES`).
fn landings(world: &World, config: &PlayerConfig, dash: bool) -> Vec<Landing> {
    let Some(mut before) = walk_off(world, config) else {
        return Vec::new();
    };
    let window = (config.coyote_seconds.max(0.0) / FIXED_STEP_SECONDS).ceil() as usize;
    let mut found = Vec::new();
    for _ in 0..=window.min(600) {
        // The flight with no dash first, keeping the state before each of its ticks.
        let mut flight = before.clone();
        let mut keys = Keys::TAKEOFF;
        let mut branches = Vec::new();
        let baseline = loop {
            if flight.tick >= MAX_TICKS {
                break None;
            }
            if dash {
                branches.push((flight.clone(), keys));
            }
            flight.advance(world, keys);
            if let Some(landing) = flight.crossing() {
                break Some(landing);
            }
            keys = Keys::FLY;
        };
        before.advance(world, Keys::WALK);
        let Some(baseline) = baseline else {
            continue;
        };
        found.push(baseline);
        let stride = branches.len().div_ceil(DASH_SAMPLES).max(1);
        for (mut branch, keys) in branches.into_iter().step_by(stride) {
            found.extend(finish(world, &mut branch, Keys { dash: true, ..keys }));
        }
    }
    found
}

/// The longest gap those attempts cross: the body's leading edge where it came back down, less
/// the platform edge. No attempt came back down: nothing is crossed.
fn distance(config: &PlayerConfig, landings: &[Landing]) -> f32 {
    landings
        .iter()
        .map(|landing| landing.x + config.size.x / 2.0 - EDGE)
        .fold(0.0, f32::max)
}

/// How far a standing jump with the button held lifts the body.
fn jump_height(world: &World, config: &PlayerConfig) -> f32 {
    let mut attempt = Attempt::new(world, config, EDGE - 300.0);
    let start = attempt.player.position().y;
    let mut highest = start;
    let mut keys = Keys {
        axis: 0.0,
        ..Keys::TAKEOFF
    };
    while attempt.tick < MAX_TICKS {
        attempt.advance(world, keys);
        highest = highest.min(attempt.player.position().y);
        if attempt.player.is_grounded() {
            break;
        }
        keys = Keys {
            axis: 0.0,
            ..Keys::FLY
        };
    }
    (start - highest).max(0.0)
}

mod editor;
#[cfg(test)]
mod tests;
mod text;

pub use editor::{outline_labels, outlines, Outlines, ReachKey};
pub use text::line;
