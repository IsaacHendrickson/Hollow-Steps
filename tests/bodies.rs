mod support;

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use hollow_steps::body::{BodyVisual, ColorSlot};
use hollow_steps::definition_editor::{serialize, write_files, DefinitionDraft};
use hollow_steps::enemy::{EnemyDefinition, EnemyDefinitions};
use hollow_steps::game::GameState;
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
    assert_eq!(runtime.points["root"], Vec2::new(0.25, -0.2));
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
