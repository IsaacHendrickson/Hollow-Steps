//! Enemies and bosses have a hitbox (`size`) and an optional, separate drawn size
//! (`visual_size`). Only the hitbox takes part in gameplay.
use std::collections::BTreeMap;
use std::sync::Arc;

use hollow_steps::behavior::BehaviorDefinition;
use hollow_steps::body::{BodyVisual, Shape};
use hollow_steps::boss_definition::BossDefinition;
use hollow_steps::boss_editor::BossInputs;
use hollow_steps::boss_special::BossSpecials;
use hollow_steps::definition_editor::serialize;
use hollow_steps::enemy::{Enemy, EnemyDefinition, EnemyDefinitions};
use hollow_steps::player::WorldBounds;
use hollow_steps::visual_box::{MAX_VISUAL_SIDE, MIN_VISUAL_SIDE};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const WALK: &str = r#"(initial_state: "walk", states: [
    (name: "walk", actions: [patrol(speed: 0.0)], transitions: [])])"#;
const ENEMY: &str =
    r#"(name: "walker", health: 3, size: (36.0, 40.0), contact_damage: 1, behavior: "walk")"#;
const ENEMY_WITH_VISUAL: &str = r#"(name: "walker", health: 3, size: (36.0, 40.0), visual_size: Some((90.0, 120.0)), contact_damage: 1, behavior: "walk")"#;

fn definition(source: &str) -> EnemyDefinition {
    ron::from_str(source).unwrap()
}

fn spawn(definition: &EnemyDefinition) -> Enemy {
    Enemy::from_definition(
        definition,
        Arc::new(BehaviorDefinition::from_ron(WALK).unwrap()),
        300.0,
        600.0,
        BOUNDS,
    )
    .unwrap()
}

fn boss_definition(visual: &str) -> String {
    include_str!("fixtures/bosses/ground_pound.ron").replacen("size:", &format!("{visual}size:"), 1)
}

fn boss_with(visual_size: Option<(f32, f32)>) -> BossDefinition {
    let mut boss =
        BossDefinition::from_ron(include_str!("fixtures/bosses/ground_pound.ron")).unwrap();
    boss.visual_size = visual_size;
    boss
}

fn bottom(center: Vec2, size: Vec2) -> f32 {
    center.y + size.y / 2.0
}

#[test]
fn an_enemy_without_visual_size_is_drawn_in_its_hitbox() {
    let parsed = definition(ENEMY);
    assert_eq!(parsed.visual_size, None);
    assert!(parsed.validate().is_ok());
    let enemy = spawn(&parsed);
    assert_eq!(enemy.visual_size(), enemy.size());
    assert_eq!(enemy.visual_center(), enemy.position());
}

#[test]
fn a_visual_size_changes_the_drawn_box_but_never_the_hitbox() {
    let enemy = spawn(&definition(ENEMY_WITH_VISUAL));
    assert_eq!(enemy.size(), Vec2::new(36.0, 40.0));
    assert_eq!(enemy.visual_size(), Vec2::new(90.0, 120.0));
    // The picture stands on the same floor line as the hitbox and rises above it.
    assert_eq!(
        bottom(enemy.visual_center(), enemy.visual_size()),
        bottom(enemy.position(), enemy.size())
    );
    assert_eq!(enemy.visual_center().x, enemy.position().x);
    assert!(enemy.visual_center().y < enemy.position().y);
}

#[test]
fn only_the_hitbox_decides_whether_an_enemy_touches_the_player() {
    let player = Vec2::new(30.0, 40.0);
    let at = |dx: f32, enemy: &Enemy| enemy.position() + Vec2::new(dx, 0.0);
    // A huge picture does not make a miss into a hit...
    let big = spawn(&definition(ENEMY_WITH_VISUAL));
    let reach = (big.size().x + player.x) / 2.0;
    assert!(big.touches(at(reach, &big), player));
    assert!(!big.touches(at(reach + 1.0, &big), player));
    // ...and a tiny picture does not turn a hit into a miss.
    let tiny = spawn(&definition(
        &ENEMY_WITH_VISUAL.replace("(90.0, 120.0)", "(4.0, 4.0)"),
    ));
    assert_eq!(tiny.visual_size(), Vec2::new(4.0, 4.0));
    assert!(tiny.touches(at(reach, &tiny), player));
    assert!(!tiny.touches(at(reach + 1.0, &tiny), player));
}

#[test]
fn a_visual_size_does_not_change_where_an_enemy_may_patrol() {
    // 90 px of picture would not fit beside the wall at 10, but the 36 px hitbox does.
    let mut wide = definition(ENEMY_WITH_VISUAL);
    wide.visual_size = Some((90.0, 120.0));
    let enemy = Enemy::from_definition(
        &wide,
        Arc::new(BehaviorDefinition::from_ron(WALK).unwrap()),
        20.0,
        600.0,
        BOUNDS,
    );
    assert!(enemy.is_ok(), "{enemy:?}");
}

#[test]
fn the_visual_size_round_trips_and_an_absent_one_is_not_written() {
    let with = definition(ENEMY_WITH_VISUAL);
    let text = serialize(&with).unwrap();
    assert!(text.contains("visual_size"), "{text}");
    assert_eq!(ron::from_str::<EnemyDefinition>(&text).unwrap(), with);

    let without = definition(ENEMY);
    let text = serialize(&without).unwrap();
    assert!(
        !text.contains("visual_size"),
        "older files stay unchanged: {text}"
    );
    assert_eq!(ron::from_str::<EnemyDefinition>(&text).unwrap(), without);
}

#[test]
fn bad_enemy_visual_sizes_are_refused_by_name_and_good_extremes_are_kept() {
    for ok in [
        (MIN_VISUAL_SIDE, MIN_VISUAL_SIDE),
        (MAX_VISUAL_SIDE, MAX_VISUAL_SIDE),
    ] {
        let mut valid = definition(ENEMY);
        valid.visual_size = Some(ok);
        assert!(valid.validate().is_ok(), "{ok:?}");
    }
    for bad in [
        (0.0, 10.0),
        (10.0, -1.0),
        (f32::NAN, 10.0),
        (10.0, f32::INFINITY),
        (MAX_VISUAL_SIDE + 1.0, 10.0),
    ] {
        let mut invalid = definition(ENEMY);
        invalid.visual_size = Some(bad);
        let error = invalid.validate().unwrap_err();
        assert!(
            error.contains("walker") && error.contains("visual size"),
            "{bad:?}: {error}"
        );
        assert!(Enemy::from_definition(
            &invalid,
            Arc::new(BehaviorDefinition::from_ron(WALK).unwrap()),
            300.0,
            600.0,
            BOUNDS
        )
        .is_err());
    }
}

#[test]
fn the_catalog_reports_the_visual_size_and_a_body_to_draw() {
    let templates = hollow_steps::body::load_body_templates(
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("bodies"),
    )
    .unwrap();
    let mut with_body = definition(ENEMY_WITH_VISUAL);
    with_body.body = Some(BodyVisual::for_template("small"));
    let mut plain = definition(ENEMY);
    plain.name = "plain".into();
    let behaviors = BTreeMap::from([(
        "walk".to_owned(),
        Arc::new(BehaviorDefinition::from_ron(WALK).unwrap()),
    )]);
    let catalog =
        EnemyDefinitions::from_definitions([with_body, plain], behaviors, templates).unwrap();

    assert_eq!(catalog.size("walker").unwrap(), Vec2::new(36.0, 40.0));
    assert_eq!(
        catalog.visual_size("walker").unwrap(),
        Vec2::new(90.0, 120.0)
    );
    assert_eq!(catalog.visual_size("plain").unwrap(), Vec2::new(36.0, 40.0));
    let body = catalog.body_instance("walker").unwrap().unwrap();
    assert!(body.template.parts.iter().any(|p| p.shape != Shape::Line));
    assert!(catalog.body_instance("plain").unwrap().is_none());
    assert!(catalog.visual_size("missing").is_err());
    assert!(catalog.body_instance("missing").is_err());
}

#[test]
fn a_boss_without_visual_size_is_drawn_in_its_hitbox() {
    let definition =
        BossDefinition::from_ron(include_str!("fixtures/bosses/ground_pound.ron")).unwrap();
    assert_eq!(definition.visual_size, None);
    let boss =
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(480.0, 400.0)).unwrap();
    assert_eq!(boss.visual_size(), boss.size());
    assert_eq!(boss.visual_center(), boss.position());
}

#[test]
fn a_boss_visual_size_changes_the_drawn_box_but_never_the_hitbox() {
    let text = boss_definition("visual_size: Some((150.0, 200.0)), ");
    let definition = BossDefinition::from_ron(&text).unwrap();
    assert_eq!(definition.visual_size, Some((150.0, 200.0)));
    let hitbox = Vec2::from(definition.size);
    let boss =
        BossSpecials::from_definition(Arc::new(definition), Vec2::new(480.0, 400.0)).unwrap();
    assert_eq!(boss.size(), hitbox);
    assert_eq!(boss.visual_size(), Vec2::new(150.0, 200.0));
    assert_eq!(
        bottom(boss.visual_center(), boss.visual_size()),
        bottom(boss.position(), boss.size())
    );
    assert_eq!(boss.visual_center().x, boss.position().x);
}

#[test]
fn bad_boss_visual_sizes_are_refused_and_good_ones_survive_a_round_trip() {
    for bad in [(0.0, 5.0), (5.0, f32::NAN), (MAX_VISUAL_SIDE + 1.0, 5.0)] {
        let error = boss_with(Some(bad)).validate().unwrap_err();
        assert!(error.contains("visual size"), "{bad:?}: {error}");
    }
    let good = boss_with(Some((77.5, 12.0)));
    let text = serialize(&good).unwrap();
    assert_eq!(BossDefinition::from_ron(&text).unwrap(), good);
    assert!(!serialize(&boss_with(None)).unwrap().contains("visual_size"));
}

#[test]
fn boss_inputs_carry_the_visual_size_and_refuse_a_bad_one_without_touching_the_boss() {
    let original = boss_with(Some((90.0, 90.0)));
    let mut inputs = BossInputs::from_definition(&original);
    assert_eq!(inputs.visual_size, Some((90.0, 90.0)));
    assert_eq!(inputs.build().unwrap(), original);

    inputs.visual_size = None;
    assert_eq!(inputs.build().unwrap().visual_size, None);

    inputs.visual_size = Some((0.0, 90.0));
    let mut boss = original.clone();
    assert!(inputs.apply(&mut boss).is_err());
    assert_eq!(boss, original, "an invalid edit never reaches the boss");
}
