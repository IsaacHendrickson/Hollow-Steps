//! The player as the editor edits it: a draft of the `player.ron` numbers (saved values, edited
//! values, dirty), reset, validation and the staged save. Pure rules; the F4 window and the
//! editor's Player panel both draw this one draft.
//!
//! The field table (key, label, group, unit, range, step) is `tuning_fields::SPECS`: a new
//! number is one `spec(..)` line there plus its `Field` variant and get/set arms. A test
//! serializes the default file and fails when a number written to it has no table entry.
//!
//! Three levels of checking, each with its own place:
//! - one field: its range (`FieldSpec::check`), applied when a value is entered, so the draft
//!   never holds a number outside its range;
//! - several fields together (`problems`): the held jump must end;
//! - the rooms (`problems` with `SpawnCheck`s): the player must not start inside a solid.
//!
//! The last two can fail for a draft that is otherwise made of valid numbers, so the draft may
//! hold them. They name the fields involved (for the message next to the field) and block Save.

use std::path::{Path, PathBuf};

use macroquad::math::Vec2;

use crate::editor::{EditorSession, RoomLayout};
use crate::room::{Room, RoomId};
use crate::terrain::Terrain;
use crate::tuning::{self, Field, TuningConfig};

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

/// The `player.ron` numbers as saved and as edited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerDraft {
    saved: TuningConfig,
    edited: TuningConfig,
}

impl Default for PlayerDraft {
    fn default() -> Self {
        Self::new(TuningConfig::default())
    }
}

impl PlayerDraft {
    /// A clean draft: the edited values are the saved ones.
    pub fn new(saved: TuningConfig) -> Self {
        Self {
            saved,
            edited: saved,
        }
    }

    pub fn saved(&self) -> &TuningConfig {
        &self.saved
    }

    pub fn edited(&self) -> &TuningConfig {
        &self.edited
    }

    /// True when the edited values differ from the saved ones.
    pub fn is_dirty(&self) -> bool {
        self.edited != self.saved
    }

    pub fn is_field_dirty(&self, field: Field) -> bool {
        field.get(&self.edited) != field.get(&self.saved)
    }

    /// Sets one number. A value outside the field's range (or not a number) is refused with
    /// the reason and the draft is unchanged.
    pub fn set_field(&mut self, field: Field, value: f32) -> Result<(), String> {
        self.edited = self.edited.with_field(field, value)?;
        Ok(())
    }

    /// Replaces every edited value at once; refused (and unchanged) if any is out of range.
    pub fn set_edited(&mut self, edited: TuningConfig) -> Result<(), String> {
        edited.validate()?;
        self.edited = edited;
        Ok(())
    }

    /// One field back to its saved value.
    pub fn reset_field(&mut self, field: Field) {
        let saved = field.get(&self.saved);
        field.set(&mut self.edited, saved);
    }

    /// Every field back to its saved value.
    pub fn reset_all(&mut self) {
        self.edited = self.saved;
    }

    /// The edited values are now the saved ones (they were just written).
    pub fn mark_saved(&mut self) {
        self.saved = self.edited;
    }

    /// Everything the draft's numbers get wrong beyond their own ranges, in a fixed order.
    pub fn problems(&self, spawns: &[SpawnCheck]) -> Vec<Problem> {
        let mut problems = Vec::new();
        problems.extend(jump_problem(&self.edited));
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
