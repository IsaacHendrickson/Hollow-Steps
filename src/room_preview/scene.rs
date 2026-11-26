//! The room as the preview sees it: plain data (a layout, plus what only a loaded room knows:
//! coins, where the enemies stand and the boss), and the list of things it is drawn and tabled
//! from. Nothing here reads a file.
use macroquad::math::{Rect, Vec2};

use crate::coin::COIN_RADIUS;
use crate::editor::{ExitPlacement, RoomLayout};
use crate::editor_files::room_filename;
use crate::exit_arrival::PLAYER_SIZE;
use crate::gates::{Entrance, ExitRef};
use crate::item::{Item, ITEM_SIZE};
use crate::room::{EnemyPlacement, Room, RoomId};
use crate::room_charms::CHARM_SIZE;

/// An enemy as the game spawned it, with the placement's own numbers.
#[derive(Clone, Debug, PartialEq)]
pub struct EnemyView {
    pub name: String,
    /// The center of the body.
    pub center: Vec2,
    pub size: Vec2,
    /// The patrol endpoints (body centers, x); `None` when they are not known.
    pub patrol: Option<(f32, f32)>,
    /// `min_tier`, `group` and `chance` of the placement (0, none and 100 are the plain case).
    pub min_tier: u32,
    pub group: Option<u8>,
    pub chance: u8,
}

/// The room's boss, where it stands.
#[derive(Clone, Debug, PartialEq)]
pub struct BossView {
    pub name: String,
    pub center: Vec2,
    pub size: Vec2,
}

/// Everything the preview draws.
#[derive(Clone, Debug)]
pub struct Scene {
    pub id: Option<RoomId>,
    pub layout: RoomLayout,
    /// Centers of the coins lying in the room (only the three built-in rooms have any).
    pub coins: Vec<Vec2>,
    pub enemies: Vec<EnemyView>,
    pub boss: Option<BossView>,
}

/// How a thing covers the cells of the picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Footprint {
    /// Every cell the rectangle touches (hazards, exits, puzzle pieces: nothing hides).
    Touch,
    /// The cells it covers at least half of, or the cell of its center (bodies, pickups).
    Majority,
    /// Terrain: drawn by coverage, so a thin platform keeps a character of its own.
    Terrain,
}

/// One placed thing: a picture mark and a table row.
#[derive(Clone, Debug)]
pub(super) struct Thing {
    pub mark: char,
    pub kind: &'static str,
    pub rect: Rect,
    pub detail: String,
    pub footprint: Footprint,
    /// An enemy's patrol endpoints, drawn as a line.
    pub patrol: Option<(f32, f32)>,
}

impl Thing {
    fn new(
        mark: char,
        kind: &'static str,
        rect: Rect,
        detail: String,
        footprint: Footprint,
    ) -> Self {
        Self {
            mark,
            kind,
            rect,
            detail,
            footprint,
            patrol: None,
        }
    }
}

/// The room's name as its file is named (`first`, `room_3`).
pub fn room_label(id: RoomId) -> String {
    room_filename(id).trim_end_matches(".ron").to_owned()
}

/// The mark of exit number `index`: `1` to `9`, then `a` to `z`, then `?`.
pub(super) fn exit_mark(index: usize) -> char {
    match index {
        0..=8 => char::from(b'1' + index as u8),
        9..=34 => char::from(b'a' + (index - 9) as u8),
        _ => '?',
    }
}

fn centered(center: Vec2, size: Vec2) -> Rect {
    Rect::new(
        center.x - size.x / 2.0,
        center.y - size.y / 2.0,
        size.x,
        size.y,
    )
}

fn tuple_rect(r: (f32, f32, f32, f32)) -> Rect {
    Rect::new(r.0, r.1, r.2, r.3)
}

fn item_mark(item: Item) -> char {
    match item {
        Item::AmberShard => '$',
        Item::Dash => 'D',
        Item::DoubleJump => 'J',
        Item::Key(_) => 'K',
    }
}

impl Scene {
    /// A scene of a layout alone: no coins, no enemies, no boss (a room built in code).
    pub fn new(layout: RoomLayout) -> Self {
        Self {
            id: None,
            layout,
            coins: Vec::new(),
            enemies: Vec::new(),
            boss: None,
        }
    }

    /// The scene of a room the game loaded. `placements` are the enemy placements of its file, in
    /// order (the game spawns them in that order); they give the patrol endpoints and the
    /// placement's tier, group and chance. When their number differs from the room's enemies the
    /// patrol is not known and no line is drawn.
    pub fn from_room(room: &Room, placements: &[EnemyPlacement]) -> Self {
        let paired = placements.len() == room.enemies.len();
        let enemies = room
            .enemies
            .iter()
            .enumerate()
            .map(|(index, enemy)| {
                let placement = paired.then(|| &placements[index]);
                EnemyView {
                    name: enemy.name().to_owned(),
                    center: enemy.position(),
                    size: enemy.size(),
                    patrol: placement.map(|p| (p.patrol_left, p.patrol_right)),
                    min_tier: placement.map_or(0, |p| p.min_tier),
                    group: placement.and_then(|p| p.group),
                    chance: placement.map_or(100, |p| p.chance),
                }
            })
            .collect();
        Self {
            id: Some(room.id),
            layout: RoomLayout::from_room(room),
            coins: room.coins.iter().map(|coin| coin.position).collect(),
            enemies,
            boss: room.boss_special.as_ref().map(|boss| BossView {
                name: boss.name().to_owned(),
                center: boss.position(),
                size: boss.size(),
            }),
        }
    }

    /// The solid rectangles the picture draws as terrain: the floor body (from the floor line to
    /// the bottom of the room, when it has any thickness) and every terrain rectangle.
    pub(super) fn solids(&self) -> Vec<Rect> {
        let bounds = self.layout.rect();
        let mut solids = Vec::new();
        if bounds.bottom() - self.layout.floor_y > 0.0 {
            solids.push(Rect::new(
                bounds.x,
                self.layout.floor_y,
                bounds.w,
                bounds.bottom() - self.layout.floor_y,
            ));
        }
        solids.extend(self.layout.terrain.iter().copied().map(tuple_rect));
        solids
    }

    /// Every placed thing, in the order of the table: spawn, exits, terrain, hazards, puzzle
    /// pieces, items, charms, coins, pickup slots, enemies, boss.
    pub(super) fn things(&self) -> Vec<Thing> {
        let layout = &self.layout;
        let mut things = vec![Thing::new(
            '@',
            "spawn",
            centered(layout.spawn.into(), PLAYER_SIZE),
            "player start".into(),
            Footprint::Majority,
        )];
        for (index, placement) in layout.exits.iter().enumerate() {
            let zone = placement.exit().zone(layout.rect());
            things.push(Thing::new(
                exit_mark(index),
                "exit",
                zone,
                self.exit_detail(placement),
                Footprint::Touch,
            ));
        }
        let bounds = layout.rect();
        let floor = bounds.bottom() - layout.floor_y;
        if floor > 0.0 {
            let rect = Rect::new(bounds.x, layout.floor_y, bounds.w, floor);
            things.push(Thing::new(
                '#',
                "floor",
                rect,
                String::new(),
                Footprint::Terrain,
            ));
        }
        for solid in &layout.terrain {
            let rect = tuple_rect(*solid);
            things.push(Thing::new(
                '#',
                "terrain",
                rect,
                String::new(),
                Footprint::Terrain,
            ));
        }
        for hazard in &layout.hazards {
            let detail = format!(
                "{:?}, facing {:?}, damage {}",
                hazard.style, hazard.facing, hazard.damage
            );
            let rect = tuple_rect(hazard.rect);
            things.push(Thing::new('^', "hazard", rect, detail, Footprint::Touch));
        }
        for piece in &layout.breakables {
            let detail = format!("breaks after {} hits", piece.hits);
            let rect = tuple_rect(piece.rect);
            things.push(Thing::new('%', "breakable", rect, detail, Footprint::Touch));
        }
        for piece in &layout.switches {
            let rect = tuple_rect(piece.rect);
            let detail = format!("switch {}", piece.id);
            things.push(Thing::new('/', "switch", rect, detail, Footprint::Touch));
        }
        for piece in &layout.barriers {
            let rect = tuple_rect(piece.rect);
            let detail = format!("opens with switch {}", piece.switch);
            things.push(Thing::new('~', "barrier", rect, detail, Footprint::Touch));
        }
        for item in &layout.items {
            let rect = centered(item.position.into(), ITEM_SIZE);
            let detail = item.item.name().to_owned();
            let mark = item_mark(item.item);
            things.push(Thing::new(mark, "item", rect, detail, Footprint::Majority));
        }
        for charm in &layout.charms {
            let rect = centered(charm.position.into(), CHARM_SIZE);
            let detail = format!("charm {:?}", charm.charm);
            things.push(Thing::new('C', "charm", rect, detail, Footprint::Majority));
        }
        for coin in &self.coins {
            let rect = centered(*coin, Vec2::splat(COIN_RADIUS * 2.0));
            things.push(Thing::new(
                '*',
                "coin",
                rect,
                String::new(),
                Footprint::Majority,
            ));
        }
        for slot in &layout.slots {
            let rect = centered(slot.position.into(), ITEM_SIZE);
            let detail = format!("pickup slot for {}", slot.kind.name());
            things.push(Thing::new('S', "slot", rect, detail, Footprint::Majority));
        }
        for enemy in &self.enemies {
            things.push(Self::enemy_thing(enemy));
        }
        if let Some(boss) = &self.boss {
            let rect = centered(boss.center, boss.size);
            let detail = format!("boss {}", boss.name);
            things.push(Thing::new('B', "boss", rect, detail, Footprint::Majority));
        }
        things
    }

    fn enemy_thing(enemy: &EnemyView) -> Thing {
        let mut detail = enemy.name.clone();
        match enemy.patrol {
            Some((left, right)) => detail.push_str(&format!(
                ", patrol x {left:.0} to {right:.0} ({:.0} px)",
                (right - left).abs()
            )),
            None => detail.push_str(", patrol unknown"),
        }
        if enemy.min_tier > 0 {
            detail.push_str(&format!(", from tier {}", enemy.min_tier));
        }
        if let Some(group) = enemy.group {
            detail.push_str(&format!(", group {group}"));
        }
        if enemy.chance != 100 {
            detail.push_str(&format!(", chance {}%", enemy.chance));
        }
        let mut thing = Thing::new(
            'E',
            "enemy",
            centered(enemy.center, enemy.size),
            detail,
            Footprint::Majority,
        );
        thing.patrol = enemy.patrol.filter(|(left, right)| left != right);
        thing
    }

    /// What an exit says in the table: where it is, where it goes, its key and its gates.
    fn exit_detail(&self, placement: &ExitPlacement) -> String {
        let place = match placement.door {
            Some(_) => "door".to_owned(),
            None => format!(
                "{} edge, offset {:.0}, length {:.0}",
                format!("{:?}", placement.edge).to_lowercase(),
                placement.offset,
                placement.length
            ),
        };
        let mut text = format!(
            "{place} -> {} arrival ({:.0}, {:.0})",
            room_label(placement.target_room),
            placement.spawn.0,
            placement.spawn.1
        );
        if let Some(key) = placement.locked_by {
            text.push_str(&format!("; locked by Key {key}"));
        }
        let this = ExitRef::of(placement);
        for gate in self.layout.gates.iter().filter(|gate| gate.exit == this) {
            let needs: Vec<&str> = gate.needs.iter().map(|ability| ability.name()).collect();
            let from = match &gate.from {
                Entrance::Spawn => "the spawn".to_owned(),
                Entrance::Arrival { room, exit } => {
                    format!("{}'s {}", room_label(*room), exit.describe())
                }
            };
            text.push_str(&format!("; gate: needs {} from {from}", needs.join(" + ")));
        }
        text
    }
}
