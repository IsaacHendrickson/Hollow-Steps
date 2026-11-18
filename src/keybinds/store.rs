//! Keeping the bindings between runs: the file format and where the file lives.
//!
//! The file is RON, one list of key names per action (primary first, then the alternate):
//!
//! ```text
//! (
//!     bindings: {
//!         "jump": ["Space", "Z"],
//!     },
//! )
//! ```
//!
//! An action missing from the file keeps its shipped keys. A file that does not parse or breaks a
//! rule (unknown action or key, no key, more than two, a key used twice) is refused with a
//! message and the game uses the shipped keys. The file lives in the user's config folder, not in
//! `assets/`, so it is never part of the repository; see `docs/keybinds.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use macroquad::input::KeyCode;
use serde::{Deserialize, Serialize};

use super::{key_from_name, key_name, Action, Keybinds, Slot};

pub const FILE_NAME: &str = "keybinds.ron";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    bindings: BTreeMap<String, Vec<String>>,
}

impl Keybinds {
    /// The file text for these bindings: every action, so the file is easy to read and edit.
    pub fn to_ron(&self) -> Result<String, String> {
        self.check()?;
        let bindings = Action::ALL
            .into_iter()
            .map(|action| {
                let names = self
                    .keys(action)
                    .filter_map(|key| key_name(key).map(str::to_owned))
                    .collect();
                (action.name().to_owned(), names)
            })
            .collect();
        crate::definition_editor::serialize(&File { bindings })
    }

    /// Reads the file text. Actions the text leaves out keep their shipped keys.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let file: File = ron::from_str(text).map_err(|error| error.to_string())?;
        let mut binds = Self::default();
        for (name, key_names) in &file.bindings {
            let action = Action::from_name(name).ok_or_else(|| {
                let known = Action::ALL.map(Action::name).join(", ");
                format!("unknown action \"{name}\" (known actions: {known})")
            })?;
            if key_names.is_empty() || key_names.len() > Slot::ALL.len() {
                return Err(format!(
                    "{name}: list one or two keys (primary, then alternate), not {}",
                    key_names.len()
                ));
            }
            let mut keys: [Option<KeyCode>; 2] = [None; 2];
            for (slot, key_name) in key_names.iter().enumerate() {
                keys[slot] = Some(
                    key_from_name(key_name)
                        .ok_or_else(|| format!("{name}: unknown key \"{key_name}\""))?,
                );
            }
            binds.set_action(action, keys);
        }
        binds.check()?;
        Ok(binds)
    }
}

fn describe(path: &Path, error: impl std::fmt::Display) -> String {
    format!("{}: {error}", path.display())
}

/// Reads the bindings at `path`. A missing file is not an error: it means the shipped keys.
pub fn load(path: &Path) -> Result<Keybinds, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Keybinds::from_ron(&text).map_err(|error| describe(path, error)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Keybinds::default()),
        Err(error) => Err(describe(path, error)),
    }
}

/// Writes the bindings to `path`, creating its folder, through a temporary file moved into
/// place so a failed write leaves the previous file untouched.
pub fn save(path: &Path, binds: &Keybinds) -> Result<(), String> {
    let text = binds.to_ron()?;
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        std::fs::create_dir_all(folder).map_err(|error| describe(folder, error))?;
    }
    crate::tuning::write_staged(path, &text)
}

/// Deletes the file at `path`; a file that is not there is already gone. Resetting the keybinds
/// does this, so the shipped keys apply again, whatever a later version ships.
pub fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(describe(path, error)),
    }
}

/// The operating systems whose config folder differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Mac,
    Other,
}

/// The folder for the game's per-user settings, from environment variable lookups (`var`).
/// `HOLLOW_STEPS_CONFIG` overrides everything; otherwise Windows uses `%APPDATA%\HollowSteps`,
/// macOS `~/Library/Application Support/HollowSteps` and the rest `$XDG_CONFIG_HOME/hollow-steps`
/// or `~/.config/hollow-steps`. `None` when no home folder can be found.
pub fn config_dir(platform: Platform, var: impl Fn(&str) -> Option<String>) -> Option<PathBuf> {
    let get = |name: &str| {
        var(name)
            .filter(|value| !value.trim().is_empty())
            .map(PathBuf::from)
    };
    if let Some(folder) = get("HOLLOW_STEPS_CONFIG") {
        return Some(folder);
    }
    match platform {
        Platform::Windows => get("APPDATA").map(|folder| folder.join("HollowSteps")),
        Platform::Mac => get("HOME").map(|home| {
            home.join("Library")
                .join("Application Support")
                .join("HollowSteps")
        }),
        Platform::Other => get("XDG_CONFIG_HOME")
            .map(|folder| folder.join("hollow-steps"))
            .or_else(|| get("HOME").map(|home| home.join(".config").join("hollow-steps"))),
    }
}

/// Where this computer keeps the keybinds file, or `None` when there is no config folder.
pub fn default_path() -> Option<PathBuf> {
    let platform = if cfg!(windows) {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Mac
    } else {
        Platform::Other
    };
    config_dir(platform, |name| std::env::var(name).ok()).map(|folder| folder.join(FILE_NAME))
}
