//! The generated-dungeon preview of the editor (`docs/dungeon-gen.md`): what a seed would build
//! from the pieces as the editor holds them, with no window. The Preview generated dungeon
//! window (`editor_panel::preview_window`) draws it.
//!
//! The preview is read-only. It reads the session's rooms (unsaved drafts included) and
//! `run.ron`'s and `variety.ron`'s rules, runs `dungeon_gen::generate` on copies, and keeps the
//! answer; it never writes a file and never touches the running game. `DungeonPreview` owns the
//! seed box (hex, checked with `parse_seed`) and Roll next, and builds the plan again only when
//! the seed or what the editor holds has changed, so drawing a frame costs no generation.
//!
//! The map is `MapLayout::build` of the plan's rooms, the hub first, so a run with the same
//! seed and the same rooms shows the same map. Rooms are named by their piece.
use crate::dungeon_gen::{generate, DungeonPlan};
use crate::dungeon_map::{EditorMap, MapLayout, MapRoomInput};
use crate::editor::RoomLayout;
use crate::editor_session::EditorSession;
use crate::item::Item;
use crate::room::RoomId;
use crate::room_name::room_title;
use crate::run::RunConfig;
use crate::seed::{format_seed, parse_seed, SeedSource};
use crate::variety::GeneratorRules;

/// What the preview generates from.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewInputs {
    /// Every room with a piece mark, in room id order.
    pub pieces: Vec<(RoomId, RoomLayout)>,
    /// The hub room, or why there is none.
    pub hub: Result<(RoomId, RoomLayout), String>,
    pub config: RunConfig,
    pub rules: GeneratorRules,
}

impl PreviewInputs {
    /// The pieces and the hub out of `catalog` (every room as the editor has it).
    pub fn of(
        catalog: Vec<(RoomId, RoomLayout)>,
        config: RunConfig,
        rules: GeneratorRules,
    ) -> Self {
        let hub = match config.hub_room {
            None => Err("run.ron has no hub_room, and a generated run starts from the hub".into()),
            Some(id) => catalog
                .iter()
                .find(|(room, _)| *room == id)
                .cloned()
                .ok_or_else(|| {
                    format!("run.ron names {id:?} as the hub, but there is no such room")
                }),
        };
        let mut pieces: Vec<_> = catalog
            .into_iter()
            .filter(|(_, layout)| layout.piece.is_some())
            .collect();
        pieces.sort_by_key(|(id, _)| id.index());
        Self {
            pieces,
            hub,
            config,
            rules,
        }
    }
}

impl EditorSession {
    /// What the preview generates from: the pieces as the session holds them (unsaved drafts in
    /// place of their files), the hub, and the rules the game gave the session.
    pub fn preview_inputs(&self) -> PreviewInputs {
        PreviewInputs::of(self.catalog(), self.run, self.generator)
    }
}

/// A key or an ability the plan put in a room.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pickup {
    /// The item's name: "Key 2", "Dash" or "Double Jump".
    pub what: String,
    /// The room it lies in, by its piece's name.
    pub room: String,
}

impl Pickup {
    pub fn line(&self) -> String {
        format!("{} in {}", self.what, self.room)
    }
}

/// A plan, ready to draw.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanView {
    pub seed: u64,
    /// The rooms of the dungeon, the hub not counted.
    pub rooms: usize,
    /// The attempt that gave the plan.
    pub attempts: u32,
    /// The hub and the plan's rooms on the map grid, with no room marked as current.
    pub map: EditorMap,
    /// Every key and ability in the dungeon's rooms, start room first.
    pub pickups: Vec<Pickup>,
}

impl PlanView {
    pub fn summary(&self) -> String {
        let rooms = if self.rooms == 1 { "room" } else { "rooms" };
        format!(
            "Seed {}: {} {rooms}, built on attempt {}",
            format_seed(self.seed),
            self.rooms,
            self.attempts
        )
    }

    fn of(plan: &DungeonPlan, pieces: &[(RoomId, RoomLayout)]) -> Self {
        let mut inputs = vec![MapRoomInput::from_layout(plan.hub_room, &plan.hub)];
        let mut pickups = Vec::new();
        for room in &plan.rooms {
            let own = pieces.iter().find(|(id, _)| *id == room.piece);
            let name = room_title(
                room.piece,
                own.and_then(|(_, layout)| layout.name.as_deref()),
            );
            let mut input = MapRoomInput::from_layout(room.instance, &room.layout);
            input.name = name.clone();
            inputs.push(input);
            for placed in &room.layout.items {
                if matches!(placed.item, Item::Key(_) | Item::Dash | Item::DoubleJump) {
                    pickups.push(Pickup {
                        what: placed.item.name().to_owned(),
                        room: name.clone(),
                    });
                }
            }
        }
        let mut map = EditorMap::plain(&MapLayout::build(&inputs));
        // Nothing leads back to the hub: a run starts there. It is not a room with no way in.
        for room in map.rooms.iter_mut().filter(|room| room.id == plan.hub_room) {
            room.no_entrance = false;
        }
        Self {
            seed: plan.seed,
            rooms: plan.rooms.len(),
            attempts: plan.attempts,
            map,
            pickups,
        }
    }
}

/// What the preview shows.
#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    /// The seed box does not hold a seed; the text says why.
    BadSeed(String),
    Plan(PlanView),
    /// The generator gave no dungeon (its `GenError`, or no hub); the text says why.
    Failed(String),
}

/// What `seed` builds from `inputs`.
pub fn outcome_for(seed: u64, inputs: &PreviewInputs) -> Outcome {
    let hub = match &inputs.hub {
        Ok(hub) => hub,
        Err(text) => return Outcome::Failed(text.clone()),
    };
    match generate(&inputs.pieces, hub, &inputs.config, &inputs.rules, seed) {
        Ok(plan) => Outcome::Plan(PlanView::of(&plan, &inputs.pieces)),
        Err(error) => Outcome::Failed(error.to_string()),
    }
}

/// The seed Roll next gives after `seed`: the next number of the sequence run seeds come from,
/// started at `seed`. The same seed always rolls to the same next one.
pub fn next_seed(seed: u64) -> u64 {
    SeedSource::new(seed).next()
}

/// The preview window's state: the seed box and the plan it last built.
#[derive(Clone, Debug)]
pub struct DungeonPreview {
    text: String,
    /// The last seed the box held while it held one; Roll next starts from it when the box
    /// does not.
    last_seed: u64,
    built: Option<(PreviewInputs, u64)>,
    outcome: Option<Outcome>,
}

impl Default for DungeonPreview {
    /// Starts at the first seed of the default run sequence, so it is the same every time.
    fn default() -> Self {
        Self::with_seed(SeedSource::default().next())
    }
}

impl DungeonPreview {
    pub fn with_seed(seed: u64) -> Self {
        Self {
            text: format_seed(seed),
            last_seed: seed,
            built: None,
            outcome: None,
        }
    }

    /// The seed box's text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The box, for the text field to edit.
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// The box as a seed: 1 to 16 hex digits (`parse_seed`), spaces around them ignored.
    pub fn seed(&self) -> Result<u64, String> {
        parse_seed(self.text.trim())
    }

    /// Puts the seed after the current one (`next_seed`) in the box. A box that holds no seed
    /// rolls from the last good one.
    pub fn roll_next(&mut self) {
        let next = next_seed(self.seed().unwrap_or(self.last_seed));
        self.set_seed(next);
    }

    pub fn set_seed(&mut self, seed: u64) {
        self.text = format_seed(seed);
        self.last_seed = seed;
    }

    /// Builds the plan for the box's seed from `inputs`, unless that is what was built last.
    pub fn refresh(&mut self, inputs: PreviewInputs) {
        match self.seed() {
            Err(text) => {
                self.built = None;
                self.outcome = Some(Outcome::BadSeed(text));
            }
            Ok(seed) => {
                self.last_seed = seed;
                let current =
                    matches!(&self.built, Some((had, was)) if *was == seed && *had == inputs);
                if !current {
                    self.outcome = Some(outcome_for(seed, &inputs));
                    self.built = Some((inputs, seed));
                }
            }
        }
    }

    /// What the last `refresh` found; `None` before the first.
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }

    /// A note for the window when the generator is switched off in `variety.ron`: the preview
    /// still builds, but a run only generates when Auto's rules say so or F4 says Always.
    pub fn note(&self) -> Option<&'static str> {
        let (inputs, _) = self.built.as_ref()?;
        (!inputs.rules.enabled).then_some(
            "generator.enabled is off in variety.ron: runs play the fixed dungeon unless F4 \
             sets Generate next run to Always.",
        )
    }
}
