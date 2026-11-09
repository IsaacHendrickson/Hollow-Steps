//! Pure definition edits and validation, with explicit load, plan, and write boundaries.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::behavior::{read_ron, ron_files, BehaviorDefinition, State};
use crate::enemy::{EnemyDefinition, EnemyDefinitions};
use crate::room::RoomEnemies;

/// Use native separators even when a caller supplies a path with mixed separators.
fn file_error(path: &Path, error: impl std::fmt::Display) -> String {
    let path: PathBuf = path.components().collect();
    format!("{}: {error}", path.display())
}

#[derive(Clone, Debug)]
pub struct DefinitionDraft {
    /// Keys are stable filenames, independent of editable entity names.
    pub entities: BTreeMap<PathBuf, EnemyDefinition>,
    pub behaviors: BTreeMap<String, BehaviorDefinition>,
    pub presets: Vec<crate::preset::BehaviorPreset>,
    original_names: BTreeMap<PathBuf, String>,
    assets: PathBuf,
    originals: BTreeMap<PathBuf, Vec<u8>>,
    layouts: Vec<(crate::room::RoomId, crate::editor::RoomLayout)>,
}

impl DefinitionDraft {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let assets: PathBuf = assets.components().collect();
        let assets = assets.as_path();
        let mut originals = BTreeMap::new();
        for directory in ["entities", "behaviors", "rooms"] {
            for path in ron_files(&assets.join(directory))? {
                originals.insert(
                    path.clone(),
                    std::fs::read(&path).map_err(|e| file_error(&path, e))?,
                );
            }
        }
        let mut entities = BTreeMap::new();
        let mut behaviors = BTreeMap::new();
        for (path, bytes) in &originals {
            if path.parent() == Some(assets.join("entities").as_path()) {
                let entity = ron::de::from_bytes::<EnemyDefinition>(bytes)
                    .map_err(|e| file_error(path, e))?;
                entities.insert(path.clone(), entity);
            } else if path.parent() == Some(assets.join("behaviors").as_path()) {
                let behavior = ron::de::from_bytes::<BehaviorDefinition>(bytes)
                    .map_err(|e| file_error(path, e))?;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .ok_or("invalid behavior filename")?;
                behaviors.insert(name.to_owned(), behavior);
            }
        }
        let original_names = entities
            .iter()
            .map(|(p, e)| (p.clone(), e.name.clone()))
            .collect();
        let layouts = crate::room::default_rooms()
            .iter()
            .map(|r| (r.id, crate::editor::RoomLayout::from_room(r)))
            .collect();
        let draft = Self {
            presets: crate::preset::load_presets(&assets.join("presets"))?,
            originals,
            layouts,
            entities,
            behaviors,
            original_names,
            assets: assets.to_owned(),
        };
        draft.definitions()?;
        draft.check_disk()?;
        Ok(draft)
    }

    pub fn entity_path(&self, original_name: &str) -> Option<PathBuf> {
        self.original_names
            .iter()
            .find(|(path, n)| n.as_str() == original_name && self.entities.contains_key(*path))
            .map(|(p, _)| p.clone())
            .or_else(|| {
                self.entities
                    .iter()
                    .find(|(_, e)| e.name == original_name)
                    .map(|(p, _)| p.clone())
            })
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

    /// Include original-name aliases while unsaved room placements still refer to them.
    pub fn placement_definitions(&self) -> Result<EnemyDefinitions, String> {
        let mut entities = self.entities.values().cloned().collect::<Vec<_>>();
        for (path, original) in &self.original_names {
            if !entities.iter().any(|e| e.name == *original) {
                if let Some(entity) = self.entities.get(path) {
                    let mut alias = entity.clone();
                    alias.name = original.clone();
                    entities.push(alias);
                }
            }
        }
        EnemyDefinitions::from_definitions(
            entities,
            self.behaviors
                .iter()
                .map(|(n, b)| (n.clone(), Arc::new(b.clone())))
                .collect(),
        )
    }

    pub fn rename_placements(&self, document: &mut RoomEnemies) {
        for enemy in &mut document.enemies {
            if let Some(path) = self.entity_path(&enemy.definition) {
                if let Some(entity) = self.entities.get(&path) {
                    enemy.definition = entity.name.clone();
                }
            }
        }
    }

    /// Read every room; caller documents are immutable overrides for unsaved room edits.
    pub fn plan(&self, rooms: &[(PathBuf, RoomEnemies)]) -> Result<SavePlan, String> {
        self.check_disk()?;
        let definitions = self.definitions()?;
        let mut documents = self.room_documents()?;
        for (path, document) in rooms {
            if path.parent() != Some(self.assets.join("rooms").as_path()) {
                return Err(file_error(path, "invalid room path"));
            }
            documents.insert(path.clone(), document.clone());
        }
        for file in ["first.ron", "second.ron", "boss.ron"] {
            let path = self.assets.join("rooms").join(file);
            if !documents.contains_key(&path) {
                return Err(file_error(&path, "missing required room"));
            }
        }
        let mut catalog = self.layouts.clone();
        for (path, document) in &documents {
            let id = room_id(path)?;
            if let Some(layout) = &document.layout {
                catalog.retain(|(other, _)| *other != id);
                catalog.push((id, layout.clone()));
            }
        }
        let mut candidates = Vec::new();
        for (path, entity) in &self.entities {
            if path.parent() != Some(self.assets.join("entities").as_path())
                || path.extension().is_none_or(|e| e != "ron")
            {
                return Err(file_error(path, "invalid entity path"));
            }
            candidates.push((path.clone(), serialize(entity)?));
        }
        for (name, behavior) in &self.behaviors {
            valid_filename(name)?;
            candidates.push((
                self.assets.join("behaviors").join(format!("{name}.ron")),
                serialize(behavior)?,
            ));
        }
        for (path, mut document) in documents {
            self.rename_placements(&mut document);
            let id = room_id(&path)?;
            let layout = catalog
                .iter()
                .find(|(other, _)| *other == id)
                .ok_or_else(|| file_error(&path, "room needs layout"))?
                .1
                .clone();
            crate::editor::RoomDraft {
                enemies: document.enemies.clone(),
                layout,
            }
            .validate(&definitions, &catalog)
            .map_err(|e| file_error(&path, e))?;
            candidates.push((path, serialize(&document)?));
        }
        let deletes = self
            .originals
            .keys()
            .filter(|path| {
                path.parent() != Some(self.assets.join("rooms").as_path())
                    && !candidates.iter().any(|(p, _)| p == *path)
            })
            .cloned()
            .collect();
        let mut writes = Vec::new();
        for (path, source) in candidates {
            let same = match self.originals.get(&path) {
                Some(bytes) => same_content(&path, bytes, source.as_bytes())?,
                None => false,
            };
            if !same {
                writes.push((path, source));
            }
        }
        let mut expected = self
            .originals
            .clone()
            .into_iter()
            .map(|(p, b)| (p, Some(b)))
            .collect::<BTreeMap<_, _>>();
        for (path, _) in &writes {
            expected.entry(path.clone()).or_insert(None);
        }
        Ok(SavePlan {
            assets: self.assets.clone(),
            writes,
            deletes,
            expected,
        })
    }

    fn room_documents(&self) -> Result<BTreeMap<PathBuf, RoomEnemies>, String> {
        ron_files(&self.assets.join("rooms"))?
            .into_iter()
            .map(|p| read_ron(&p).map(|d| (p, d)))
            .collect()
    }

    fn check_disk(&self) -> Result<(), String> {
        let expected = self
            .originals
            .iter()
            .map(|(p, b)| (p.clone(), Some(b.clone())))
            .collect();
        check_snapshot(&self.assets, &expected)
    }

    /// Prepare the post-save snapshot and every runtime room without writing or mutating self.
    pub fn prepare(&self, plan: &SavePlan) -> Result<(Self, Vec<crate::room::Room>), String> {
        let mut next = self.clone();
        for path in &plan.deletes {
            next.originals.remove(path);
        }
        for (path, source) in &plan.writes {
            next.originals
                .insert(path.clone(), source.as_bytes().to_vec());
        }
        next.original_names = next
            .entities
            .iter()
            .map(|(p, e)| (p.clone(), e.name.clone()))
            .collect();
        let definitions = next.definitions()?;
        let mut rooms = Vec::new();
        for (path, bytes) in &next.originals {
            if path.parent() != Some(next.assets.join("rooms").as_path()) {
                continue;
            }
            let document: RoomEnemies =
                ron::de::from_bytes(bytes).map_err(|e| file_error(path, e))?;
            let id = room_id(path)?;
            let layout = document
                .layout
                .or_else(|| {
                    next.layouts
                        .iter()
                        .find(|(other, _)| *other == id)
                        .map(|(_, l)| l.clone())
                })
                .ok_or("room needs layout")?;
            rooms.push(
                crate::editor::RoomDraft {
                    enemies: document.enemies,
                    layout,
                }
                .build(id, &definitions)?,
            );
        }
        Ok((next, rooms))
    }

    /// Save and refresh rename origins and conflict snapshots for the next edit.
    pub fn commit(&mut self, rooms: &[(PathBuf, RoomEnemies)]) -> Result<(), String> {
        let plan = self.plan(rooms)?;
        let (next, _) = self.prepare(&plan)?;
        write_files(&plan)?;
        *self = next;
        Ok(())
    }

    /// Create an independent empty behavior and entity together, or leave the draft unchanged.
    pub fn add_blank_entity(&mut self, name: &str) -> Result<PathBuf, String> {
        let behavior = (0..)
            .map(|index| format!("blank_{index}"))
            .find(|name| {
                !self.behaviors.contains_key(name)
                    && !self
                        .originals
                        .contains_key(&self.assets.join("behaviors").join(format!("{name}.ron")))
            })
            .expect("available behavior filename");
        self.add_behavior(&behavior, BehaviorDefinition::blank())?;
        let result = self.add_entity(EnemyDefinition {
            name: name.to_owned(),
            health: 1,
            size: (32.0, 40.0),
            contact_damage: 1,
            behavior: behavior.clone(),
        });
        if result.is_err() {
            self.behaviors.remove(&behavior);
        }
        result
    }

    pub fn add_entity(&mut self, mut entity: EnemyDefinition) -> Result<PathBuf, String> {
        entity.name = entity.name.trim().to_owned();
        entity.behavior = entity.behavior.trim().to_owned();
        entity.validate()?;
        if self.entities.values().any(|e| e.name == entity.name) {
            return Err("duplicate entity name".into());
        }
        if !self.behaviors.contains_key(&entity.behavior) {
            return Err("missing behavior".into());
        }
        let mut index = 0;
        let path = loop {
            let path = self
                .assets
                .join("entities")
                .join(format!("entity_{index}.ron"));
            if !self.entities.contains_key(&path) && !self.originals.contains_key(&path) {
                break path;
            }
            index += 1;
        };
        self.entities.insert(path.clone(), entity);
        Ok(path)
    }

    pub fn duplicate_entity(&mut self, path: &Path, name: &str) -> Result<PathBuf, String> {
        let mut entity = self.entities.get(path).ok_or("missing entity")?.clone();
        entity.name = name.to_owned();
        self.add_entity(entity)
    }

    pub fn delete_entity(&mut self, path: &Path) -> Result<(), String> {
        let entity = self.entities.get(path).ok_or("missing entity")?;
        let original = self.original_names.get(path);
        for (room_path, bytes) in &self.originals {
            if room_path.parent() == Some(self.assets.join("rooms").as_path()) {
                let document: RoomEnemies =
                    ron::de::from_bytes(bytes).map_err(|e| file_error(room_path, e))?;
                if document
                    .enemies
                    .iter()
                    .any(|e| e.definition == entity.name || original == Some(&e.definition))
                {
                    return Err(file_error(room_path, "entity is referenced by this room"));
                }
            }
        }
        self.entities.remove(path);
        Ok(())
    }

    pub fn add_behavior(&mut self, name: &str, behavior: BehaviorDefinition) -> Result<(), String> {
        let name = name.trim();
        valid_filename(name)?;
        if self.behaviors.contains_key(name) {
            return Err("duplicate behavior name".into());
        }
        behavior.validate()?;
        self.behaviors.insert(name.to_owned(), behavior);
        Ok(())
    }

    /// Count entity definitions, including unused entities; room placements are instances.
    pub fn behavior_users(&self, name: &str) -> usize {
        self.entities
            .values()
            .filter(|e| e.behavior == name)
            .count()
    }

    pub fn behavior_needs_warning(&self, name: &str, approved: Option<&str>) -> bool {
        self.behavior_users(name) > 1 && approved != Some(name)
    }

    pub fn assign_behavior(&mut self, entity: &Path, name: &str) -> Result<(), String> {
        if !self.behaviors.contains_key(name) {
            return Err("missing behavior".into());
        }
        self.entities
            .get_mut(entity)
            .ok_or("missing entity")?
            .behavior = name.to_owned();
        Ok(())
    }

    pub fn create_blank_behavior(
        &mut self,
        name: &str,
        assign_to: Option<&Path>,
    ) -> Result<(), String> {
        self.create_behavior(name, BehaviorDefinition::blank(), assign_to)
    }

    pub fn create_behavior_from_preset(
        &mut self,
        preset: &crate::preset::BehaviorPreset,
        name: &str,
        assign_to: Option<&Path>,
    ) -> Result<(), String> {
        preset.validate()?;
        self.create_behavior(name, preset.behavior.clone(), assign_to)
    }

    /// Copy before editing a shared behavior, optionally switching just one entity definition.
    pub fn copy_behavior(
        &mut self,
        source: &str,
        name: &str,
        assign_to: Option<&Path>,
    ) -> Result<(), String> {
        let behavior = self
            .behaviors
            .get(source)
            .ok_or("missing behavior")?
            .clone();
        self.create_behavior(name, behavior, assign_to)
    }

    fn create_behavior(
        &mut self,
        name: &str,
        behavior: BehaviorDefinition,
        assign_to: Option<&Path>,
    ) -> Result<(), String> {
        if assign_to.is_some_and(|path| !self.entities.contains_key(path)) {
            return Err("missing entity".into());
        }
        self.add_behavior(name, behavior)?;
        if let Some(path) = assign_to {
            self.assign_behavior(path, name.trim())?;
        }
        Ok(())
    }

    pub fn rename_behavior(&mut self, old: &str, name: &str) -> Result<(), String> {
        let name = name.trim();
        valid_filename(name)?;
        if !self.behaviors.contains_key(old) {
            return Err("missing behavior".into());
        }
        if old == name {
            return Ok(());
        }
        if self.behaviors.contains_key(name) {
            return Err("duplicate behavior name".into());
        }
        let behavior = self.behaviors.remove(old).ok_or("missing behavior")?;
        self.behaviors.insert(name.to_owned(), behavior);
        for entity in self.entities.values_mut() {
            if entity.behavior == old {
                entity.behavior = name.to_owned();
            }
        }
        Ok(())
    }

    pub fn delete_behavior(&mut self, name: &str) -> Result<(), String> {
        if self.entities.values().any(|e| e.behavior == name) {
            return Err("behavior is referenced by an entity".into());
        }
        self.behaviors.remove(name).ok_or("missing behavior")?;
        Ok(())
    }
}

fn valid_filename(name: &str) -> Result<(), String> {
    let base = name
        .split('.')
        .next()
        .unwrap_or_default()
        .trim_end_matches(' ')
        .to_uppercase();
    let reserved = matches!(
        base.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        base.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if name.is_empty()
        || name.ends_with(['.', ' '])
        || reserved
        || name.chars().any(|c| c < ' ' || "<>:\"/\\|?*".contains(c))
    {
        return Err("invalid behavior filename".into());
    }
    Ok(())
}

fn room_id(path: &Path) -> Result<crate::room::RoomId, String> {
    use crate::room::RoomId;
    match path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
    {
        "first" => Ok(RoomId::First),
        "second" => Ok(RoomId::Second),
        "boss" => Ok(RoomId::Boss),
        name => name
            .strip_prefix("room_")
            .and_then(|n| n.parse().ok())
            .filter(|id| name == format!("room_{id}"))
            .map(RoomId::Custom)
            .ok_or_else(|| file_error(path, "invalid room filename")),
    }
}

fn same_content(path: &Path, before: &[u8], after: &[u8]) -> Result<bool, String> {
    fn compare<T: serde::de::DeserializeOwned + PartialEq>(
        before: &[u8],
        after: &[u8],
    ) -> Result<bool, String> {
        let before: T = ron::de::from_bytes(before).map_err(|e| e.to_string())?;
        let after: T = ron::de::from_bytes(after).map_err(|e| e.to_string())?;
        Ok(before == after)
    }
    match path
        .parent()
        .and_then(Path::file_name)
        .and_then(|s| s.to_str())
    {
        Some("entities") => compare::<EnemyDefinition>(before, after),
        Some("behaviors") => compare::<BehaviorDefinition>(before, after),
        _ => compare::<RoomEnemies>(before, after),
    }
}

#[derive(Debug)]
pub struct SavePlan {
    assets: PathBuf,
    writes: Vec<(PathBuf, String)>,
    deletes: Vec<PathBuf>,
    expected: BTreeMap<PathBuf, Option<Vec<u8>>>,
}
impl std::ops::Deref for SavePlan {
    type Target = [(PathBuf, String)];
    fn deref(&self) -> &Self::Target {
        &self.writes
    }
}

fn check_snapshot(
    assets: &Path,
    expected: &BTreeMap<PathBuf, Option<Vec<u8>>>,
) -> Result<(), String> {
    check_expected(expected)?;
    for directory in ["entities", "behaviors", "rooms"] {
        for path in ron_files(&assets.join(directory))? {
            if !expected.contains_key(&path) {
                return Err(file_error(&path, "changed on disk; reload before saving"));
            }
        }
    }
    Ok(())
}

fn check_expected(expected: &BTreeMap<PathBuf, Option<Vec<u8>>>) -> Result<(), String> {
    for (path, previous) in expected {
        let current = match std::fs::read(path) {
            Ok(bytes) => Some(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(file_error(path, e)),
        };
        if &current != previous {
            return Err(file_error(path, "changed on disk; reload before saving"));
        }
    }
    Ok(())
}

/// Decode the six side-panel fields without touching the existing draft on failure.
pub fn entity_from_fields(fields: &[String; 6]) -> Result<EnemyDefinition, String> {
    let fields = fields.clone().map(|s| s.trim().to_owned());
    for (i, text) in fields.iter().enumerate() {
        validate_entity_field(i, text)?;
    }
    let entity = EnemyDefinition {
        name: fields[0].clone(),
        health: fields[1].parse().map_err(|_| "health must be 1..255")?,
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

pub fn serialize(value: &impl serde::Serialize) -> Result<String, String> {
    ron::ser::to_string_pretty(
        value,
        ron::ser::PrettyConfig::default().new_line("\n".to_owned()),
    )
    .map_err(|e| e.to_string())
}

/// Stage all files before replacing any. Roll back previous replacements on an I/O failure.
pub fn write_files(plan: &SavePlan) -> Result<(), String> {
    check_snapshot(&plan.assets, &plan.expected)?;
    let files = &plan.writes;
    use std::io::Write;
    let mut staged = Vec::new();
    let result = (|| {
        for (path, source) in files {
            let previous = match std::fs::read(path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(file_error(path, e)),
            };
            let temp = path.with_extension("ron.tmp");
            match std::fs::remove_file(&temp) {
                Ok(()) => (),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(file_error(&temp, e)),
            }
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp)
                .map_err(|e| file_error(&temp, e))?;
            staged.push((path, temp, previous));
            file.write_all(source.as_bytes())
                .map_err(|e| file_error(path, e))?;
            file.sync_all().map_err(|e| file_error(path, e))?;
        }
        check_snapshot(&plan.assets, &plan.expected)?;
        for path in &plan.deletes {
            staged.push((path, PathBuf::new(), plan.expected[path].clone()));
        }
        for (index, (path, temp, _)) in staged.iter().enumerate() {
            let result = if temp.as_os_str().is_empty() {
                std::fs::remove_file(path)
            } else {
                std::fs::rename(temp, path)
            };
            if let Err(error) = result {
                let mut errors = vec![file_error(path, error)];
                for (path, _, previous) in staged[..index].iter().rev() {
                    let rollback = match previous {
                        Some(bytes) => std::fs::write(path, bytes),
                        None => std::fs::remove_file(path),
                    };
                    if let Err(e) = rollback {
                        errors.push(file_error(path, format_args!("rollback: {e}")));
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
            action.set_parameter(i, value.trim())?;
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

pub fn validate_entity_field(index: usize, text: &str) -> Result<(), String> {
    let text = text.trim();
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

#[cfg(test)]
mod tests {
    use super::file_error;
    use std::path::{Path, MAIN_SEPARATOR};

    #[test]
    fn file_errors_use_native_separators_for_caller_paths() {
        let expected = Path::new("assets").join("rooms").join("second.ron");
        // Simulate a caller supplying mixed separators on Windows.
        let caller = expected.to_string_lossy().replacen(MAIN_SEPARATOR, "/", 1);
        let error = file_error(Path::new(&caller), "invalid patrol interval");
        let (reported, reason) = error.split_once(": ").unwrap();
        assert_eq!(Path::new(reported), expected);
        assert_eq!(Path::new(reported).file_name(), Some("second.ron".as_ref()));
        assert!(reported
            .chars()
            .filter(|c| matches!(c, '/' | '\\'))
            .all(|c| c == MAIN_SEPARATOR));
        assert_eq!(reason, "invalid patrol interval");
    }
}
