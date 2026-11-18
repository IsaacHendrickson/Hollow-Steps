use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

/// A private empty folder under the system temp folder, removed afterwards.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-keybinds-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
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

fn changed() -> Keybinds {
    let mut binds = Keybinds::default();
    binds
        .rebind(Action::Jump, Slot::Primary, KeyCode::Q)
        .unwrap();
    binds
        .rebind(Action::Attack, Slot::Alternate, KeyCode::RightControl)
        .unwrap();
    binds.clear(Action::Dash, Slot::Alternate).unwrap();
    binds
        .rebind(Action::Heal, Slot::Primary, KeyCode::Z)
        .unwrap();
    binds
}

#[test]
fn bindings_survive_a_round_trip_through_the_file_text() {
    for binds in [Keybinds::default(), changed()] {
        let text = binds.to_ron().unwrap();
        assert_eq!(Keybinds::from_ron(&text), Ok(binds));
    }
    let text = changed().to_ron().unwrap();
    for action in Action::ALL {
        assert!(text.contains(action.name()), "{action:?} is written");
    }
}

#[test]
fn an_action_the_file_leaves_out_keeps_its_shipped_keys_and_one_key_means_no_alternate() {
    let binds = Keybinds::from_ron(r#"(bindings: {"jump": ["q"]})"#).unwrap();
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::Q));
    assert_eq!(binds.key(Action::Jump, Slot::Alternate), None);
    assert_eq!(binds.key(Action::Attack, Slot::Primary), Some(KeyCode::X));
    assert_eq!(
        binds.key(Action::MoveLeft, Slot::Alternate),
        Some(KeyCode::Left)
    );
    let empty = Keybinds::from_ron("(bindings: {})").unwrap();
    assert!(empty.is_default());
}

#[test]
fn a_bad_file_is_refused_with_a_message_that_names_the_problem() {
    let cases = [
        ("not ron at all", ""),
        (r#"(bindings: {}, extra: 1)"#, "extra"),
        (r#"(bindings: {"fly": ["A"]})"#, "unknown action \"fly\""),
        (
            r#"(bindings: {"jump": ["Banana"]})"#,
            "unknown key \"Banana\"",
        ),
        (
            r#"(bindings: {"jump": ["Escape"]})"#,
            "unknown key \"Escape\"",
        ),
        (r#"(bindings: {"jump": []})"#, "jump: list one or two keys"),
        (
            r#"(bindings: {"jump": ["Q", "W", "E"]})"#,
            "jump: list one or two keys",
        ),
        (
            r#"(bindings: {"jump": ["Q"], "attack": ["Q"]})"#,
            "Q is bound to both",
        ),
        // A listed action may not take a key an unlisted action still has.
        (
            r#"(bindings: {"attack": ["Z"]})"#,
            "Z is bound to both Jump and Attack",
        ),
    ];
    for (text, expected) in cases {
        let error = Keybinds::from_ron(text).unwrap_err();
        assert!(error.contains(expected), "{text}: {error}");
    }
}

#[test]
fn a_missing_file_means_the_shipped_keys() {
    let dir = TempDir::new();
    assert_eq!(load(&dir.file()), Ok(Keybinds::default()));
}

#[test]
fn saving_then_loading_gives_the_same_bindings_and_leaves_no_temporary_file() {
    let dir = TempDir::new();
    save(&dir.file(), &changed()).unwrap();
    assert_eq!(load(&dir.file()), Ok(changed()));
    let names = std::fs::read_dir(&dir.0)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect::<Vec<_>>();
    assert_eq!(names, [std::ffi::OsString::from(FILE_NAME)]);
    // A second save replaces the first.
    save(&dir.file(), &Keybinds::default()).unwrap();
    assert_eq!(load(&dir.file()), Ok(Keybinds::default()));
}

#[test]
fn saving_creates_the_config_folder_when_it_is_not_there_yet() {
    let dir = TempDir::new();
    let path = dir.0.join("not").join("there").join("yet").join(FILE_NAME);
    save(&path, &changed()).unwrap();
    assert_eq!(load(&path), Ok(changed()));
}

#[test]
fn saving_where_a_folder_cannot_be_made_fails_and_names_the_place() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join("blocker"), "a file, not a folder").unwrap();
    let path = dir.0.join("blocker").join(FILE_NAME);
    let error = save(&path, &changed()).unwrap_err();
    assert!(error.contains("blocker"), "{error}");
}

#[test]
fn a_bad_saved_file_is_an_error_that_names_the_file_and_is_left_alone() {
    let dir = TempDir::new();
    std::fs::write(dir.file(), r#"(bindings: {"jump": ["Banana"]})"#).unwrap();
    let error = load(&dir.file()).unwrap_err();
    assert!(
        error.contains(FILE_NAME) && error.contains("Banana"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(dir.file()).unwrap(),
        r#"(bindings: {"jump": ["Banana"]})"#
    );
}

#[test]
fn remove_deletes_the_file_and_a_missing_file_is_fine() {
    let dir = TempDir::new();
    save(&dir.file(), &changed()).unwrap();
    assert_eq!(remove(&dir.file()), Ok(()));
    assert!(!dir.file().exists());
    assert_eq!(load(&dir.file()), Ok(Keybinds::default()));
    assert_eq!(remove(&dir.file()), Ok(()));
}

fn env<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        pairs
            .iter()
            .find(|(key, _)| *key == name)
            .map(|(_, value)| (*value).to_owned())
    }
}

#[test]
fn the_config_folder_follows_the_platform() {
    let windows = config_dir(
        Platform::Windows,
        env(&[("APPDATA", "appdata"), ("HOME", "home")]),
    );
    assert_eq!(windows, Some(PathBuf::from("appdata").join("HollowSteps")));
    let mac = config_dir(Platform::Mac, env(&[("HOME", "home")]));
    assert_eq!(
        mac,
        Some(
            PathBuf::from("home")
                .join("Library")
                .join("Application Support")
                .join("HollowSteps")
        )
    );
    let xdg = config_dir(
        Platform::Other,
        env(&[("XDG_CONFIG_HOME", "xdg"), ("HOME", "home")]),
    );
    assert_eq!(xdg, Some(PathBuf::from("xdg").join("hollow-steps")));
    let home = config_dir(Platform::Other, env(&[("HOME", "home")]));
    assert_eq!(
        home,
        Some(PathBuf::from("home").join(".config").join("hollow-steps"))
    );
}

#[test]
fn the_config_folder_can_be_overridden_and_empty_variables_count_as_unset() {
    let pairs = [("HOLLOW_STEPS_CONFIG", "mine"), ("APPDATA", "appdata")];
    for platform in [Platform::Windows, Platform::Mac, Platform::Other] {
        assert_eq!(
            config_dir(platform, env(&pairs)),
            Some(PathBuf::from("mine"))
        );
    }
    let blank = [
        ("HOLLOW_STEPS_CONFIG", "  "),
        ("APPDATA", ""),
        ("HOME", "home"),
    ];
    assert_eq!(config_dir(Platform::Windows, env(&blank)), None);
    assert_eq!(
        config_dir(Platform::Other, env(&blank)),
        Some(PathBuf::from("home").join(".config").join("hollow-steps"))
    );
    assert_eq!(config_dir(Platform::Other, env(&[])), None);
}
