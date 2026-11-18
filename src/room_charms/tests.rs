use super::*;
use crate::editor::{RoomDraft, RoomLayout};
use crate::player::{PlayerConfig, WorldBounds};
use crate::room::{test_rooms, RoomEnemies};

/// The first sample room's layout (floor at 460, nothing solid in it).
fn layout() -> RoomLayout {
    let [room, _, _] = test_rooms();
    RoomLayout::from_room(&room)
}

/// No enemy definitions: the drafts here place no enemies.
fn no_definitions() -> crate::enemy::EnemyDefinitions {
    crate::enemy::EnemyDefinitions::from_definitions(
        Vec::new(),
        Default::default(),
        Default::default(),
    )
    .unwrap()
}

fn placed(charm: &str, at: (f32, f32)) -> CharmPlacement {
    CharmPlacement {
        charm: charm.into(),
        position: at,
    }
}

#[test]
fn a_layout_without_charms_loads_with_none_and_writes_none() {
    let layout = layout();
    assert!(layout.charms.is_empty());
    let text = ron::ser::to_string_pretty(&layout, Default::default()).unwrap();
    assert!(!text.contains("charms"), "{text}");
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, layout);
}

#[test]
fn charms_round_trip_through_ron_as_id_and_position() {
    let mut with = layout();
    with.charms = vec![
        placed("ward", (330.0, 452.0)),
        placed("heart", (100.5, 300.25)),
    ];
    let text = ron::ser::to_string_pretty(&with, Default::default()).unwrap();
    assert!(text.contains("charm: \"ward\""), "{text}");
    assert!(text.contains("position: (330.0, 452.0)"), "{text}");
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, with);
    // The whole room document too.
    let document = RoomEnemies {
        enemies: Vec::new(),
        boss: None,
        layout: Some(with.clone()),
    };
    let text = ron::ser::to_string_pretty(&document, Default::default()).unwrap();
    let back: RoomEnemies = ron::from_str(&text).unwrap();
    assert_eq!(back.layout.unwrap().charms, with.charms);
}

#[test]
fn a_placement_with_an_unknown_field_is_refused() {
    let text = r#"(charm: "ward", position: (1.0, 2.0), color: 3)"#;
    assert!(ron::from_str::<CharmPlacement>(text).is_err());
    assert!(ron::from_str::<CharmPlacement>(r#"(charm: "ward")"#).is_err());
}

#[test]
fn a_charm_must_lie_inside_the_room_above_the_floor() {
    let mut layout = layout();
    let (x, y, w, _) = layout.bounds;
    let floor = layout.floor_y;
    for (at, fits) in [
        ((x + 8.0, floor - 8.0), true),
        ((x + 7.9, floor - 8.0), false),
        ((x + w - 8.0, floor - 8.0), true),
        ((x + w - 7.9, floor - 8.0), false),
        ((100.0, floor - 7.9), false),
        ((100.0, y + 8.0), true),
        ((100.0, y + 7.9), false),
        ((f32::NAN, 100.0), false),
    ] {
        layout.charms = vec![placed("ward", at)];
        let result = layout.validate_geometry();
        assert_eq!(result.is_ok(), fits, "{at:?}: {result:?}");
        if !fits {
            let error = result.unwrap_err();
            assert!(
                error.contains("charm \"ward\" must be inside the room"),
                "{error}"
            );
        }
    }
}

#[test]
fn a_charm_inside_a_solid_is_refused_and_one_beside_it_is_not() {
    let mut layout = layout();
    layout.terrain = vec![(200.0, 380.0, 100.0, 40.0)];
    layout.charms = vec![placed("ward", (250.0, 400.0))];
    let error = layout.validate_geometry().unwrap_err();
    assert!(
        error.contains("charm \"ward\" must not be inside terrain"),
        "{error}"
    );
    layout.charms = vec![placed("ward", (200.0 - 8.0, 400.0))];
    assert!(layout.validate_geometry().is_ok(), "touching is not inside");
    layout.charms = vec![placed("ward", (200.0 - 7.9, 400.0))];
    assert!(layout.validate_geometry().is_err());
    // The draft refuses it the same way when placed or moved.
    let mut draft = RoomDraft::new_room(&layout);
    draft.layout.terrain = vec![(200.0, 380.0, 100.0, 40.0)];
    let definitions = no_definitions();
    let at = macroquad::math::Vec2::new(250.0, 400.0);
    let error = draft
        .place(
            crate::editor::Placement::Charm("ward".into()),
            at,
            &definitions,
        )
        .unwrap_err();
    assert!(error.contains("inside terrain"), "{error}");
    assert!(
        draft.layout.charms.is_empty(),
        "a refused placement changes nothing"
    );
}

#[test]
fn an_unknown_charm_id_is_named_by_the_id_check() {
    let mut layout = layout();
    layout.charms = vec![
        placed("heart", (100.0, 440.0)),
        placed("nope", (140.0, 440.0)),
    ];
    let error = layout.validate_charm_ids(&|id| id == "heart").unwrap_err();
    assert_eq!(error, "charm \"nope\" does not exist");
    assert!(layout.validate_charm_ids(&|_| true).is_ok());
    layout.charms.clear();
    assert!(layout.validate_charm_ids(&|_| false).is_ok());
}

#[test]
fn place_move_and_delete_edit_the_layout_and_select_the_charm() {
    use crate::editor_selection::Selection;
    let definitions = no_definitions();
    let mut draft = RoomDraft::new_room(&layout());
    let at = macroquad::math::Vec2::new(300.0, 440.0);
    let first = draft
        .place(
            crate::editor::Placement::Charm("ward".into()),
            at,
            &definitions,
        )
        .unwrap();
    let second = draft
        .place(
            crate::editor::Placement::Charm("heart".into()),
            at + macroquad::math::Vec2::X * 40.0,
            &definitions,
        )
        .unwrap();
    assert_eq!((first, second), (Selection::Charm(0), Selection::Charm(1)));
    draft
        .move_to(
            Selection::Charm(0),
            macroquad::math::Vec2::new(120.0, 420.0),
            &definitions,
        )
        .unwrap();
    assert_eq!(draft.layout.charms[0], placed("ward", (120.0, 420.0)));
    assert!(draft
        .move_to(Selection::Charm(5), at, &definitions)
        .is_err());
    draft.delete(Selection::Charm(0)).unwrap();
    assert_eq!(draft.layout.charms, vec![placed("heart", (340.0, 440.0))]);
    assert!(draft.delete(Selection::Charm(3)).is_err());
}

#[test]
fn a_room_built_from_a_layout_lies_its_charms_and_the_layout_reads_them_back() {
    let definitions = no_definitions();
    let mut draft = RoomDraft::new_room(&layout());
    draft.layout.charms = vec![placed("ward", (330.0, 452.0))];
    let room = draft
        .build(crate::room::RoomId::First, &definitions)
        .unwrap();
    assert_eq!(
        room.charms,
        vec![WorldCharm {
            charm: "ward".into(),
            position: macroquad::math::Vec2::new(330.0, 452.0)
        }]
    );
    assert_eq!(RoomLayout::from_room(&room).charms, draft.layout.charms);
}

#[test]
fn a_new_room_starts_without_the_templates_charms() {
    let mut template = layout();
    template.charms = vec![placed("ward", (330.0, 452.0))];
    assert!(RoomDraft::new_room(&template).layout.charms.is_empty());
}

#[test]
fn the_reach_rule_is_the_items_rule() {
    let bounds = WorldBounds::new(0.0, 960.0, 460.0);
    let mut player = Player::new(macroquad::math::Vec2::ZERO, PlayerConfig::default());
    player.place_on_floor(200.0, bounds);
    let edge = 200.0 + 16.0 + CHARM_SIZE.x / 2.0 + item::PICKUP_REACH;
    let mut charm = WorldCharm {
        charm: "ward".into(),
        position: macroquad::math::Vec2::new(edge, 450.0),
    };
    assert!(charm.can_pick_up(&player));
    charm.position.x += 0.1;
    assert!(!charm.can_pick_up(&player));
    charm.position = macroquad::math::Vec2::new(200.0, 300.0);
    assert!(!charm.can_pick_up(&player), "too high to reach");
    // Not while airborne, like an item.
    let mut airborne = Player::new(
        macroquad::math::Vec2::new(200.0, 100.0),
        PlayerConfig::default(),
    );
    airborne.place_at(macroquad::math::Vec2::new(200.0, 100.0), bounds);
    charm.position = airborne.position();
    assert!(!airborne.is_grounded() && !charm.can_pick_up(&airborne));
}
