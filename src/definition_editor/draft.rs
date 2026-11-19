//! The editable draft of every definition file, its load, and read-only queries.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::plan::file_error;
use crate::behavior::{ron_files, BehaviorDefinition};
use crate::body::{load_body_templates, BodyTemplates};
use crate::boss_definition::{BossDefinition, BossDefinitions};
use crate::enemy::{EnemyDefinition, EnemyDefinitions};
use crate::room::RoomEnemies;

#[derive(Clone, Debug)]
pub struct DefinitionDraft {
    /// Keys are stable filenames, independent of editable entity names.
    pub entities: BTreeMap<PathBuf, EnemyDefinition>,
    pub behaviors: BTreeMap<String, BehaviorDefinition>,
    pub bodies: BodyTemplates,
    pub poses: crate::pose::Poses,
    pub bosses: BTreeMap<PathBuf, BossDefinition>,
    pub(super) original_boss_names: BTreeMap<PathBuf, String>,
    pub presets: Vec<crate::preset::BehaviorPreset>,
    pub(super) original_names: BTreeMap<PathBuf, String>,
    pub(super) assets: PathBuf,
    pub(super) originals: BTreeMap<PathBuf, Vec<u8>>,
    pub(super) layouts: Vec<(crate::room::RoomId, crate::editor::RoomLayout)>,
}

impl DefinitionDraft {
    pub fn load(assets: &Path) -> Result<Self, String> {
        let assets: PathBuf = assets.components().collect();
        let assets = assets.as_path();
        let mut originals = BTreeMap::new();
        for directory in [
            "entities",
            "behaviors",
            "rooms",
            "bosses",
            "bodies",
            "poses",
        ] {
            if matches!(directory, "bosses" | "bodies" | "poses")
                && !assets.join(directory).exists()
            {
                continue;
            }
            for path in ron_files(&assets.join(directory))? {
                originals.insert(
                    path.clone(),
                    std::fs::read(&path).map_err(|e| file_error(&path, e))?,
                );
            }
        }
        let mut entities = BTreeMap::new();
        let mut behaviors = BTreeMap::new();
        let mut bosses = BTreeMap::new();
        for (path, bytes) in &originals {
            if path.parent() == Some(assets.join("entities").as_path()) {
                let entity = ron::de::from_bytes::<EnemyDefinition>(bytes)
                    .map_err(|e| file_error(path, e))?;
                entities.insert(path.clone(), entity);
            } else if path.parent() == Some(assets.join("bosses").as_path()) {
                let boss = ron::de::from_bytes::<BossDefinition>(bytes)
                    .map_err(|e| file_error(path, e))?;
                bosses.insert(path.clone(), boss);
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
            bodies: load_body_templates(&assets.join("bodies"))?,
            poses: crate::pose::load_poses(&assets.join("poses"))?,
            presets: crate::preset::load_presets(&assets.join("presets"))?,
            originals,
            layouts,
            entities,
            behaviors,
            original_names,
            original_boss_names: bosses
                .iter()
                .map(|(p, b)| (p.clone(), b.name.clone()))
                .collect(),
            bosses,
            assets: assets.to_owned(),
        };
        draft.definitions()?;
        draft.boss_definitions()?;
        draft.check_disk()?;
        Ok(draft)
    }

    pub fn create_pose(&mut self, name: &str) -> Result<(), String> {
        let name = name.trim();
        crate::pose::validate_name(name)?;
        if self.poses.contains_key(name) {
            return Err(format!("pose {name:?} already exists"));
        }
        self.poses.insert(
            name.into(),
            Arc::new(crate::pose::Pose {
                offsets: crate::pose::SEMANTIC_POINTS
                    .iter()
                    .map(|name| ((*name).into(), (0.0, 0.0)))
                    .collect(),
            }),
        );
        Ok(())
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

    pub fn boss_path(&self, name: &str) -> Option<PathBuf> {
        self.original_boss_names
            .iter()
            .find(|(p, n)| n.as_str() == name && self.bosses.contains_key(*p))
            .map(|(p, _)| p.clone())
            .or_else(|| {
                self.bosses
                    .iter()
                    .find(|(_, b)| b.name == name)
                    .map(|(p, _)| p.clone())
            })
    }

    pub fn boss_definitions(&self) -> Result<BossDefinitions, String> {
        BossDefinitions::from_definitions_with_assets(
            self.bosses.values().cloned(),
            self.bodies.clone(),
            self.poses.clone(),
        )
    }

    /// One catalog shared by the picker and canvas placement, with explicit entity types.
    pub fn placement_catalog(&self) -> Vec<(String, crate::editor::Placement)> {
        use crate::editor::Placement;
        self.entities
            .values()
            .map(|e| (e.name.clone(), Placement::Enemy(e.name.clone())))
            .chain(
                self.bosses
                    .values()
                    .map(|b| (format!("Boss: {}", b.name), Placement::Boss(b.name.clone()))),
            )
            .chain(
                crate::item::Item::ALL
                    .iter()
                    .map(|i| (format!("Item: {}", i.name()), Placement::Item(*i))),
            )
            .collect()
    }

    pub fn definitions(&self) -> Result<EnemyDefinitions, String> {
        for (name, behavior) in &self.behaviors {
            behavior
                .validate()
                .and_then(|_| crate::pose::validate_behavior(behavior, &self.poses))
                .map_err(|error| {
                    file_error(
                        &self.assets.join("behaviors").join(format!("{name}.ron")),
                        error,
                    )
                })?;
        }
        for (path, entity) in &self.entities {
            if let Some(behavior) = self.behaviors.get(&entity.behavior) {
                crate::pose::validate_entity(entity, behavior, &self.poses)
                    .map_err(|e| file_error(path, e))?;
            }
        }
        EnemyDefinitions::from_definitions_with_poses(
            self.entities.values().cloned(),
            self.behaviors
                .iter()
                .map(|(n, b)| (n.clone(), Arc::new(b.clone())))
                .collect(),
            self.bodies.clone(),
            self.poses.clone(),
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
        EnemyDefinitions::from_definitions_with_poses(
            entities,
            self.behaviors
                .iter()
                .map(|(n, b)| (n.clone(), Arc::new(b.clone())))
                .collect(),
            self.bodies.clone(),
            self.poses.clone(),
        )
    }

    pub fn rename_placements(&self, document: &mut RoomEnemies) {
        if let Some(placement) = &mut document.boss {
            if let Some(boss) = self
                .boss_path(&placement.definition)
                .and_then(|p| self.bosses.get(&p))
            {
                placement.definition = boss.name.clone();
            }
        }
        for enemy in &mut document.enemies {
            if let Some(path) = self.entity_path(&enemy.definition) {
                if let Some(entity) = self.entities.get(&path) {
                    enemy.definition = entity.name.clone();
                }
            }
        }
    }

    /// Compare authored boss content against the load snapshot, preserving untouched encounters.
    pub fn boss_changed(
        &self,
        id: crate::room::RoomId,
        edited: Option<&RoomEnemies>,
    ) -> Result<bool, String> {
        let path = self
            .assets
            .join("rooms")
            .join(crate::editor::room_filename(id));
        let before: RoomEnemies = match self.originals.get(&path) {
            Some(bytes) => ron::de::from_bytes(bytes).map_err(|e| file_error(&path, e))?,
            None => return Ok(true),
        };
        let mut after = edited.unwrap_or(&before).clone();
        self.rename_placements(&mut after);
        if before.boss != after.boss {
            return Ok(true);
        }
        let Some(placement) = after.boss else {
            return Ok(false);
        };
        let path = self
            .boss_path(&placement.definition)
            .ok_or("missing boss definition")?;
        let Some(bytes) = self.originals.get(&path) else {
            return Ok(true);
        };
        let original: BossDefinition =
            ron::de::from_bytes(bytes).map_err(|e| file_error(&path, e))?;
        Ok(self.bosses[&path] != original)
    }
}
