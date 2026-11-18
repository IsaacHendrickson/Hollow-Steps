//! The Player panel's health, damage, dash length and mana rows.
use super::*;
use hollow_steps::editor_history::HistoryKey;
use hollow_steps::vitals::VitalsField;

fn key_id(key: &str) -> egui::Id {
    egui::Id::new(("field", key))
}

fn vitals_id(field: VitalsField) -> egui::Id {
    key_id(field.spec().key)
}

/// Clicks a drag box by its key, types `text` over its value and presses Enter.
fn type_into_key(
    ctx: &egui::Context,
    key: &str,
    text: &str,
    editor: &mut EditorSession,
    state: &mut PlayerUi,
) -> String {
    click(ctx, key_id(key), editor, state);
    frame(ctx, vec![egui::Event::Text(text.into())], editor, state);
    frame(ctx, press_enter(), editor, state);
    frame(ctx, vec![], editor, state)
}

#[test]
fn the_panel_shows_health_damage_speed_dash_and_mana_per_kill() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let text = frame(&ctx, vec![], editor, &mut state);
    for label in [
        "Max health",
        "Invincible after a hit",
        "Sword damage",
        "Swing time",
        "Attack cooldown",
        "Move speed",
        "Dash speed",
        "Dash duration",
        "Dash cooldown",
        "Mana per kill",
        "Max mana",
        "Starting mana",
    ] {
        assert!(text.contains(label), "missing {label}\n{text}");
    }
    for field in [
        Field::MaxHealth,
        Field::StrikeDamage,
        Field::MoveSpeed,
        Field::DashSpeed,
    ] {
        assert!(rect_of(&ctx, field_id(field)).width() > 20.0);
    }
}

#[test]
fn every_vitals_row_fits_the_narrow_panel_with_a_reset_button_and_its_group_heading() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let text = frame(&ctx, vec![], editor, &mut state);
    for field in VitalsField::ALL {
        let spec = field.spec();
        assert!(text.contains(spec.label), "missing {}\n{text}", spec.label);
        assert!(text.contains(spec.group), "missing {}", spec.group);
        let rect = rect_of(&ctx, vitals_id(field));
        assert!(rect.width() > 20.0, "{} is squeezed: {rect:?}", spec.key);
        assert!(rect.right() <= PANEL_WIDTH, "{} sticks out", spec.key);
        rect_of(&ctx, egui::Id::new(("reset", spec.key)));
    }
}

#[test]
fn typing_mana_per_kill_sets_the_draft_and_reset_puts_it_back() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let key = VitalsField::ManaPerKill.spec().key;
    let text = type_into_key(&ctx, key, "25", editor, &mut state);
    assert_eq!(editor.player.edited_vitals().mana_per_kill, 25.0);
    assert!(editor.player.is_vitals_dirty() && !editor.player.is_stats_dirty());
    assert!(text.contains("Unsaved changes."), "{text}");

    click(&ctx, egui::Id::new(("reset", key)), editor, &mut state);
    assert_eq!(editor.player.edited_vitals().mana_per_kill, 10.0);
    assert!(!editor.player.is_dirty());
}

#[test]
fn a_typed_vitals_value_outside_its_range_is_refused_with_the_reason() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let key = VitalsField::ManaPerKill.spec().key;
    let text = type_into_key(&ctx, key, "-5", editor, &mut state);
    assert_eq!(editor.player.edited_vitals().mana_per_kill, 10.0);
    assert!(
        text.contains("vitals.mana_per_kill must be between"),
        "{text}"
    );
    assert!(!editor.player.is_dirty());
}

#[test]
fn a_max_mana_below_the_starting_mana_is_refused_under_its_own_field() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    editor
        .player
        .set_vitals_field(VitalsField::ManaStart, 60.0)
        .unwrap();
    let (ctx, mut state) = open_panel(editor);
    let key = VitalsField::MaxMana.spec().key;
    let text = type_into_key(&ctx, key, "40", editor, &mut state);
    assert_eq!(editor.player.edited_vitals().max_mana, 100.0, "unchanged");
    assert!(text.contains("mana_start must not be more than"), "{text}");
    // The message goes away with the next edit that works.
    let text = type_into_key(&ctx, key, "80", editor, &mut state);
    assert_eq!(editor.player.edited_vitals().max_mana, 80.0);
    assert!(!text.contains("mana_start must not be more than"), "{text}");
}

#[test]
fn the_dash_length_line_follows_the_speed_and_the_duration() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Dash length: 160 px"), "{text}");
    type_into(&ctx, Field::DashSpeed, "1000", editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Dash length: 200 px"), "{text}");
    type_into(&ctx, Field::DashDuration, "0.5", editor, &mut state);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Dash length: 500 px"), "{text}");
}

#[test]
fn typing_max_health_and_sword_damage_sets_whole_numbers_in_the_draft() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    type_into(&ctx, Field::MaxHealth, "80", editor, &mut state);
    type_into(&ctx, Field::StrikeDamage, "25", editor, &mut state);
    assert_eq!(editor.player.edited().player.max_health, 80);
    assert_eq!(editor.player.edited().player.melee.damage, 25);
    type_into(&ctx, Field::MaxHealth, "0", editor, &mut state);
    assert_eq!(editor.player.edited().player.max_health, 80, "0 is refused");
}

#[test]
fn undo_and_redo_cover_a_vitals_edit_and_reset_all_clears_it() {
    let mut game = editing_game();
    let editor = game.editor_mut().unwrap();
    let (ctx, mut state) = open_panel(editor);
    let key = VitalsField::ManaPerKill.spec().key;
    type_into_key(&ctx, key, "25", editor, &mut state);
    assert!(state.apply_history(editor, HistoryKey::Undo));
    assert_eq!(editor.player.edited_vitals().mana_per_kill, 10.0);
    assert!(state.apply_history(editor, HistoryKey::Redo));
    assert_eq!(editor.player.edited_vitals().mana_per_kill, 25.0);

    click(
        &ctx,
        egui::Id::new(("button", "Reset all")),
        editor,
        &mut state,
    );
    assert_eq!(editor.player.edited_vitals().mana_per_kill, 10.0);
    let text = frame(&ctx, vec![], editor, &mut state);
    assert!(text.contains("Matches the saved values."), "{text}");
}
