//! A private, writable copy of the fixture assets for tests that save, so nothing is written to
//! the repository. `with_body` adds the wanderer body and its poses (`fixtures/player_look`),
//! which the plain fixtures leave out, so the player has a body to edit.
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::game::GameState;

pub struct Assets(pub PathBuf);

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &to.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), to.join(entry.file_name())).unwrap();
        }
    }
}

impl Assets {
    pub fn new(with_body: bool) -> Self {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-player-assets-{}-{}",
            std::process::id(),
            NEXT_ID.fetch_add(1, Ordering::Relaxed)
        ));
        let fixtures = super::fixture_assets();
        copy_dir(&fixtures, &path);
        std::fs::remove_dir_all(path.join("player_look")).unwrap();
        if with_body {
            copy_dir(&fixtures.join("player_look"), &path);
        }
        Self(path)
    }

    /// A game with its editor open.
    pub fn editing(&self) -> GameState {
        let mut game = GameState::try_new(&self.0).unwrap();
        game.toggle_editor().unwrap();
        game
    }

    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
