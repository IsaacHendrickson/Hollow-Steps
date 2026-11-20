use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::charm_editor::EffectKind;
use crate::charms::{MAX_SPEED_PERCENT, MIN_SPEED_PERCENT};

/// Three upgrades written for the tests (never the shipped numbers, which the owner tunes): one of
/// each kind, so index 0 is health, 1 speed and 2 damage.
const SAMPLE: &str = r#"(upgrades: [
    (id: "vitality", name: "Vitality", description: "More health.",
     effect: MaxHealth(10), costs: [30, 60, 100]),
    (id: "swift_feet", name: "Swift Feet", description: "Faster.",
     effect: MoveSpeedPercent(5.0), costs: [40, 80, 140]),
    (id: "keen_edge", name: "Keen Edge", description: "Sharper.",
     effect: SwordDamage(3), costs: [50, 100, 180]),
])"#;

fn catalog() -> UpgradeCatalog {
    UpgradeCatalog::from_ron(SAMPLE).expect("the sample catalog is valid")
}

fn draft() -> UpgradeDraft {
    UpgradeDraft::from_catalog(&catalog())
}

fn temp_dir() -> PathBuf {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "hollow-steps-upgrade-editor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn problem_fields(draft: &UpgradeDraft) -> Vec<(usize, &'static str)> {
    draft
        .problems()
        .into_iter()
        .map(|problem| (problem.upgrade, problem.field))
        .collect()
}

// ---- dirty and clean -------------------------------------------------------------------

#[test]
fn a_fresh_draft_is_clean_and_an_edit_makes_it_dirty() {
    let mut draft = draft();
    assert!(!draft.is_dirty());
    assert!(!draft.is_upgrade_dirty(0));
    draft.set_cost(0, 0, 35.0).unwrap();
    assert!(draft.is_dirty() && draft.is_upgrade_dirty(0) && !draft.is_upgrade_dirty(1));
    draft.reset_all();
    assert!(!draft.is_dirty());
    // Putting the same value back is clean again, not dirty.
    draft.set_cost(0, 0, 35.0).unwrap();
    draft.set_cost(0, 0, 30.0).unwrap();
    assert!(!draft.is_dirty());
    // Every kind of edit counts.
    for edit in [
        (|d: &mut UpgradeDraft| d.set_name(1, "Quick Feet").unwrap()) as fn(&mut UpgradeDraft),
        |d| d.set_description(1, "Quicker.").unwrap(),
        |d| d.set_effect_amount(1, 6.0).unwrap(),
        |d| d.set_effect_kind(1, EffectKind::SwordDamage).unwrap(),
        |d| d.add_level(1).unwrap(),
        |d| d.remove_level(1).unwrap(),
        |d| {
            d.new_upgrade();
        },
        |d| {
            d.delete(1).unwrap();
        },
    ] {
        let mut draft = draft.clone();
        edit(&mut draft);
        assert!(draft.is_dirty());
    }
}

// ---- every field's range ----------------------------------------------------------------

#[test]
fn a_price_outside_its_range_is_refused_and_changes_nothing() {
    let mut draft = draft();
    let before = draft.clone();
    for bad in [0.0, 0.4, -5.0, MAX_COST + 1.0, f32::NAN, f32::INFINITY] {
        let error = draft.set_cost(0, 1, bad).unwrap_err();
        assert!(error.contains("costs.2"), "{bad}: {error}");
    }
    assert_eq!(draft, before);
    // A level the upgrade does not have, and an upgrade that is not there.
    assert!(draft.set_cost(0, 3, 10.0).is_err());
    assert!(draft.set_cost(0, MAX_LEVELS, 10.0).is_err());
    assert!(draft.set_cost(9, 0, 10.0).is_err());
    assert_eq!(draft, before);
}

#[test]
fn the_edges_of_the_price_range_are_accepted_and_prices_are_whole() {
    let mut draft = draft();
    draft.set_cost(0, 0, 1.0).unwrap();
    assert_eq!(draft.edited()[0].costs[0], 1);
    draft.set_cost(0, 0, MAX_COST).unwrap();
    assert_eq!(draft.edited()[0].costs[0], 1_000_000);
    draft.set_cost(0, 2, 12.6).unwrap();
    assert_eq!(draft.edited()[0].costs[2], 13);
    assert!(draft.problems().is_empty());
}

#[test]
fn an_amount_outside_the_range_of_its_kind_is_refused_and_changes_nothing() {
    let mut draft = draft();
    let before = draft.clone();
    for (index, kind) in EffectKind::ALL.into_iter().enumerate() {
        let spec = kind.spec();
        assert_eq!(EffectKind::of(&draft.edited()[index].effect), kind);
        for bad in [spec.min - 0.5, spec.max + 0.5, f32::NAN, f32::NEG_INFINITY] {
            let error = draft.set_effect_amount(index, bad).unwrap_err();
            assert!(error.contains(spec.key), "{error}");
        }
    }
    assert_eq!(draft, before);
    assert!(draft.set_effect_amount(9, 1.0).is_err());
}

#[test]
fn the_edges_of_every_amount_range_are_accepted_and_whole_kinds_round() {
    let mut draft = draft();
    for (index, kind) in EffectKind::ALL.into_iter().enumerate() {
        for edge in [kind.spec().min, kind.spec().max] {
            draft.set_effect_amount(index, edge).unwrap();
            assert_eq!(amount(&draft.edited()[index].effect), edge);
        }
    }
    draft.set_effect_amount(0, 12.6).unwrap();
    assert_eq!(draft.edited()[0].effect, Effect::MaxHealth(13));
    draft.set_effect_amount(1, 12.6).unwrap();
    assert_eq!(draft.edited()[1].effect, Effect::MoveSpeedPercent(12.6));
    assert!(draft.problems().is_empty());
}

#[test]
fn the_amount_range_is_the_one_the_rules_enforce_and_is_labeled_per_level() {
    for kind in EffectKind::ALL {
        let spec = amount_spec(kind);
        assert_eq!(spec.label, "Per level");
        assert_eq!(
            (spec.min, spec.max, spec.key),
            (kind.spec().min, kind.spec().max, kind.spec().key)
        );
        assert!(kind.effect(spec.min).validate().is_ok(), "{kind:?} min");
        assert!(kind.effect(spec.max).validate().is_ok(), "{kind:?} max");
        assert!(kind.effect(spec.max + 1.0).validate().is_err(), "{kind:?}");
        assert!(kind.effect(spec.min - 1.0).validate().is_err(), "{kind:?}");
    }
    assert_eq!(
        amount_spec(EffectKind::MoveSpeedPercent).min,
        MIN_SPEED_PERCENT
    );
    assert_eq!(
        amount_spec(EffectKind::MoveSpeedPercent).max,
        MAX_SPEED_PERCENT
    );
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
    assert!(draft.problems().is_empty());
}

#[test]
fn the_number_of_levels_stays_between_one_and_ten() {
    let mut draft = draft();
    // Up to ten: each new level is priced like the one before it.
    for _ in 3..MAX_LEVELS {
        draft.add_level(0).unwrap();
    }
    assert_eq!(draft.edited()[0].costs.len(), MAX_LEVELS);
    assert_eq!(draft.edited()[0].costs[9], 100);
    let error = draft.add_level(0).unwrap_err();
    assert!(error.contains("at most 10"), "{error}");
    assert_eq!(draft.edited()[0].costs.len(), MAX_LEVELS);
    assert!(draft.problems().is_empty());
    // Down to one.
    for _ in 1..MAX_LEVELS {
        draft.remove_level(0).unwrap();
    }
    assert_eq!(draft.edited()[0].costs, [30]);
    let error = draft.remove_level(0).unwrap_err();
    assert!(error.contains("at least one"), "{error}");
    assert_eq!(draft.edited()[0].costs, [30]);
    assert!(draft.add_level(7).is_err() && draft.remove_level(7).is_err());
}

#[test]
fn a_price_above_the_ceiling_is_a_problem_the_catalog_agrees_with() {
    let mut draft = draft();
    draft.edited[0].costs[2] = upgrades::MAX_COST;
    assert!(draft.problems().is_empty() && draft.catalog().is_ok());
    draft.edited[0].costs[2] = upgrades::MAX_COST + 1;
    assert_eq!(problem_fields(&draft), vec![(0, "costs")]);
    let message = draft.problem_for(0, "costs").unwrap();
    assert!(
        message.contains("level 3 must be at most 1000000"),
        "{message}"
    );
    assert!(draft.catalog().is_err());
    assert!(draft.save(&temp_dir()).is_err());
}

#[test]
fn a_list_with_no_levels_or_too_many_or_a_free_level_is_a_problem_of_the_costs() {
    let mut upgrades = catalog().upgrades().to_vec();
    upgrades[0].costs.clear();
    upgrades[1].costs = vec![5; MAX_LEVELS + 1];
    upgrades[2].costs = vec![5, 0, 5, 0];
    let draft = UpgradeDraft::new(upgrades);
    assert_eq!(
        problem_fields(&draft),
        vec![(0, "costs"), (1, "costs"), (2, "costs"), (2, "costs")]
    );
    let messages: Vec<String> = draft.problems().into_iter().map(|p| p.message).collect();
    assert!(
        messages[0].contains("1 to 10 levels, not 0"),
        "{messages:?}"
    );
    assert!(messages[1].contains("not 11"), "{messages:?}");
    assert!(
        messages[2].contains("level 2 must be above 0"),
        "{messages:?}"
    );
    assert!(
        messages[3].contains("level 4 must be above 0"),
        "{messages:?}"
    );
    assert!(draft.validate().unwrap_err().contains("Vitality"));
}

// ---- text, ids, unique names -------------------------------------------------------------

#[test]
fn text_problems_name_their_field() {
    let mut draft = draft();
    draft.set_name(0, "").unwrap();
    draft.set_name(1, &"x".repeat(33)).unwrap();
    draft.set_name(2, " Edge ").unwrap();
    assert_eq!(
        problem_fields(&draft),
        vec![(0, "name"), (1, "name"), (2, "name")]
    );
    draft.reset_all();
    draft.set_description(0, &"d".repeat(201)).unwrap();
    draft.set_description(1, "two\nlines").unwrap();
    assert_eq!(
        problem_fields(&draft),
        vec![(0, "description"), (1, "description")]
    );
    draft.set_description(0, &"d".repeat(200)).unwrap();
    draft.set_name(0, &"n".repeat(32)).unwrap();
    assert_eq!(draft.problem_for(0, "name"), None);
    assert_eq!(draft.problem_for(0, "description"), None);
}

#[test]
fn an_invalid_id_is_a_problem_of_the_id() {
    let mut upgrade = catalog().upgrades()[0].clone();
    upgrade.id = "bad id!".into();
    let draft = UpgradeDraft::new(vec![upgrade]);
    assert_eq!(problem_fields(&draft), vec![(0, "id")]);
}

#[test]
fn names_and_ids_must_be_unique_ignoring_case() {
    let mut draft = draft();
    draft.set_name(1, "VITALITY").unwrap();
    let message = draft.problem_for(1, "name").unwrap();
    assert!(message.contains("ignoring case"), "{message}");
    assert_eq!(draft.problem_for(0, "name"), None);
    draft.reset_all();
    let mut upgrades = draft.edited().to_vec();
    upgrades[2].id = upgrades[0].id.clone();
    let draft = UpgradeDraft::new(upgrades);
    assert_eq!(problem_fields(&draft), vec![(2, "id")]);
    assert!(draft.problem_for(2, "id").unwrap().contains("used twice"));
}

#[test]
fn problems_agree_with_what_a_catalog_accepts() {
    let base = draft();
    let mutations: [fn(&mut UpgradeDraft); 10] = [
        |_| {},
        |d| d.set_name(0, "").unwrap(),
        |d| d.set_name(0, "Swift Feet").unwrap(),
        |d| d.set_name(0, "Vitality ").unwrap(),
        |d| d.set_description(0, "a\nb").unwrap(),
        |d| d.edited[0].costs[1] = 0,
        |d| d.edited[0].costs.clear(),
        |d| d.edited[0].id = "x y".into(),
        |d| d.edited[0].effect = Effect::MaxHealth(0),
        |d| {
            d.new_upgrade();
        },
    ];
    for (n, mutate) in mutations.into_iter().enumerate() {
        let mut draft = base.clone();
        mutate(&mut draft);
        assert_eq!(
            draft.problems().is_empty(),
            UpgradeCatalog::new(draft.edited().to_vec()).is_ok(),
            "mutation {n}"
        );
        assert_eq!(draft.catalog().is_ok(), draft.validate().is_ok());
    }
}

// ---- New, Delete, Reset --------------------------------------------------------------------

#[test]
fn new_upgrade_takes_the_next_free_id_and_name_and_is_valid() {
    let mut draft = draft();
    let first = draft.new_upgrade();
    assert_eq!(first, 3);
    let upgrade = &draft.edited()[first];
    assert_eq!(
        (upgrade.id.as_str(), upgrade.name.as_str()),
        ("upgrade_1", "Upgrade 1")
    );
    assert_eq!(
        (upgrade.effect, upgrade.costs.as_slice()),
        (Effect::MaxHealth(5), [10].as_slice())
    );
    assert!(draft.problems().is_empty());
    assert!(draft.is_dirty() && draft.is_upgrade_dirty(first));
    // A taken name (ignoring case) or a taken id skips that number.
    draft.set_name(first, "upgrade 2").unwrap();
    let second = draft.new_upgrade();
    assert_eq!(draft.edited()[second].id, "upgrade_3");
    assert!(draft.problems().is_empty());
    let third = draft.new_upgrade();
    assert_eq!(draft.edited()[third].name, "Upgrade 4");
    assert!(draft.problems().is_empty());
}

#[test]
fn delete_removes_the_upgrade_and_reset_all_brings_it_back() {
    let mut draft = draft();
    let removed = draft.delete(0).unwrap();
    assert_eq!(removed.id, "vitality");
    assert_eq!(draft.edited().len(), 2);
    assert_eq!(draft.deleted_ids(), vec!["vitality"]);
    assert!(draft.is_dirty());
    assert!(draft.delete(5).is_err());
    // Deleting everything is allowed (an empty shop) and still a valid list.
    draft.delete(0).unwrap();
    draft.delete(0).unwrap();
    assert!(draft.edited().is_empty() && draft.problems().is_empty());
    assert!(draft.catalog().unwrap().upgrades().is_empty());
    draft.reset_all();
    assert_eq!(draft.edited()[0].id, "vitality");
    assert!(draft.deleted_ids().is_empty());
}

#[test]
fn deleting_leaves_the_levels_in_the_profile_with_no_effect() {
    use crate::profile::Profile;
    use crate::upgrades::{level, permanent_effects};

    let mut profile = Profile::default();
    profile.upgrades.insert("vitality".into(), 2);
    let mut draft = draft();
    assert_eq!(
        permanent_effects(&profile, &draft.catalog().unwrap()).len(),
        2
    );
    draft.delete(0).unwrap();
    let applied = draft.catalog().unwrap();
    assert!(permanent_effects(&profile, &applied).is_empty());
    assert_eq!(level(&profile, "vitality"), 2, "the profile is untouched");
    // Listed again, the levels count again.
    draft.reset_all();
    assert_eq!(
        permanent_effects(&profile, &draft.catalog().unwrap()).len(),
        2
    );
}

#[test]
fn reset_one_upgrade_restores_it_in_place_and_removes_a_new_one() {
    let mut draft = draft();
    draft.set_cost(1, 0, 99.0).unwrap();
    draft.set_name(2, "Edge").unwrap();
    draft.reset_upgrade(1);
    assert_eq!(draft.edited()[1].costs[0], 40);
    assert_eq!(
        draft.edited()[2].name,
        "Edge",
        "other upgrades keep their edits"
    );
    let new = draft.new_upgrade();
    draft.reset_upgrade(new);
    assert_eq!(draft.edited().len(), 3);
    draft.reset_upgrade(7); // nothing there: nothing happens
    assert_eq!(draft.edited().len(), 3);
}

// ---- saving ----------------------------------------------------------------------------------

#[test]
fn save_writes_the_file_marks_it_saved_and_loads_back_the_same() {
    let dir = temp_dir();
    let mut draft = draft();
    draft.set_cost(0, 0, 33.0).unwrap();
    draft.set_effect_amount(1, 7.5).unwrap();
    draft.set_name(2, "Edge").unwrap();
    draft.add_level(2).unwrap();
    let new = draft.new_upgrade();
    draft
        .set_effect_kind(new, EffectKind::MoveSpeedPercent)
        .unwrap();
    draft.set_description(new, "Brand new.").unwrap();
    let path = draft.save(&dir).unwrap();
    assert_eq!(path, dir.join(FILE_NAME));
    assert!(!draft.is_dirty());
    assert!(
        !dir.join("upgrades.ron.tmp").exists(),
        "no staging file is left"
    );
    let loaded = UpgradeCatalog::load(&dir).unwrap();
    assert_eq!(loaded.upgrades(), draft.edited());
    assert_eq!(UpgradeDraft::from_catalog(&loaded), draft);
}

#[test]
fn an_invalid_draft_writes_nothing_and_stays_dirty() {
    let dir = temp_dir();
    let mut draft = draft();
    draft.set_name(0, "").unwrap();
    let error = draft.save(&dir).unwrap_err();
    assert!(error.contains("name"), "{error}");
    assert!(draft.is_dirty());
    assert!(!dir.join(FILE_NAME).exists() && !dir.join("upgrades.ron.tmp").exists());
    // A valid file already there is left alone by a refused save.
    draft.reset_all();
    draft.save(&dir).unwrap();
    let written = std::fs::read_to_string(dir.join(FILE_NAME)).unwrap();
    draft.edited[0].costs[0] = 0;
    assert!(draft.save(&dir).unwrap_err().contains("above 0"));
    assert_eq!(
        std::fs::read_to_string(dir.join(FILE_NAME)).unwrap(),
        written
    );
    assert!(!dir.join("upgrades.ron.tmp").exists());
}

// ---- the field table ---------------------------------------------------------------------------

#[test]
fn every_field_an_upgrade_writes_has_a_table_entry() {
    let text = ron::ser::to_string(&catalog().upgrades()[0]).unwrap();
    let value: ron::Value = ron::from_str(&text).unwrap();
    let ron::Value::Map(map) = value else {
        panic!("an upgrade is written as a struct: {text}");
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
            "the upgrade field `{key}` has no entry in FIELDS"
        );
    }
    for field in FIELDS {
        assert!(written.iter().any(|key| key == field.key), "{}", field.key);
    }
}

#[test]
fn the_price_table_has_one_range_per_possible_level() {
    assert_eq!(COSTS.len(), MAX_LEVELS);
    for (index, spec) in COSTS.iter().enumerate() {
        assert_eq!(spec.key, format!("costs.{}", index + 1));
        assert_eq!(spec.label, format!("Level {}", index + 1));
        assert!(spec.integer && spec.min == 1.0 && spec.max == MAX_COST);
    }
}

// ---- totals --------------------------------------------------------------------------------------

#[test]
fn the_draft_totals_follow_the_edited_list_even_when_it_has_a_problem() {
    let mut draft = draft();
    assert_eq!(
        draft.totals().text(),
        "All upgrades at maximum: +30 max health, +15% move speed, +9 sword damage"
    );
    draft.add_level(0).unwrap();
    draft.set_name(0, "").unwrap();
    assert!(draft.validate().is_err());
    assert_eq!(draft.totals().max_health, 40);
}
