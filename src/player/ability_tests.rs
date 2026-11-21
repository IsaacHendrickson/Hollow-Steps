//! Abilities on the player: the dash needs `Ability::Dash`, picking up the Dash item grants it,
//! and a run's reset takes it away again.
use super::*;

const BOUNDS: WorldBounds = WorldBounds::new(0.0, 960.0, 460.0);
const DASH_INPUT: PlayerInput = PlayerInput {
    dash_pressed: true,
    move_axis: 0.0,
    jump_pressed: false,
    pickup_pressed: false,
    jump_held: false,
    attack_pressed: false,
    up_held: false,
    down_held: false,
    heal_held: false,
};

fn standing_player() -> Player {
    let mut player = Player::new(Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(400.0, BOUNDS);
    player
}

fn dashed(player: &mut Player) -> bool {
    let before = player.position().x;
    player.update(DASH_INPUT, 0.1, BOUNDS);
    player.position().x != before
}

#[test]
fn dash_is_blocked_without_the_ability() {
    let mut player = standing_player();
    assert!(!player.abilities().has(Ability::Dash));
    assert!(!dashed(&mut player));
    assert_eq!(player.position().x, 400.0);
}

#[test]
fn collecting_the_dash_item_grants_the_ability_and_allows_the_dash() {
    let mut player = standing_player();
    player.collect_item(Item::Dash);

    assert!(player.abilities().has(Ability::Dash));
    assert_eq!(player.collected_items(), &[Item::Dash], "still an item");
    assert!(dashed(&mut player));
}

#[test]
fn an_item_that_grants_nothing_does_not_allow_the_dash() {
    let mut player = standing_player();
    player.collect_item(Item::AmberShard);

    assert!(player.abilities().is_empty());
    assert!(!dashed(&mut player));
}

#[test]
fn the_ability_is_granted_by_the_pickup_and_not_by_holding_the_item_list_alone() {
    // The reverse of the rule above: with the ability and no Dash item the dash still works,
    // because the rules read the ability.
    let mut player = standing_player();
    player.abilities.grant(Ability::Dash);
    assert!(player.collected_items().is_empty());
    assert!(dashed(&mut player));
}

#[test]
fn a_run_reset_takes_the_ability_and_the_items_away() {
    let mut player = standing_player();
    player.collect_item(Item::Dash);
    player.collect_item(Item::AmberShard);

    player.reset_for_run();

    assert!(player.abilities().is_empty());
    assert!(player.collected_items().is_empty());
    assert!(!dashed(&mut player), "the dash is locked again");
}

#[test]
fn only_the_dash_item_grants_an_ability_today() {
    assert_eq!(Item::Dash.grants(), Some(Ability::Dash));
    assert_eq!(Item::AmberShard.grants(), None);
    for item in Item::ALL {
        if let Some(ability) = item.grants() {
            assert!(Ability::ALL.contains(&ability), "{}", item.name());
        }
    }
}

#[test]
fn collecting_the_dash_twice_still_grants_it_once() {
    let mut player = standing_player();
    player.collect_item(Item::Dash);
    player.collect_item(Item::Dash);
    assert!(player.abilities().has(Ability::Dash));
    assert!(dashed(&mut player));
}
