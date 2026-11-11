use std::path::Path;

use hollow_steps::game::GameState;

#[test]
fn shipped_rooms_load_and_validate() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // Startup validates room geometry, entities, behaviors, and exit destinations,
    // including saved custom rooms. Authored coordinates and contents may change.
    GameState::try_new(&assets).expect("shipped rooms must load and validate");
    hollow_steps::preset::load_presets(&assets.join("presets"))
        .expect("shipped behavior presets must load and validate");
}

#[test]
fn shipped_player_body_loads_and_validates() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // The wanderer template, its four poses and the look must load and fit together.
    let body = hollow_steps::player_body::PlayerBody::load(&assets)
        .expect("shipped player body must load and validate");
    assert!(body.is_some(), "the wanderer body template is shipped");
}

#[test]
fn shipped_boss_body_loads_and_validates() {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // The Ash Warden's template, hurt pose and colors must load and fit together.
    let bosses = hollow_steps::boss_definition::BossDefinitions::load(&assets)
        .expect("shipped bosses must load and validate");
    let boss = bosses
        .spawn("Ash Warden", macroquad::math::Vec2::new(400.0, 400.0))
        .expect("the shipped Ash Warden spawns");
    assert!(boss.body().is_some(), "the Ash Warden has a body");
    assert!(boss.size().min_element() > 0.0);
}

#[test]
fn shipped_bird_body_and_wing_poses_load_and_fit_together() {
    use macroquad::math::Vec2;

    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // Loading validates every template and pose; the bird must be among them.
    let bodies = hollow_steps::body::load_body_templates(&assets.join("bodies"))
        .expect("shipped body templates must load and validate");
    let poses = hollow_steps::pose::load_poses(&assets.join("poses"))
        .expect("shipped poses must load and validate");
    let bird = bodies
        .get("bird")
        .expect("the bird body template is shipped");

    // The bird flies: no leg points, and both wing tips plus the tail are posable.
    for legs in ["foot_l", "foot_r"] {
        assert!(!bird.pose_map.contains_key(legs), "a flier has no {legs}");
    }
    for posable in ["hand_l", "hand_r", "tail"] {
        assert!(
            bird.pose_map.contains_key(posable),
            "the bird maps {posable}"
        );
    }

    // Both wing poses exist, only move points the bird has, and really differ: the wing tips
    // sit higher in the up pose than in the down pose.
    let up = poses.get("fly_up").expect("fly_up pose is shipped");
    let down = poses.get("fly_down").expect("fly_down pose is shipped");
    for pose in [up, down] {
        for point in pose.offsets.keys() {
            assert!(
                bird.pose_map.contains_key(point),
                "wing pose moves {point}, which the bird does not map"
            );
        }
    }
    let wing_y = |pose: &hollow_steps::pose::Pose, wing: &str| {
        let point = &bird.pose_map[wing];
        pose.resolve(bird)[point].y
    };
    for wing in ["hand_l", "hand_r"] {
        assert!(
            wing_y(up, wing) < wing_y(down, wing) - 0.5,
            "{wing}: wings-up must sit clearly above wings-down"
        );
    }

    // The hurt pose the bird uses still exists, so the hurt flash and pose keep working.
    assert!(poses.contains_key("hurt"));

    // Facing mirrors the whole body: the beak flips to the other side of the bird's centre.
    let size = Vec2::new(32.0, 40.0);
    let beak_x = |facing_left: bool| {
        let body = hollow_steps::body::BodyInstance::new(
            hollow_steps::body::BodyVisual::for_template("bird"),
            &bodies,
        )
        .expect("the bird body instantiates");
        let beak = body
            .template
            .parts
            .iter()
            .find(|part| part.name == "beak")
            .expect("the bird has a beak");
        body.part_transform(beak, Vec2::ZERO, size, facing_left)
            .expect("the beak is drawable")
            .center
            .x
    };
    let (right, left) = (beak_x(false), beak_x(true));
    assert!(right > 0.0 && left < 0.0, "beak right {right}, left {left}");
    assert!((right + left).abs() < 1e-3, "facing mirrors exactly");
}
