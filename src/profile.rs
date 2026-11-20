//! The profile: what the player keeps between runs, and the file it is saved in.
//!
//! Pure data and file handling, no window. `GameState` owns one (`game::run_link`) and saves it
//! when a run ends; `docs/run-and-profile.md` has the decisions. The file is RON, written
//! through a staged temporary file, in the user's config folder next to `keybinds.ron`:
//!
//! ```text
//! (
//!     version: 1,
//!     coins: 120,
//!     tier: 2,
//!     cleared: 2,
//! )
//! ```
//!
//! A missing file is a fresh profile. A file that cannot be used (it does not parse, has an
//! unknown version, or cannot be read) is moved to `profile.ron.bak`, replacing an older one, so
//! it is never lost or overwritten; the game starts on a fresh profile and the load says why.
//! Nothing is saved before `load` has run: `GameState` only learns the path through it.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "profile.ron";
/// The file format this game reads and writes.
pub const VERSION: u32 = 1;

/// What is kept forever: coins banked, the difficulty tier and how many dungeons were cleared.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Profile {
    pub coins: u32,
    pub tier: u32,
    pub cleared: u32,
}

/// The file's fields. Unknown fields are refused, so a typo is found instead of ignored (the bad
/// file is kept as `.bak`).
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    version: u32,
    coins: u32,
    tier: u32,
    cleared: u32,
}

/// Just the version, read first so a file from a newer game says "unknown version" rather than
/// "unknown field".
#[derive(Deserialize)]
struct Header {
    version: u32,
}

impl Profile {
    /// The file text, in the pretty format the editors use.
    pub fn to_ron(&self) -> Result<String, String> {
        crate::definition_editor::serialize(&File {
            version: VERSION,
            coins: self.coins,
            tier: self.tier,
            cleared: self.cleared,
        })
    }

    /// Reads the file text. An unknown version is an error, whatever else the text holds.
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let header: Header = ron::from_str(text).map_err(|error| error.to_string())?;
        if header.version != VERSION {
            return Err(format!(
                "unknown profile version {} (this game reads version {VERSION})",
                header.version
            ));
        }
        let file: File = ron::from_str(text).map_err(|error| error.to_string())?;
        Ok(Self {
            coins: file.coins,
            tier: file.tier,
            cleared: file.cleared,
        })
    }
}

fn describe(path: &Path, error: impl std::fmt::Display) -> String {
    format!("{}: {error}", path.display())
}

/// Where a file that cannot be used is kept: `profile.ron.bak` beside it.
pub fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("ron.bak")
}

/// Moves the unusable file at `path` to its backup name and builds the message for the screen.
fn set_aside(path: &Path, reason: &str) -> String {
    let backup = backup_path(path);
    match std::fs::rename(path, &backup) {
        Ok(()) => format!(
            "Profile not loaded, starting fresh: {reason}. The old file is kept as {}.",
            backup.display()
        ),
        Err(error) => format!(
            "Profile not loaded, starting fresh: {reason}. It could not be moved aside: {}.",
            describe(&backup, error)
        ),
    }
}

/// Reads the profile at `path`. A missing file gives a fresh profile and no message. A file that
/// cannot be used is renamed to `profile.ron.bak` (replacing an older backup) and gives a fresh
/// profile and the reason, for the front end to show.
pub fn load(path: &Path) -> (Profile, Option<String>) {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return (Profile::default(), None);
        }
        Err(error) => {
            return (
                Profile::default(),
                Some(set_aside(path, &describe(path, error))),
            );
        }
    };
    match Profile::from_ron(&text) {
        Ok(profile) => (profile, None),
        Err(error) => (
            Profile::default(),
            Some(set_aside(path, &describe(path, error))),
        ),
    }
}

/// Writes the profile to `path`, creating its folder, through a temporary file moved into place
/// so a failed write leaves the previous file untouched.
pub fn save(path: &Path, profile: &Profile) -> Result<(), String> {
    let text = profile.to_ron()?;
    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        std::fs::create_dir_all(folder).map_err(|error| describe(folder, error))?;
    }
    crate::tuning::write_staged(path, &text)
}

/// Where this computer keeps the profile: `profile.ron` in the same config folder as the
/// keybinds (`keybinds::default_dir`, so `HOLLOW_STEPS_CONFIG` applies), or `None` when there is
/// no config folder.
pub fn default_path() -> Option<PathBuf> {
    crate::keybinds::default_dir().map(|folder| folder.join(FILE_NAME))
}

#[cfg(test)]
mod tests;
