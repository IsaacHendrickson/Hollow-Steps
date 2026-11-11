//! Sound hooks: which sound each `GameEvent` plays.
//!
//! Pure rules, no audio device. `data/audio.ron` maps events to sound names (an
//! `assets/audio.ron` replaces it whole); the front end turns a name into a file in
//! `assets/audio/` and plays it. An event with no cue is silent, and a cue whose file is
//! missing plays nothing, so the game runs with no sound files at all.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::assets::read_override;
use crate::events::GameEvent;

/// Name of the optional file in the assets folder that replaces the embedded defaults.
pub const FILE_NAME: &str = "audio.ron";
/// Folder inside the assets folder that holds the sound files.
pub const SOUND_DIR: &str = "audio";
/// File extensions a sound may have, in the order they are tried.
pub const EXTENSIONS: [&str; 2] = ["wav", "ogg"];
/// Longest sound name, in characters.
pub const MAX_NAME_LEN: usize = 32;

/// One mapping: when `on` happens, play `sound` at `volume`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cue {
    pub on: GameEvent,
    /// File name without extension (lowercase letters, digits, underscores).
    pub sound: String,
    /// 0.0 (silent) to 1.0, before the master volume.
    pub volume: f32,
}

/// A sound to start now, with its final volume.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Play<'a> {
    pub sound: &'a str,
    pub volume: f32,
}

/// Sound tunables, from the embedded `data/audio.ron`.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SoundConfig {
    /// 0.0 (silent) to 1.0; multiplies every cue's volume.
    pub master_volume: f32,
    pub cues: Vec<Cue>,
}

impl Default for SoundConfig {
    fn default() -> Self {
        Self::from_ron(include_str!("../data/audio.ron")).expect("valid audio defaults")
    }
}

impl SoundConfig {
    pub fn from_ron(text: &str) -> Result<Self, String> {
        let config: Self = ron::from_str(text).map_err(|error| format!("audio: {error}"))?;
        config.validate()?;
        Ok(config)
    }

    /// The `audio.ron` in `assets` if there is one (it replaces the defaults whole),
    /// otherwise the embedded `data/audio.ron`.
    pub fn load(assets: &Path) -> Result<Self, String> {
        match read_override(assets, FILE_NAME)? {
            Some(text) => Self::from_ron(&text)
                .map_err(|error| format!("{}: {error}", assets.join(FILE_NAME).display())),
            None => Ok(Self::default()),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !is_volume(self.master_volume) {
            return Err("audio master_volume must be between 0 and 1".into());
        }
        for (index, cue) in self.cues.iter().enumerate() {
            if !is_valid_name(&cue.sound) {
                return Err(format!(
                    "audio cue for {:?}: sound name {:?} must be 1 to {MAX_NAME_LEN} lowercase \
                     letters, digits or underscores",
                    cue.on, cue.sound
                ));
            }
            if !is_volume(cue.volume) {
                return Err(format!(
                    "audio cue for {:?}: volume must be between 0 and 1",
                    cue.on
                ));
            }
            if self.cues[..index].iter().any(|other| other.on == cue.on) {
                return Err(format!("audio has more than one cue for {:?}", cue.on));
            }
        }
        Ok(())
    }

    /// The cue for `event`, if it has one. An event with no cue is silent, not an error.
    pub fn cue_for(&self, event: GameEvent) -> Option<&Cue> {
        self.cues.iter().find(|cue| cue.on == event)
    }

    /// Every distinct sound name the cues use, in the order they first appear.
    pub fn sound_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = Vec::new();
        for cue in &self.cues {
            if !names.contains(&cue.sound.as_str()) {
                names.push(&cue.sound);
            }
        }
        names
    }

    /// The sounds to start for `events`, oldest first: one per event that has a cue, at the
    /// cue's volume times the master volume.
    pub fn plan(&self, events: impl IntoIterator<Item = GameEvent>) -> Vec<Play<'_>> {
        events
            .into_iter()
            .filter_map(|event| self.cue_for(event))
            .map(|cue| Play {
                sound: &cue.sound,
                volume: (cue.volume * self.master_volume).clamp(0.0, 1.0),
            })
            .collect()
    }
}

fn is_volume(value: f32) -> bool {
    value.is_finite() && (0.0..=1.0).contains(&value)
}

/// True for 1 to `MAX_NAME_LEN` lowercase ASCII letters, digits or underscores, so a name can
/// never reach outside the sound folder.
pub fn is_valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= MAX_NAME_LEN
        && name
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// The file for sound `name` in `dir`: `<name>.wav`, else `<name>.ogg`. `None` when neither
/// exists (the sound stays silent), or when `name` is not a valid sound name.
pub fn find_sound_file(dir: &Path, name: &str) -> Option<PathBuf> {
    if !is_valid_name(name) {
        return None;
    }
    EXTENSIONS
        .iter()
        .map(|extension| dir.join(format!("{name}.{extension}")))
        .find(|path| path.is_file())
}

/// Cheap check that `bytes` start like a WAV or Ogg file, so obviously wrong files are
/// refused with a reason before the audio backend, which stops the game on data it cannot
/// read, sees them.
pub fn check_format(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.is_empty() {
        return Err("the file is empty");
    }
    let is_wav = bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE";
    let is_ogg = bytes.starts_with(b"OggS");
    if is_wav || is_ogg {
        Ok(())
    } else {
        Err("it is not a WAV or Ogg Vorbis file")
    }
}

#[cfg(test)]
mod tests;
