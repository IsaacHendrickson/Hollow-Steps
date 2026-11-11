use super::*;

const FIXTURE: &str = r#"(
    master_volume: 0.5,
    cues: [
        (on: PlayerJumped, sound: "jump", volume: 1.0),
        (on: EnemyHit, sound: "hit", volume: 0.8),
        (on: EnemyDied, sound: "hit", volume: 0.4),
        (on: CoinCollected, sound: "coin_2", volume: 0.0),
    ],
)"#;

fn fixture() -> SoundConfig {
    SoundConfig::from_ron(FIXTURE).expect("fixture parses")
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-sound-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn touch(&self, name: &str) {
        std::fs::write(self.0.join(name), b"x").unwrap();
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

// ---- shipped defaults -----------------------------------------------------------------

#[test]
fn shipped_defaults_validate() {
    let config = SoundConfig::default();
    config.validate().unwrap();
    assert!(!config.cues.is_empty());
}

// ---- event to sound -------------------------------------------------------------------

#[test]
fn an_event_maps_to_its_sound_name_and_an_unmapped_event_to_nothing() {
    let config = fixture();
    let cue = config.cue_for(GameEvent::PlayerJumped).unwrap();
    assert_eq!(cue.sound, "jump");
    assert_eq!(cue.volume, 1.0);
    // A missing mapping is fine: the event is simply silent.
    assert!(config.cue_for(GameEvent::RoomEntered).is_none());
    assert!(config.cue_for(GameEvent::PlayerDamaged).is_none());
}

#[test]
fn a_config_with_no_cues_is_valid_and_silent() {
    let config = SoundConfig::from_ron("(master_volume: 1.0, cues: [])").unwrap();
    assert!(config.cue_for(GameEvent::PlayerJumped).is_none());
    assert!(config
        .plan([GameEvent::PlayerJumped, GameEvent::EnemyHit])
        .is_empty());
}

#[test]
fn plan_lists_one_sound_per_mapped_event_in_order_at_cue_times_master_volume() {
    let config = fixture();
    let plan = config.plan([
        GameEvent::EnemyHit,
        GameEvent::RoomEntered, // no cue: skipped
        GameEvent::PlayerJumped,
        GameEvent::EnemyHit, // a repeat plays again
        GameEvent::EnemyDied,
    ]);
    assert_eq!(
        plan,
        [
            Play {
                sound: "hit",
                volume: 0.4
            },
            Play {
                sound: "jump",
                volume: 0.5
            },
            Play {
                sound: "hit",
                volume: 0.4
            },
            Play {
                sound: "hit",
                volume: 0.2
            },
        ]
    );
    assert!(config.plan([]).is_empty());
}

#[test]
fn the_final_volume_always_stays_between_silent_and_full() {
    let loud = SoundConfig::from_ron(
        r#"(master_volume: 1.0, cues: [(on: EnemyHit, sound: "hit", volume: 1.0)])"#,
    )
    .unwrap();
    assert_eq!(loud.plan([GameEvent::EnemyHit])[0].volume, 1.0);
    let muted = SoundConfig::from_ron(
        r#"(master_volume: 0.0, cues: [(on: EnemyHit, sound: "hit", volume: 1.0)])"#,
    )
    .unwrap();
    assert_eq!(muted.plan([GameEvent::EnemyHit])[0].volume, 0.0);
}

#[test]
fn sound_names_are_listed_once_in_first_use_order() {
    assert_eq!(fixture().sound_names(), ["jump", "hit", "coin_2"]);
}

// ---- validation -----------------------------------------------------------------------

fn with(change: impl FnOnce(&mut SoundConfig)) -> Result<(), String> {
    let mut config = fixture();
    change(&mut config);
    config.validate()
}

#[test]
fn volumes_must_be_between_zero_and_one() {
    assert!(with(|_| {}).is_ok());
    for bad in [-0.1, 1.1, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(
            with(|c| c.master_volume = bad)
                .unwrap_err()
                .contains("master_volume"),
            "master {bad}"
        );
        assert!(
            with(|c| c.cues[0].volume = bad)
                .unwrap_err()
                .contains("volume"),
            "cue {bad}"
        );
    }
    // The ends of the range are fine.
    assert!(with(|c| c.master_volume = 0.0).is_ok());
    assert!(with(|c| c.cues[0].volume = 1.0).is_ok());
}

#[test]
fn sound_names_must_be_simple() {
    for good in [
        "jump",
        "a",
        "enemy_die",
        "coin_2",
        &"x".repeat(MAX_NAME_LEN),
    ] {
        assert!(is_valid_name(good), "{good}");
    }
    let too_long = "x".repeat(MAX_NAME_LEN + 1);
    for bad in [
        "",
        "Jump",
        "has space",
        "dot.wav",
        "../escape",
        "a/b",
        "a\\b",
        "..",
        "naïve",
        &too_long,
    ] {
        assert!(!is_valid_name(bad), "{bad:?}");
        assert!(
            with(|c| c.cues[0].sound = bad.to_string())
                .unwrap_err()
                .contains("sound name"),
            "{bad:?}"
        );
    }
}

#[test]
fn an_event_may_have_only_one_cue() {
    let error = with(|c| c.cues[1].on = GameEvent::PlayerJumped).unwrap_err();
    assert!(error.contains("PlayerJumped"), "{error}");
    // Two events may share a sound.
    assert!(with(|_| {}).is_ok());
}

#[test]
fn bad_files_are_rejected() {
    for text in [
        "(master_volume: 1.0)", // no cues
        "(cues: [])",           // no master
        r#"(master_volume: 1.0, cues: [(on: EnemyHitt, sound: "a", volume: 1.0)])"#, // event
        r#"(master_volume: 1.0, cues: [(on: EnemyHit, sound: "a")])"#, // volume
        r#"(master_volume: 1.0, cues: [], extra: 1)"#, // field
        r#"(master_volume: 1.0, cues: [(on: EnemyHit, sound: "a", volume: 1.0, loop: true)])"#,
    ] {
        assert!(SoundConfig::from_ron(text).is_err(), "{text}");
    }
    // A well-formed file with an out-of-range number fails validation.
    let error = SoundConfig::from_ron(
        r#"(master_volume: 2.0, cues: [(on: EnemyHit, sound: "a", volume: 1.0)])"#,
    )
    .unwrap_err();
    assert!(error.contains("master_volume"), "{error}");
}

// ---- loading --------------------------------------------------------------------------

#[test]
fn load_uses_the_defaults_without_a_file_and_replaces_them_whole_with_one() {
    let dir = TempDir::new();
    assert_eq!(SoundConfig::load(&dir.0).unwrap(), SoundConfig::default());

    std::fs::write(dir.0.join(FILE_NAME), FIXTURE).unwrap();
    assert_eq!(SoundConfig::load(&dir.0).unwrap(), fixture());
}

#[test]
fn load_names_the_file_when_it_is_invalid_instead_of_falling_back() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(master_volume: 9.0, cues: [])").unwrap();
    let error = SoundConfig::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
}

// ---- finding files --------------------------------------------------------------------

#[test]
fn a_sound_file_is_found_by_name_with_either_extension_and_wav_wins() {
    let dir = TempDir::new();
    assert_eq!(find_sound_file(&dir.0, "jump"), None, "no file, no sound");

    dir.touch("jump.ogg");
    assert_eq!(
        find_sound_file(&dir.0, "jump"),
        Some(dir.0.join("jump.ogg"))
    );

    dir.touch("jump.wav");
    assert_eq!(
        find_sound_file(&dir.0, "jump"),
        Some(dir.0.join("jump.wav"))
    );

    // Other names and other extensions do not match.
    dir.touch("land.mp3");
    assert_eq!(find_sound_file(&dir.0, "land"), None);
    assert_eq!(find_sound_file(&dir.0, "hit"), None);
}

#[test]
fn a_missing_sound_folder_finds_nothing() {
    let dir = TempDir::new();
    assert_eq!(find_sound_file(&dir.0.join("no_such_folder"), "jump"), None);
}

#[test]
fn a_folder_named_like_a_sound_is_not_a_sound_file() {
    let dir = TempDir::new();
    std::fs::create_dir(dir.0.join("jump.wav")).unwrap();
    assert_eq!(find_sound_file(&dir.0, "jump"), None);
}

#[test]
fn an_invalid_name_finds_nothing_even_if_a_file_would_match() {
    let dir = TempDir::new();
    let inner = dir.0.join("inner");
    std::fs::create_dir(&inner).unwrap();
    dir.touch("secret.wav");
    // Reaching up out of the sound folder must not work.
    assert_eq!(find_sound_file(&inner, "../secret"), None);
    assert_eq!(find_sound_file(&inner, ""), None);
    assert_eq!(find_sound_file(&inner, "UPPER"), None);
}

// ---- format check ---------------------------------------------------------------------

#[test]
fn only_wav_and_ogg_headers_pass_the_format_check() {
    let mut wav = b"RIFF\x24\x00\x00\x00WAVEfmt ".to_vec();
    assert_eq!(check_format(&wav), Ok(()));
    wav.truncate(12);
    assert_eq!(check_format(&wav), Ok(()));
    assert_eq!(check_format(b"OggS\x00\x02rest"), Ok(()));

    assert!(check_format(b"").unwrap_err().contains("empty"));
    assert!(
        check_format(b"RIFF").is_err(),
        "too short for a WAVE header"
    );
    assert!(check_format(b"RIFF\x24\x00\x00\x00AVI LIST").is_err());
    assert!(check_format(b"ID3\x03\x00\x00mp3 data").is_err());
    assert!(check_format(b"\x89PNG\r\n\x1a\n....").is_err());
    assert!(check_format(&[0u8; 64]).is_err());
}
