use super::*;
use crate::editor_ui::editor_markers;
use hollow_steps::body::BodyVisual;
use hollow_steps::game::GameState;
use hollow_steps::room::RoomId;

fn game_editing(room: RoomId) -> GameState {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures");
    let mut game = GameState::try_new(&assets).unwrap();
    game.edit_room(room).unwrap();
    game
}

fn previews(editor: &EditorSession) -> (Vec<(Selection, Rect, Color)>, Vec<VisualPreview>) {
    let markers = editor_markers(editor, None, None);
    let previews = visual_previews(editor, &markers);
    (markers, previews)
}

fn hitbox(markers: &[(Selection, Rect, Color)], selection: Selection) -> Rect {
    markers
        .iter()
        .find(|(s, _, _)| *s == selection)
        .map(|(_, rect, _)| *rect)
        .unwrap()
}

fn preview(previews: &[VisualPreview], selection: Selection) -> &VisualPreview {
    previews.iter().find(|p| p.selection == selection).unwrap()
}

fn set_enemy(editor: &mut EditorSession, change: impl FnOnce(&mut EnemyDefinition)) {
    let path = editor.content.entity_path("chase").unwrap();
    let mut entity = editor.content.entities[&path].clone();
    change(&mut entity);
    editor.update_entity(&path, entity).unwrap();
}

use hollow_steps::enemy::EnemyDefinition;

#[test]
fn an_enemy_without_a_visual_size_is_previewed_in_its_hitbox() {
    let game = game_editing(RoomId::First);
    let editor = game.editor().unwrap();
    let (markers, previews) = previews(editor);
    let enemy = preview(&previews, Selection::Enemy(0));
    assert_eq!(enemy.rect, hitbox(&markers, Selection::Enemy(0)));
    assert!(enemy.body.is_none(), "the fixture enemy is a plain box");
}

#[test]
fn a_visual_size_is_previewed_beside_an_unchanged_hitbox_on_the_same_floor_line() {
    let mut game = game_editing(RoomId::First);
    let editor = game.editor_mut().unwrap();
    let (before, _) = previews(editor);
    set_enemy(editor, |e| e.visual_size = Some((90.0, 120.0)));
    let (markers, previews) = previews(editor);
    let enemy = preview(&previews, Selection::Enemy(0));
    let hit = hitbox(&markers, Selection::Enemy(0));
    assert_eq!(
        hit,
        hitbox(&before, Selection::Enemy(0)),
        "the hitbox is not resized"
    );
    assert_eq!(enemy.rect.size(), vec2(90.0, 120.0));
    assert_eq!(enemy.rect.center().x, hit.center().x);
    assert_eq!(enemy.rect.bottom(), hit.bottom());
    assert!(
        enemy.rect.top() < hit.top(),
        "a taller picture rises above the hitbox"
    );
}

#[test]
fn a_smaller_visual_sits_on_the_hitbox_floor_line_inside_it() {
    let mut game = game_editing(RoomId::First);
    let editor = game.editor_mut().unwrap();
    set_enemy(editor, |e| e.visual_size = Some((10.0, 12.0)));
    let (markers, previews) = previews(editor);
    let enemy = preview(&previews, Selection::Enemy(0));
    let hit = hitbox(&markers, Selection::Enemy(0));
    assert_eq!(enemy.rect.size(), vec2(10.0, 12.0));
    assert_eq!(enemy.rect.bottom(), hit.bottom());
    assert!(hit.contains(enemy.rect.center()));
}

#[test]
fn an_enemy_with_a_body_previews_that_body_at_the_visual_size() {
    let mut game = game_editing(RoomId::First);
    let editor = game.editor_mut().unwrap();
    set_enemy(editor, |e| {
        e.body = Some(BodyVisual::for_template("small"));
        e.visual_size = Some((60.0, 80.0));
    });
    let (_, previews) = previews(editor);
    let enemy = preview(&previews, Selection::Enemy(0));
    assert!(enemy.body.is_some());
    assert_eq!(enemy.rect.size(), vec2(60.0, 80.0));
}

#[test]
fn an_edit_shows_at_once_and_clearing_it_returns_the_preview_to_the_hitbox() {
    let mut game = game_editing(RoomId::First);
    let editor = game.editor_mut().unwrap();
    set_enemy(editor, |e| e.visual_size = Some((50.0, 50.0)));
    assert_eq!(
        preview(&previews(editor).1, Selection::Enemy(0))
            .rect
            .size(),
        vec2(50.0, 50.0)
    );
    set_enemy(editor, |e| e.visual_size = None);
    let (markers, previews) = previews(editor);
    assert_eq!(
        preview(&previews, Selection::Enemy(0)).rect,
        hitbox(&markers, Selection::Enemy(0))
    );
}

#[test]
fn the_boss_is_previewed_at_its_visual_size_and_a_plain_boss_has_no_body() {
    let mut game = game_editing(RoomId::Boss);
    let editor = game.editor_mut().unwrap();
    let (markers, plain) = previews(editor);
    let hit = hitbox(&markers, Selection::Boss);
    assert_eq!(preview(&plain, Selection::Boss).rect, hit);

    let path = editor.content.boss_path("Ash Warden").unwrap();
    let mut boss = editor.content.bosses[&path].clone();
    boss.visual_size = Some((150.0, 200.0));
    editor.update_boss(&path, boss).unwrap();
    let (markers, previews) = previews(editor);
    let boss = preview(&previews, Selection::Boss);
    assert_eq!(
        hitbox(&markers, Selection::Boss),
        hit,
        "the hitbox is not resized"
    );
    assert_eq!(boss.rect.size(), vec2(150.0, 200.0));
    assert_eq!(boss.rect.bottom(), hit.bottom());
    assert_eq!(boss.rect.center().x, hit.center().x);
    assert!(boss.body.is_none());
}

#[test]
fn a_boss_with_a_body_previews_that_body_at_the_visual_size() {
    let mut game = game_editing(RoomId::Boss);
    let editor = game.editor_mut().unwrap();
    let path = editor.content.boss_path("Ash Warden").unwrap();
    let mut boss = editor.content.bosses[&path].clone();
    boss.body = Some(BodyVisual::for_template("small"));
    boss.visual_size = Some((120.0, 160.0));
    editor.update_boss(&path, boss).unwrap();
    let (markers, previews) = previews(editor);
    let boss = preview(&previews, Selection::Boss);
    assert!(boss.body.is_some());
    assert_eq!(boss.rect.size(), vec2(120.0, 160.0));
    assert_eq!(
        boss.rect.bottom(),
        hitbox(&markers, Selection::Boss).bottom()
    );
}

#[test]
fn only_enemies_and_the_boss_get_a_preview() {
    let game = game_editing(RoomId::First);
    let editor = game.editor().unwrap();
    let (markers, previews) = previews(editor);
    assert!(
        markers.len() > previews.len(),
        "spawn, exits and items stay plain markers"
    );
    assert!(previews
        .iter()
        .all(|p| matches!(p.selection, Selection::Enemy(_) | Selection::Boss)));
    // A marker for an enemy the draft does not have previews nothing instead of panicking.
    let stray = [(Selection::Enemy(99), Rect::new(0.0, 0.0, 10.0, 10.0), RED)];
    assert!(visual_previews(editor, &stray).is_empty());
}
