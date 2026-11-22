//! The player as the editor edits it: a draft of the `player.ron` numbers (saved values, edited
//! values, dirty), reset, validation and the staged save. Pure rules; the F4 window and the
//! editor's Player panel both draw this one draft.
//!
//! The field table (key, label, group, unit, range, step) is `tuning_fields::SPECS`: a new
//! number is one `spec(..)` line there plus its `Field` variant and get/set arms. A test
//! serializes the default file and fails when a number written to it has no table entry.
//!
//! The draft also carries the `vitals.ron` numbers (mana per kill, the mana cap, healing and the
//! trailing health bar), which the Player panel shows beside the stats. They keep their own saved
//! and edited copies, their own range table (`vitals::VitalsField`) and their own file, so the
//! stats' `is_stats_dirty` / `save` are as they were and `is_dirty` is true for either.
//!
//! Three levels of checking, each with its own place:
//! - one field: its range (`FieldSpec::check`), applied when a value is entered, so the draft
//!   never holds a number outside its range;
//! - several fields together (`problems`): the held jump must end, and neither it nor the held
//!   air jump of Double Jump may rise past `MAX_JUMP_APEX`;
//! - the rooms (`problems` with `SpawnCheck`s): the player must not start inside a solid.
//!
//! The last two can fail for a draft that is otherwise made of valid numbers, so the draft may
//! hold them. They name the fields involved (for the message next to the field) and block Save.
//!
//! One more rule is about the vitals file and the charm file together: `vitals.ron`'s max mana
//! must cover the cost of every charm in the charm list that is saved with it (`CharmCost`,
//! `vitals_problem`). Without it a Save can write a file that the next start refuses to load.

use std::path::{Path, PathBuf};

use macroquad::math::Vec2;

use crate::charms::{number_text, Charm};
use crate::editor::{EditorSession, RoomLayout};
use crate::room::{Room, RoomId};
use crate::terrain::Terrain;
use crate::tuning::{self, Field, TuningConfig};
use crate::vitals::{self, VitalsConfig, VitalsField};

/// The highest a jump may rise while the jump key is held, in pixels. Without a limit the
/// held-jump gravity can be tuned so low that the jump never ends.
pub const MAX_JUMP_APEX: f32 = 1_000.0;

/// What a room says about where the player starts: the spawn point and the solids around it.
#[derive(Clone, Debug, PartialEq)]
pub struct SpawnCheck {
    pub room: String,
    pub spawn: Vec2,
    pub terrain: Terrain,
}

impl SpawnCheck {
    pub fn from_room(room: &Room) -> Self {
        Self {
            room: format!("{:?}", room.id),
            spawn: room.spawn,
            terrain: room.terrain.clone(),
        }
    }

    pub fn from_layout(id: RoomId, layout: &RoomLayout) -> Self {
        Self {
            room: format!("{id:?}"),
            spawn: layout.spawn.into(),
            terrain: Terrain::from_tuples(&layout.terrain),
        }
    }
}

/// What a charm costs in mana, for `PlayerDraft::vitals_problem`: the draft is given the charm
/// list's costs the way it is given the rooms' spawns.
#[derive(Clone, Debug, PartialEq)]
pub struct CharmCost {
    pub name: String,
    pub cost: f32,
}

impl CharmCost {
    /// The cost of every charm in `charms`, in order.
    pub fn from_charms(charms: &[Charm]) -> Vec<Self> {
        charms
            .iter()
            .map(|charm| Self {
                name: charm.name.clone(),
                cost: charm.mana_cost,
            })
            .collect()
    }
}

impl EditorSession {
    /// Where the player starts in every room: each saved room, with the room being edited as
    /// its draft has it.
    pub fn spawn_checks(&self) -> Vec<SpawnCheck> {
        self.catalog()
            .iter()
            .map(|(id, layout)| SpawnCheck::from_layout(*id, layout))
            .collect()
    }
}

/// Why a draft cannot be saved, and which fields it is about.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    pub fields: Vec<Field>,
    pub message: String,
}

/// The `player.ron` and `vitals.ron` numbers as saved and as edited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerDraft {
    saved: TuningConfig,
    edited: TuningConfig,
    saved_vitals: VitalsConfig,
    edited_vitals: VitalsConfig,
}

impl Default for PlayerDraft {
    fn default() -> Self {
        Self::new(TuningConfig::default())
    }
}

impl PlayerDraft {
    /// A clean draft: the edited values are the saved ones. The vitals are the shipped defaults
    /// (`with_vitals` gives others).
    pub fn new(saved: TuningConfig) -> Self {
        Self {
            saved,
            edited: saved,
            saved_vitals: VitalsConfig::default(),
            edited_vitals: VitalsConfig::default(),
        }
    }

    /// The same draft with `vitals` as both its saved and its edited vitals.
    pub fn with_vitals(mut self, vitals: VitalsConfig) -> Self {
        self.saved_vitals = vitals;
        self.edited_vitals = vitals;
        self
    }

    pub fn saved(&self) -> &TuningConfig {
        &self.saved
    }

    pub fn edited(&self) -> &TuningConfig {
        &self.edited
    }

    pub fn saved_vitals(&self) -> &VitalsConfig {
        &self.saved_vitals
    }

    pub fn edited_vitals(&self) -> &VitalsConfig {
        &self.edited_vitals
    }

    /// True when the edited stats or the edited vitals differ from the saved ones.
    pub fn is_dirty(&self) -> bool {
        self.is_stats_dirty() || self.is_vitals_dirty()
    }

    /// True when the edited `player.ron` numbers differ from the saved ones.
    pub fn is_stats_dirty(&self) -> bool {
        self.edited != self.saved
    }

    /// True when the edited `vitals.ron` numbers differ from the saved ones.
    pub fn is_vitals_dirty(&self) -> bool {
        self.edited_vitals != self.saved_vitals
    }

    pub fn is_field_dirty(&self, field: Field) -> bool {
        field.get(&self.edited) != field.get(&self.saved)
    }

    pub fn is_vitals_field_dirty(&self, field: VitalsField) -> bool {
        field.get(&self.edited_vitals) != field.get(&self.saved_vitals)
    }

    /// Sets one number. A value outside the field's range (or not a number) is refused with
    /// the reason and the draft is unchanged.
    pub fn set_field(&mut self, field: Field, value: f32) -> Result<(), String> {
        self.edited = self.edited.with_field(field, value)?;
        Ok(())
    }

    /// Sets one vitals number. A value outside its range, or one that puts the starting mana
    /// above the mana cap, is refused with the reason and the draft is unchanged.
    pub fn set_vitals_field(&mut self, field: VitalsField, value: f32) -> Result<(), String> {
        self.edited_vitals = self.edited_vitals.with_field(field, value)?;
        Ok(())
    }

    /// Replaces every edited value at once; refused (and unchanged) if any is out of range.
    pub fn set_edited(&mut self, edited: TuningConfig) -> Result<(), String> {
        edited.validate()?;
        self.edited = edited;
        Ok(())
    }

    /// Replaces every edited vitals value at once; refused (and unchanged) if any is invalid.
    pub fn set_edited_vitals(&mut self, edited: VitalsConfig) -> Result<(), String> {
        edited.validate()?;
        self.edited_vitals = edited;
        Ok(())
    }

    /// One field back to its saved value.
    pub fn reset_field(&mut self, field: Field) {
        let saved = field.get(&self.saved);
        field.set(&mut self.edited, saved);
    }

    /// One vitals field back to its saved value. Refused (and unchanged) when that would put the
    /// starting mana above the mana cap, as an edit would be: reset the mana cap first.
    pub fn reset_vitals_field(&mut self, field: VitalsField) -> Result<(), String> {
        let saved = field.get(&self.saved_vitals);
        let mut next = self.edited_vitals;
        field.set(&mut next, saved);
        next.validate()?;
        self.edited_vitals = next;
        Ok(())
    }

    /// Every field, stats and vitals, back to its saved value.
    pub fn reset_all(&mut self) {
        self.edited = self.saved;
        self.edited_vitals = self.saved_vitals;
    }

    /// The edited stats are now the saved ones (they were just written). The vitals are not
    /// touched: `vitals.ron` is a file of its own (`mark_vitals_saved`).
    pub fn mark_saved(&mut self) {
        self.saved = self.edited;
    }

    /// The edited vitals are now the saved ones (they were just written).
    pub fn mark_vitals_saved(&mut self) {
        self.saved_vitals = self.edited_vitals;
    }

    /// Both the saved and the edited vitals become `vitals` (they were just loaded).
    pub fn reload_vitals(&mut self, vitals: VitalsConfig) {
        self.saved_vitals = vitals;
        self.edited_vitals = vitals;
    }

    /// Everything the draft's numbers get wrong beyond their own ranges, in a fixed order.
    pub fn problems(&self, spawns: &[SpawnCheck]) -> Vec<Problem> {
        let mut problems = Vec::new();
        problems.extend(jump_problem(&self.edited));
        problems.extend(double_jump_problem(&self.edited));
        problems.extend(size_problems(&self.edited, spawns));
        problems
    }

    /// The first problem that involves `field`, for the message next to it.
    pub fn problem_for(&self, field: Field, spawns: &[SpawnCheck]) -> Option<String> {
        self.problems(spawns)
            .into_iter()
            .find(|problem| problem.fields.contains(&field))
            .map(|problem| problem.message)
    }

    /// `Ok` when the draft may be saved or played.
    pub fn validate(&self, spawns: &[SpawnCheck]) -> Result<(), String> {
        self.edited.validate()?;
        self.edited_vitals.validate()?;
        match self.problems(spawns).into_iter().next() {
            Some(problem) => Err(problem.message),
            None => Ok(()),
        }
    }

    /// Writes the edited values to `player.ron` in `assets`, staged so a failed write leaves
    /// the old file, and marks them saved. An invalid draft writes nothing.
    pub fn save(&mut self, assets: &Path, spawns: &[SpawnCheck]) -> Result<PathBuf, String> {
        self.validate(spawns)?;
        let path = tuning::save(assets, &self.edited)?;
        self.mark_saved();
        Ok(path)
    }

    /// Why the edited max mana cannot be saved: it is below the cost of a charm in `costs`, the
    /// charm list that will be on disk with it. Names the dearest charm (the first of equals), the
    /// number that was compared and the lowest max mana that is allowed. `None` when every cost is
    /// covered (a max mana equal to the highest cost is), and for an empty list. A raise of a
    /// saved max mana always passes: the saved list loaded under the saved max mana.
    pub fn vitals_problem(&self, costs: &[CharmCost]) -> Option<String> {
        let max_mana = self.edited_vitals.max_mana;
        let dearest = costs
            .iter()
            .reduce(|best, charm| if charm.cost > best.cost { charm } else { best })?;
        (dearest.cost > max_mana).then(|| {
            format!(
                "max mana {} is below the mana cost of the {} ({}) in charms.ron: the lowest \
                 allowed max mana is {}",
                number_text(max_mana),
                dearest.name,
                number_text(dearest.cost),
                number_text(dearest.cost),
            )
        })
    }

    /// `Ok` when the edited vitals may be saved next to the charm list `costs` describes.
    pub fn validate_vitals(&self, costs: &[CharmCost]) -> Result<(), String> {
        self.edited_vitals.validate()?;
        match self.vitals_problem(costs) {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }

    /// Writes the edited vitals to `vitals.ron` in `assets`, staged like `save`, and marks them
    /// saved. Invalid vitals write nothing, and neither do vitals that `vitals_problem` refuses
    /// for `costs`.
    pub fn save_vitals_checked(
        &mut self,
        assets: &Path,
        costs: &[CharmCost],
    ) -> Result<PathBuf, String> {
        self.validate_vitals(costs)?;
        self.save_vitals(assets)
    }

    /// Writes the edited vitals to `vitals.ron` in `assets`, staged like `save`, and marks them
    /// saved. Invalid vitals write nothing. This is the bare write: it does not know the charm
    /// list, so the game saves through `save_vitals_checked`.
    pub fn save_vitals(&mut self, assets: &Path) -> Result<PathBuf, String> {
        let path = vitals::save(assets, &self.edited_vitals)?;
        self.mark_vitals_saved();
        Ok(path)
    }
}

/// The apex of a held jump is `jump_speed² / (2 · gravity · held multiplier)`; it must be
/// finite and at most `MAX_JUMP_APEX`.
fn jump_problem(config: &TuningConfig) -> Option<Problem> {
    let player = &config.player;
    let rising_gravity = player.gravity * player.held_jump_gravity_multiplier;
    let fields = vec![
        Field::JumpSpeed,
        Field::Gravity,
        Field::HeldJumpGravityMultiplier,
    ];
    if rising_gravity <= 0.0 {
        return Some(Problem {
            fields,
            message: "a held jump would never end: gravity while jump held must be above 0".into(),
        });
    }
    let apex = player.jump_speed * player.jump_speed / (2.0 * rising_gravity);
    (apex > MAX_JUMP_APEX).then(|| Problem {
        fields,
        message: format!(
            "a held jump would rise {apex:.0} px, more than the {MAX_JUMP_APEX:.0} px allowed: \
             lower the jump speed or raise gravity"
        ),
    })
}

/// The air jump of Double Jump, held, rises `double_jump_speed² / (2 · gravity · held multiplier)`
/// on its own; that is held to `MAX_JUMP_APEX` too. A held jump that never ends is the jump's
/// problem (`jump_problem`, same gravity numbers), so it is not repeated here.
fn double_jump_problem(config: &TuningConfig) -> Option<Problem> {
    let player = &config.player;
    let rising_gravity = player.gravity * player.held_jump_gravity_multiplier;
    if rising_gravity <= 0.0 {
        return None;
    }
    let apex = player.double_jump_speed * player.double_jump_speed / (2.0 * rising_gravity);
    (apex > MAX_JUMP_APEX).then(|| Problem {
        fields: vec![
            Field::DoubleJumpSpeed,
            Field::Gravity,
            Field::HeldJumpGravityMultiplier,
        ],
        message: format!(
            "a held double jump would rise {apex:.0} px, more than the {MAX_JUMP_APEX:.0} px \
             allowed: lower the double jump speed or raise gravity"
        ),
    })
}

/// A player of the draft's size must not overlap a solid where it starts in any room.
fn size_problems(config: &TuningConfig, spawns: &[SpawnCheck]) -> Vec<Problem> {
    let size = config.player.size;
    spawns
        .iter()
        .filter(|check| check.terrain.overlaps_body(check.spawn, size))
        .map(|check| Problem {
            fields: vec![Field::Width, Field::Height],
            message: format!(
                "a {} x {} px player would start inside terrain in room {}",
                size.x, size.y, check.room
            ),
        })
        .collect()
}
