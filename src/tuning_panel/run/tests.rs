use super::*;
use hollow_steps::profile;
use hollow_steps::room::RoomId;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

const HUB_FILE: &str = r#"(enemies: [], layout: Some((
    bounds: (0.0, 0.0, 960.0, 460.0),
    floor_y: 460.0,
    spawn: (180.0, 436.0),
    items: [],
    exits: [(edge: Right, offset: 364.0, length: 96.0, target_room: First, spawn: (180.0, 436.0))],
)))"#;

/// A private copy of the fixtures with a hub, so nothing touches the repository or a real
/// config folder.
pub(super) struct Scratch(pub(super) PathBuf);

impl Scratch {
    pub(super) fn new(with_hub: bool) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-run-debug-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
            &path,
        );
        if with_hub {
            std::fs::write(path.join("rooms").join("room_0.ron"), HUB_FILE).unwrap();
            std::fs::write(
                path.join("run.ron"),
                "(hub_room: Custom(0), start_room: First, final_boss_room: Boss)",
            )
            .unwrap();
        }
        Self(path)
    }

    pub(super) fn profile_file(&self) -> PathBuf {
        self.0.join("profile.ron")
    }

    pub(super) fn game(&self) -> GameState {
        let mut game = GameState::try_new(&self.0).unwrap();
        game.use_profile_file(self.profile_file()).unwrap();
        game
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}

pub(super) fn saved(scratch: &Scratch) -> Profile {
    let (profile, problem) = profile::load(&scratch.profile_file());
    assert_eq!(problem, None);
    profile
}

#[test]
fn the_tier_is_set_and_saved_and_coins_and_clears_stay() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.set_profile(Profile {
        coins: 40,
        tier: 0,
        cleared: 2,
        ..Profile::default()
    })
    .unwrap();

    set_tier(&mut game, 7).unwrap();

    let expected = Profile {
        coins: 40,
        tier: 7,
        cleared: 2,
        ..Profile::default()
    };
    assert_eq!(*game.profile(), expected);
    assert_eq!(saved(&scratch), expected);
}

#[test]
fn the_coins_are_set_and_saved_and_the_rest_stays() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let mut start = Profile {
        coins: 3,
        tier: 6,
        cleared: 2,
        ..Profile::default()
    };
    start.upgrades.insert("vitality".into(), 2);
    game.set_profile(start.clone()).unwrap();

    set_coins(&mut game, 500).unwrap();

    let expected = Profile {
        coins: 500,
        ..start
    };
    assert_eq!(*game.profile(), expected);
    assert_eq!(saved(&scratch), expected);
}

#[test]
fn the_coins_never_go_past_the_limit_and_can_go_back_to_zero() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    set_coins(&mut game, MAX_COINS).unwrap();
    assert_eq!(game.profile().coins, MAX_COINS);
    set_coins(&mut game, u32::MAX).unwrap();
    assert_eq!(game.profile().coins, MAX_COINS);
    set_coins(&mut game, 0).unwrap();
    assert_eq!(saved(&scratch).coins, 0);
}

#[test]
fn the_tier_never_goes_past_the_limit() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    set_tier(&mut game, MAX_TIER).unwrap();
    assert_eq!(game.profile().tier, MAX_TIER);
    set_tier(&mut game, MAX_TIER + 1).unwrap();
    set_tier(&mut game, u32::MAX).unwrap();
    assert_eq!(game.profile().tier, MAX_TIER);
    set_tier(&mut game, 0).unwrap();
    assert_eq!(saved(&scratch).tier, 0);
}

#[test]
fn reset_profile_zeroes_everything_and_saves_it() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    game.set_profile(Profile {
        coins: 99,
        tier: 5,
        cleared: 3,
        ..Profile::default()
    })
    .unwrap();

    reset_profile(&mut game).unwrap();

    assert_eq!(*game.profile(), Profile::default());
    assert_eq!(saved(&scratch), Profile::default());
}

#[test]
fn reset_profile_also_takes_the_bought_upgrades_away() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    let base = *game.player().config();
    let mut bought = Profile::default();
    bought.upgrades.insert("vitality".into(), 3);
    bought.upgrades.insert("long_gone".into(), 1);
    game.set_profile(bought).unwrap();
    assert_ne!(*game.player().config(), base, "the upgrade is in effect");

    reset_profile(&mut game).unwrap();

    assert!(game.profile().upgrades.is_empty());
    assert!(saved(&scratch).upgrades.is_empty());
    assert_eq!(*game.player().config(), base, "and its effect is gone");
}

#[test]
fn a_failed_save_keeps_the_change_for_the_session_and_says_why() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    // A folder where the staged file goes makes every save fail.
    std::fs::create_dir(scratch.0.join("profile.ron.tmp")).unwrap();

    let error = set_tier(&mut game, 4).unwrap_err();

    assert!(!error.is_empty());
    assert_eq!(game.profile().tier, 4);
    assert!(!scratch.profile_file().exists());
    assert!(set_coins(&mut game, 9).is_err());
    assert_eq!(
        game.profile().coins,
        9,
        "the coins stay for the session too"
    );
}

#[test]
fn changing_the_profile_does_not_move_the_player_or_start_a_run() {
    let scratch = Scratch::new(true);
    let mut game = scratch.game();
    set_tier(&mut game, 3).unwrap();
    reset_profile(&mut game).unwrap();
    assert_eq!(game.room().id, RoomId::Custom(0));
    assert!(game.in_hub() && !game.run_in_progress());
}
