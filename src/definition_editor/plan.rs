//! Save planning and staged writes: nothing is written until every file is validated and staged.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::draft::DefinitionDraft;
use super::entities::valid_filename;
use crate::behavior::{read_ron, ron_files, BehaviorDefinition};
use crate::boss_definition::BossDefinition;
use crate::enemy::EnemyDefinition;
use crate::room::RoomEnemies;

/// Use native separators even when a caller supplies a path with mixed separators.
pub(super) fn file_error(path: &Path, error: impl std::fmt::Display) -> String {
    let path: PathBuf = path.components().collect();
    format!("{}: {error}", path.display())
}

impl DefinitionDraft {
    /// Read every room; caller documents are immutable overrides for unsaved room edits.
    pub fn plan(&self, rooms: &[(PathBuf, RoomEnemies)]) -> Result<SavePlan, String> {
        self.check_disk()?;
        let definitions = self.definitions()?;
        let bosses = self.boss_definitions()?;
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
        for (path, boss) in &self.bosses {
            if path.parent() != Some(self.assets.join("bosses").as_path())
                || path.extension().is_none_or(|e| e != "ron")
            {
                return Err(file_error(path, "invalid boss path"));
            }
            candidates.push((path.clone(), serialize(boss)?));
        }
        for (name, behavior) in &self.behaviors {
            valid_filename(name)?;
            candidates.push((
                self.assets.join("behaviors").join(format!("{name}.ron")),
                serialize(behavior)?,
            ));
        }
        for (name, pose) in &self.poses {
            valid_filename(name)?;
            candidates.push((
                self.assets.join("poses").join(format!("{name}.ron")),
                serialize(pose.as_ref())?,
            ));
        }
        for (name, body) in &self.bodies {
            valid_filename(name)?;
            candidates.push((
                self.assets.join("bodies").join(format!("{name}.ron")),
                serialize(body.as_ref())?,
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
            let draft = crate::editor::RoomDraft {
                enemies: document.enemies.clone(),
                boss: document.boss.clone(),
                layout,
            };
            draft
                .validate(&definitions, &catalog)
                .map_err(|e| file_error(&path, e))?;
            draft
                .validate_boss(&bosses)
                .map_err(|e| file_error(&path, e))?;
            candidates.push((path, serialize(&document)?));
        }
        let deletes = self
            .originals
            .keys()
            .filter(|path| {
                path.parent() != Some(self.assets.join("rooms").as_path())
                    && path.parent() != Some(self.assets.join("bodies").as_path())
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

    pub(super) fn check_disk(&self) -> Result<(), String> {
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
        next.original_boss_names = next
            .bosses
            .iter()
            .map(|(p, b)| (p.clone(), b.name.clone()))
            .collect();
        let definitions = next.definitions()?;
        let mut rooms = Vec::new();
        let bosses = next.boss_definitions()?;
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
            let placement = document.boss.clone();
            let mut room = crate::editor::RoomDraft {
                enemies: document.enemies,
                boss: document.boss.clone(),
                layout,
            }
            .build(id, &definitions)?;
            crate::room::place_boss(&mut room, placement.as_ref(), &bosses)
                .map_err(|e| file_error(path, e))?;
            rooms.push(room);
        }
        // `originals` is in file-name order as text; rooms are listed by id, as at start-up.
        rooms.sort_by_key(|room| room.id.index());
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
        Some("bosses") => compare::<BossDefinition>(before, after),
        Some("bodies") => compare::<crate::body::BodyTemplate>(before, after),
        Some("poses") => compare::<crate::pose::Pose>(before, after),
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
    for directory in [
        "entities",
        "behaviors",
        "rooms",
        "bosses",
        "bodies",
        "poses",
    ] {
        if matches!(directory, "bosses" | "bodies" | "poses") && !assets.join(directory).exists() {
            continue;
        }
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
