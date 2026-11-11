//! The player's body: look validation, pose choice, template/pose loading and facing mirror.
//! Uses a template built here, not the shipped assets (`shipped_rooms.rs` covers those).

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::body::{load_body_templates, BodyInstance};
use hollow_steps::player_body::{PlayerBody, PlayerLook, PlayerPose};
use macroquad::math::Vec2;

const TEMPLATE: &str = r#"(
    pose_map: {"head": "head", "torso": "torso"},
    points: {"head": (rest: (0.0, -0.3)), "torso": (rest: (0.0, 0.0))},
    parts: [
        (name: "torso", shape: Rect, anchor: Some("torso"), offset: Some((0.1, 0.0)), size: (0.4, 0.4), rotation: 20.0, color_slot: primary),
        (name: "neck", shape: Line, from: Some("torso"), to: Some("head"), size: (0.1, 0.1), rotation: 0.0, color_slot: secondary),
    ],
)"#;

const LOOK: &str = r#"(
    visual: (template: "wanderer", primary: (0.3, 0.5, 0.8, 1.0), secondary: (0.1, 0.1, 0.3, 1.0),
             accent: (1.0, 0.7, 0.3, 1.0), eye: (1.0, 1.0, 0.8, 1.0), scale: 1.0),
    idle_pose: "p_idle", run_pose: "p_run", jump_pose: "p_jump", attack_pose: "p_attack",
    run_speed: 30.0,
)"#;

struct Assets(PathBuf);
impl Assets {
    fn new(with_poses: &[&str]) -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-player-body-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(path.join("bodies")).unwrap();
        std::fs::create_dir_all(path.join("poses")).unwrap();
        std::fs::write(path.join("bodies").join("wanderer.ron"), TEMPLATE).unwrap();
        std::fs::write(path.join("player_body.ron"), LOOK).unwrap();
        for pose in with_poses {
            let text = if *pose == "p_run" {
                r#"(offsets: {"head": (0.2, 0.0)})"#
            } else {
                "(offsets: {})"
            };
            std::fs::write(path.join("poses").join(format!("{pose}.ron")), text).unwrap();
        }
        Self(path)
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const ALL_POSES: [&str; 4] = ["p_idle", "p_run", "p_jump", "p_attack"];

fn look() -> PlayerLook {
    PlayerLook::parse(LOOK).unwrap()
}

#[test]
fn shipped_default_look_is_valid() {
    // The embedded defaults must parse and validate with no assets folder at all.
    let empty = std::env::temp_dir().join("hollow-player-body-no-such-folder");
    PlayerLook::load(&empty).unwrap();
}

#[test]
fn pose_priority_is_attack_then_air_then_run_then_idle() {
    let look = look();
    assert_eq!(look.pose_for(true, 0.0, false), PlayerPose::Idle);
    assert_eq!(look.pose_for(true, 200.0, false), PlayerPose::Run);
    assert_eq!(look.pose_for(true, -200.0, false), PlayerPose::Run);
    assert_eq!(look.pose_for(false, 200.0, false), PlayerPose::Jump);
    assert_eq!(look.pose_for(false, 200.0, true), PlayerPose::Attack);
    assert_eq!(look.pose_for(true, 0.0, true), PlayerPose::Attack);
}

#[test]
fn slow_or_non_finite_speed_is_idle() {
    let look = look();
    assert_eq!(look.pose_for(true, 30.0, false), PlayerPose::Idle);
    assert_eq!(look.pose_for(true, f32::NAN, false), PlayerPose::Idle);
}

#[test]
fn invalid_looks_are_refused() {
    for (from, to) in [
        ("run_speed: 30.0", "run_speed: 0.0"),
        ("run_speed: 30.0", "run_speed: -1.0"),
        ("\"p_run\"", "\"Bad Name\""),
        ("scale: 1.0", "scale: 0.0"),
        ("(1.0, 0.7, 0.3, 1.0)", "(2.0, 0.7, 0.3, 1.0)"),
    ] {
        assert!(PlayerLook::parse(&LOOK.replace(from, to)).is_err(), "{to}");
    }
    assert!(PlayerLook::parse(&LOOK.replace("run_speed", "speed")).is_err());
}

#[test]
fn loads_template_and_poses_and_follows_the_player_state() {
    let assets = Assets::new(&ALL_POSES);
    let mut body = PlayerBody::load(&assets.0)
        .unwrap()
        .expect("template exists");
    body.update(true, Vec2::new(300.0, 0.0), false, 1.0 / 120.0);
    assert_eq!(body.current_pose(), Some("p_run"));
    body.update(false, Vec2::new(300.0, -100.0), false, 1.0 / 120.0);
    assert_eq!(body.current_pose(), Some("p_jump"));
    body.update(false, Vec2::ZERO, true, 1.0 / 120.0);
    assert_eq!(body.current_pose(), Some("p_attack"));
    body.update(true, Vec2::ZERO, false, 1.0 / 120.0);
    assert_eq!(body.current_pose(), Some("p_idle"));
}

#[test]
fn zero_nan_or_huge_steps_do_not_break_the_body() {
    let assets = Assets::new(&ALL_POSES);
    let mut body = PlayerBody::load(&assets.0).unwrap().unwrap();
    for dt in [0.0, f32::NAN, -1.0, 1000.0] {
        body.update(true, Vec2::ZERO, false, dt);
    }
    let head = body.body().points["head"];
    assert!(head.x.is_finite() && head.y.is_finite());
}

#[test]
fn a_missing_pose_is_an_error_and_a_missing_template_keeps_the_box() {
    let missing_pose = Assets::new(&["p_idle", "p_run", "p_jump"]);
    let error = PlayerBody::load(&missing_pose.0).unwrap_err();
    assert!(error.contains("p_attack"), "{error}");

    let no_template = Assets::new(&ALL_POSES);
    std::fs::remove_file(no_template.0.join("bodies").join("wanderer.ron")).unwrap();
    assert!(PlayerBody::load(&no_template.0).unwrap().is_none());
}

#[test]
fn a_bad_override_names_its_file() {
    let assets = Assets::new(&ALL_POSES);
    std::fs::write(assets.0.join("player_body.ron"), "(run_speed: 1.0)").unwrap();
    let error = PlayerBody::load(&assets.0).unwrap_err();
    assert!(error.contains("player_body.ron"), "{error}");
}

#[test]
fn facing_left_mirrors_every_part_about_the_player_center() {
    let assets = Assets::new(&ALL_POSES);
    let templates = load_body_templates(&assets.0.join("bodies")).unwrap();
    let body = BodyInstance::new(look().visual, &templates).unwrap();
    let center = Vec2::new(100.0, 50.0);
    let size = Vec2::new(40.0, 60.0);
    for part in &body.template.parts {
        let right = body.part_transform(part, center, size, false).unwrap();
        let left = body.part_transform(part, center, size, true).unwrap();
        assert!((left.center.x - center.x + (right.center.x - center.x)).abs() < 1e-4);
        assert!((left.center.y - right.center.y).abs() < 1e-4);
        assert!((left.rotation + right.rotation).abs() < 1e-4);
        if let (Some((rf, rt)), Some((lf, lt))) = (right.endpoints, left.endpoints) {
            assert!(((lf.x - center.x) + (rf.x - center.x)).abs() < 1e-4);
            assert!(((lt.x - center.x) + (rt.x - center.x)).abs() < 1e-4);
        }
    }
}
