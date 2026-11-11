//! Behavior definitions: states and transitions, their validation, and loading from RON.
use super::validators::nonnegative;
use super::{Action, Condition};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehaviorDefinition {
    pub initial_state: String,
    pub states: Vec<State>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct State {
    pub name: String,
    pub actions: Vec<Action>,
    pub transitions: Vec<Transition>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub target: String,
    pub condition: Condition,
    /// Inversion allows an exact complement, including distance equality.
    #[serde(default)]
    pub invert: bool,
}

impl BehaviorDefinition {
    /// A stationary starting point for authoring, with no inherited actions or links.
    pub fn blank() -> Self {
        Self {
            initial_state: "idle".into(),
            states: vec![State {
                name: "idle".into(),
                actions: vec![],
                transitions: vec![],
            }],
        }
    }

    pub fn from_ron(source: &str) -> Result<Self, String> {
        let definition: Self = ron::from_str(source).map_err(|error| error.to_string())?;
        definition.validate()?;
        Ok(definition)
    }

    pub fn validate(&self) -> Result<(), String> {
        let mut names = BTreeSet::new();
        for state in &self.states {
            if state.name.trim().is_empty() || !names.insert(state.name.as_str()) {
                return Err(format!("empty or duplicate state name {:?}", state.name));
            }
        }
        if !names.contains(self.initial_state.as_str()) {
            return Err(format!("missing initial state {:?}", self.initial_state));
        }
        for state in &self.states {
            for (index, action) in state.actions.iter().enumerate() {
                action
                    .validate()
                    .map_err(|error| format!("state {:?}, action {index}: {error}", state.name))?;
            }
            for transition in &state.transitions {
                if !names.contains(transition.target.as_str()) {
                    return Err(format!(
                        "state {:?}: transition to missing state {:?}",
                        state.name, transition.target
                    ));
                }
                match transition.condition {
                    Condition::TimerElapsed(value)
                    | Condition::PlayerDistanceLessThan(value)
                    | Condition::PlayerDistanceGreaterThan(value) => {
                        nonnegative(value).map_err(|error| {
                            format!("state {:?}, transition condition: {error}", state.name)
                        })?;
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn read_ron<T: DeserializeOwned>(path: &Path) -> Result<T, String> {
    let source =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    ron::from_str(&source).map_err(|error| format!("{}: {error}", path.display()))
}

pub(crate) fn ron_files(directory: &Path) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in
        std::fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("{}: {error}", directory.display()))?
            .path();
        if path.extension().is_some_and(|extension| extension == "ron") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

pub fn load_behaviors(
    directory: &Path,
) -> Result<BTreeMap<String, Arc<BehaviorDefinition>>, String> {
    let mut definitions = BTreeMap::new();
    for path in ron_files(directory)? {
        let definition: BehaviorDefinition = read_ron(&path)?;
        definition
            .validate()
            .map_err(|error| format!("{}: {error}", path.display()))?;
        let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("{}: invalid behavior filename", path.display()))?;
        definitions.insert(name.to_owned(), Arc::new(definition));
    }
    Ok(definitions)
}
