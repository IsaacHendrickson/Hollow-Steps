use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::run::RunOutcome;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-profile-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn file(&self) -> PathBuf {
        self.0.join(FILE_NAME)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn profile() -> Profile {
    Profile {
        coins: 120,
        tier: 2,
        cleared: 3,
        ..Profile::default()
    }
}

fn file_name(path: &Path) -> &str {
    path.file_name().and_then(|name| name.to_str()).unwrap()
}

#[test]
fn a_profile_survives_a_ron_round_trip() {
    let text = profile().to_ron().unwrap();
    assert!(text.contains("version: 1"), "the file says its version");
    assert_eq!(Profile::from_ron(&text).unwrap(), profile());
    assert_eq!(
        Profile::from_ron(&Profile::default().to_ron().unwrap()).unwrap(),
        Profile::default()
    );
}

#[test]
fn a_saved_profile_loads_back_without_a_message() {
    let dir = TempDir::new();
    save(&dir.file(), &profile()).unwrap();
    assert_eq!(load(&dir.file()), (profile(), None));
}

#[test]
fn saving_creates_the_config_folder() {
    let dir = TempDir::new();
    let path = dir.0.join("new").join("folder").join(FILE_NAME);
    save(&path, &profile()).unwrap();
    assert_eq!(load(&path).0, profile());
}

#[test]
fn a_missing_file_is_a_fresh_profile_with_no_message() {
    let dir = TempDir::new();
    assert_eq!(load(&dir.file()), (Profile::default(), None));
    assert!(!dir.file().exists(), "loading never creates the file");
}

#[test]
fn a_file_that_does_not_parse_is_set_aside_and_the_reason_returned() {
    let dir = TempDir::new();
    std::fs::write(dir.file(), "this is ( not a profile").unwrap();

    let (loaded, message) = load(&dir.file());

    assert_eq!(loaded, Profile::default(), "a fresh profile is used");
    let message = message.expect("the front end shows why");
    assert!(message.contains(FILE_NAME), "names the file: {message}");
    assert!(message.contains("starting fresh"), "{message}");
    assert!(
        message.contains("profile.ron.bak"),
        "names the backup: {message}"
    );
    assert!(!dir.file().exists(), "the bad file is out of the way");
    assert_eq!(
        std::fs::read_to_string(backup_path(&dir.file())).unwrap(),
        "this is ( not a profile",
        "the player's bytes are kept"
    );
    assert_eq!(file_name(&backup_path(&dir.file())), "profile.ron.bak");
}

#[test]
fn a_new_bad_file_replaces_an_older_backup() {
    let dir = TempDir::new();
    std::fs::write(backup_path(&dir.file()), "the old backup").unwrap();
    std::fs::write(dir.file(), "(version: 1, coins: \"many\")").unwrap();

    let (_, message) = load(&dir.file());

    assert!(message.is_some());
    assert_eq!(
        std::fs::read_to_string(backup_path(&dir.file())).unwrap(),
        "(version: 1, coins: \"many\")"
    );
}

#[test]
fn an_unknown_version_is_set_aside_and_named() {
    let dir = TempDir::new();
    let text = "(version: 2, coins: 5, tier: 1, cleared: 1, upgrades: {})";
    std::fs::write(dir.file(), text).unwrap();

    let (loaded, message) = load(&dir.file());

    assert_eq!(loaded, Profile::default());
    let message = message.unwrap();
    assert!(
        message.contains("unknown profile version 2"),
        "a newer file says its version, not an unknown field: {message}"
    );
    assert_eq!(
        std::fs::read_to_string(backup_path(&dir.file())).unwrap(),
        text
    );
}

#[test]
fn a_file_with_a_missing_or_unknown_field_is_not_used() {
    for text in [
        "(version: 1, coins: 5, tier: 1)",
        "(version: 1, coins: 5, tier: 1, cleared: 0, bonus: 3)",
        "(coins: 5, tier: 1, cleared: 0)",
    ] {
        assert!(Profile::from_ron(text).is_err(), "{text}");
    }
}

#[test]
fn a_file_that_is_not_text_is_set_aside_too() {
    let dir = TempDir::new();
    std::fs::write(dir.file(), [0xff, 0xfe, 0x00, 0x9f]).unwrap();

    let (loaded, message) = load(&dir.file());

    assert_eq!(loaded, Profile::default());
    assert!(message.is_some());
    assert!(backup_path(&dir.file()).exists());
}

#[test]
fn a_failed_staged_write_leaves_the_old_file_and_says_so() {
    let dir = TempDir::new();
    save(&dir.file(), &profile()).unwrap();
    // A folder where the temporary file goes makes the staged write fail.
    std::fs::create_dir(dir.0.join("profile.ron.tmp")).unwrap();
    let next = Profile {
        coins: 999,
        ..profile()
    };

    let error = save(&dir.file(), &next).unwrap_err();

    assert!(error.contains("profile.ron"), "names the file: {error}");
    assert_eq!(load(&dir.file()).0, profile(), "the old file is untouched");
}

#[test]
fn saving_after_a_bad_file_was_set_aside_writes_a_clean_file() {
    let dir = TempDir::new();
    std::fs::write(dir.file(), "garbage").unwrap();
    let (mut loaded, _) = load(&dir.file());
    loaded.bank(40, 100.0, RunOutcome::Died);

    save(&dir.file(), &loaded).unwrap();

    assert_eq!(load(&dir.file()), (loaded, None));
    assert_eq!(
        std::fs::read_to_string(backup_path(&dir.file())).unwrap(),
        "garbage",
        "the backup still holds the old bytes"
    );
}

#[test]
fn the_default_path_is_the_profile_file_in_the_keybinds_folder() {
    // Same folder lookup as the keybinds file; only the file name differs.
    match (crate::keybinds::default_path(), default_path()) {
        (Some(binds), Some(profile)) => {
            assert_eq!(binds.parent(), profile.parent());
            assert_eq!(file_name(&profile), FILE_NAME);
        }
        (binds, profile) => assert_eq!(binds.is_none(), profile.is_none()),
    }
}

// ---- upgrades --------------------------------------------------------------------------

fn bought() -> Profile {
    let mut profile = profile();
    profile.upgrades.insert("vitality".into(), 2);
    profile.upgrades.insert("swift_feet".into(), 1);
    profile
}

#[test]
fn bought_upgrades_survive_a_ron_round_trip_and_a_save_and_reload() {
    let text = bought().to_ron().unwrap();
    assert!(
        text.contains("vitality"),
        "the levels are in the file: {text}"
    );
    assert_eq!(Profile::from_ron(&text).unwrap(), bought());

    let dir = TempDir::new();
    save(&dir.file(), &bought()).unwrap();
    assert_eq!(load(&dir.file()), (bought(), None));
}

#[test]
fn a_version_one_file_without_upgrades_loads_with_none() {
    // What the game wrote before upgrades existed: the version stays 1.
    let old = "(version: 1, coins: 9, tier: 1, cleared: 1)";
    let loaded = Profile::from_ron(old).unwrap();
    assert_eq!(loaded.coins, 9);
    assert!(loaded.upgrades.is_empty());

    let dir = TempDir::new();
    std::fs::write(dir.file(), old).unwrap();
    let (profile, message) = load(&dir.file());
    assert_eq!((profile.coins, message), (9, None), "no message, no backup");
    assert!(!backup_path(&dir.file()).exists());
}

#[test]
fn a_profile_without_upgrades_is_written_the_way_it_always_was() {
    let text = profile().to_ron().unwrap();
    assert!(!text.contains("upgrades"), "left out while empty: {text}");
}

#[test]
fn an_id_nobody_lists_any_more_is_kept_by_a_save() {
    let mut profile = profile();
    profile.upgrades.insert("removed_long_ago".into(), 3);
    let dir = TempDir::new();
    save(&dir.file(), &profile).unwrap();
    let (loaded, _) = load(&dir.file());
    assert_eq!(loaded.upgrades.get("removed_long_ago"), Some(&3));
    // And a save of what was loaded writes it again.
    save(&dir.file(), &loaded).unwrap();
    assert_eq!(load(&dir.file()).0, profile);
}

#[test]
fn a_misspelled_upgrades_field_is_still_refused() {
    let dir = TempDir::new();
    std::fs::write(
        dir.file(),
        "(version: 1, coins: 1, tier: 0, cleared: 0, upgrade: {})",
    )
    .unwrap();
    let (loaded, message) = load(&dir.file());
    assert_eq!(loaded, Profile::default());
    assert!(message.unwrap().contains("upgrade"));
    assert!(
        backup_path(&dir.file()).exists(),
        "the file is kept as .bak"
    );
}
