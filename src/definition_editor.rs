//! Pure definition edits and validation, with explicit load, plan, and write boundaries.
//!
//! One module per job: `draft` (the draft, its load and queries), `plan` (save planning and
//! staged writes), `entities` (entity and behavior edits), `states` (state, action and
//! transition edits). Every public item is re-exported here, so paths do not change.
mod draft;
mod entities;
mod plan;
mod states;
#[cfg(test)]
mod tests;

pub use draft::DefinitionDraft;
pub use entities::{
    entity_from_fields, entity_with_body, entity_with_body_and_death,
    entity_with_body_death_and_movement, validate_entity_field,
};
#[cfg(test)]
use plan::file_error;
pub use plan::{serialize, write_files, SavePlan};
pub use states::{
    add_state, edit_state_lists, move_row, remove_row, remove_state, rename_state,
    set_initial_state, ActionInput, StateInputs, TransitionInput,
};
