//! Where the game finds its `assets` folder.
//!
//! The order is: the `HOLLOW_STEPS_ASSETS` environment variable, an `assets`
//! folder next to the executable, then the `assets` folder of the source tree
//! the binary was built from. The first two let a built game run on a machine
//! other than the one that compiled it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub const ASSETS_ENV: &str = "HOLLOW_STEPS_ASSETS";

/// Picks the assets folder from already-read inputs, so the order can be tested.
///
/// A non-empty `env` value always wins, even if the folder is missing, so a wrong
/// path shows up as a load error instead of silently using another folder. An
/// `assets` entry next to the executable counts only if it is a directory.
pub fn resolve_assets_root(
    env: Option<OsString>,
    exe_dir: Option<&Path>,
    source_assets: &Path,
) -> PathBuf {
    if let Some(configured) = env.filter(|value| !value.is_empty()) {
        return PathBuf::from(configured);
    }
    if let Some(beside_exe) = exe_dir.map(|dir| dir.join("assets")).filter(|p| p.is_dir()) {
        return beside_exe;
    }
    source_assets.to_owned()
}

/// The assets folder for this process: environment, executable, then source tree.
pub fn assets_root() -> PathBuf {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    resolve_assets_root(
        std::env::var_os(ASSETS_ENV),
        exe_dir.as_deref(),
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets"),
    )
}

/// Adds the folder that was tried and how to point elsewhere to a load error.
pub fn describe_load_failure(root: &Path, error: &str) -> String {
    format!(
        "{error}\n\nAssets folder: {}\nSet {ASSETS_ENV} to load assets from another folder.",
        root.display()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct TempDir(PathBuf);
    impl TempDir {
        fn new() -> Self {
            static NEXT: AtomicUsize = AtomicUsize::new(0);
            let path = std::env::temp_dir().join(format!(
                "hollow-assets-root-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
        fn with_assets_dir(self) -> Self {
            std::fs::create_dir_all(self.0.join("assets")).unwrap();
            self
        }
    }
    impl Drop for TempDir {
        fn drop(&mut self) {
            std::fs::remove_dir_all(&self.0).unwrap();
        }
    }

    fn source() -> PathBuf {
        PathBuf::from("source_tree_assets")
    }

    #[test]
    fn the_environment_variable_wins_over_the_executable_and_source_folders() {
        let exe = TempDir::new().with_assets_dir();
        let chosen = resolve_assets_root(Some("custom_assets".into()), Some(&exe.0), &source());
        assert_eq!(chosen, PathBuf::from("custom_assets"));
    }

    #[test]
    fn a_configured_folder_is_used_even_when_it_does_not_exist() {
        let exe = TempDir::new().with_assets_dir();
        let missing = exe.0.join("no_such_folder");
        let chosen = resolve_assets_root(Some(missing.clone().into()), Some(&exe.0), &source());
        assert_eq!(chosen, missing);
    }

    #[test]
    fn an_empty_environment_variable_is_ignored() {
        let exe = TempDir::new().with_assets_dir();
        let chosen = resolve_assets_root(Some(OsString::new()), Some(&exe.0), &source());
        assert_eq!(chosen, exe.0.join("assets"));
    }

    #[test]
    fn an_assets_folder_next_to_the_executable_beats_the_source_tree() {
        let exe = TempDir::new().with_assets_dir();
        let chosen = resolve_assets_root(None, Some(&exe.0), &source());
        assert_eq!(chosen, exe.0.join("assets"));
    }

    #[test]
    fn without_an_environment_variable_or_executable_assets_the_source_tree_is_used() {
        let bare = TempDir::new();
        assert_eq!(
            resolve_assets_root(None, Some(&bare.0), &source()),
            source()
        );
        assert_eq!(resolve_assets_root(None, None, &source()), source());
    }

    #[test]
    fn a_file_named_assets_next_to_the_executable_is_not_a_folder() {
        let exe = TempDir::new();
        std::fs::write(exe.0.join("assets"), "not a folder").unwrap();
        assert_eq!(resolve_assets_root(None, Some(&exe.0), &source()), source());
    }

    #[test]
    fn load_failures_name_the_folder_and_the_environment_variable() {
        let message = describe_load_failure(Path::new("missing_assets"), "rooms: not found");
        assert!(message.starts_with("rooms: not found"));
        assert!(message.contains("missing_assets"));
        assert!(message.contains(ASSETS_ENV));
    }
}
