//! The "Separate visual size" rows shared by the Entity and Boss panels: the size the body (or
//! plain box) is drawn at, apart from the hitbox set by the Hitbox width and height fields.
use egui_macroquad::egui;
use hollow_steps::visual_box::{self, MAX_VISUAL_SIDE, MIN_VISUAL_SIDE};

/// What an unreadable (half-typed) hitbox field falls back to when the switch is turned on.
const FALLBACK_HITBOX_SIDE: f32 = hollow_steps::enemy::ENEMY_SIZE.x;

/// A hitbox side from its text field, for starting the visual size from it.
pub(super) fn hitbox_side(text: &str) -> f32 {
    text.trim().parse().unwrap_or(FALLBACK_HITBOX_SIDE)
}

/// The checkbox and, while it is on, a labeled drag box per side. `visual` stays `None`
/// (drawn in the hitbox) until the checkbox is ticked; ticking starts from `hitbox` so the
/// picture does not jump. Dragging past either end of the range stops there.
pub(super) fn visual_size_rows(
    ui: &mut egui::Ui,
    visual: &mut Option<(f32, f32)>,
    hitbox: (f32, f32),
) {
    let mut separate = visual.is_some();
    if ui
        .checkbox(&mut separate, "Separate visual size")
        .on_hover_text(
            "Draw the body at its own size instead of the hitbox size. The hitbox still \
             decides collisions and damage. Both boxes stand on the same floor line.",
        )
        .changed()
    {
        *visual = visual_box::with_separate(*visual, hitbox, separate);
    }
    if let Some((width, height)) = visual {
        side_row(ui, "Visual width", width);
        side_row(ui, "Visual height", height);
    }
}

fn side_row(ui: &mut egui::Ui, label: &str, side: &mut f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(side)
                .range(MIN_VISUAL_SIDE..=MAX_VISUAL_SIDE)
                .speed(0.5)
                .max_decimals(1)
                .suffix(" px"),
        )
        .on_hover_text("Only the picture changes; the thin light box in the editor shows it.")
    });
}

#[cfg(test)]
mod tests {
    use super::super::test_support::{drawn_text, editing_game};
    use super::super::{draw_panel, EditorSession, EditorUi, PropertyUi};
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn an_unreadable_hitbox_side_starts_from_the_default_size() {
        assert_eq!(hitbox_side(" 48.5 "), 48.5);
        assert_eq!(hitbox_side("4x"), FALLBACK_HITBOX_SIDE);
        assert_eq!(hitbox_side(""), FALLBACK_HITBOX_SIDE);
    }

    fn frame(ctx: &egui::Context, editor: &mut EditorSession, state: &mut EditorUi) -> String {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 3000.0),
                )),
                ..Default::default()
            },
            |ctx| {
                draw_panel(ctx, editor, state);
            },
        );
        drawn_text(&output)
    }

    /// The "chase" entity open in the property panel, with its stored visual size set.
    fn open(
        game: &mut hollow_steps::game::GameState,
        visual: Option<(f32, f32)>,
    ) -> (PathBuf, EditorUi) {
        let editor = game.editor_mut().unwrap();
        let path = editor.content.entity_path("chase").unwrap();
        editor.content.entities.get_mut(&path).unwrap().visual_size = visual;
        let state = EditorUi {
            properties: true,
            property: PropertyUi::for_entity(editor, Some(path.clone())),
            ..Default::default()
        };
        (path, state)
    }

    #[test]
    fn the_panel_labels_hitbox_and_visual_sizes_apart() {
        let mut game = editing_game();
        let (_, mut state) = open(&mut game, Some((90.0, 120.5)));
        assert_eq!(state.property.visual_size, Some((90.0, 120.5)));
        let ctx = egui::Context::default();
        let text = frame(&ctx, game.editor_mut().unwrap(), &mut state);
        for expected in [
            "Hitbox width",
            "Hitbox height",
            "Separate visual size",
            "Visual width",
            "Visual height",
            "90.0 px",
            "120.5 px",
        ] {
            assert!(text.contains(expected), "{expected:?} missing in {text:?}");
        }
    }

    #[test]
    fn without_a_visual_size_only_the_switch_is_shown() {
        let mut game = editing_game();
        let (_, mut state) = open(&mut game, None);
        assert_eq!(state.property.visual_size, None);
        let ctx = egui::Context::default();
        let text = frame(&ctx, game.editor_mut().unwrap(), &mut state);
        assert!(text.contains("Separate visual size"), "{text:?}");
        assert!(!text.contains("Visual width"), "{text:?}");
    }

    #[test]
    fn a_visual_size_edit_reaches_the_shared_definition_and_leaves_the_hitbox_alone() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, None);
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        let hitbox = editor.content.entities[&path].size;
        frame(&ctx, editor, &mut state);
        assert!(!editor.dirty, "drawing alone changes nothing");

        state.property.visual_size = Some((70.0, 90.0));
        frame(&ctx, editor, &mut state);
        let entity = &editor.content.entities[&path];
        assert_eq!(entity.visual_size, Some((70.0, 90.0)));
        assert_eq!(entity.size, hitbox);
        assert!(editor.dirty);

        state.property.visual_size = None;
        frame(&ctx, editor, &mut state);
        assert_eq!(editor.content.entities[&path].visual_size, None);
    }

    #[test]
    fn an_ordinary_field_edit_keeps_the_stored_visual_size() {
        let mut game = editing_game();
        let (path, mut state) = open(&mut game, Some((64.0, 64.0)));
        state.property.fields.health = "7".into();
        let ctx = egui::Context::default();
        let editor = game.editor_mut().unwrap();
        frame(&ctx, editor, &mut state);
        let entity = &editor.content.entities[&path];
        assert_eq!(entity.health, 7);
        assert_eq!(entity.visual_size, Some((64.0, 64.0)));
    }

    #[test]
    fn an_out_of_range_visual_size_never_reaches_the_definition() {
        for bad in [(0.0, 10.0), (10.0, 5000.0), (f32::NAN, 10.0), (-3.0, 10.0)] {
            let mut game = editing_game();
            let (path, mut state) = open(&mut game, None);
            state.property.visual_size = Some(bad);
            let ctx = egui::Context::default();
            let editor = game.editor_mut().unwrap();
            frame(&ctx, editor, &mut state);
            assert!(
                visual_box::validate(editor.content.entities[&path].visual_size).is_ok(),
                "{bad:?} reached the definition"
            );
            assert!(
                editor.content.definitions().is_ok(),
                "{bad:?} left the content invalid"
            );
        }
    }

    /// The first fixture boss open in the boss panel, with its stored visual size set.
    fn open_boss(
        game: &mut hollow_steps::game::GameState,
        visual: Option<(f32, f32)>,
    ) -> (PathBuf, super::super::boss::BossPropertyUi) {
        let editor = game.editor_mut().unwrap();
        let path = editor.content.bosses.keys().next().cloned().unwrap();
        editor.content.bosses.get_mut(&path).unwrap().visual_size = visual;
        let ui = super::super::boss::BossPropertyUi::new(editor, path.clone());
        (path, ui)
    }

    fn boss_frame(
        ctx: &egui::Context,
        editor: &mut EditorSession,
        boss: &mut super::super::boss::BossPropertyUi,
    ) -> String {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 3000.0),
                )),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    super::super::boss::draw_boss_panel(ui, editor, boss)
                });
            },
        );
        drawn_text(&output)
    }

    #[test]
    fn the_boss_panel_labels_hitbox_and_visual_sizes_apart() {
        let mut game = editing_game();
        let (_, mut boss) = open_boss(&mut game, Some((150.0, 200.0)));
        assert_eq!(boss.inputs.visual_size, Some((150.0, 200.0)));
        let ctx = egui::Context::default();
        let text = boss_frame(&ctx, game.editor_mut().unwrap(), &mut boss);
        for expected in [
            "Hitbox width",
            "Hitbox height",
            "Separate visual size",
            "Visual width",
            "Visual height",
            "150.0 px",
            "200.0 px",
        ] {
            assert!(text.contains(expected), "{expected:?} missing in {text:?}");
        }
    }

    #[test]
    fn a_boss_visual_size_edit_reaches_the_definition_and_leaves_the_hitbox_alone() {
        let mut game = editing_game();
        let (path, mut boss) = open_boss(&mut game, None);
        let editor = game.editor_mut().unwrap();
        let hitbox = editor.content.bosses[&path].size;
        boss.inputs.visual_size = Some((120.0, 160.0));
        boss.sync(editor);
        assert_eq!(boss.error, None);
        assert_eq!(
            editor.content.bosses[&path].visual_size,
            Some((120.0, 160.0))
        );
        assert_eq!(editor.content.bosses[&path].size, hitbox);
        assert!(editor.dirty);
    }

    #[test]
    fn an_out_of_range_boss_visual_size_is_reported_and_never_stored() {
        for bad in [(0.0, 10.0), (10.0, 5000.0), (f32::NAN, 10.0)] {
            let mut game = editing_game();
            let (path, mut boss) = open_boss(&mut game, Some((100.0, 100.0)));
            let editor = game.editor_mut().unwrap();
            boss.inputs.visual_size = Some(bad);
            boss.sync(editor);
            assert!(
                boss.error
                    .as_deref()
                    .is_some_and(|e| e.contains("visual size")),
                "{bad:?}: {:?}",
                boss.error
            );
            assert_eq!(
                editor.content.bosses[&path].visual_size,
                Some((100.0, 100.0)),
                "{bad:?} reached the definition"
            );
        }
    }
}
