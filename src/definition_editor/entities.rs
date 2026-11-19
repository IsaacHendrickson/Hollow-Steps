//! Entity and behavior edits on the draft, and the entity field decoding the side panel uses.
use std::path::{Path, PathBuf};

use super::draft::DefinitionDraft;
use super::plan::file_error;
use crate::behavior::BehaviorDefinition;
use crate::enemy::EnemyDefinition;
use crate::room::RoomEnemies;

impl DefinitionDraft {
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
            health: 10,
            size: (32.0, 40.0),
            visual_size: None,
            contact_damage: 10,
            behavior: behavior.clone(),
            body: None,
            death_secs: crate::enemy::DEFAULT_DEATH_SECS,
            movement: Default::default(),
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

pub(super) fn valid_filename(name: &str) -> Result<(), String> {
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

/// Decode the six side-panel fields without touching the existing draft on failure.
pub fn entity_from_fields(fields: &[String; 6]) -> Result<EnemyDefinition, String> {
    let fields = fields.clone().map(|s| s.trim().to_owned());
    for (i, text) in fields.iter().enumerate() {
        validate_entity_field(i, text)?;
    }
    let entity = EnemyDefinition {
        name: fields[0].clone(),
        health: fields[1].parse().map_err(|_| "health must be 1..65535")?,
        size: (
            fields[2].parse().map_err(|_| "invalid width")?,
            fields[3].parse().map_err(|_| "invalid height")?,
        ),
        visual_size: None,
        contact_damage: fields[4]
            .parse()
            .map_err(|_| "contact damage must be 0..65535")?,
        behavior: fields[5].clone(),
        body: None,
        death_secs: crate::enemy::DEFAULT_DEATH_SECS,
        movement: Default::default(),
    };
    entity.validate()?;
    Ok(entity)
}

/// Apply entity fields and body settings together so ordinary edits retain the visual.
/// The death time is the default; `entity_with_body_and_death` carries an edited one.
pub fn entity_with_body(
    fields: &[String; 6],
    body: Option<crate::body::BodyVisual>,
) -> Result<EnemyDefinition, String> {
    entity_with_body_and_death(fields, body, crate::enemy::DEFAULT_DEATH_SECS)
}

/// `entity_with_body` with the death time (a drag box in the panel, not a text field), so an
/// ordinary edit keeps the value the author set.
pub fn entity_with_body_and_death(
    fields: &[String; 6],
    body: Option<crate::body::BodyVisual>,
    death_secs: f32,
) -> Result<EnemyDefinition, String> {
    entity_with_body_death_and_movement(fields, body, death_secs, Default::default())
}

/// `entity_with_body_and_death` with the movement (a picker and drag boxes in the panel), so an
/// ordinary edit keeps a flier flying.
pub fn entity_with_body_death_and_movement(
    fields: &[String; 6],
    body: Option<crate::body::BodyVisual>,
    death_secs: f32,
    movement: crate::enemy::Movement,
) -> Result<EnemyDefinition, String> {
    let mut entity = entity_from_fields(fields)?;
    entity.body = body;
    entity.death_secs = death_secs;
    entity.movement = movement;
    entity.validate()?;
    Ok(entity)
}

pub fn validate_entity_field(index: usize, text: &str) -> Result<(), String> {
    let text = text.trim();
    match index {
        0 | 5 if text.trim().is_empty() => Err("must not be empty".into()),
        1 => match text.parse::<u16>() {
            Ok(1..=65535) => Ok(()),
            _ => Err("expected integer 1..65535".into()),
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
            .parse::<u16>()
            .map(|_| ())
            .map_err(|_| "expected integer 0..65535".into()),
        _ => Ok(()),
    }
}
