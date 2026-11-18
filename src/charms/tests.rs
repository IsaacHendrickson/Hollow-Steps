use super::*;

/// The base maximum mana the tests budget against.
const MANA: f32 = 100.0;

fn charm(id: &str, name: &str, cost: f32, effect: Effect) -> Charm {
    Charm {
        id: id.into(),
        name: name.into(),
        description: "A test charm.".into(),
        mana_cost: cost,
        effect,
        color: [10, 20, 30, 255],
        owned_at_start: true,
    }
}

fn catalog(charms: Vec<Charm>) -> CharmCatalog {
    CharmCatalog { charms }
}

/// Heart (+10 health, 15), swift (+15 %, 10), sharp (+5 damage, 20) and a ward that is not
/// owned at the start (+20 health, 5).
fn sample() -> CharmCatalog {
    let mut ward = charm("ward", "Ward Charm", 5.0, Effect::MaxHealth(20));
    ward.owned_at_start = false;
    catalog(vec![
        charm("heart", "Heart Charm", 15.0, Effect::MaxHealth(10)),
        charm("swift", "Swift Charm", 10.0, Effect::MoveSpeedPercent(15.0)),
        charm("sharp", "Sharp Charm", 20.0, Effect::SwordDamage(5)),
        ward,
    ])
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} is not close to {b}");
}

/// A file with one charm line written by hand.
fn file_with(body: &str) -> String {
    format!("(charms: [({body})])")
}

const GOOD: &str = r#"id: "a", name: "A", description: "", mana_cost: 10.0,
    effect: MaxHealth(5), color: (1, 2, 3, 255), owned_at_start: true"#;

fn parse(text: &str) -> Result<CharmCatalog, String> {
    CharmCatalog::from_ron(text, MANA)
}

// ---- the file --------------------------------------------------------------------------

#[test]
fn the_shipped_file_loads_and_has_the_three_starting_charms() {
    let shipped = CharmCatalog::default();
    shipped.validate(MANA).unwrap();
    let ids = shipped.charms().iter().map(|c| c.id.as_str());
    assert_eq!(ids.collect::<Vec<_>>(), ["heart", "swift", "sharp"]);
    assert!(matches!(
        shipped.get("heart").unwrap().effect,
        Effect::MaxHealth(_)
    ));
    assert!(matches!(
        shipped.get("swift").unwrap().effect,
        Effect::MoveSpeedPercent(_)
    ));
    assert!(matches!(
        shipped.get("sharp").unwrap().effect,
        Effect::SwordDamage(_)
    ));
    // With no assets/charms.ron the embedded file is what loads.
    let empty = std::env::temp_dir().join("hollow-steps-no-charms-here");
    assert_eq!(CharmCatalog::load(&empty, MANA).unwrap(), shipped);
}

#[test]
fn a_valid_file_loads_and_an_empty_list_is_allowed() {
    let loaded = parse(&file_with(GOOD)).unwrap();
    assert_eq!(loaded.charms().len(), 1);
    assert_eq!(loaded.get("a").unwrap().effect, Effect::MaxHealth(5));
    assert!(parse("(charms: [])").unwrap().charms().is_empty());
}

#[test]
fn an_unknown_field_or_effect_is_refused() {
    let extra = file_with(&format!("{GOOD}, speed: 3"));
    assert!(parse(&extra).unwrap_err().contains("speed"));
    let effect = GOOD.replace("MaxHealth(5)", "Teleport(5)");
    assert!(parse(&file_with(&effect)).is_err());
    assert!(parse("(charms: [], extra: 1)").is_err());
}

#[test]
fn a_duplicate_id_or_name_is_refused() {
    let one = charm("a", "Alpha", 1.0, Effect::MaxHealth(1));
    let same_id = charm("a", "Beta", 1.0, Effect::MaxHealth(1));
    let error = catalog(vec![one.clone(), same_id])
        .validate(MANA)
        .unwrap_err();
    assert!(
        error.contains("\"a\"") && error.contains("twice"),
        "{error}"
    );
    // Names are compared ignoring case.
    let same_name = charm("b", "ALPHA", 1.0, Effect::MaxHealth(1));
    let error = catalog(vec![one, same_name]).validate(MANA).unwrap_err();
    assert!(
        error.contains("ALPHA") && error.contains("twice"),
        "{error}"
    );
}

#[test]
fn names_are_one_to_thirty_two_characters_without_edge_spaces() {
    let named = |name: &str| charm("a", name, 1.0, Effect::MaxHealth(1)).validate(MANA);
    assert!(named("x").is_ok());
    assert!(named(&"x".repeat(MAX_NAME_CHARS)).is_ok());
    assert!(named("").is_err());
    assert!(named("   ").is_err());
    assert!(named(&"x".repeat(MAX_NAME_CHARS + 1)).is_err());
    assert!(named(" Heart").is_err());
    assert!(named("Heart ").is_err());
    // Characters, not bytes.
    assert!(named(&"é".repeat(MAX_NAME_CHARS)).is_ok());
}

#[test]
fn ids_are_short_and_plain() {
    let with_id = |id: &str| charm(id, "Name", 1.0, Effect::MaxHealth(1)).validate(MANA);
    assert!(with_id("heart_2-b").is_ok());
    assert!(with_id("").is_err());
    assert!(with_id("two words").is_err());
    assert!(with_id("naïve").is_err());
    assert!(with_id(&"i".repeat(MAX_ID_CHARS + 1)).is_err());
}

#[test]
fn the_cost_is_zero_up_to_the_base_max_mana() {
    let cost = |cost: f32, base: f32| charm("a", "A", cost, Effect::MaxHealth(1)).validate(base);
    assert!(cost(0.0, MANA).is_ok());
    assert!(cost(MANA, MANA).is_ok());
    assert!(cost(MANA + 0.5, MANA).unwrap_err().contains("mana_cost"));
    assert!(cost(-1.0, MANA).is_err());
    assert!(cost(f32::NAN, MANA).is_err());
    assert!(cost(f32::INFINITY, MANA).is_err());
    // The limit is the base max mana given, not a fixed number.
    assert!(cost(20.0, 12.0).is_err());
    assert!(CharmCatalog::from_ron(&file_with(GOOD), 5.0).is_err());
}

#[test]
fn effect_amounts_stay_in_range() {
    let effect = |effect| charm("a", "A", 1.0, effect).validate(MANA);
    assert!(effect(Effect::MaxHealth(1)).is_ok());
    assert!(effect(Effect::MaxHealth(MAX_HEALTH_BONUS)).is_ok());
    assert!(effect(Effect::MaxHealth(0)).is_err());
    assert!(effect(Effect::MaxHealth(MAX_HEALTH_BONUS + 1)).is_err());
    assert!(effect(Effect::SwordDamage(0)).is_err());
    assert!(effect(Effect::SwordDamage(MAX_DAMAGE_BONUS + 1)).is_err());
    assert!(effect(Effect::MoveSpeedPercent(15.0)).is_ok());
    assert!(effect(Effect::MoveSpeedPercent(0.0)).is_err());
    assert!(effect(Effect::MoveSpeedPercent(-5.0)).is_err());
    assert!(effect(Effect::MoveSpeedPercent(MAX_SPEED_PERCENT + 1.0)).is_err());
    assert!(effect(Effect::MoveSpeedPercent(f32::NAN)).is_err());
}

#[test]
fn a_bad_color_is_refused() {
    // Out of range for a byte, or the wrong number of parts: the file does not parse.
    let bad = GOOD.replace("(1, 2, 3, 255)", "(1, 2, 3, 300)");
    assert!(parse(&file_with(&bad)).is_err());
    let short = GOOD.replace("(1, 2, 3, 255)", "(1, 2, 3)");
    assert!(parse(&file_with(&short)).is_err());
    let negative = GOOD.replace("(1, 2, 3, 255)", "(1, -2, 3, 255)");
    assert!(parse(&file_with(&negative)).is_err());
    // A swatch nobody could see.
    let clear = GOOD.replace("(1, 2, 3, 255)", "(1, 2, 3, 0)");
    assert!(parse(&file_with(&clear))
        .unwrap_err()
        .contains("transparent"));
}

#[test]
fn descriptions_are_one_short_line() {
    let described = |text: &str| {
        let mut charm = charm("a", "A", 1.0, Effect::MaxHealth(1));
        charm.description = text.into();
        charm.validate(MANA)
    };
    assert!(described("").is_ok());
    assert!(described(&"d".repeat(MAX_DESCRIPTION_CHARS)).is_ok());
    assert!(described(&"d".repeat(MAX_DESCRIPTION_CHARS + 1)).is_err());
    assert!(described("two\nlines").is_err());
}

#[test]
fn effects_read_as_text() {
    assert_eq!(Effect::MaxHealth(10).text(), "+10 max health");
    assert_eq!(Effect::SwordDamage(5).text(), "+5 sword damage");
    assert_eq!(Effect::MoveSpeedPercent(15.0).text(), "+15% move speed");
    assert_eq!(Effect::MoveSpeedPercent(7.5).text(), "+7.5% move speed");
    assert_eq!(number_text(15.0), "15");
    assert_eq!(number_text(0.0), "0");
    assert_eq!(number_text(2.25), "2.3");
}

// ---- which charms are owned and equipped ----------------------------------------------------

#[test]
fn a_new_run_owns_the_starting_charms_and_wears_none() {
    let state = CharmState::starting(&sample());
    assert_eq!(state.owned(), ["heart", "swift", "sharp"]);
    assert!(state.equipped().is_empty());
    assert!(!state.is_owned("ward"));
}

#[test]
fn equipping_keeps_order_and_never_duplicates() {
    let catalog = sample();
    let mut state = CharmState::starting(&catalog);
    state.equip(&catalog, "sharp", MANA).unwrap();
    state.equip(&catalog, "heart", MANA).unwrap();
    assert_eq!(state.equipped(), ["sharp", "heart"]);
    let error = state.equip(&catalog, "sharp", MANA).unwrap_err();
    assert!(error.contains("already equipped"), "{error}");
    assert_eq!(state.equipped(), ["sharp", "heart"]);
    let names = state.equipped_charms(&catalog);
    assert_eq!(names[0].id, "sharp");
    assert_eq!(names[1].id, "heart");
}

#[test]
fn a_charm_over_the_budget_is_refused_naming_the_missing_mana() {
    let catalog = catalog(vec![
        charm("big", "Big Charm", 60.0, Effect::MaxHealth(1)),
        charm("bigger", "Bigger Charm", 50.0, Effect::MaxHealth(1)),
    ]);
    let mut state = CharmState::starting(&catalog);
    state.equip(&catalog, "big", MANA).unwrap();
    let before = state.clone();
    let error = state.can_equip(&catalog, "bigger", MANA).unwrap_err();
    assert!(error.contains("10 more"), "{error}");
    assert!(error.contains("Bigger Charm"), "{error}");
    assert_eq!(state.equip(&catalog, "bigger", MANA).unwrap_err(), error);
    assert_eq!(state, before, "a refusal changes nothing");
}

#[test]
fn the_budget_can_be_used_up_exactly() {
    let catalog = catalog(vec![
        charm("a", "A", 60.0, Effect::MaxHealth(1)),
        charm("b", "B", 40.0, Effect::MaxHealth(1)),
        charm("c", "C", 0.0, Effect::MaxHealth(1)),
    ]);
    let mut state = CharmState::starting(&catalog);
    state.equip(&catalog, "a", MANA).unwrap();
    state.equip(&catalog, "b", MANA).unwrap();
    assert_eq!(state.max_mana(&catalog, MANA), 0.0);
    // A free charm still fits.
    state.equip(&catalog, "c", MANA).unwrap();
}

#[test]
fn a_charm_you_do_not_own_or_that_does_not_exist_is_refused() {
    let catalog = sample();
    let mut state = CharmState::starting(&catalog);
    assert!(state
        .equip(&catalog, "ward", MANA)
        .unwrap_err()
        .contains("do not own"));
    assert!(state.equip(&catalog, "nothing", MANA).is_err());
    assert!(state.equipped().is_empty());
}

#[test]
fn unequipping_always_works_even_when_the_budget_shrank() {
    let catalog = sample();
    let mut state = CharmState::starting(&catalog);
    state.equip(&catalog, "heart", MANA).unwrap();
    state.equip(&catalog, "sharp", MANA).unwrap();
    // The base max mana is lowered below what is worn: still no problem taking them off.
    assert_eq!(state.max_mana(&catalog, 20.0), 0.0);
    assert!(state.unequip("heart"));
    assert!(state.unequip("sharp"));
    assert!(!state.unequip("sharp"), "nothing left to take off");
    assert!(state.equipped().is_empty());
}

#[test]
fn toggle_equips_unequips_and_reports_a_refusal() {
    let catalog = sample();
    let mut state = CharmState::starting(&catalog);
    assert_eq!(state.toggle(&catalog, "heart", MANA), Toggle::Equipped);
    assert!(state.is_equipped("heart"));
    assert_eq!(state.toggle(&catalog, "heart", MANA), Toggle::Unequipped);
    assert!(!state.is_equipped("heart"));
    let Toggle::Refused(message) = state.toggle(&catalog, "heart", 10.0) else {
        panic!("a 15 mana charm does not fit in 10 mana");
    };
    assert!(message.contains("5 more"), "{message}");
    assert!(state.equipped().is_empty());
}

#[test]
fn max_mana_is_the_base_minus_the_cost_and_never_negative() {
    let catalog = sample();
    let mut state = CharmState::starting(&catalog);
    assert_eq!(state.max_mana(&catalog, MANA), MANA);
    state.equip(&catalog, "heart", MANA).unwrap();
    state.equip(&catalog, "swift", MANA).unwrap();
    assert_eq!(state.mana_used(&catalog), 25.0);
    assert_eq!(state.max_mana(&catalog, MANA), 75.0);
    assert_eq!(state.max_mana(&catalog, 10.0), 0.0);
}

#[test]
fn charms_missing_after_a_reload_are_forgotten() {
    let full = sample();
    let mut state = CharmState::starting(&full);
    state.equip(&full, "heart", MANA).unwrap();
    state.equip(&full, "sharp", MANA).unwrap();
    let smaller = catalog(vec![charm(
        "sharp",
        "Sharp Charm",
        20.0,
        Effect::SwordDamage(5),
    )]);
    state.retain_known(&smaller);
    assert_eq!(state.owned(), ["sharp"]);
    assert_eq!(state.equipped(), ["sharp"]);
}

// ---- layering the bonuses on the base numbers -----------------------------------------------

fn base() -> (TuningConfig, VitalsConfig) {
    (TuningConfig::default(), VitalsConfig::default())
}

fn worn<'a>(catalog: &'a CharmCatalog, ids: &[&str]) -> Vec<&'a Charm> {
    ids.iter().map(|id| catalog.get(id).unwrap()).collect()
}

#[test]
fn nothing_equipped_leaves_the_numbers_as_they_are() {
    let (tuning, vitals) = base();
    assert_eq!(effective(&tuning, &vitals, &[]), (tuning, vitals));
}

#[test]
fn the_heart_charm_raises_max_health_and_takes_its_mana() {
    let (tuning, vitals) = base();
    let catalog = sample();
    let (with, vitals_with) = effective(&tuning, &vitals, &worn(&catalog, &["heart"]));
    assert_eq!(with.player.max_health, tuning.player.max_health + 10);
    assert_eq!(vitals_with.max_mana, vitals.max_mana - 15.0);
    // Everything else is the base.
    assert_eq!(with.player.move_speed, tuning.player.move_speed);
    assert_eq!(with.player.melee, tuning.player.melee);
    assert_eq!(with.enemy, tuning.enemy);
    assert_eq!(vitals_with.heal_move_scale, vitals.heal_move_scale);
}

#[test]
fn swift_and_sharp_change_speed_and_damage() {
    let (tuning, vitals) = base();
    let catalog = sample();
    let (swift, _) = effective(&tuning, &vitals, &worn(&catalog, &["swift"]));
    close(swift.player.move_speed, tuning.player.move_speed * 1.15);
    assert_eq!(swift.player.melee.damage, tuning.player.melee.damage);
    let (sharp, _) = effective(&tuning, &vitals, &worn(&catalog, &["sharp"]));
    assert_eq!(sharp.player.melee.damage, tuning.player.melee.damage + 5);
    assert_eq!(sharp.player.move_speed, tuning.player.move_speed);
}

#[test]
fn two_charms_add_up() {
    let (tuning, vitals) = base();
    let catalog = catalog(vec![
        charm("h1", "H1", 10.0, Effect::MaxHealth(10)),
        charm("h2", "H2", 5.0, Effect::MaxHealth(20)),
        charm("s1", "S1", 1.0, Effect::MoveSpeedPercent(10.0)),
        charm("s2", "S2", 2.0, Effect::MoveSpeedPercent(20.0)),
    ]);
    let (health, vitals_health) = effective(&tuning, &vitals, &worn(&catalog, &["h1", "h2"]));
    assert_eq!(health.player.max_health, tuning.player.max_health + 30);
    assert_eq!(vitals_health.max_mana, vitals.max_mana - 15.0);
    // Percentages add (10 + 20 = 30 %), they do not compound.
    let (speed, _) = effective(&tuning, &vitals, &worn(&catalog, &["s1", "s2"]));
    close(speed.player.move_speed, tuning.player.move_speed * 1.3);
}

#[test]
fn the_base_numbers_are_only_read() {
    let (tuning, vitals) = base();
    let (tuning_before, vitals_before) = (tuning, vitals);
    let catalog = sample();
    let all = worn(&catalog, &["heart", "swift", "sharp"]);
    let (changed, vitals_changed) = effective(&tuning, &vitals, &all);
    assert_ne!(changed, tuning);
    assert_ne!(vitals_changed, vitals);
    assert_eq!((tuning, vitals), (tuning_before, vitals_before));
}

#[test]
fn bonuses_stop_at_the_largest_whole_number() {
    let (mut tuning, vitals) = base();
    tuning.player.max_health = u16::MAX - 5;
    let catalog = sample();
    let (with, _) = effective(&tuning, &vitals, &worn(&catalog, &["heart", "ward"]));
    assert_eq!(with.player.max_health, u16::MAX);
}

#[test]
fn max_mana_never_goes_below_zero_and_the_starting_mana_follows_it() {
    let (tuning, mut vitals) = base();
    vitals.max_mana = 20.0;
    vitals.mana_start = 20.0;
    let catalog = sample();
    let (_, with) = effective(&tuning, &vitals, &worn(&catalog, &["heart"]));
    assert_eq!((with.max_mana, with.mana_start), (5.0, 5.0));
    // Worn charms that now cost more than the base: the cap bottoms out at zero.
    let (_, with) = effective(&tuning, &vitals, &worn(&catalog, &["heart", "sharp"]));
    assert_eq!((with.max_mana, with.mana_start), (0.0, 0.0));
}
