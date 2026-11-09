//! Validated editor templates; instantiated behaviors keep the existing file format.
use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::behavior::{read_ron, ron_files, BehaviorDefinition};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BehaviorPreset {
    pub name: String,
    pub description: String,
    pub behavior: BehaviorDefinition,
}

impl BehaviorPreset {
    pub fn from_ron(source: &str) -> Result<Self, String> {
        let preset: Self = ron::from_str(source).map_err(|e| e.to_string())?;
        preset.validate()?;
        Ok(preset)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("preset name must not be empty".into());
        }
        if self.description.trim().is_empty() {
            return Err(format!(
                "preset {:?}: description must not be empty",
                self.name
            ));
        }
        self.behavior
            .validate()
            .map_err(|e| format!("preset {:?}: {e}", self.name))
    }
}

/// A missing directory is an empty catalog, for older asset folders.
/// Any invalid file rejects the entire catalog and identifies its source path.
pub fn load_presets(directory: &Path) -> Result<Vec<BehaviorPreset>, String> {
    if !directory
        .try_exists()
        .map_err(|e| format!("{}: {e}", directory.display()))?
    {
        return Ok(Vec::new());
    }
    let mut presets = Vec::new();
    let mut names = BTreeSet::new();
    for path in ron_files(directory)? {
        let preset: BehaviorPreset = read_ron(&path)?;
        preset
            .validate()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if !names.insert(preset.name.trim().to_lowercase()) {
            return Err(format!(
                "{}: duplicate preset name {:?}",
                path.display(),
                preset.name
            ));
        }
        presets.push(preset);
    }
    presets.sort_by_key(|p| p.name.to_lowercase());
    Ok(presets)
}
