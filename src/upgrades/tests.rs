use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::charms::{effective, effective_with, Charm};
use crate::tuning::TuningConfig;
use crate::vitals::VitalsConfig;

/// Three upgrades written for the tests (never the shipped numbers, which the owner tunes).
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

fn rich(coins: u32) -> Profile {
    Profile {
        coins,
        ..Profile::default()
    }
}

/// A file with one upgrade written by hand.
fn file_with(body: &str) -> String {
    format!("(upgrades: [({body})])")
}

const GOOD: &str = r#"id: "a", name: "A", description: "",
    effect: MaxHealth(5), costs: [10, 20]"#;

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "{a} is not close to {b}");
}

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-upgrades-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

// ---- buying ----------------------------------------------------------------------------

#[test]
fn buying_takes_the_price_and_raises_the_level_by_one() {
    let catalog = catalog();
    let mut profile = rich(100);
    assert_eq!(level(&profile, "vitality"), 0);
    assert_eq!(next_cost(&profile, &catalog, "vitality"), Some(30));

    assert_eq!(buy(&mut profile, &catalog, "vitality"), Purchase::Bought);
    assert_eq!((profile.coins, level(&profile, "vitality")), (70, 1));
    assert_eq!(next_cost(&profile, &catalog, "vitality"), Some(60));

    assert_eq!(buy(&mut profile, &catalog, "vitality"), Purchase::Bought);
    assert_eq!((profile.coins, level(&profile, "vitality")), (10, 2));
    // Other upgrades are untouched.
    assert_eq!(level(&profile, "swift_feet"), 0);
    assert_eq!(profile.upgrades.len(), 1);
}

#[test]
fn the_exact_price_is_enough_and_leaves_no_coins() {
    let mut profile = rich(30);
    assert_eq!(buy(&mut profile, &catalog(), "vitality"), Purchase::Bought);
    assert_eq!(profile.coins, 0);
}

#[test]
fn too_few_coins_is_refused_by_how_many_more_are_needed_and_changes_nothing() {
    let catalog = catalog();
    let mut profile = rich(29);
    let Purchase::Refused(message) = buy(&mut profile, &catalog, "vitality") else {
        panic!("29 coins must not buy a 30 coin upgrade");
    };
    assert!(message.contains("you need 1 more"), "{message}");
    assert!(message.contains("Vitality"), "{message}");
    assert_eq!(profile, rich(29), "a refusal changes nothing");
}

#[test]
fn an_unknown_upgrade_is_refused_and_changes_nothing() {
    let mut profile = rich(1_000);
    let before = profile.clone();
    let Purchase::Refused(message) = buy(&mut profile, &catalog(), "no_such_upgrade") else {
        panic!("an unknown id cannot be bought");
    };
    assert!(message.contains("no such upgrade"), "{message}");
    assert_eq!(profile, before);
}

#[test]
fn the_third_level_is_the_last() {
    let catalog = catalog();
    let mut profile = rich(1_000);
    for _ in 0..3 {
        assert_eq!(buy(&mut profile, &catalog, "vitality"), Purchase::Bought);
    }
    assert_eq!(level(&profile, "vitality"), 3);
    assert_eq!(profile.coins, 1_000 - 30 - 60 - 100);
    assert_eq!(next_cost(&profile, &catalog, "vitality"), None);

    let before = profile.clone();
    let Purchase::Refused(message) = buy(&mut profile, &catalog, "vitality") else {
        panic!("there is no fourth level");
    };
    assert!(message.contains("maximum level"), "{message}");
    assert_eq!(profile, before, "the refusal changes nothing, however rich");
}

#[test]
fn a_saved_level_above_the_last_cannot_be_bought_past() {
    // The catalog was edited down to fewer levels after the profile bought more.
    let mut profile = rich(1_000);
    profile.upgrades.insert("vitality".into(), 7);
    assert_eq!(next_cost(&profile, &catalog(), "vitality"), None);
    assert!(matches!(
        buy(&mut profile, &catalog(), "vitality"),
        Purchase::Refused(_)
    ));
    assert_eq!(level(&profile, "vitality"), 7, "never lowered");
}

// ---- the effects -----------------------------------------------------------------------

#[test]
fn every_owned_level_gives_its_effect_once() {
    let catalog = catalog();
    let mut profile = rich(0);
    assert!(permanent_effects(&profile, &catalog).is_empty());

    profile.upgrades.insert("vitality".into(), 2);
    profile.upgrades.insert("keen_edge".into(), 1);
    assert_eq!(
        permanent_effects(&profile, &catalog),
        [
            Effect::MaxHealth(10),
            Effect::MaxHealth(10),
            Effect::SwordDamage(3)
        ]
    );
}

#[test]
fn an_id_the_catalog_does_not_list_has_no_effect_and_stays_in_the_profile() {
    let mut profile = rich(0);
    profile.upgrades.insert("removed_upgrade".into(), 3);
    assert!(permanent_effects(&profile, &catalog()).is_empty());
    assert_eq!(level(&profile, "removed_upgrade"), 3, "still there");
}

#[test]
fn a_level_above_the_catalogs_last_counts_as_the_last() {
    let mut profile = rich(0);
    profile.upgrades.insert("vitality".into(), 99);
    assert_eq!(permanent_effects(&profile, &catalog()).len(), 3);
}

#[test]
fn permanent_effects_add_up_with_charms_and_cost_no_mana() {
    let (tuning, vitals) = (TuningConfig::default(), VitalsConfig::default());
    let swift = Charm {
        id: "swift".into(),
        name: "Swift Charm".into(),
        description: String::new(),
        mana_cost: 10.0,
        effect: Effect::MoveSpeedPercent(15.0),
        color: [1, 2, 3, 255],
        owned_at_start: true,
    };
    // Two upgrade levels of 5 percent and the charm's 15: 25 percent of the base speed.
    let permanent = [
        Effect::MoveSpeedPercent(5.0),
        Effect::MoveSpeedPercent(5.0),
        Effect::MaxHealth(10),
        Effect::SwordDamage(3),
    ];
    let (with, with_vitals) = effective_with(&tuning, &vitals, &[&swift], &permanent);
    close(with.player.move_speed, tuning.player.move_speed * 1.25);
    assert_eq!(with.player.max_health, tuning.player.max_health + 10);
    assert_eq!(with.player.melee.damage, tuning.player.melee.damage + 3);
    // Only the equipped charm costs mana.
    close(with_vitals.max_mana, vitals.max_mana - 10.0);
    let (without_charm, without_vitals) = effective_with(&tuning, &vitals, &[], &permanent);
    close(
        without_charm.player.move_speed,
        tuning.player.move_speed * 1.10,
    );
    close(without_vitals.max_mana, vitals.max_mana);
}

#[test]
fn with_no_permanent_effects_effective_with_is_effective() {
    let (tuning, vitals) = (TuningConfig::default(), VitalsConfig::default());
    let heart = Charm {
        id: "heart".into(),
        name: "Heart Charm".into(),
        description: String::new(),
        mana_cost: 15.0,
        effect: Effect::MaxHealth(10),
        color: [1, 2, 3, 255],
        owned_at_start: true,
    };
    assert_eq!(
        effective_with(&tuning, &vitals, &[], &[]),
        effective(&tuning, &vitals, &[]),
    );
    assert_eq!(
        effective_with(&tuning, &vitals, &[&heart], &[]),
        effective(&tuning, &vitals, &[&heart]),
    );
}

#[test]
fn permanent_health_meets_the_same_cap_as_charm_health() {
    let (tuning, vitals) = (TuningConfig::default(), VitalsConfig::default());
    let permanent = [Effect::MaxHealth(1_000); 80];
    let (with, _) = effective_with(&tuning, &vitals, &[], &permanent);
    assert_eq!(with.player.max_health, u16::MAX);
}

// ---- the file --------------------------------------------------------------------------

fn parse(text: &str) -> Result<UpgradeCatalog, String> {
    UpgradeCatalog::from_ron(text)
}

#[test]
fn the_embedded_file_loads_and_validates() {
    // The numbers are the owner's to tune: only that it loads.
    let shipped = UpgradeCatalog::default();
    shipped.validate().unwrap();
    assert!(!shipped.upgrades().is_empty());
}

#[test]
fn a_catalog_lists_its_upgrades_in_file_order_and_finds_them_by_id() {
    let catalog = catalog();
    let ids: Vec<&str> = catalog.upgrades().iter().map(|u| u.id.as_str()).collect();
    assert_eq!(ids, ["vitality", "swift_feet", "keen_edge"]);
    assert_eq!(catalog.get("swift_feet").unwrap().name, "Swift Feet");
    assert!(catalog.get("nothing").is_none());
    assert_eq!(catalog.get("vitality").unwrap().max_level(), 3);
}

#[test]
fn a_good_single_upgrade_parses() {
    assert!(parse(&file_with(GOOD)).is_ok());
}

#[test]
fn a_duplicate_id_or_name_is_refused() {
    let one = r#"(id: "a", name: "A", description: "", effect: MaxHealth(5), costs: [1])"#;
    let same_id = r#"(id: "a", name: "B", description: "", effect: MaxHealth(5), costs: [1])"#;
    let same_name = r#"(id: "b", name: "a", description: "", effect: MaxHealth(5), costs: [1])"#;
    let error = parse(&format!("(upgrades: [{one}, {same_id}])")).unwrap_err();
    assert!(error.contains("id \"a\" is used twice"), "{error}");
    let error = parse(&format!("(upgrades: [{one}, {same_name}])")).unwrap_err();
    assert!(error.contains("name \"a\" is used twice"), "{error}");
    assert!(error.contains("ignoring case"), "{error}");
}

#[test]
fn a_bad_cost_list_is_refused() {
    let with_costs = |costs: &str| {
        parse(&file_with(&format!(
            r#"id: "a", name: "A", description: "", effect: MaxHealth(5), costs: {costs}"#
        )))
    };
    assert!(with_costs("[]").unwrap_err().contains("1 to 10 prices"));
    let eleven = format!("[{}]", ["1"; 11].join(", "));
    assert!(with_costs(&eleven).unwrap_err().contains("1 to 10 prices"));
    let ten = format!("[{}]", ["1"; 10].join(", "));
    assert!(with_costs(&ten).is_ok(), "ten levels is the most");
    let zero = with_costs("[10, 0, 30]").unwrap_err();
    assert!(zero.contains("level 2 must be above 0"), "{zero}");
    assert!(
        with_costs("[-5]").is_err(),
        "a negative price does not parse"
    );
}

#[test]
fn an_unknown_field_is_refused() {
    let error = parse(&file_with(&format!("{GOOD}, refundable: true"))).unwrap_err();
    assert!(error.contains("refundable"), "{error}");
    let error = parse(r#"(upgrades: [], shop: 1)"#).unwrap_err();
    assert!(error.contains("shop"), "{error}");
}

#[test]
fn the_id_name_description_and_effect_rules_are_the_charms_rules() {
    let with = |fields: &str| parse(&format!("(upgrades: [({fields})])"));
    let body = |id: &str, name: &str, description: &str, effect: &str| {
        format!(
            r#"id: "{id}", name: "{name}", description: "{description}", effect: {effect}, costs: [5]"#
        )
    };
    assert!(with(&body("", "A", "", "MaxHealth(1)")).is_err());
    assert!(with(&body("has space", "A", "", "MaxHealth(1)")).is_err());
    assert!(with(&body(&"x".repeat(49), "A", "", "MaxHealth(1)")).is_err());
    assert!(with(&body("ok-id_1", "A", "", "MaxHealth(1)")).is_ok());
    assert!(with(&body("a", "", "", "MaxHealth(1)")).is_err());
    assert!(with(&body("a", " A", "", "MaxHealth(1)")).is_err());
    assert!(with(&body("a", &"n".repeat(33), "", "MaxHealth(1)")).is_err());
    assert!(with(&body("a", &"n".repeat(32), "", "MaxHealth(1)")).is_ok());
    assert!(with(&body("a", "A", &"d".repeat(201), "MaxHealth(1)")).is_err());
    assert!(with(&body("a", "A", "", "MaxHealth(0)")).is_err());
    assert!(with(&body("a", "A", "", "SwordDamage(0)")).is_err());
    assert!(with(&body("a", "A", "", "MoveSpeedPercent(0.0)")).is_err());
    assert!(with(&body("a", "A", "", "MoveSpeedPercent(5.0)")).is_ok());
}

#[test]
fn an_assets_file_replaces_the_embedded_one_whole() {
    let dir = TempDir::new();
    // No file: the embedded defaults.
    assert_eq!(
        UpgradeCatalog::load(&dir.0).unwrap(),
        UpgradeCatalog::default()
    );
    std::fs::write(
        dir.0.join(FILE_NAME),
        file_with(
            r#"id: "only", name: "Only", description: "", effect: SwordDamage(2), costs: [9]"#,
        ),
    )
    .unwrap();
    let loaded = UpgradeCatalog::load(&dir.0).unwrap();
    let ids: Vec<&str> = loaded.upgrades().iter().map(|u| u.id.as_str()).collect();
    assert_eq!(ids, ["only"], "replaced, not merged");
}

#[test]
fn a_broken_assets_file_is_an_error_naming_it_never_a_fallback() {
    let dir = TempDir::new();
    std::fs::write(dir.0.join(FILE_NAME), "(upgrades: [(id: ").unwrap();
    let error = UpgradeCatalog::load(&dir.0).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");

    std::fs::write(dir.0.join(FILE_NAME), file_with(r#"id: "a", name: "A""#)).unwrap();
    assert!(UpgradeCatalog::load(&dir.0).is_err());
}

// ---- the totals line -----------------------------------------------------------------------

#[test]
fn max_totals_add_every_level_of_every_upgrade_by_kind() {
    // Three levels each: 10 x 3 health, 5 x 3 percent, 3 x 3 damage.
    let totals = max_totals(&catalog());
    assert_eq!(totals.max_health, 30);
    close(totals.move_speed_percent, 15.0);
    assert_eq!(totals.sword_damage, 9);
    assert_eq!(
        totals.text(),
        "All upgrades at maximum: +30 max health, +15% move speed, +9 sword damage"
    );
}

#[test]
fn max_totals_of_two_upgrades_of_one_kind_add_up_and_an_empty_list_is_zero() {
    let two = UpgradeCatalog::from_ron(
        r#"(upgrades: [
            (id: "a", name: "A", description: "", effect: MoveSpeedPercent(2.5), costs: [1, 2]),
            (id: "b", name: "B", description: "", effect: MoveSpeedPercent(7.5), costs: [1]),
            (id: "c", name: "C", description: "", effect: MaxHealth(4), costs: [1, 2, 3, 4]),
        ])"#,
    )
    .unwrap();
    let totals = max_totals(&two);
    close(totals.move_speed_percent, 2.5 * 2.0 + 7.5);
    assert_eq!((totals.max_health, totals.sword_damage), (16, 0));
    assert_eq!(
        totals.text(),
        "All upgrades at maximum: +16 max health, +12.5% move speed, +0 sword damage"
    );
    let none = UpgradeCatalog::from_ron("(upgrades: [])").unwrap();
    assert_eq!(max_totals(&none), MaxTotals::default());
    assert_eq!(
        max_totals(&none).text(),
        "All upgrades at maximum: +0 max health, +0% move speed, +0 sword damage"
    );
}

#[test]
fn max_totals_are_what_buying_every_level_actually_gives() {
    // The line is a promise about the game: the effects of a fully bought catalog add up to it.
    let catalog = catalog();
    let mut profile = rich(u32::MAX);
    for upgrade in catalog.upgrades() {
        while buy(&mut profile, &catalog, &upgrade.id) == Purchase::Bought {}
    }
    let (tuning, _) = effective_with(
        &TuningConfig::default(),
        &VitalsConfig::default(),
        &[],
        &permanent_effects(&profile, &catalog),
    );
    let base = TuningConfig::default().player;
    let totals = max_totals(&catalog);
    assert_eq!(
        tuning.player.max_health,
        base.max_health + totals.max_health as u16
    );
    close(
        tuning.player.move_speed,
        base.move_speed * (1.0 + totals.move_speed_percent / 100.0),
    );
    assert_eq!(
        tuning.player.melee.damage,
        base.melee.damage + totals.sword_damage as u16
    );
}

#[test]
fn max_totals_ignore_a_speed_that_is_not_a_number_and_do_not_overflow() {
    let upgrade = |effect| Upgrade {
        id: "x".into(),
        name: "X".into(),
        description: String::new(),
        effect,
        costs: vec![1; MAX_LEVELS],
    };
    let list = [
        upgrade(Effect::MoveSpeedPercent(f32::NAN)),
        upgrade(Effect::MaxHealth(u16::MAX)),
        upgrade(Effect::MaxHealth(u16::MAX)),
    ];
    let totals = max_totals_of(&list);
    assert_eq!(totals.move_speed_percent, 0.0);
    assert_eq!(totals.max_health, u32::from(u16::MAX) * 20);
}

#[test]
fn a_catalog_made_from_a_list_is_validated_like_a_file() {
    let list = catalog().upgrades().to_vec();
    assert_eq!(UpgradeCatalog::new(list.clone()).unwrap(), catalog());
    let mut twice = list.clone();
    twice.push(list[0].clone());
    assert!(UpgradeCatalog::new(twice)
        .unwrap_err()
        .contains("used twice"));
    let mut free = list;
    free[1].costs[0] = 0;
    assert!(UpgradeCatalog::new(free).is_err());
}

#[test]
fn a_price_must_be_at_most_the_ceiling() {
    let with = |price: u32| {
        UpgradeCatalog::from_ron(&file_with(&format!(
            r#"id: "a", name: "A", description: "", effect: MaxHealth(5), costs: [10, {price}]"#
        )))
    };
    assert!(with(MAX_COST).is_ok());
    let error = with(MAX_COST + 1).unwrap_err();
    assert!(error.contains("level 2 must be at most 1000000"), "{error}");
    assert!(with(u32::MAX).is_err());
}
