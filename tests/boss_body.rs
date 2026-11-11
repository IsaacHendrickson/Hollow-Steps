//! A boss's optional body: definition loading, validation, hurt flash, facing, death fade and
//! the unchanged hitbox. Uses fixtures only, never `assets/`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hollow_steps::body::{BodyTemplate, BodyTemplates, BodyVisual};
use hollow_steps::boss_definition::{BossDefinition, BossDefinitions};
use hollow_steps::boss_editor::BossInputs;
use hollow_steps::boss_special::{BossSpecials, DEATH_EFFECT_SECONDS};
use hollow_steps::player::WorldBounds;
use hollow_steps::pose::{Pose, Poses};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const START: Vec2 = Vec2::new(400.0, 436.0);
const PLAIN: &str = include_str!("fixtures/bosses/ash_warden.ron");
const SMALL: &str = include_str!("fixtures/bodies/small.ron");
const HURT: &str = include_str!("fixtures/poses/hurt.ron");
const WINDUP: &str = include_str!("fixtures/poses/windup.ron");

fn templates() -> BodyTemplates {
    let mut template: BodyTemplate = ron::from_str(SMALL).unwrap();
    template.pose_map = [
        ("head".into(), "root".into()),
        ("hand_l".into(), "tip".into()),
    ]
    .into();
    BodyTemplates::from([("small".into(), Arc::new(template))])
}

fn poses() -> Poses {
    let pose = |source: &str| Arc::new(ron::from_str::<Pose>(source).unwrap());
    Poses::from([("hurt".into(), pose(HURT)), ("windup".into(), pose(WINDUP))])
}

fn visual() -> BodyVisual {
    ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap()
}

fn with_body() -> BossDefinition {
    let mut definition = BossDefinition::from_ron(PLAIN).unwrap();
    definition.body = Some(visual());
    definition
}

fn catalog(definition: BossDefinition) -> Result<BossDefinitions, String> {
    BossDefinitions::from_definitions_with_assets([definition], templates(), poses())
}

fn spawn(definition: BossDefinition) -> BossSpecials {
    catalog(definition)
        .unwrap()
        .spawn("Ash Warden", START)
        .unwrap()
}

#[test]
fn a_definition_without_a_body_loads_unchanged() {
    let definition = BossDefinition::from_ron(PLAIN).unwrap();
    assert!(definition.body.is_none());
    let boss = spawn(definition);
    assert!(boss.body().is_none());
    assert_eq!(boss.opacity(), 1.0);
}

#[test]
fn a_definition_with_a_body_loads_and_spawns_it() {
    let text = ron::to_string(&with_body()).unwrap();
    let definition = BossDefinition::from_ron(&text).unwrap();
    assert_eq!(definition.body, Some(visual()));
    assert!(spawn(definition).body().is_some());
}

#[test]
fn an_unknown_body_template_is_rejected() {
    let mut definition = with_body();
    definition.body.as_mut().unwrap().template = "missing".into();
    let error = catalog(definition.clone()).unwrap_err();
    assert!(error.contains("unknown body template"), "{error}");
    assert!(error.contains("Ash Warden"), "{error}");
    // Without any templates at all the same boss is rejected too, and cannot be built.
    assert!(BossDefinitions::from_definitions([with_body()]).is_err());
    assert!(BossSpecials::from_definition(Arc::new(with_body()), START).is_err());
}

#[test]
fn an_unknown_hurt_pose_is_rejected() {
    let mut definition = with_body();
    definition.body.as_mut().unwrap().hurt.pose = Some("nope".into());
    let error = catalog(definition).unwrap_err();
    assert!(error.contains("unknown pose"), "{error}");
    let mut known = with_body();
    known.body.as_mut().unwrap().hurt.pose = Some("hurt".into());
    assert!(catalog(known).is_ok());
}

#[test]
fn an_invalid_body_is_rejected_when_the_definition_loads() {
    let mut definition = with_body();
    definition.body.as_mut().unwrap().scale = 0.0;
    let error = BossDefinition::from_ron(&ron::to_string(&definition).unwrap()).unwrap_err();
    assert!(error.contains("scale"), "{error}");
}

#[test]
fn hurt_flash_starts_on_damage_and_wears_off() {
    let mut boss = spawn(with_body());
    assert_eq!(boss.body().unwrap().flash_strength(), 0.0);
    boss.damage_boss(0);
    assert_eq!(boss.body().unwrap().flash_strength(), 0.0);
    boss.damage_boss(10);
    assert!(boss.body().unwrap().flash_strength() > 0.0);
    boss.update(1.0, Vec2::new(100.0, 436.0), BOUNDS);
    assert_eq!(boss.body().unwrap().flash_strength(), 0.0);
}

#[test]
fn the_hitbox_is_the_definition_size_whatever_the_body() {
    let plain = spawn(BossDefinition::from_ron(PLAIN).unwrap());
    let mut definition = with_body();
    definition.body.as_mut().unwrap().scale = 3.0;
    let bodied = spawn(definition);
    assert_eq!(plain.size(), Vec2::new(48.0, 48.0));
    assert_eq!(bodied.size(), plain.size());
    assert_eq!(bodied.contact_damage(), plain.contact_damage());
}

#[test]
fn it_faces_the_player_while_idle() {
    let mut boss = spawn(with_body());
    boss.update(1.0 / 60.0, Vec2::new(100.0, 436.0), BOUNDS);
    assert!(boss.facing_left());
    boss.update(1.0 / 60.0, Vec2::new(800.0, 436.0), BOUNDS);
    assert!(!boss.facing_left());
}

#[test]
fn it_faces_its_direction_of_travel_during_a_glide() {
    for (player_x, left) in [(100.0, true), (800.0, false)] {
        let mut boss = spawn(with_body());
        // Face the wrong way first, so a glide has to turn it.
        boss.update(
            1.0 / 60.0,
            Vec2::new(800.0 - player_x + 100.0, 436.0),
            BOUNDS,
        );
        let player = Vec2::new(player_x, 436.0);
        boss.start_attack("Slam", player, BOUNDS);
        let before = boss.position().x;
        boss.update(1.0 / 60.0, player, BOUNDS);
        assert_ne!(boss.position().x, before);
        assert_eq!(boss.facing_left(), left, "player at {player_x}");
    }
}

#[test]
fn a_plain_telegraph_action_pulses_the_body_tint_only_while_it_lasts() {
    let mut boss = spawn(with_body());
    assert!(boss.body_warning().is_none());
    let player = Vec2::new(400.0, 436.0);
    // The glide that opens Slam is instantaneous, so the 0.5 s telegraph starts at once.
    boss.start_attack("Slam", player, BOUNDS);
    let (_, strength) = boss.body_warning().expect("telegraph tints the body");
    assert_eq!(strength, 1.0);
    boss.update(0.25, player, BOUNDS);
    let (_, strength) = boss.body_warning().expect("still telegraphing");
    assert!((0.0..=1.0).contains(&strength));
    boss.update(0.3, player, BOUNDS);
    assert!(boss.body_warning().is_none(), "the telegraph is over");
}

#[test]
fn the_body_fades_out_over_the_death_effect_and_freezes_its_flash() {
    let mut boss = spawn(with_body());
    assert_eq!(boss.opacity(), 1.0);
    boss.damage_boss(boss.health());
    assert!(boss.is_defeated());
    assert_eq!(boss.opacity(), 1.0);
    let flash = boss.body().unwrap().flash_strength();
    assert!(flash > 0.0, "the killing blow flashes");
    boss.update(DEATH_EFFECT_SECONDS / 2.0, Vec2::ZERO, BOUNDS);
    assert!((boss.opacity() - 0.5).abs() < 1e-5);
    assert_eq!(boss.body().unwrap().flash_strength(), flash);
    boss.update(DEATH_EFFECT_SECONDS, Vec2::ZERO, BOUNDS);
    assert_eq!(boss.opacity(), 0.0);
}

#[test]
fn zero_and_invalid_steps_leave_the_body_alone() {
    let mut boss = spawn(with_body());
    boss.damage_boss(10);
    let flash = boss.body().unwrap().flash_strength();
    for dt in [0.0, -1.0, f32::NAN] {
        boss.update(dt, Vec2::new(100.0, 436.0), BOUNDS);
    }
    assert_eq!(boss.body().unwrap().flash_strength(), flash);
}

#[test]
fn the_boss_editor_carries_the_body_through_a_save() {
    let definition = with_body();
    let rebuilt = BossInputs::from_definition(&definition).build().unwrap();
    assert_eq!(rebuilt.body, definition.body);
    assert_eq!(rebuilt, definition);
}

struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-boss-body-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for directory in ["bosses", "bodies", "poses"] {
            std::fs::create_dir_all(path.join(directory)).unwrap();
        }
        std::fs::write(path.join("bodies").join("small.ron"), SMALL).unwrap();
        std::fs::write(path.join("poses").join("hurt.ron"), HURT).unwrap();
        Self(path)
    }
    fn boss(&self, definition: &BossDefinition) -> PathBuf {
        let path = self.0.join("bosses").join("warden.ron");
        std::fs::write(&path, ron::to_string(definition).unwrap()).unwrap();
        path
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn boss_files_load_their_body_from_the_asset_folders() {
    let assets = Assets::new();
    let mut definition = with_body();
    definition.body.as_mut().unwrap().hurt.pose = Some("hurt".into());
    assets.boss(&definition);
    let boss = BossDefinitions::load(&assets.0)
        .unwrap()
        .spawn("Ash Warden", START)
        .unwrap();
    assert!(boss.body().is_some());
}

#[test]
fn a_boss_file_naming_a_missing_template_or_pose_reports_its_path() {
    let assets = Assets::new();
    let mut definition = with_body();
    definition.body.as_mut().unwrap().template = "missing".into();
    let path = assets.boss(&definition);
    let error = BossDefinitions::load(&assets.0).unwrap_err();
    assert!(error.contains("unknown body template"), "{error}");
    assert!(
        error.contains(path.file_name().unwrap().to_str().unwrap()),
        "{error}"
    );

    let mut definition = with_body();
    definition.body.as_mut().unwrap().hurt.pose = Some("nope".into());
    assets.boss(&definition);
    let error = BossDefinitions::load(&assets.0).unwrap_err();
    assert!(error.contains("unknown pose"), "{error}");
}
