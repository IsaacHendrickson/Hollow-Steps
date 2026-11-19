//! State, action and transition edits on one behavior, with text buffers and list helpers.
use crate::behavior::{BehaviorDefinition, State};

pub fn set_initial_state(behavior: &mut BehaviorDefinition, index: usize) -> Result<(), String> {
    behavior.initial_state = behavior
        .states
        .get(index)
        .ok_or("missing state")?
        .name
        .clone();
    Ok(())
}

pub fn add_state(behavior: &mut BehaviorDefinition, name: String) -> Result<(), String> {
    let name = name.trim().to_owned();
    if name.is_empty() || behavior.states.iter().any(|s| s.name == name) {
        return Err("state name must be nonempty and unique".into());
    }
    if behavior.states.is_empty() {
        behavior.initial_state = name.clone();
    }
    behavior.states.push(State {
        name,
        actions: Vec::new(),
        transitions: Vec::new(),
    });
    Ok(())
}

pub fn rename_state(
    behavior: &mut BehaviorDefinition,
    index: usize,
    name: String,
) -> Result<(), String> {
    let name = name.trim().to_owned();
    let old = behavior
        .states
        .get(index)
        .ok_or("missing state")?
        .name
        .clone();
    if name.trim().is_empty()
        || behavior
            .states
            .iter()
            .enumerate()
            .any(|(i, s)| i != index && s.name == name)
    {
        return Err("state name must be nonempty and unique".into());
    }
    if behavior.initial_state == old {
        behavior.initial_state = name.clone();
    }
    for state in &mut behavior.states {
        for transition in &mut state.transitions {
            if transition.target == old {
                transition.target = name.clone();
            }
        }
    }
    behavior.states[index].name = name;
    Ok(())
}

pub fn remove_state(behavior: &mut BehaviorDefinition, index: usize) -> Result<(), String> {
    if index >= behavior.states.len() {
        return Err("missing state".into());
    }
    if behavior.states.len() == 1 {
        return Err("cannot remove the last state".into());
    }
    let removed = behavior.states.remove(index).name;
    if behavior.initial_state == removed {
        behavior.initial_state = behavior.states[0].name.clone();
    }
    for state in &mut behavior.states {
        state.transitions.retain(|t| t.target != removed);
    }
    Ok(())
}

/// Parse entire ordered lists, preserving the draft on syntax errors. Semantic errors are shown at save.
pub fn edit_state_lists(state: &mut State, actions: &str, transitions: &str) -> Result<(), String> {
    let actions = ron::from_str(actions).map_err(|e| format!("actions: {e}"))?;
    let transitions = ron::from_str(transitions).map_err(|e| format!("transitions: {e}"))?;
    state.actions = actions;
    state.transitions = transitions;
    Ok(())
}

/// Text buffers preserve partially typed numbers without corrupting loaded values.
#[derive(Clone, Debug)]
pub struct ActionInput {
    pub kind: usize,
    pub values: Vec<String>,
}
impl ActionInput {
    pub fn new(kind: usize) -> Result<Self, String> {
        let action = crate::behavior::Action::defaults(kind).ok_or("unknown action")?;
        Ok(Self::from_action(&action))
    }
    pub fn from_action(action: &crate::behavior::Action) -> Self {
        Self {
            kind: action.kind(),
            values: action.parameters().into_iter().map(|(_, v)| v).collect(),
        }
    }
    pub fn build(&self) -> Result<crate::behavior::Action, String> {
        let mut action = crate::behavior::Action::defaults(self.kind).ok_or("unknown action")?;
        if self.values.len() != action.parameters().len() {
            return Err("missing action parameter".into());
        }
        for (i, value) in self.values.iter().enumerate() {
            action.set_parameter(i, value.trim())?;
        }
        action.validate()?;
        Ok(action)
    }
}
#[derive(Clone, Debug)]
pub struct TransitionInput {
    pub target: String,
    pub kind: usize,
    pub value: String,
    pub invert: bool,
}
impl TransitionInput {
    pub fn new(target: String, kind: usize) -> Result<Self, String> {
        let condition = crate::behavior::Condition::defaults(kind).ok_or("unknown condition")?;
        Ok(Self {
            target,
            kind,
            value: condition.parameter().map(|(_, v)| v).unwrap_or_default(),
            invert: false,
        })
    }
    pub fn change_kind(&mut self, kind: usize) -> Result<(), String> {
        let new = Self::new(self.target.clone(), kind)?;
        self.kind = new.kind;
        self.value = new.value;
        Ok(())
    }
    pub fn build(&self, names: &[String]) -> Result<crate::behavior::Transition, String> {
        if !names.iter().any(|n| n == self.target.trim()) {
            return Err("missing target state".into());
        }
        let mut condition =
            crate::behavior::Condition::defaults(self.kind).ok_or("unknown condition")?;
        if condition.parameter().is_some() {
            condition.set_parameter(self.value.trim())?;
        }
        Ok(crate::behavior::Transition {
            target: self.target.trim().to_owned(),
            condition,
            invert: self.invert,
        })
    }
}
#[derive(Clone, Debug, Default)]
pub struct StateInputs {
    pub actions: Vec<ActionInput>,
    pub transitions: Vec<TransitionInput>,
}
impl StateInputs {
    pub fn from_state(state: &State) -> Self {
        Self {
            actions: state.actions.iter().map(ActionInput::from_action).collect(),
            transitions: state
                .transitions
                .iter()
                .map(|t| TransitionInput {
                    target: t.target.clone(),
                    kind: t.condition.kind(),
                    value: t.condition.parameter().map(|(_, v)| v).unwrap_or_default(),
                    invert: t.invert,
                })
                .collect(),
        }
    }
    pub fn add_action(&mut self, kind: usize) -> Result<(), String> {
        self.actions.push(ActionInput::new(kind)?);
        Ok(())
    }
    pub fn add_transition(&mut self, target: String, kind: usize) -> Result<(), String> {
        self.transitions.push(TransitionInput::new(target, kind)?);
        Ok(())
    }
    /// Transactional: invalid input never changes the state being saved.
    pub fn apply(&self, state: &mut State, names: &[String]) -> Result<(), String> {
        let actions = self
            .actions
            .iter()
            .map(ActionInput::build)
            .collect::<Result<Vec<_>, _>>()?;
        let transitions = self
            .transitions
            .iter()
            .map(|t| t.build(names))
            .collect::<Result<Vec<_>, _>>()?;
        state.actions = actions;
        state.transitions = transitions;
        Ok(())
    }
}
/// Bounds-checked ordered list edits, shared by actions and transitions.
pub fn remove_row<T>(rows: &mut Vec<T>, index: usize) -> Result<(), String> {
    if index >= rows.len() {
        return Err("missing row".into());
    }
    rows.remove(index);
    Ok(())
}
pub fn move_row<T>(rows: &mut [T], index: usize, up: bool) -> Result<(), String> {
    let target = if up {
        index.checked_sub(1)
    } else {
        index.checked_add(1)
    }
    .ok_or("list boundary")?;
    if index >= rows.len() || target >= rows.len() {
        return Err("list boundary".into());
    }
    rows.swap(index, target);
    Ok(())
}
