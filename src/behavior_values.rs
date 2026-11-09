//! Numeric behavior controls derived from every action and condition in every state.
use crate::behavior::{Action, BehaviorDefinition, Condition};

#[derive(Clone, Debug, PartialEq)]
enum Parameter {
    Action {
        state: usize,
        row: usize,
        field: usize,
    },
    Condition {
        state: usize,
        row: usize,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct KeyValue {
    pub label: String,
    pub value: String,
    parameter: Parameter,
}

#[derive(Clone, Debug, Default)]
pub struct KeyValues {
    pub values: Vec<KeyValue>,
}

impl KeyValues {
    pub fn from_behavior(behavior: &BehaviorDefinition) -> Self {
        let mut values = Vec::new();
        for (state, definition) in behavior.states.iter().enumerate() {
            for (row, action) in definition.actions.iter().enumerate() {
                let description = match action {
                    Action::MoveTowardPlayer { .. } => "speed toward player (pixels/second)",
                    Action::MoveAwayFromPlayer { .. } => "speed away from player (pixels/second)",
                    Action::Patrol { .. } => "patrol speed (pixels/second)",
                    Action::Wait { .. } => "wait duration (seconds)",
                    Action::Telegraph { .. } => "telegraph duration (seconds)",
                    _ => continue, // Boss actions have no enemy editor controls.
                };
                for (field, (_, value)) in action.parameters().into_iter().enumerate() {
                    values.push(KeyValue {
                        label: format!("{}: {description} [action {}]", definition.name, row + 1),
                        value,
                        parameter: Parameter::Action { state, row, field },
                    });
                }
            }
            for (row, transition) in definition.transitions.iter().enumerate() {
                let Some((_, value)) = transition.condition.parameter() else {
                    continue;
                };
                let description = match (&transition.condition, transition.invert) {
                    (Condition::TimerElapsed(_), false) => "time in state at least (seconds)",
                    (Condition::TimerElapsed(_), true) => "time in state below (seconds)",
                    (Condition::PlayerDistanceLessThan(_), false) => "player closer than (pixels)",
                    (Condition::PlayerDistanceLessThan(_), true) => {
                        "player at least this far (pixels)"
                    }
                    (Condition::PlayerDistanceGreaterThan(_), false) => {
                        "player farther than (pixels)"
                    }
                    (Condition::PlayerDistanceGreaterThan(_), true) => {
                        "player no farther than (pixels)"
                    }
                    (Condition::HealthBelow(_), false) => "health below (integer 0..255)",
                    (Condition::HealthBelow(_), true) => "health at least (integer 0..255)",
                    (Condition::ActionFinished, _) => unreachable!("no numeric parameter"),
                };
                values.push(KeyValue {
                    label: format!(
                        "{}: switch to {} when {description} [transition {}]",
                        definition.name,
                        transition.target,
                        row + 1
                    ),
                    value,
                    parameter: Parameter::Condition { state, row },
                });
            }
        }
        Self { values }
    }

    /// Invalid input never changes the model. Uses the same numeric rules as advanced rows.
    pub fn apply(&self, behavior: &mut BehaviorDefinition) -> Result<(), String> {
        let mut candidate = behavior.clone();
        for entry in &self.values {
            let result = match entry.parameter {
                Parameter::Action { state, row, field } => candidate
                    .states
                    .get_mut(state)
                    .and_then(|s| s.actions.get_mut(row))
                    .ok_or_else(|| "missing action".to_owned())
                    .and_then(|a| a.set_parameter(field, &entry.value)),
                Parameter::Condition { state, row } => candidate
                    .states
                    .get_mut(state)
                    .and_then(|s| s.transitions.get_mut(row))
                    .ok_or_else(|| "missing transition".to_owned())
                    .and_then(|t| t.condition.set_parameter(&entry.value)),
            };
            result.map_err(|e| format!("{}: {e}", entry.label))?;
        }
        candidate.validate()?;
        *behavior = candidate;
        Ok(())
    }
}
