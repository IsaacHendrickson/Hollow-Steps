//! Pure authored-room editing. Only the explicitly named load/save functions do I/O.
use macroquad::math::{Rect, Vec2};
use serde::{Deserialize, Serialize};

use crate::item::{Item, WorldItem, ITEM_SIZE};
use crate::player::WorldBounds;
use crate::room::{Exit, Room, RoomEdge, RoomId};
use crate::room_charms::CharmPlacement;
use crate::terrain::Terrain;

pub use crate::editor_draft::RoomDraft;
pub use crate::editor_selection::Selection;
pub use crate::editor_session::EditorSession;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ItemPlacement {
    pub item: Item,
    pub position: (f32, f32),
}

/// An exit in a room file. Serialized by hand in `door_exit`: an edge exit holds `edge`,
/// `offset` and `length`, a door holds `door` instead.
#[derive(Clone, Debug, PartialEq)]
pub struct ExitPlacement {
    pub edge: RoomEdge,
    pub offset: f32,
    pub length: f32,
    pub target_room: RoomId,
    pub spawn: (f32, f32),
    /// A door's `(x, y, width, height)` in room coordinates; the edge fields mean nothing then.
    pub door: Option<(f32, f32, f32, f32)>,
    /// The key that locks this exit, `Key 1` to `Key 8`; absent in older room files.
    pub locked_by: Option<u8>,
}

impl ExitPlacement {
    pub fn exit(&self) -> Exit {
        Exit {
            edge: self.edge,
            offset: self.offset,
            length: self.length,
            target_room: self.target_room,
            spawn: self.spawn.into(),
            door: self.door.map(|(x, y, w, h)| Rect::new(x, y, w, h)),
            locked_by: self.locked_by,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RoomLayout {
    /// The room's name; absent in older files and unnamed rooms.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub bounds: (f32, f32, f32, f32),
    pub floor_y: f32,
    pub spawn: (f32, f32),
    pub items: Vec<ItemPlacement>,
    /// Charms lying in the room (charm id and position); absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub charms: Vec<CharmPlacement>,
    pub exits: Vec<ExitPlacement>,
    /// Solid `(x, y, width, height)` rectangles; absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub terrain: Vec<(f32, f32, f32, f32)>,
    /// Hazards that hurt and knock back the player; absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hazards: Vec<crate::hazard::HazardPlacement>,
    /// Puzzle pieces (`crate::puzzle`); each list is absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub breakables: Vec<crate::puzzle::BreakablePlacement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub switches: Vec<crate::puzzle::SwitchPlacement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub barriers: Vec<crate::puzzle::BarrierPlacement>,
    /// Parallax background layers, back to front; absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub background: Vec<crate::background::BackgroundLayer>,
    /// The terrain's look: a preset plus the room's overrides; absent in older room files.
    #[serde(
        default,
        skip_serializing_if = "crate::terrain_styles::RoomStyle::is_default"
    )]
    pub terrain_style: crate::terrain_styles::RoomStyle,
    /// What crossing from an entrance to an exit needs (`crate::gates`); the game never reads
    /// it. Absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub gates: Vec<crate::gates::Gate>,
    /// Where the dungeon map puts this room, `(column, row)` from -64 to 64 (`crate::dungeon_map`);
    /// absent means placed by its exits, and in older room files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_cell: Option<(i32, i32)>,
    /// Marks the room as a piece of a dungeon a generator can build (`crate::piece`); the game
    /// never reads it. Absent means the room is not one, and in older room files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piece: Option<crate::piece::PieceInfo>,
    /// Spots where the generator may put a key or an ability; kept when the mark is removed.
    /// Absent in older room files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub slots: Vec<crate::piece::PickupSlot>,
}

impl RoomLayout {
    pub fn from_room(room: &Room) -> Self {
        Self {
            name: room.name.clone(),
            bounds: (room.bounds.x, room.bounds.y, room.bounds.w, room.bounds.h),
            floor_y: room.floor_y,
            spawn: room.spawn.into(),
            terrain: room
                .terrain
                .solids()
                .iter()
                .map(|r| (r.x, r.y, r.w, r.h))
                .collect(),
            hazards: room.hazards.iter().map(Into::into).collect(),
            breakables: room.puzzles.breakable_placements(),
            switches: room.puzzles.switch_placements(),
            barriers: room.puzzles.barrier_placements(),
            background: room.background.clone(),
            terrain_style: room.terrain_style.clone(),
            gates: room.gates.clone(),
            map_cell: room.map_cell,
            piece: room.piece,
            slots: room.slots.clone(),
            items: room
                .items
                .iter()
                .map(|item| ItemPlacement {
                    item: item.item,
                    position: item.position.into(),
                })
                .collect(),
            charms: Self::charm_placements(room),
            exits: room
                .exits
                .iter()
                .map(|exit| ExitPlacement {
                    edge: exit.edge,
                    offset: exit.offset,
                    length: exit.length,
                    target_room: exit.target_room,
                    spawn: exit.spawn.into(),
                    door: exit.door.map(|zone| (zone.x, zone.y, zone.w, zone.h)),
                    locked_by: exit.locked_by,
                })
                .collect(),
        }
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.bounds.0, self.bounds.1, self.bounds.2, self.bounds.3)
    }

    pub fn world_bounds(&self) -> WorldBounds {
        WorldBounds::new(self.bounds.0, self.bounds.0 + self.bounds.2, self.floor_y)
            .with_top(self.bounds.1)
    }

    pub(crate) fn contains_body(&self, center: (f32, f32), size: Vec2) -> bool {
        let p = Vec2::from(center);
        let r = self.rect();
        p.is_finite()
            && p.x - size.x / 2.0 >= r.x
            && p.x + size.x / 2.0 <= r.right()
            && p.y - size.y / 2.0 >= r.y
            && p.y + size.y / 2.0 <= self.floor_y
    }

    pub fn validate_geometry(&self) -> Result<(), String> {
        let (x, y, w, h) = self.bounds;
        if ![x, y, w, h, x + w, y + h, self.floor_y]
            .iter()
            .all(|n| n.is_finite())
            || w < 32.0
            || h < 48.0
            || self.floor_y > y + h
            || self.floor_y < y + 48.0
        {
            return Err("invalid room bounds or floor".into());
        }
        Terrain::validate(&self.terrain, self.rect())?;
        self.validate_hazards()?;
        self.validate_puzzles()?;
        crate::background::validate(&self.background)?;
        self.terrain_style.validate()?;
        let terrain = Terrain::from_tuples(&self.terrain);
        if !self.contains_body(self.spawn, Vec2::new(32.0, 48.0)) {
            return Err("spawn must keep the player's body inside the room above the floor".into());
        }
        if terrain.overlaps_body(self.spawn.into(), Vec2::new(32.0, 48.0)) {
            return Err("spawn must not be inside terrain".into());
        }
        for item in &self.items {
            item.item.check()?;
            if !self.contains_body(item.position, ITEM_SIZE) {
                return Err("item must be inside the room above the floor".into());
            }
            if terrain.overlaps_body(item.position.into(), ITEM_SIZE) {
                return Err("item must not be inside terrain".into());
            }
        }
        self.validate_charm_positions(&terrain)?;
        for exit in &self.exits {
            crate::exit_lock::check_lock(exit.locked_by)?;
            if exit.is_door() {
                exit.check_door(self, &terrain)?;
                continue;
            }
            let span = match exit.edge {
                RoomEdge::Left | RoomEdge::Right => h,
                _ => w,
            };
            if !exit.offset.is_finite()
                || !exit.length.is_finite()
                || exit.offset < 0.0
                || exit.length <= 0.0
                || exit.offset + exit.length > span
                || !Vec2::from(exit.spawn).is_finite()
            {
                return Err("exit span or destination spawn is invalid".into());
            }
            if terrain.overlaps_rect(&exit.exit().zone(self.rect())) {
                return Err("exit must not be inside terrain".into());
            }
        }
        if let Some(cell) = self.map_cell {
            crate::dungeon_map::check_cell(cell)?;
        }
        self.check_piece(&terrain)
            .map_err(|error| error.to_string())?;
        self.validate_gates()
    }

    pub fn apply(&self, room: &mut Room) -> Result<(), String> {
        self.validate_geometry()?;
        room.name = self.name.clone();
        room.bounds = self.rect();
        room.floor_y = self.floor_y;
        room.terrain = Terrain::from_tuples(&self.terrain);
        room.hazards = self.hazards.iter().map(Into::into).collect();
        room.puzzles = crate::puzzle::RoomPuzzles::from_layout(self);
        room.background = self.background.clone();
        room.terrain_style = self.terrain_style.clone();
        room.spawn = self.spawn.into();
        room.items = self
            .items
            .iter()
            .map(|p| WorldItem {
                item: p.item,
                position: p.position.into(),
            })
            .collect();
        room.charms = self.world_charms();
        room.exits = self.exits.iter().map(ExitPlacement::exit).collect();
        room.gates = self.gates.clone();
        room.map_cell = self.map_cell;
        room.piece = self.piece;
        room.slots = self.slots.clone();
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Placement {
    Enemy(String),
    Boss(String),
    Item(Item),
    /// A charm lying in the room, by its id in `charms.ron`.
    Charm(String),
}

pub use crate::editor_files::{load_extra_rooms, room_filename, save_room};
