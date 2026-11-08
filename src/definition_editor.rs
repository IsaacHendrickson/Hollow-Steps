//! Pure definition edits and save planning; only load and write_files touch disk.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::behavior::{read_ron, ron_files, BehaviorDefinition, State};
use crate::enemy::{EnemyDefinition, EnemyDefinitions};
use crate::room::RoomEnemies;

#[derive(Debug)]
pub struct DefinitionDraft {
    /// Keys are stable filenames, independent of editable entity names.
    pub entities: BTreeMap<PathBuf, EnemyDefinition>,
    pub behaviors: BTreeMap<String, BehaviorDefinition>,
    original_names: BTreeMap<PathBuf, String>,
    assets: PathBuf,
}

impl DefinitionDraft {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let mut entities = BTreeMap::new();
        for path in ron_files(&assets.join("entities"))? {
            entities.insert(path.clone(), read_ron::<EnemyDefinition>(&path)?);
        }
        let behaviors = crate::behavior::load_behaviors(&assets.join("behaviors"))?
            .into_iter()
            .map(|(name, value)| (name, (*value).clone()))
            .collect();
        let original_names = entities
            .iter()
            .map(|(p, e)| (p.clone(), e.name.clone()))
            .collect();
        Ok(Self {
            entities,
            behaviors,
            original_names,
            assets: assets.to_owned(),
        })
    }

    pub fn entity_path(&self, original_name: &str) -> Option<PathBuf> {
        self.original_names
            .iter()
            .find(|(_, n)| n.as_str() == original_name)
            .map(|(p, _)| p.clone())
    }

    pub fn definitions(&self) -> Result<EnemyDefinitions, String> {
        EnemyDefinitions::from_definitions(
            self.entities.values().cloned(),
            self.behaviors
                .iter()
                .map(|(n, b)| (n.clone(), Arc::new(b.clone())))
                .collect(),
        )
    }

    pub fn rename_placements(&self, document: &mut RoomEnemies) {
        for enemy in &mut document.enemies {
            if let Some(path) = self.entity_path(&enemy.definition) {
                enemy.definition = self.entities[&path].name.clone();
            }
        }
    }

    /// No writes: validate and serialize every definition and every referencing room first.
    pub fn plan(
        &self,
        rooms: &mut [(PathBuf, RoomEnemies)],
    ) -> Result<Vec<(PathBuf, String)>, String> {
        let definitions = self.definitions()?;
        let mut files = Vec::new();
        for (path, entity) in &self.entities {
            files.push((path.clone(), serialize(entity)?));
        }
        for (name, behavior) in &self.behaviors {
            files.push((
                self.assets.join("behaviors").join(format!("{name}.ron")),
                serialize(behavior)?,
            ));
        }
        for (path, document) in rooms {
            self.rename_placements(document);
            for enemy in &document.enemies {
                definitions.size(&enemy.definition)?;
            }
            files.push((path.clone(), serialize(document)?));
        }
        Ok(files)
    }
}

/// Decode the six side-panel fields without touching the existing draft on failure.
pub fn entity_from_fields(fields: &[String; 6]) -> Result<EnemyDefinition, String> {
    for (i, text) in fields.iter().enumerate() {
        validate_entity_field(i, text)?;
    }
    let entity = EnemyDefinition {
        name: fields[0].clone(),
        health: fields[1].parse().map_err(|_| "health must be 0..255")?,
        size: (
            fields[2].parse().map_err(|_| "invalid width")?,
            fields[3].parse().map_err(|_| "invalid height")?,
        ),
        contact_damage: fields[4]
            .parse()
            .map_err(|_| "contact damage must be 0..255")?,
        behavior: fields[5].clone(),
    };
    entity.validate()?;
    Ok(entity)
}

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
    if name.trim().is_empty() || behavior.states.iter().any(|s| s.name == name) {
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

pub fn serialize(value: &impl serde::Serialize) -> Result<String, String> {
    ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())
}

/// Stage all files before replacing any. Roll back previous replacements on an I/O failure.
pub fn write_files(files: &[(PathBuf, String)]) -> Result<(), String> {
    use std::io::Write;
    let mut staged = Vec::new();
    let result = (|| {
        for (path, source) in files {
            let previous = match std::fs::read(path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e.to_string()),
            };
            let temp = path.with_extension("ron.tmp");
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| format!("{}: {e}", temp.display()))?;
            staged.push((path, temp, previous));
            file.write_all(source.as_bytes())
                .map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        for (index, (path, temp, _)) in staged.iter().enumerate() {
            if let Err(error) = std::fs::rename(temp, path) {
                let mut errors = vec![error.to_string()];
                for (path, _, previous) in staged[..index].iter().rev() {
                    let rollback = match previous {
                        Some(bytes) => std::fs::write(path, bytes),
                        None => std::fs::remove_file(path),
                    };
                    if let Err(e) = rollback {
                        errors.push(format!("rollback {}: {e}", path.display()));
                    }
                }
                return Err(errors.join("; "));
            }
        }
        Ok(())
    })();
    for (_, temp, _) in staged {
        let _ = std::fs::remove_file(temp);
    }
    result
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
            action.set_parameter(i, value)?;
        }
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
        if !names.contains(&self.target) {
            return Err("missing target state".into());
        }
        let mut condition =
            crate::behavior::Condition::defaults(self.kind).ok_or("unknown condition")?;
        if condition.parameter().is_some() {
            condition.set_parameter(&self.value)?;
        }
        Ok(crate::behavior::Transition {
            target: self.target.clone(),
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

/// Fixed preferred width, capped to the viewport; Save has its own reserved strip.
#[derive(Clone, Copy, Debug)]
pub struct PanelLayout {
    pub panel: macroquad::math::Rect,
    pub body: macroquad::math::Rect,
    pub save: macroquad::math::Rect,
}
impl PanelLayout {
    pub fn new(width: f32, height: f32) -> Self {
        use macroquad::math::Rect;
        let width = if width.is_finite() {
            width.max(0.0)
        } else {
            0.0
        };
        let height = if height.is_finite() {
            height.max(0.0)
        } else {
            0.0
        };
        let w = width.min(360.0);
        let footer = height.min(34.0);
        Self {
            panel: Rect::new(width - w, 0.0, w, height),
            body: Rect::new(width - w, 0.0, w, height - footer),
            save: Rect::new(width - w, height - footer, w, footer),
        }
    }
    pub fn scroll_bounds(&self, content_height: f32, requested: f32) -> f32 {
        requested
            .max(0.0)
            .min((content_height - self.body.h).max(0.0))
    }
}
/// Character wrapping also splits unbroken identifiers and filesystem paths.
pub fn wrap_label(text: &str, columns: usize) -> Vec<String> {
    let columns = columns.max(1);
    text.lines()
        .flat_map(|line| {
            let chars: Vec<_> = line.chars().collect();
            chars
                .chunks(columns)
                .map(|chunk| chunk.iter().collect())
                .collect::<Vec<String>>()
        })
        .collect()
}

pub fn validate_entity_field(index: usize, text: &str) -> Result<(), String> {
    match index {
        0 | 5 if text.trim().is_empty() => Err("must not be empty".into()),
        1 => match text.parse::<u8>() {
            Ok(1..=255) => Ok(()),
            _ => Err("expected integer 1..255".into()),
        },
        2 | 3 => {
            let value = crate::behavior::parse_nonnegative(text)?;
            if value > 0.0 {
                Ok(())
            } else {
                Err("must be positive".into())
            }
        }
        4 => text
            .parse::<u8>()
            .map(|_| ())
            .map_err(|_| "expected integer 0..255".into()),
        _ => Ok(()),
    }
}
