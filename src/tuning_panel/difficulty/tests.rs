//! The Difficulty section's logic with no window: setting a percent or the cap, what the rules
//! come to at a tier, and saving.
use super::*;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub(super) const RULES: &str = "(enemy_health_percent: 10.0, enemy_damage_percent: 20.0, \
    boss_health_percent: 20.0, boss_damage_percent: 50.0, coin_percent: 10.0, max_effective_tier: 20)";

/// A private copy of the fixtures with its own `difficulty.ron`, so nothing here depends on the
/// shipped numbers or touches the repository.
pub(super) struct Scratch(pub(super) PathBuf);

impl Scratch {
    pub(super) fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-steps-difficulty-panel-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        copy_dir(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures"),
            &path,
        );
        std::fs::write(path.join("difficulty.ron"), RULES).unwrap();
        Self(path)
    }

    pub(super) fn file(&self) -> PathBuf {
        self.0.join("difficulty.ron")
    }

    pub(super) fn game(&self) -> GameState {
        let mut game = GameState::try_new(&self.0).unwrap();
        game.use_profile_file(self.0.join("profile.ron")).unwrap();
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

fn rules() -> Difficulty {
    Difficulty::from_ron(RULES).unwrap()
}

// ---- the fields -----------------------------------------------------------------------------------

#[test]
fn each_field_reads_and_changes_only_its_own_percent() {
    let before = rules();
    let read: Vec<f32> = Percent::ALL.iter().map(|f| f.get(&before)).collect();
    assert_eq!(read, [10.0, 20.0, 20.0, 50.0, 10.0]);
    for (i, field) in Percent::ALL.into_iter().enumerate() {
        let after = field.with(&before, 77.0);
        let now: Vec<f32> = Percent::ALL.iter().map(|f| f.get(&after)).collect();
        let mut expected = read.clone();
        expected[i] = 77.0;
        assert_eq!(now, expected, "{field:?}");
        assert_eq!(after.max_effective_tier, before.max_effective_tier);
    }
}

#[test]
fn every_field_has_its_own_label_and_a_hint() {
    let labels: Vec<_> = Percent::ALL.iter().map(|f| f.label()).collect();
    for (i, label) in labels.iter().enumerate() {
        assert!(!label.is_empty() && !labels[..i].contains(label), "{label}");
    }
    assert!(Percent::ALL.iter().all(|f| f.hint().len() > 20));
}

// ---- set ------------------------------------------------------------------------------------------

#[test]
fn a_set_percent_is_in_effect_for_the_next_run_and_unsaved() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    set_percent(&mut game, Percent::BossDamage, 33.5).unwrap();
    assert_eq!(game.difficulty().boss_damage_percent, 33.5);
    assert_eq!(
        game.difficulty().boss_health_percent,
        20.0,
        "the others stay"
    );
    assert!(game.difficulty_unsaved());
    assert_eq!(
        Difficulty::load(&scratch.0).unwrap(),
        rules(),
        "nothing is written until Save"
    );
}

#[test]
fn the_limits_are_accepted_and_anything_outside_them_is_refused_with_the_reason() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    for field in Percent::ALL {
        set_percent(&mut game, field, 0.0).unwrap();
        set_percent(&mut game, field, 100.0).unwrap();
        let before = *game.difficulty();
        for bad in [
            -0.1,
            100.1,
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            1.0e9,
        ] {
            let error = set_percent(&mut game, field, bad).unwrap_err();
            assert!(error.contains("from 0 to 100"), "{field:?} {bad}: {error}");
            assert_eq!(game.difficulty(), &before, "{field:?} {bad}");
        }
    }
}

#[test]
fn the_tier_cap_takes_1_to_99_and_nothing_else() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    set_cap(&mut game, 1).unwrap();
    set_cap(&mut game, 99).unwrap();
    for bad in [0, 100, u32::MAX] {
        let error = set_cap(&mut game, bad).unwrap_err();
        assert!(error.contains("max_effective_tier"), "{error}");
        assert_eq!(game.difficulty().max_effective_tier, 99, "{bad}");
    }
}

// ---- the read-out ---------------------------------------------------------------------------------

#[test]
fn the_readout_at_tier_0_says_every_number_is_as_authored() {
    let lines = readout(&rules(), 0);
    assert_eq!(lines[0], "Tier 0: every number as authored");
    assert_eq!(lines[1], "Enemies: health x1.00, damage x1.00");
    assert_eq!(lines[2], "Bosses: health x1.00, damage x1.00");
    assert_eq!(lines[3], "Coins: x1.00");
}

#[test]
fn the_readout_at_tier_3_shows_each_multiplier() {
    let lines = readout(&rules(), 3);
    assert_eq!(lines[0], "Tier 3");
    assert_eq!(lines[1], "Enemies: health x1.30, damage x1.60");
    assert_eq!(lines[2], "Bosses: health x1.60, damage x2.50");
    assert_eq!(lines[3], "Coins: x1.30");
}

#[test]
fn the_readout_above_the_cap_names_the_tier_it_plays_as() {
    let lines = readout(&rules(), 25);
    assert_eq!(lines[0], "Tier 25 plays as tier 20, the cap");
    assert_eq!(
        lines[1], "Enemies: health x3.00, damage x5.00",
        "tier 20's numbers"
    );
    assert_eq!(
        readout(&rules(), 20)[0],
        "Tier 20",
        "the cap itself is not above it"
    );
    assert_eq!(readout(&rules(), 20)[1], lines[1]);
}

#[test]
fn the_readout_follows_the_rules_it_is_given() {
    let doubled = Percent::EnemyHealth.with(&rules(), 100.0);
    assert_eq!(
        readout(&doubled, 1)[1],
        "Enemies: health x2.00, damage x1.20"
    );
}

// ---- save -----------------------------------------------------------------------------------------

#[test]
fn save_writes_the_rules_to_the_assets_folder_and_says_where() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    set_percent(&mut game, Percent::Coins, 42.0).unwrap();
    set_cap(&mut game, 5).unwrap();

    let path = save(&mut game).unwrap();

    assert!(path.ends_with("difficulty.ron"), "{path}");
    let loaded = Difficulty::load(&scratch.0).unwrap();
    assert_eq!(loaded.coin_percent, 42.0);
    assert_eq!(loaded.max_effective_tier, 5);
    assert_eq!(loaded.enemy_health_percent, 10.0);
    assert!(!game.difficulty_unsaved());
    assert!(!scratch.0.join("difficulty.ron.tmp").exists());
}

#[test]
fn a_failed_save_names_the_file_and_leaves_the_rules_unsaved() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    set_percent(&mut game, Percent::Coins, 42.0).unwrap();
    std::fs::remove_file(scratch.file()).unwrap();
    std::fs::create_dir(scratch.file()).unwrap();

    let error = save(&mut game).unwrap_err();

    assert!(error.contains("difficulty.ron"), "{error}");
    assert!(game.difficulty_unsaved());
    assert_eq!(game.difficulty().coin_percent, 42.0, "still in effect");
}

#[test]
fn saving_nothing_new_still_writes_a_valid_file() {
    let scratch = Scratch::new();
    let mut game = scratch.game();
    save(&mut game).unwrap();
    assert_eq!(Difficulty::load(&scratch.0).unwrap(), rules());
}
