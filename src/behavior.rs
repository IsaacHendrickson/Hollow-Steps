//! Window-free, data-driven enemy state machines. All time is simulation time.
//!
//! The action registry (`action`, `action_serde`, `action_info`), conditions, definitions and
//! the per-entity runtime each have a module; this root re-exports the public API.
mod action;
mod action_info;
mod action_serde;
mod condition;
mod definition;
mod runtime;
mod validators;

pub use action::Action;
pub use condition::{parse_nonnegative, Condition, ConditionContext};
pub use definition::{load_behaviors, BehaviorDefinition, State, Transition};
pub(crate) use definition::{read_ron, ron_files};
pub use runtime::{Behavior, BehaviorContext, Blast};
pub(crate) use validators::{nonnegative, positive};
