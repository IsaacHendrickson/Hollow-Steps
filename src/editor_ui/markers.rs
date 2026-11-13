//! The markers the editor draws and hit-tests on the canvas: enemies, the boss, items, exits,
//! the room spawn, the selected enemy's patrol handles, or the arrival point of one exit.
use hollow_steps::editor::{EditorSession, Selection};
use hollow_steps::item::ITEM_SIZE;
use macroquad::prelude::*;

pub(crate) fn editor_markers(
    editor: &EditorSession,
    destination: Option<usize>,
    selected: Option<Selection>,
) -> Vec<(Selection, Rect, Color)> {
    let body =
        |p: Vec2, size: Vec2| Rect::new(p.x - size.x / 2.0, p.y - size.y / 2.0, size.x, size.y);
    if let Some(i) = destination {
        return editor
            .draft
            .layout
            .exits
            .get(i)
            .map(|exit| {
                vec![(
                    Selection::Destination(i),
                    body(exit.spawn.into(), vec2(32.0, 48.0)),
                    PURPLE,
                )]
            })
            .unwrap_or_default();
    }
    let mut markers = Vec::new();
    for (i, enemy) in editor.draft.enemies.iter().enumerate() {
        if let Ok(size) = editor.definitions.size(&enemy.definition) {
            markers.push((
                Selection::Enemy(i),
                body(
                    vec2(
                        enemy.patrol_left,
                        editor.draft.layout.floor_y - size.y / 2.0,
                    ),
                    size,
                ),
                RED,
            ));
        }
    }
    if let Some(boss) = &editor.draft.boss {
        if let Some(definition) = editor
            .content
            .boss_path(&boss.definition)
            .and_then(|p| editor.content.bosses.get(&p))
        {
            markers.push((
                Selection::Boss,
                body(boss.position.into(), definition.size.into()),
                PURPLE,
            ));
        }
    }
    for (i, item) in editor.draft.layout.items.iter().enumerate() {
        markers.push((
            Selection::Item(i),
            body(item.position.into(), ITEM_SIZE),
            GOLD,
        ));
    }
    for (i, exit) in editor.draft.layout.exits.iter().enumerate() {
        markers.push((
            Selection::Exit(i),
            exit.exit().zone(editor.draft.layout.rect()),
            GREEN,
        ));
    }
    markers.push((
        Selection::Spawn,
        body(editor.draft.layout.spawn.into(), vec2(32.0, 48.0)),
        SKYBLUE,
    ));
    if let Some(Selection::Enemy(i) | Selection::PatrolLeft(i) | Selection::PatrolRight(i)) =
        selected
    {
        if let Some(enemy) = editor.draft.enemies.get(i) {
            if let Ok(size) = editor.definitions.size(&enemy.definition) {
                let y = editor.draft.layout.floor_y - size.y - 14.0;
                for (selection, x, handle_y) in [
                    (Selection::PatrolLeft(i), enemy.patrol_left, y),
                    (Selection::PatrolRight(i), enemy.patrol_right, y - 24.0),
                ] {
                    markers.push((selection, body(vec2(x, handle_y), vec2(14.0, 20.0)), ORANGE));
                }
            }
        }
    }
    markers
}
