//! Pieces built in code for the dungeon generator's tests: a small builder for a room with a
//! floor, exits on chosen edges, optional pickup slots and an optional Dash gate, and the fixture
//! pool made from it. Nothing here reads a file.
use hollow_steps::editor::RoomLayout;
use hollow_steps::piece::{PieceRole, SlotKind};
use hollow_steps::room::RoomId;
use hollow_steps::run::RunConfig;
use hollow_steps::variety::generator::CountRange;
use hollow_steps::variety::GeneratorRules;

/// Where an exit is: an edge, or a door.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
    Door,
}

impl Side {
    /// The exit as a room file writes it, aimed at `target`.
    fn exit(self, target: usize) -> String {
        let place = match self {
            Self::Left => "edge: Left, offset: 364.0, length: 96.0".to_owned(),
            Self::Right => "edge: Right, offset: 364.0, length: 96.0".to_owned(),
            Self::Top => "edge: Top, offset: 400.0, length: 160.0".to_owned(),
            Self::Bottom => "edge: Bottom, offset: 400.0, length: 160.0".to_owned(),
            Self::Door => "door: (700.0, 388.0, 48.0, 72.0)".to_owned(),
        };
        format!("({place}, target_room: Custom({target}), spawn: (480.0, 436.0))")
    }

    /// The same exit as a gate names it.
    fn reference(self) -> &'static str {
        match self {
            Self::Left => "Edge(edge: Left, offset: 364.0)",
            Self::Right => "Edge(edge: Right, offset: 364.0)",
            Self::Top => "Edge(edge: Top, offset: 400.0)",
            Self::Bottom => "Edge(edge: Bottom, offset: 400.0)",
            Self::Door => "Door(x: 700.0, y: 388.0, width: 48.0, height: 72.0)",
        }
    }
}

/// A room being described: build it with `build`.
#[derive(Clone)]
pub struct PieceBuilder {
    id: RoomId,
    tag: usize,
    name: String,
    mark: Option<(PieceRole, u8)>,
    exits: Vec<Side>,
    slots: Vec<SlotKind>,
    gate: Option<(Side, Side)>,
}

/// A room with this id. `tag` makes its exits' placeholder targets differ from every other
/// room's, so no two rooms look joined to the same neighbour.
pub fn room(id: RoomId, tag: usize, name: &str) -> PieceBuilder {
    PieceBuilder {
        id,
        tag,
        name: name.to_owned(),
        mark: None,
        exits: Vec::new(),
        slots: Vec::new(),
        gate: None,
    }
}

impl PieceBuilder {
    /// Marks the room as a piece of this role and weight.
    pub fn piece(mut self, role: PieceRole, weight: u8) -> Self {
        self.mark = Some((role, weight));
        self
    }

    /// One exit on each side listed (a side only once).
    pub fn exits(mut self, sides: &[Side]) -> Self {
        self.exits = sides.to_vec();
        self
    }

    /// A pickup slot of this kind, on the floor.
    pub fn slot(mut self, kind: SlotKind) -> Self {
        self.slots.push(kind);
        self
    }

    /// Crossing from the arrival at `from` to the exit at `to` needs Dash.
    pub fn dash_gate(mut self, from: Side, to: Side) -> Self {
        self.gate = Some((from, to));
        self
    }

    fn target(&self, side: Side) -> usize {
        let place = self.exits.iter().position(|known| *known == side);
        1000 + self.tag * 10 + place.expect("the gate names an exit the room has")
    }

    pub fn build(self) -> (RoomId, RoomLayout) {
        let exits: Vec<String> = self
            .exits
            .iter()
            .enumerate()
            .map(|(place, side)| side.exit(1000 + self.tag * 10 + place))
            .collect();
        // A bottom exit needs a gap in the floor.
        let floor = if self.exits.contains(&Side::Bottom) {
            "(0.0, 460.0, 400.0, 80.0), (560.0, 460.0, 400.0, 80.0)"
        } else {
            "(0.0, 460.0, 960.0, 80.0)"
        };
        let slots: Vec<String> = self
            .slots
            .iter()
            .enumerate()
            .map(|(place, kind)| {
                format!(
                    "(position: ({}.0, 440.0), kind: {})",
                    180 + 80 * place,
                    kind.name()
                )
            })
            .collect();
        let gates = match self.gate {
            Some((from, to)) => format!(
                "(from: Arrival(room: Custom({}), exit: {}), exit: {}, needs: [Dash])",
                self.target(from),
                Side::Right.reference(),
                to.reference()
            ),
            None => String::new(),
        };
        let piece = match self.mark {
            Some((role, weight)) => format!("Some((role: {}, weight: {weight}))", role.name()),
            None => "None".to_owned(),
        };
        let text = format!(
            "(name: Some({:?}), bounds: (0.0, 0.0, 960.0, 540.0), floor_y: 460.0, \
             spawn: (480.0, 436.0), items: [], exits: [{}], terrain: [{floor}], gates: [{gates}], \
             piece: {piece}, slots: [{}])",
            self.name,
            exits.join(", "),
            slots.join(", ")
        );
        let layout = ron::from_str(&text).unwrap_or_else(|error| panic!("{text}: {error}"));
        (self.id, layout)
    }
}

// ---- the fixture pool ----

pub const HUB: RoomId = RoomId::Custom(0);
pub const START_A: RoomId = RoomId::First;
pub const START_B: RoomId = RoomId::Custom(1);
pub const BOSS_A: RoomId = RoomId::Boss;
pub const BOSS_B: RoomId = RoomId::Custom(2);
/// The four corridors, the four forks, the three dead ends with a Key slot, the two with an
/// Ability slot and the two Dash crossings: ids `Custom(3)` to `Custom(17)`.
pub const CORRIDORS: [RoomId; 4] = middle_ids(3);
pub const FORKS: [RoomId; 4] = middle_ids(7);
pub const KEY_ENDS: [RoomId; 3] = [RoomId::Custom(11), RoomId::Custom(12), RoomId::Custom(13)];
pub const ABILITY_ENDS: [RoomId; 2] = [RoomId::Custom(14), RoomId::Custom(15)];
pub const DASH_CROSSINGS: [RoomId; 2] = [RoomId::Custom(16), RoomId::Custom(17)];

const fn middle_ids(first: usize) -> [RoomId; 4] {
    [
        RoomId::Custom(first),
        RoomId::Custom(first + 1),
        RoomId::Custom(first + 2),
        RoomId::Custom(first + 3),
    ]
}

/// A number that differs for every id, for the placeholder targets of a room's exits.
fn number(id: RoomId) -> usize {
    match id {
        RoomId::First => 0,
        RoomId::Second => 1,
        RoomId::Boss => 2,
        RoomId::Custom(number) => 3 + number,
    }
}

fn middle(id: RoomId, name: &str, weight: u8, sides: &[Side]) -> PieceBuilder {
    room(id, number(id) + 10, name)
        .piece(PieceRole::Middle, weight)
        .exits(sides)
}

/// The fixture pool: 2 Start pieces, 2 Boss pieces and 15 Middle pieces (4 two-port corridors,
/// 4 three-port forks, 3 dead ends with a Key slot, 2 dead ends with an Ability slot and 2
/// Dash-gated crossings with the gate on the exit).
pub fn pool() -> Vec<(RoomId, RoomLayout)> {
    use Side::{Bottom, Door, Left, Right, Top};
    let start = |id, name, weight, sides: &[Side]| {
        room(id, number(id) + 10, name)
            .piece(PieceRole::Start, weight)
            .exits(sides)
            .build()
    };
    let boss = |id, name, weight, sides: &[Side]| {
        room(id, number(id) + 10, name)
            .piece(PieceRole::Boss, weight)
            .exits(sides)
            .build()
    };
    vec![
        start(START_A, "Gate Hall", 3, &[Right, Top]),
        start(START_B, "Cistern Landing", 2, &[Left, Right, Bottom]),
        boss(BOSS_A, "Warden's Pit", 3, &[Left]),
        boss(BOSS_B, "Hollow Throne", 2, &[Left, Top]),
        middle(CORRIDORS[0], "Long Corridor", 2, &[Left, Right]).build(),
        middle(CORRIDORS[1], "Dusty Corridor", 2, &[Left, Right]).build(),
        middle(CORRIDORS[2], "Stairwell", 2, &[Top, Bottom]).build(),
        middle(CORRIDORS[3], "Elbow", 2, &[Right, Bottom]).build(),
        middle(FORKS[0], "Crossroads", 3, &[Left, Right, Top]).build(),
        middle(FORKS[1], "Sump", 3, &[Left, Right, Bottom]).build(),
        middle(FORKS[2], "Shaft Junction", 3, &[Left, Top, Bottom]).build(),
        middle(FORKS[3], "Door Hub", 3, &[Left, Right, Door]).build(),
        middle(KEY_ENDS[0], "Keeper's Nook", 2, &[Left])
            .slot(SlotKind::Key)
            .build(),
        middle(KEY_ENDS[1], "Locksmith", 2, &[Top])
            .slot(SlotKind::Key)
            .build(),
        middle(KEY_ENDS[2], "Ossuary", 2, &[Right])
            .slot(SlotKind::Key)
            .build(),
        middle(ABILITY_ENDS[0], "Shrine", 2, &[Bottom])
            .slot(SlotKind::Ability)
            .build(),
        middle(ABILITY_ENDS[1], "Hidden Alcove", 2, &[Door])
            .slot(SlotKind::Ability)
            .build(),
        middle(DASH_CROSSINGS[0], "Broken Bridge", 1, &[Left, Right])
            .dash_gate(Left, Right)
            .build(),
        middle(DASH_CROSSINGS[1], "Chimney", 1, &[Top, Bottom])
            .dash_gate(Bottom, Top)
            .build(),
    ]
}

/// The hub: a room that is not a piece, with one exit to `START_A`'s id, the start room.
pub fn hub() -> (RoomId, RoomLayout) {
    room(HUB, 99, "Hub").exits(&[Side::Right]).build_hub()
}

impl PieceBuilder {
    /// The hub: its one exit leads to the start room.
    fn build_hub(self) -> (RoomId, RoomLayout) {
        let (id, mut layout) = self.build();
        layout.exits[0].target_room = START_A;
        (id, layout)
    }
}

/// The run rules the fixture dungeons are built and checked under.
pub fn config() -> RunConfig {
    RunConfig::from_ron("(hub_room: Custom(0), start_room: First, final_boss_room: Boss)")
        .expect("valid run rules")
}

/// Rules that suit the fixture pool: up to seven rooms on the spine, up to thirteen in all, up
/// to three keys, half the pieces mirrored.
pub fn rules() -> GeneratorRules {
    GeneratorRules {
        enabled: true,
        rooms: CountRange { min: 7, max: 13 },
        spine: CountRange { min: 4, max: 7 },
        keys: CountRange { min: 0, max: 3 },
        mirror_percent: 50.0,
        attempts: 100,
    }
}
