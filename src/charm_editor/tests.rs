use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::charms::{CharmCatalog, CharmState};

const BASE: f32 = 100.0;

fn draft() -> CharmDraft {
    CharmDraft::from_catalog(&CharmCatalog::default())
}

fn temp_dir() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "hollow-steps-charm-editor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn problem_fields(draft: &CharmDraft, base: f32) -> Vec<(usize, &'static str)> {
    draft
        .problems(base)
        .into_iter()
        .map(|problem| (problem.charm, problem.field))
        .collect()
}

#[test]
fn a_fresh_draft_is_clean_and_an_edit_makes_it_dirty() {
    let mut draft = draft();
    assert!(!draft.is_dirty());
    assert!(!draft.is_charm_dirty(0));
    draft.set_mana_cost(0, 20.0).unwrap();
    assert!(draft.is_dirty() && draft.is_charm_dirty(0) && !draft.is_charm_dirty(1));
    draft.reset_all();
    assert!(!draft.is_dirty());
    // Putting the same value back is clean again, not dirty.
    let original = draft.edited()[0].mana_cost;
    draft.set_mana_cost(0, 20.0).unwrap();
    draft.set_mana_cost(0, original).unwrap();
    assert!(!draft.is_dirty());
}

#[test]
fn numbers_outside_their_range_are_refused_and_change_nothing() {
    let mut draft = draft();
    let before = draft.clone();
    for bad in [-0.1, MAX_MANA_COST + 0.5, f32::NAN, f32::INFINITY] {
        assert!(draft.set_mana_cost(0, bad).is_err(), "{bad}");
    }
    // The three shipped charms are one of each kind: health, speed, damage.
    for (index, kind) in EffectKind::ALL.into_iter().enumerate() {
        let spec = kind.spec();
        assert_eq!(EffectKind::of(&draft.edited()[index].effect), kind);
        for bad in [spec.min - 0.5, spec.max + 0.5, f32::NAN, f32::NEG_INFINITY] {
            let error = draft.set_effect_amount(index, bad).unwrap_err();
            assert!(error.contains(spec.key), "{error}");
        }
    }
    assert_eq!(draft, before);
    assert!(draft.set_mana_cost(9, 1.0).is_err());
}

#[test]
fn the_edges_of_every_range_are_accepted() {
    let mut draft = draft();
    draft.set_mana_cost(0, 0.0).unwrap();
    draft.set_mana_cost(0, MAX_MANA_COST).unwrap();
    for (index, kind) in EffectKind::ALL.into_iter().enumerate() {
        for edge in [kind.spec().min, kind.spec().max] {
            draft.set_effect_amount(index, edge).unwrap();
            assert_eq!(amount(&draft.edited()[index].effect), edge);
        }
    }
}

#[test]
fn whole_number_kinds_round_what_is_set() {
    let mut draft = draft();
    draft.set_effect_amount(0, 12.6).unwrap();
    assert_eq!(draft.edited()[0].effect, Effect::MaxHealth(13));
    draft.set_effect_amount(1, 12.6).unwrap();
    assert_eq!(draft.edited()[1].effect, Effect::MoveSpeedPercent(12.6));
}

#[test]
fn text_and_color_problems_name_their_field() {
    let mut draft = draft();
    draft.set_name(0, "").unwrap();
    draft.set_name(1, &"x".repeat(33)).unwrap();
    draft.set_name(2, " Sharp ").unwrap();
    assert_eq!(
        problem_fields(&draft, BASE),
        vec![(0, "name"), (1, "name"), (2, "name")]
    );
    draft.reset_all();
    draft.set_description(0, &"d".repeat(201)).unwrap();
    draft.set_description(1, "two\nlines").unwrap();
    draft.set_color(2, [1, 2, 3, 0]).unwrap();
    assert_eq!(
        problem_fields(&draft, BASE),
        vec![(0, "description"), (1, "description"), (2, "color")]
    );
    draft.set_description(0, &"d".repeat(200)).unwrap();
    draft.set_name(0, &"n".repeat(32)).unwrap();
    assert_eq!(draft.problem_for(0, "name", BASE), None);
    assert_eq!(draft.problem_for(0, "description", BASE), None);
}

#[test]
fn an_invalid_id_is_a_problem_of_the_id() {
    let mut charm = CharmCatalog::default().charms()[0].clone();
    charm.id = "bad id!".into();
    let draft = CharmDraft::new(vec![charm]);
    assert_eq!(problem_fields(&draft, BASE), vec![(0, "id")]);
}

#[test]
fn a_cost_above_the_base_max_mana_is_a_problem_at_exactly_the_base_it_is_not() {
    let mut draft = draft();
    draft.set_mana_cost(0, BASE).unwrap();
    assert!(draft.problems(BASE).is_empty());
    let message = draft.problem_for(0, "mana_cost", BASE - 1.0).unwrap();
    assert!(message.contains("base max mana (99)"), "{message}");
    // Only the charms above the base are flagged.
    draft.reset_all();
    assert_eq!(
        problem_fields(&draft, 12.0),
        vec![(0, "mana_cost"), (2, "mana_cost")]
    );
    assert!(draft.validate(12.0).unwrap_err().contains("Heart Charm"));
    assert_eq!(
        draft.catalog(12.0).unwrap_err(),
        draft.validate(12.0).unwrap_err()
    );
}

#[test]
fn names_and_ids_must_be_unique_ignoring_case() {
    let mut draft = draft();
    draft.set_name(1, "HEART charm").unwrap();
    let message = draft.problem_for(1, "name", BASE).unwrap();
    assert!(message.contains("ignoring case"), "{message}");
    assert_eq!(draft.problem_for(0, "name", BASE), None);
    draft.reset_all();
    let mut charms = draft.edited().to_vec();
    charms[2].id = charms[0].id.clone();
    let draft = CharmDraft::new(charms);
    assert_eq!(problem_fields(&draft, BASE), vec![(2, "id")]);
}

#[test]
fn problems_agree_with_what_a_catalog_accepts() {
    let base = draft();
    let mutations: [fn(&mut CharmDraft); 8] = [
        |_| {},
        |d| d.set_name(0, "").unwrap(),
        |d| d.set_name(0, "Swift Charm").unwrap(),
        |d| d.set_name(0, "Heart ").unwrap(),
        |d| d.set_description(0, "a\nb").unwrap(),
        |d| d.set_mana_cost(0, 150.0).unwrap(),
        |d| d.set_color(0, [0, 0, 0, 0]).unwrap(),
        |d| {
            d.new_charm();
        },
    ];
    for (n, mutate) in mutations.into_iter().enumerate() {
        let mut draft = base.clone();
        mutate(&mut draft);
        assert_eq!(
            draft.problems(BASE).is_empty(),
            CharmCatalog::new(draft.edited().to_vec(), BASE).is_ok(),
            "mutation {n}"
        );
        assert_eq!(draft.catalog(BASE).is_ok(), draft.validate(BASE).is_ok());
    }
}

#[test]
fn changing_the_kind_keeps_the_amount_inside_the_new_range() {
    let mut draft = draft();
    draft.set_effect_amount(0, 500.0).unwrap();
    draft
        .set_effect_kind(0, EffectKind::MoveSpeedPercent)
        .unwrap();
    assert_eq!(
        draft.edited()[0].effect,
        Effect::MoveSpeedPercent(MAX_SPEED_PERCENT)
    );
    draft.set_effect_kind(0, EffectKind::SwordDamage).unwrap();
    assert_eq!(draft.edited()[0].effect, Effect::SwordDamage(200));
    assert!(draft.problems(BASE).is_empty());
}

#[test]
fn new_charm_takes_the_next_free_id_and_name_and_is_valid() {
    let mut draft = draft();
    let first = draft.new_charm();
    assert_eq!(first, 3);
    let charm = &draft.edited()[first];
    assert_eq!(
        (charm.id.as_str(), charm.name.as_str()),
        ("charm_1", "Charm 1")
    );
    assert!(!charm.owned_at_start && charm.mana_cost == 0.0);
    assert!(draft.problems(BASE).is_empty());
    assert!(draft.is_dirty() && draft.is_charm_dirty(first));
    // A taken name (ignoring case) or a taken id skips that number.
    draft.set_name(first, "charm 2").unwrap();
    let second = draft.new_charm();
    assert_eq!(draft.edited()[second].id, "charm_3");
    assert!(draft.problems(BASE).is_empty());
    let third = draft.new_charm();
    assert_eq!(draft.edited()[third].name, "Charm 4");
    assert!(draft.problems(BASE).is_empty());
}

#[test]
fn delete_removes_the_charm_and_reset_all_brings_it_back() {
    let mut draft = draft();
    let removed = draft.delete(0).unwrap();
    assert_eq!(removed.id, "heart");
    assert_eq!(draft.edited().len(), 2);
    assert_eq!(draft.deleted_ids(), vec!["heart"]);
    assert!(draft.is_dirty());
    assert!(draft.delete(5).is_err());
    draft.reset_all();
    assert_eq!(draft.edited()[0].id, "heart");
    assert!(draft.deleted_ids().is_empty());
}

#[test]
fn deleting_a_charm_unequips_it_when_the_list_is_applied() {
    let catalog = CharmCatalog::default();
    let mut state = CharmState::starting(&catalog);
    state.equip(&catalog, "heart", BASE).unwrap();
    state.equip(&catalog, "swift", BASE).unwrap();
    let mut draft = CharmDraft::from_catalog(&catalog);
    draft.delete(0).unwrap();
    let applied = draft.catalog(BASE).unwrap();
    // In effect at once: the deleted charm no longer counts.
    assert_eq!(state.equipped_charms(&applied).len(), 1);
    assert_eq!(state.max_mana(&applied, BASE), BASE - 10.0);
    // And gone for good once the state forgets what the list no longer has.
    state.retain_known(&applied);
    assert!(!state.is_owned("heart") && !state.is_equipped("heart"));
    assert_eq!(state.equipped(), ["swift".to_owned()]);
}

#[test]
fn reset_one_charm_restores_it_in_place_and_removes_a_new_one() {
    let mut draft = draft();
    draft.set_mana_cost(1, 40.0).unwrap();
    draft.set_name(2, "Edge").unwrap();
    draft.reset_charm(1);
    assert_eq!(draft.edited()[1].mana_cost, 10.0);
    assert_eq!(
        draft.edited()[2].name,
        "Edge",
        "other charms keep their edits"
    );
    let new = draft.new_charm();
    draft.reset_charm(new);
    assert_eq!(draft.edited().len(), 3);
    draft.reset_charm(7); // nothing there: nothing happens
    assert_eq!(draft.edited().len(), 3);
}

#[test]
fn save_writes_the_file_marks_it_saved_and_loads_back_the_same() {
    let dir = temp_dir();
    let mut draft = draft();
    draft.set_mana_cost(0, 33.5).unwrap();
    draft.set_effect_amount(1, 22.5).unwrap();
    draft.set_name(2, "Edge Charm").unwrap();
    let new = draft.new_charm();
    draft.set_color(new, [10, 20, 30, 200]).unwrap();
    draft.set_owned_at_start(new, true).unwrap();
    let path = draft.save(&dir, BASE).unwrap();
    assert_eq!(path, dir.join(FILE_NAME));
    assert!(!draft.is_dirty());
    assert!(
        !dir.join("charms.ron.tmp").exists(),
        "no staging file is left"
    );
    let loaded = CharmCatalog::load(&dir, BASE).unwrap();
    assert_eq!(loaded.charms(), draft.edited());
    assert_eq!(CharmDraft::from_catalog(&loaded), draft);
}

#[test]
fn an_invalid_draft_writes_nothing_and_stays_dirty() {
    let dir = temp_dir();
    let mut draft = draft();
    draft.set_name(0, "").unwrap();
    let error = draft.save(&dir, BASE).unwrap_err();
    assert!(error.contains("name"), "{error}");
    assert!(draft.is_dirty());
    assert!(!dir.join(FILE_NAME).exists() && !dir.join("charms.ron.tmp").exists());
    // A valid file already there is left alone by a refused save.
    draft.reset_all();
    draft.save(&dir, BASE).unwrap();
    let written = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
    draft.set_mana_cost(0, 500.0).unwrap();
    assert!(draft.save(&dir, BASE).is_err());
    assert_eq!(
        std::fs::read_to_string(dir.join(FILE_NAME)).unwrap(),
        written
    );
}

#[test]
fn every_field_a_charm_writes_has_a_table_entry() {
    let text = ron::ser::to_string(&CharmCatalog::default().charms()[0]).unwrap();
    let value: ron::Value = ron::from_str(&text).unwrap();
    let ron::Value::Map(map) = value else {
        panic!("a charm is written as a struct: {text}");
    };
    let written: Vec<String> = map
        .iter()
        .map(|(key, _)| match key {
            ron::Value::String(key) => key.clone(),
            other => panic!("unexpected key {other:?}"),
        })
        .collect();
    assert_eq!(written.len(), FIELDS.len(), "{written:?}");
    for key in &written {
        assert!(
            FIELDS.iter().any(|field| field.key == key),
            "the charm field `{key}` has no entry in FIELDS"
        );
    }
    for field in FIELDS {
        assert!(written.iter().any(|key| key == field.key), "{}", field.key);
    }
}

#[test]
fn the_amount_table_matches_the_rules_of_charms_rs() {
    for (index, kind) in EffectKind::ALL.into_iter().enumerate() {
        assert_eq!(kind as usize, index, "AMOUNTS is in EffectKind::ALL order");
        let spec = kind.spec();
        assert!(spec.key.ends_with(&format!("{:?}", kind)), "{}", spec.key);
        assert!(kind.effect(spec.min).validate().is_ok(), "{kind:?} min");
        assert!(kind.effect(spec.max).validate().is_ok(), "{kind:?} max");
        // One unit outside is refused by the rules too.
        assert!(kind.effect(spec.max + 1.0).validate().is_err(), "{kind:?}");
        assert!(kind.effect(spec.min - 1.0).validate().is_err(), "{kind:?}");
    }
}
