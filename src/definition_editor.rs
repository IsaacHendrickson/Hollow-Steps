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
