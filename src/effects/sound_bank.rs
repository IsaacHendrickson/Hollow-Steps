//! Loads sound files and plays them with Macroquad's audio.
//!
//! With the `audio` cargo feature this plays real sound. Without it the bank is empty and
//! plays nothing (Macroquad's stub would print a warning on every sound), except for one
//! line at load time if sound files are present, so a missing feature is never a mystery.

use std::path::Path;

use hollow_steps::sound::{self, Play, SoundConfig};

#[cfg(feature = "audio")]
pub use with_audio::SoundBank;
#[cfg(not(feature = "audio"))]
pub use without_audio::SoundBank;

#[cfg(feature = "audio")]
mod with_audio {
    use std::collections::HashMap;
    use std::future::Future;
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::task::{Context, Poll, Waker};

    use macroquad::audio::{load_sound_from_bytes, play_sound, PlaySoundParams, Sound};

    use super::*;

    /// The sounds that were found and loaded, by sound name.
    #[derive(Default)]
    pub struct SoundBank {
        sounds: HashMap<String, Sound>,
    }

    impl SoundBank {
        /// Loads every sound `config` names from `dir`. A name with no file is skipped
        /// silently; a file that cannot be used is skipped with a line on stderr saying why.
        pub fn load(config: &SoundConfig, dir: &Path) -> Self {
            let mut sounds = HashMap::new();
            for name in config.sound_names() {
                let Some(path) = sound::find_sound_file(dir, name) else {
                    continue;
                };
                let loaded = std::fs::read(&path)
                    .map_err(|error| error.to_string())
                    .and_then(|bytes| {
                        sound::check_format(&bytes).map_err(str::to_owned)?;
                        load_guarded(&bytes)
                    });
                match loaded {
                    Ok(sound) => {
                        sounds.insert(name.to_owned(), sound);
                    }
                    Err(reason) => eprintln!("sound {name}: {}: {reason}", path.display()),
                }
            }
            Self { sounds }
        }

        /// Starts each sound in `plays` that was loaded.
        pub fn play(&self, plays: &[Play<'_>]) {
            for play in plays {
                if let Some(sound) = self.sounds.get(play.sound) {
                    play_sound(
                        sound,
                        PlaySoundParams {
                            looped: false,
                            volume: play.volume,
                        },
                    );
                }
            }
        }
    }

    /// Loads one sound without letting a file the backend cannot decode stop the game: the
    /// backend panics on data it cannot read, so the call is made under `catch_unwind`.
    /// Loading finishes on its first poll on desktop, so no executor is needed.
    fn load_guarded(bytes: &[u8]) -> Result<Sound, String> {
        let mut load = std::pin::pin!(load_sound_from_bytes(bytes));
        let mut context = Context::from_waker(Waker::noop());
        match catch_unwind(AssertUnwindSafe(|| load.as_mut().poll(&mut context))) {
            Ok(Poll::Ready(Ok(sound))) => Ok(sound),
            Ok(Poll::Ready(Err(error))) => Err(error.to_string()),
            Ok(Poll::Pending) => Err("the audio backend was not ready".to_owned()),
            Err(_) => Err(
                "the audio backend could not decode it (use 16-bit PCM WAV or Ogg Vorbis, \
                 mono or stereo)"
                    .to_owned(),
            ),
        }
    }
}

#[cfg(not(feature = "audio"))]
mod without_audio {
    use super::*;

    /// Always empty: this build has no audio.
    #[derive(Default)]
    pub struct SoundBank;

    impl SoundBank {
        pub fn load(config: &SoundConfig, dir: &Path) -> Self {
            if config
                .sound_names()
                .into_iter()
                .any(|name| sound::find_sound_file(dir, name).is_some())
            {
                eprintln!(
                    "sound files found in {}, but this build has no audio: \
                     run with `cargo run --features audio`",
                    dir.display()
                );
            }
            Self
        }

        pub fn play(&self, _plays: &[Play<'_>]) {}
    }
}
