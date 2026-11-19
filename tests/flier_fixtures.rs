//! A flier built from a small fixture set (`tests/fixtures/flier/`): it hovers off the floor,
//! patrols inside its interval and flaps; its body has no legs, its wing poses fit it and the
//! whole body mirrors with facing. The files go through the same loaders as the shipped content
//! but are written for these tests, so tuning the shipped bird in the editor never breaks them.
mod support;

use std::collections::BTreeSet;
use std::path::PathBuf;

use hollow_steps::body::{load_body_templates, BodyInstance, BodyVisual};
use hollow_steps::enemy::{EnemyDefinitions, Movement};
use hollow_steps::player::WorldBounds;
use hollow_steps::pose::{load_poses, Pose};
use macroquad::math::Vec2;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DT: f32 = 1.0 / 120.0;
/// A made-up patrol interval: the test checks the kind of enemy the fixture is, not a placement.
const LEFT: f32 = 300.0;
const RIGHT: f32 = 400.0;

fn root() -> PathBuf {
    support::fixture_assets().join("flier")
}

#[test]
fn a_fixture_flier_hovers_off_the_floor_flaps_and_patrols_inside_its_interval() {
    let catalog =
        EnemyDefinitions::load(&root()).expect("the flier fixture must load and validate");
    let mut flier = catalog
        .spawn("flier", LEFT, RIGHT, BOUNDS)
        .expect("the fixture flier spawns");
    let Movement::Flying {
        hover_height,
        bob_amplitude,
        flap,
        ..
    } = flier.movement().clone()
    else {
        panic!("the fixture flier flies");
    };
    assert!(flap.is_some(), "and beats its wings");
    assert!(flier.body().is_some(), "with a body to flap");

    // Far from the player it patrols along its interval, always on its hover line (the bob moves
    // it by at most its amplitude), well off the floor.
    let floor_rest_y = BOUNDS.floor_y - flier.size().y / 2.0;
    let hover_y = floor_rest_y - hover_height;
    let player = Vec2::new(0.0, BOUNDS.floor_y - 24.0);
    let start = flier.position().x;
    let mut travelled = 0.0_f32;
    let mut poses = BTreeSet::new();
    for _ in 0..1200 {
        let before = flier.position().x;
        flier.update(player, DT, BOUNDS);
        travelled += (flier.position().x - before).abs();
        assert!(
            (flier.position().y - hover_y).abs() <= bob_amplitude + 0.01,
            "it holds its hover line (y {}, line {hover_y})",
            flier.position().y
        );
        assert!(
            flier.position().y < floor_rest_y - 20.0,
            "it hovers well off the floor"
        );
        poses.extend(flier.body().and_then(|body| body.pose_name.clone()));
    }
    assert!(
        travelled > 20.0,
        "it patrols (moved {travelled} px from {start})"
    );
    assert!(
        (LEFT..=RIGHT).contains(&flier.position().x),
        "inside its interval"
    );
    assert!(
        poses.contains("fly_up") && poses.contains("fly_down"),
        "it beats its wings between both poses, saw {poses:?}"
    );
}

#[test]
fn a_fixture_flier_body_has_no_legs_and_its_wing_poses_fit_it() {
    let bodies = load_body_templates(&root().join("bodies"))
        .expect("the flier body template must load and validate");
    let poses = load_poses(&root().join("poses")).expect("the flier poses must load and validate");
    let flier = bodies
        .get("flier")
        .expect("the flier template is in the fixture");

    // A flier has no leg points, and both wing tips plus the tail are posable.
    for legs in ["foot_l", "foot_r"] {
        assert!(!flier.pose_map.contains_key(legs), "a flier has no {legs}");
    }
    for posable in ["hand_l", "hand_r", "tail"] {
        assert!(
            flier.pose_map.contains_key(posable),
            "the flier maps {posable}"
        );
    }

    // Both wing poses exist, only move points the flier has, and really differ: the wing tips
    // sit higher in the up pose than in the down pose.
    let up = poses.get("fly_up").expect("fly_up pose is in the fixture");
    let down = poses
        .get("fly_down")
        .expect("fly_down pose is in the fixture");
    for pose in [up, down] {
        for point in pose.offsets.keys() {
            assert!(
                flier.pose_map.contains_key(point),
                "wing pose moves {point}, which the flier does not map"
            );
        }
    }
    let wing_y = |pose: &Pose, wing: &str| {
        let point = &flier.pose_map[wing];
        pose.resolve(flier)[point].y
    };
    for wing in ["hand_l", "hand_r"] {
        assert!(
            wing_y(up, wing) < wing_y(down, wing) - 0.5,
            "{wing}: wings-up must sit clearly above wings-down"
        );
    }

    // The hurt pose the flier uses exists, so the hurt flash and pose keep working.
    assert!(poses.contains_key("hurt"));
}

#[test]
fn a_fixture_flier_body_without_legs_mirrors_exactly_with_facing() {
    let bodies = load_body_templates(&root().join("bodies"))
        .expect("the flier body template must load and validate");
    let size = Vec2::new(32.0, 40.0);
    let beak_x = |facing_left: bool| {
        let body = BodyInstance::new(BodyVisual::for_template("flier"), &bodies)
            .expect("the flier body instantiates");
        let beak = body
            .template
            .parts
            .iter()
            .find(|part| part.name == "beak")
            .expect("the flier has a beak");
        body.part_transform(beak, Vec2::ZERO, size, facing_left)
            .expect("the beak is drawable")
            .center
            .x
    };
    // Facing mirrors the whole body: the beak flips to the other side of the centre.
    let (right, left) = (beak_x(false), beak_x(true));
    assert!(right > 0.0 && left < 0.0, "beak right {right}, left {left}");
    assert!((right + left).abs() < 1e-3, "facing mirrors exactly");
}
