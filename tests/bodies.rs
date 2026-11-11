mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::body::{BodyTemplate, BodyVisual, ColorSlot};
use hollow_steps::combat::SWORD_DAMAGE;
use hollow_steps::definition_editor::{serialize, write_files, DefinitionDraft};
use hollow_steps::enemy::{EnemyDefinition, EnemyDefinitions};
use hollow_steps::game::GameState;
use hollow_steps::player::PlayerInput;
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

struct Assets(PathBuf);
impl Assets {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-bodies-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for dir in ["entities", "behaviors", "rooms", "bosses", "bodies"] {
            std::fs::create_dir_all(path.join(dir)).unwrap();
            for entry in std::fs::read_dir(support::fixture_assets().join(dir)).unwrap() {
                let entry = entry.unwrap();
                std::fs::copy(entry.path(), path.join(dir).join(entry.file_name())).unwrap();
            }
        }
        Self(path)
    }

    fn entity(&self) -> EnemyDefinition {
        ron::from_str(&std::fs::read_to_string(self.0.join("entities").join("chase.ron")).unwrap())
            .unwrap()
    }

    fn set_body(&self, body: Option<BodyVisual>) {
        let mut entity = self.entity();
        entity.body = body;
        std::fs::write(
            self.0.join("entities").join("chase.ron"),
            serialize(&entity).unwrap(),
        )
        .unwrap();
    }
}
impl Drop for Assets {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

fn visual() -> BodyVisual {
    ron::from_str(include_str!("fixtures/body_visual.ron")).unwrap()
}

fn motion_visual() -> BodyVisual {
    BodyVisual {
        template: "motion".into(),
        ..visual()
    }
}

#[test]
fn game_time_animates_bodies_and_inventory_and_editor_freeze_motion() {
    let assets = Assets::new();
    assets.set_body(Some(motion_visual()));
    let mut game = GameState::try_new(&assets.0).unwrap();
    let body_state = |game: &GameState| format!("{:?}", game.room().enemies[0].body());
    let spawned = body_state(&game);
    game.update(PlayerInput::default(), 0.25);
    let moving = body_state(&game);
    assert_ne!(spawned, moving);
    game.toggle_inventory();
    game.update(PlayerInput::default(), 10.0);
    assert_eq!(body_state(&game), moving);
    game.toggle_inventory();
    game.toggle_editor().unwrap();
    game.update(PlayerInput::default(), 10.0);
    assert_eq!(body_state(&game), moving);
    game.toggle_editor().unwrap();
    for delta in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        game.update(PlayerInput::default(), delta);
        assert_eq!(body_state(&game), moving);
    }
    game.update(PlayerInput::default(), 0.25);
    assert_ne!(body_state(&game), moving);
}

#[test]
fn idle_continues_during_knockback_but_dead_enemies_do_not_animate() {
    let assets = Assets::new();
    assets.set_body(Some(motion_visual()));
    let bounds = WorldBounds::new(0.0, 960.0, 460.0);
    let mut enemy = EnemyDefinitions::load(&assets.0)
        .unwrap()
        .spawn("chase", 560.0, 800.0, bounds)
        .unwrap();
    let points = enemy.body().unwrap().points.clone();
    assert!(enemy.hit(1, 1.0));
    enemy.update(Vec2::ZERO, 0.01, bounds);
    assert_ne!(enemy.body().unwrap().points, points);
    for id in 2..=u64::from(enemy.max_health().div_ceil(SWORD_DAMAGE)) {
        assert!(enemy.hit(id, 1.0));
    }
    assert!(enemy.is_dead());
    let dead = format!("{:?}", enemy.body());
    enemy.update(Vec2::ZERO, 0.25, bounds);
    assert_eq!(format!("{:?}", enemy.body()), dead);
}

#[test]
fn f5_keeps_moving_points_and_phase_and_initializes_new_points_at_targets() {
    let assets = Assets::new();
    assets.set_body(Some(motion_visual()));
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.25);
    let previous = game.room().enemies[0].body().unwrap().clone();
    let path = assets.0.join("bodies").join("motion.ron");
    let mut replacement: BodyTemplate =
        ron::from_str(include_str!("fixtures/bodies/motion.ron")).unwrap();
    replacement.points.get_mut("root").unwrap().rest = (0.2, -0.1);
    replacement.points.insert(
        "new".into(),
        hollow_steps::body::BodyPoint { rest: (0.2, 0.3) },
    );
    replacement.idle.push(hollow_steps::body::IdleMotion {
        point: "new".into(),
        amplitude: (0.01, 0.02),
        period_secs: 4.0,
        phase_offset_secs: 1.0,
    });
    std::fs::write(&path, serialize(&replacement).unwrap()).unwrap();
    game.reload_rooms().unwrap();
    let current = game.room().enemies[0].body().unwrap();
    for name in ["root", "tip"] {
        assert_eq!(current.points[name], previous.points[name]);
        assert_eq!(current.velocities[name], previous.velocities[name]);
    }
    assert_eq!(current.points["new"], current.targets["new"]);
    assert_eq!(current.velocities["new"], Vec2::ZERO);
    assert_eq!(
        current.targets["root"],
        Vec2::new(0.2, -0.1) + replacement.idle[0].offset_at(0.25)
    );
    for bad in ["follow override", "idle"] {
        let mut invalid = replacement.clone();
        if bad == "idle" {
            invalid.idle[0].point = "missing".into();
        } else {
            invalid
                .follow_overrides
                .insert("missing".into(), invalid.follow);
        }
        std::fs::write(&path, serialize(&invalid).unwrap()).unwrap();
        let before = format!("{game:?}");
        let error = game.reload_rooms().unwrap_err();
        assert!(
            error.contains("motion.ron") && error.contains(bad) && error.contains("missing"),
            "{error}"
        );
        assert_eq!(format!("{game:?}"), before);
    }
}

#[test]
fn f5_matches_duplicate_entities_by_placement_even_when_reordered() {
    let assets = Assets::new();
    assets.set_body(Some(motion_visual()));
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.update(PlayerInput::default(), 0.25);
    let path = assets.0.join("rooms").join("first.ron");
    let first = r#"(definition: "chase", patrol_left: 560.0, patrol_right: 800.0)"#;
    let second = r#"(definition: "chase", patrol_left: 300.0, patrol_right: 300.0)"#;
    std::fs::write(&path, format!("(enemies: [{first}, {second}])")).unwrap();
    game.reload_rooms().unwrap();
    game.update(PlayerInput::default(), 0.01);
    let bodies = game
        .room()
        .enemies
        .iter()
        .map(|e| e.body().unwrap().points.clone())
        .collect::<Vec<_>>();
    assert_ne!(bodies[0], bodies[1]);
    std::fs::write(&path, format!("(enemies: [{second}, {first}])")).unwrap();
    game.reload_rooms().unwrap();
    assert_eq!(game.room().enemies[0].body().unwrap().points, bodies[1]);
    assert_eq!(game.room().enemies[1].body().unwrap().points, bodies[0]);
}

#[test]
fn editor_motion_values_validate_and_round_trip_without_changing_geometry() {
    let assets = Assets::new();
    assets.set_body(Some(motion_visual()));
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    let original = draft.bodies["motion"].clone();
    let edited = std::sync::Arc::make_mut(draft.bodies.get_mut("motion").unwrap());
    edited.follow.stiffness = 100.0;
    edited.follow_overrides.get_mut("tip").unwrap().damping = 9.0;
    edited.idle[0].amplitude = (0.02, 0.01);
    edited.idle[0].period_secs = 3.0;
    edited.idle[0].phase_offset_secs = 0.5;
    let expected = edited.clone();
    let plan = draft.plan(&[]).unwrap();
    write_files(&plan).unwrap();
    let mut saved = DefinitionDraft::load(&assets.0).unwrap();
    assert_eq!(saved.bodies["motion"].as_ref(), &expected);
    assert_eq!(saved.bodies["motion"].parts, original.parts);
    assert_eq!(saved.bodies["motion"].points, original.points);
    std::sync::Arc::make_mut(saved.bodies.get_mut("motion").unwrap()).idle[0].period_secs = 0.0;
    assert!(saved.plan(&[]).unwrap_err().contains("period_secs"));
}

#[test]
fn body_is_optional_and_does_not_change_motion_hitboxes_health_or_damage() {
    let assets = Assets::new();
    let bounds = WorldBounds::new(0.0, 960.0, 460.0);
    let mut plain = EnemyDefinitions::load(&assets.0)
        .unwrap()
        .spawn("chase", 100.0, 300.0, bounds)
        .unwrap();
    assert!(plain.body().is_none());
    let mut body = visual();
    body.scale = 2.5;
    assets.set_body(Some(body));
    let definitions = EnemyDefinitions::load(&assets.0).unwrap();
    let mut dressed = definitions.spawn("chase", 100.0, 300.0, bounds).unwrap();
    let second = definitions.spawn("chase", 100.0, 300.0, bounds).unwrap();
    assert_eq!(dressed.body().unwrap().visual.scale, 2.5);
    assert_eq!(
        dressed.body().unwrap().points,
        dressed.body().unwrap().template.rest_positions()
    );
    assert!(!std::ptr::eq(
        &dressed.body().unwrap().points,
        &second.body().unwrap().points
    ));
    for target in [Vec2::new(500.0, 440.0), Vec2::new(0.0, 440.0)] {
        plain.update(target, 0.25, bounds);
        dressed.update(target, 0.25, bounds);
        assert_eq!(plain.position(), dressed.position());
        assert_eq!(plain.size(), dressed.size());
        assert_eq!(plain.health(), dressed.health());
        assert_eq!(plain.contact_damage(), dressed.contact_damage());
        assert_eq!(
            plain.touches(target, Vec2::splat(36.0)),
            dressed.touches(target, Vec2::splat(36.0))
        );
    }
    assert!(dressed.facing_left());
    let points = dressed.body().unwrap().points.clone();
    dressed.update(Vec2::ZERO, 0.0, bounds);
    assert!(dressed.facing_left());
    assert_eq!(dressed.body().unwrap().points, points);
}

#[test]
fn palette_default_scale_and_invalid_visual_values_are_validated() {
    let body = visual();
    assert_eq!(body.scale, 1.0);
    assert_eq!(body.color(ColorSlot::Primary), body.primary);
    assert_eq!(body.color(ColorSlot::Secondary), body.secondary);
    assert_eq!(body.color(ColorSlot::Accent), body.accent);
    assert_eq!(body.color(ColorSlot::Eye), body.eye);
    for scale in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        let mut bad = body.clone();
        bad.scale = scale;
        assert!(bad.validate().unwrap_err().contains("scale"));
    }
    for channel in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        let mut bad = body.clone();
        bad.eye.0 = channel;
        assert!(bad.validate().unwrap_err().contains("Eye"));
    }
}

#[test]
fn loader_errors_identify_unknown_template_point_and_color_and_their_file() {
    let assets = Assets::new();
    let mut body = visual();
    body.template = "missing".into();
    assets.set_body(Some(body));
    let error = EnemyDefinitions::load(&assets.0).unwrap_err();
    assert!(
        error.contains("chase.ron") && error.contains("unknown body template \"missing\""),
        "{error}"
    );
    assets.set_body(Some(visual()));
    let path = assets.0.join("bodies").join("small.ron");
    let source = include_str!("fixtures/bodies/small.ron");
    for (bad, expected) in [
        (
            source.replace("Some(\"root\")", "Some(\"missing\")"),
            "unknown point \"missing\"",
        ),
        (
            source.replace("color_slot: eye", "color_slot: wrong"),
            "wrong",
        ),
    ] {
        std::fs::write(&path, bad).unwrap();
        let error = EnemyDefinitions::load(&assets.0).unwrap_err();
        assert!(
            error.contains("small.ron") && error.contains(expected),
            "{error}"
        );
    }
}

#[test]
fn staged_saves_validate_template_names_preserve_settings_and_guard_body_files() {
    let assets = Assets::new();
    assets.set_body(Some(visual()));
    let mut draft = DefinitionDraft::load(&assets.0).unwrap();
    assert!(draft.bodies.contains_key("small"));
    let path = draft.entity_path("chase").unwrap();
    draft
        .entities
        .get_mut(&path)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .template = "missing".into();
    let error = draft.plan(&[]).unwrap_err();
    assert!(
        error.contains("unknown body template \"missing\""),
        "{error}"
    );
    draft.entities.get_mut(&path).unwrap().body = Some(visual());
    draft
        .entities
        .get_mut(&path)
        .unwrap()
        .body
        .as_mut()
        .unwrap()
        .scale = 1.5;
    let body_file = assets.0.join("bodies").join("small.ron");
    let original = std::fs::read(&body_file).unwrap();
    let plan = draft.plan(&[]).unwrap();
    write_files(&plan).unwrap();
    assert_eq!(std::fs::read(&body_file).unwrap(), original);
    assert_eq!(assets.entity().body.unwrap().scale, 1.5);
    let draft = DefinitionDraft::load(&assets.0).unwrap();
    let plan = draft.plan(&[]).unwrap();
    std::fs::write(&body_file, "invalid RON").unwrap();
    let error = write_files(&plan).unwrap_err();
    assert!(
        error.contains("small.ron") && error.contains("changed on disk"),
        "{error}"
    );
}

#[test]
fn hot_reload_updates_body_geometry_and_palette_atomically_and_allows_retry() {
    let assets = Assets::new();
    assets.set_body(Some(visual()));
    let mut game = GameState::try_new(&assets.0).unwrap();
    let before = format!("{game:?}");
    let path = assets.0.join("bodies").join("small.ron");
    let source = include_str!("fixtures/bodies/small.ron");
    std::fs::write(
        &path,
        source.replace("color_slot: eye", "color_slot: missing"),
    )
    .unwrap();
    assert!(game.reload_rooms().unwrap_err().contains("small.ron"));
    assert_eq!(format!("{game:?}"), before);
    std::fs::write(
        &path,
        source.replace("rest: (0.1, -0.2)", "rest: (0.25, -0.2)"),
    )
    .unwrap();
    let mut body = visual();
    body.eye = (0.5, 1.0, 1.0, 1.0);
    body.scale = 1.25;
    assets.set_body(Some(body.clone()));
    game.reload_rooms().unwrap();
    let runtime = game
        .room()
        .enemies
        .iter()
        .find(|e| e.name() == "chase")
        .unwrap()
        .body()
        .unwrap();
    assert_eq!(runtime.visual, body);
    assert_eq!(runtime.template.points["root"].rest, (0.25, -0.2));
    // F5 updates the rest target but preserves the surviving point's current position.
    assert_eq!(runtime.targets["root"], Vec2::new(0.25, -0.2));
    assert_eq!(runtime.points["root"], Vec2::new(0.1, -0.2));
    assets.set_body(None);
    game.reload_rooms().unwrap();
    assert!(game
        .room()
        .enemies
        .iter()
        .find(|e| e.name() == "chase")
        .unwrap()
        .body()
        .is_none());
}

#[test]
fn editor_field_edits_retain_body_and_reject_invalid_scale_without_mutating_draft() {
    use hollow_steps::definition_editor::entity_with_body;
    let assets = Assets::new();
    assets.set_body(Some(visual()));
    let mut game = GameState::try_new(&assets.0).unwrap();
    game.toggle_editor().unwrap();
    let editor = game.editor_mut().unwrap();
    let path = editor.content.entity_path("chase").unwrap();
    let original = editor.content.entities[&path].clone();
    let mut fields = [
        original.name.clone(),
        original.health.to_string(),
        original.size.0.to_string(),
        original.size.1.to_string(),
        original.contact_damage.to_string(),
        original.behavior.clone(),
    ];
    fields[1] = "8".into();
    let edited = entity_with_body(&fields, original.body.clone()).unwrap();
    editor.update_entity(&path, edited).unwrap();
    assert_eq!(editor.content.entities[&path].body, original.body);
    assert_eq!(editor.content.entities[&path].health, 8);
    let before = editor.content.entities[&path].clone();
    let mut invalid = before.clone();
    invalid.body.as_mut().unwrap().template = "missing".into();
    assert!(editor
        .update_entity(&path, invalid)
        .unwrap_err()
        .contains("unknown body template"));
    assert_eq!(editor.content.entities[&path], before);
    let mut invalid = original.body.clone().unwrap();
    invalid.scale = 0.0;
    assert!(entity_with_body(&fields, Some(invalid))
        .unwrap_err()
        .contains("scale"));
    let default = BodyVisual::for_template("small");
    default.validate_template(&editor.content.bodies).unwrap();
    assert_eq!(default.eye.3, 1.0);
}
