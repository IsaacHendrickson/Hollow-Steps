//! A scratch world with a pool of dungeon pieces for the generated-run tests: the loop's test
//! world (`run_tests::world`: a hub whose exit leads to the first room) with fourteen piece files
//! added and a `variety.ron` that turns the generator on. Everything is written into a private
//! copy of `tests/fixtures`, never into `assets/`.
//!
//! The pieces are a small version of the generator's own test pool: two Start pieces (spawn at
//! x 200, so a mirrored one differs), two Boss pieces with a boss, corridors, forks, a stairwell,
//! an elbow and three dead ends with a Key slot. Every piece exit leads to the hub in its own
//! file, which is what makes the authored world load; the generator rewires them.
use super::super::charm_tests::Scratch;
use super::super::run_tests::world;
use crate::room::RoomId;

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn exit(self) -> &'static str {
        match self {
            Self::Left => "(edge: Left, offset: 364.0, length: 96.0, target_room: Custom(0), spawn: (480.0, 436.0))",
            Self::Right => "(edge: Right, offset: 364.0, length: 96.0, target_room: Custom(0), spawn: (480.0, 436.0))",
            Self::Top => "(edge: Top, offset: 400.0, length: 160.0, target_room: Custom(0), spawn: (480.0, 436.0))",
            Self::Bottom => "(edge: Bottom, offset: 400.0, length: 160.0, target_room: Custom(0), spawn: (480.0, 436.0))",
        }
    }
}

use Side::{Bottom, Left, Right, Top};

/// One piece file: its number (`room_<id>.ron`), what it is and where its exits are.
struct Spec {
    id: usize,
    name: &'static str,
    role: &'static str,
    weight: u8,
    sides: &'static [Side],
    /// A Key slot on the floor.
    key: bool,
    /// A boss placement (a Boss piece), or one enemy (any other).
    boss: bool,
    enemy: bool,
    spawn_x: f32,
}

const fn spec(id: usize, name: &'static str, role: &'static str, sides: &'static [Side]) -> Spec {
    Spec {
        id,
        name,
        role,
        weight: 2,
        sides,
        key: false,
        boss: false,
        enemy: false,
        spawn_x: 480.0,
    }
}

const POOL: [Spec; 14] = [
    Spec {
        spawn_x: 200.0,
        enemy: true,
        weight: 3,
        ..spec(1, "Gate Hall", "Start", &[Right, Top])
    },
    Spec {
        spawn_x: 200.0,
        ..spec(2, "Cistern Landing", "Start", &[Left, Right, Bottom])
    },
    Spec {
        boss: true,
        weight: 3,
        ..spec(3, "Warden's Pit", "Boss", &[Left])
    },
    Spec {
        boss: true,
        ..spec(4, "Hollow Throne", "Boss", &[Left, Top])
    },
    spec(5, "Long Corridor", "Middle", &[Left, Right]),
    Spec {
        enemy: true,
        ..spec(6, "Dusty Corridor", "Middle", &[Left, Right])
    },
    spec(7, "Stairwell", "Middle", &[Top, Bottom]),
    spec(8, "Elbow", "Middle", &[Right, Bottom]),
    Spec {
        weight: 3,
        ..spec(9, "Crossroads", "Middle", &[Left, Right, Top])
    },
    Spec {
        weight: 3,
        ..spec(10, "Sump", "Middle", &[Left, Right, Bottom])
    },
    Spec {
        weight: 3,
        ..spec(11, "Shaft Junction", "Middle", &[Left, Top, Bottom])
    },
    Spec {
        key: true,
        ..spec(12, "Keeper's Nook", "Middle", &[Left])
    },
    Spec {
        key: true,
        ..spec(13, "Locksmith", "Middle", &[Top])
    },
    Spec {
        key: true,
        ..spec(14, "Ossuary", "Middle", &[Right])
    },
];

/// The ids of the pieces: `Custom(1)` to `Custom(14)`.
pub(super) fn piece_ids() -> Vec<RoomId> {
    POOL.iter().map(|piece| RoomId::Custom(piece.id)).collect()
}

/// The ids of the Start pieces and of the Boss pieces.
pub(super) fn start_pieces() -> [RoomId; 2] {
    [RoomId::Custom(1), RoomId::Custom(2)]
}

pub(super) fn boss_pieces() -> [RoomId; 2] {
    [RoomId::Custom(3), RoomId::Custom(4)]
}

impl Spec {
    /// The room file: placements and a layout with the piece mark.
    fn file(&self) -> String {
        let exits: Vec<&str> = self.sides.iter().map(|side| side.exit()).collect();
        // A bottom exit needs a gap in the floor.
        let floor = if self.sides.iter().any(|side| matches!(side, Bottom)) {
            "(0.0, 460.0, 400.0, 80.0), (560.0, 460.0, 400.0, 80.0)"
        } else {
            "(0.0, 460.0, 960.0, 80.0)"
        };
        let slots = if self.key {
            "(position: (180.0, 440.0), kind: Key)"
        } else {
            ""
        };
        let enemy = if self.enemy {
            r#"(definition: "chase", patrol_left: 560.0, patrol_right: 800.0)"#
        } else {
            ""
        };
        let boss = if self.boss {
            r#"Some((definition: "Ash Warden", position: (560.0, 436.0)))"#
        } else {
            "None"
        };
        format!(
            "(enemies: [{enemy}], boss: {boss}, layout: Some((name: Some({:?}), \
             bounds: (0.0, 0.0, 960.0, 540.0), floor_y: 460.0, spawn: ({:?}, 436.0), items: [], \
             exits: [{}], terrain: [{floor}], piece: Some((role: {}, weight: {})), \
             slots: [{slots}])))",
            self.name,
            self.spawn_x,
            exits.join(", "),
            self.role,
            self.weight
        )
    }
}

/// The generator's rules for the test world: seven to ten rooms with a spine of four to six, and
/// `keys` locked links (`(min, max)`).
fn variety_file(keys: (u8, u8)) -> String {
    format!(
        "(flip_percent: 0.0, room_flip_percent: 0.0, generator: (enabled: true, \
         rooms: (min: 7, max: 10), spine: (min: 4, max: 6), keys: (min: {}, max: {}), \
         mirror_percent: 50.0, attempts: 200))",
        keys.0, keys.1
    )
}

/// A scratch world with the hub, the authored rooms, the fourteen pieces and a generator that is
/// on, asking for between `keys.0` and `keys.1` keys.
pub(super) fn generated_world(keys: (u8, u8)) -> Scratch {
    let scratch = world(100.0);
    let rooms = scratch.0.join("rooms");
    for piece in &POOL {
        std::fs::write(rooms.join(format!("room_{}.ron", piece.id)), piece.file()).unwrap();
    }
    scratch.write("variety.ron", &variety_file(keys));
    scratch
}
