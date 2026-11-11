use hollow_steps::behavior::Action;
use hollow_steps::boss_definition::{AttackSelection, BossDefinition, BossDefinitions};
use hollow_steps::boss_editor::{BossActionInput, BossInputs};
use hollow_steps::editor::{RoomDraft, RoomLayout, Selection};
use hollow_steps::room::test_rooms;
use macroquad::math::Vec2;

fn boss() -> BossDefinition {
    BossDefinition::from_ron(include_str!("fixtures/bosses/ground_pound.ron")).unwrap()
}

#[test]
fn attack_and_action_operations_are_pure_ordered_and_bounds_checked() {
    let definition = boss();
    let mut inputs = BossInputs::from_definition(&definition);
    let before = inputs.clone();
    assert!(inputs.add_attack(" ").is_err());
    assert!(inputs.add_attack(" Ground pound ").is_err());
    assert!(inputs.rename_attack(99, "missing").is_err());
    assert!(inputs.remove_attack(99).is_err());
    assert!(inputs.move_attack(0, true).is_err());
    assert!(inputs.move_action(0, usize::MAX, false).is_err());
    assert!(inputs.remove_action(99, 0).is_err());
    assert!(inputs.add_action(0, usize::MAX).is_err());
    assert_eq!(inputs, before);
    let second = inputs.add_attack(" Second ").unwrap();
    let wait = Action::ALL_TYPES.iter().position(|n| *n == "wait").unwrap();
    inputs.add_action(second, wait).unwrap();
    inputs.rename_attack(second, "Rest").unwrap();
    inputs.move_attack(second, true).unwrap();
    assert_eq!(inputs.attacks[0].name, "Rest");
    inputs.move_action(1, 1, true).unwrap();
    assert_eq!(
        inputs.attacks[1].actions[0].build().unwrap(),
        Action::Hover { seconds: 0.5 }
    );
    inputs.remove_action(1, 1).unwrap();
    inputs.remove_attack(1).unwrap();
    let mut edited = definition.clone();
    inputs.apply(&mut edited).unwrap();
    assert_eq!(
        edited.attacks[0].actions,
        vec![Action::Wait { seconds: 1.0 }]
    );
    assert_eq!(definition, boss());
}

#[test]
fn every_boss_action_has_valid_data_defaults_and_labeled_editable_parameters() {
    let catalog = BossActionInput::catalog();
    assert_eq!(
        catalog,
        Action::ALL_TYPES
            .iter()
            .enumerate()
            .map(|(kind, name)| (kind, *name))
            .collect::<Vec<_>>()
    );
    for name in [
        "lunge",
        "jump_to",
        "slam_down",
        "shoot_projectile",
        "area_blast",
    ] {
        let kind = Action::ALL_TYPES
            .iter()
            .position(|entry| *entry == name)
            .unwrap();
        assert!(BossActionInput::new(kind).is_ok(), "shared action {name}");
    }
    for (kind, name) in catalog {
        let input = BossActionInput::new(kind).unwrap();
        let action = input.build().unwrap();
        assert_eq!(BossActionInput::from_action(&action), input);
        assert_eq!(action.parameters().len(), action.parameter_labels().len());
        assert!(
            action
                .parameter_labels()
                .iter()
                .all(|label| label.contains('(') || hollow_steps::pose::is_pose_parameter(label)),
            "{name}"
        );
        let mut definition = boss();
        definition.attacks[0].actions = vec![action.clone()];
        definition.validate().unwrap();
        for i in 0..input.values.len() {
            let mut invalid = input.clone();
            invalid.values[i] =
                if hollow_steps::pose::is_pose_parameter(action.parameters()[i].0) {
                    "bad/name"
                } else {
                    "NaN"
                }
                .into();
            assert!(invalid.build().is_err(), "{name} parameter {i}");
        }
    }
    assert!(BossActionInput::new(Action::ALL_TYPES.len()).is_err());
}

#[test]
fn invalid_properties_and_unselected_attack_fields_never_mutate_the_definition() {
    let original = boss();
    let mut definition = original.clone();
    for (field, value) in [
        (0, " "),
        (1, "0"),
        (1, "4294967296"),
        (2, "0"),
        (3, "NaN"),
        (4, "65536"),
        (5, "-1"),
    ] {
        let mut inputs = BossInputs::from_definition(&original);
        inputs.fields[field] = value.into();
        assert!(inputs.apply(&mut definition).is_err());
        assert_eq!(definition, original);
    }
    let mut inputs = BossInputs::from_definition(&original);
    inputs.attacks[0].actions[0].values[0] = "0".into(); // bosses require positive jump speed
    assert!(inputs.apply(&mut definition).is_err());
    assert_eq!(definition, original);
    let mut inputs = BossInputs::from_definition(&original);
    inputs.add_attack("Empty").unwrap();
    assert!(inputs.apply(&mut definition).is_err());
    inputs.remove_attack(1).unwrap();
    while !inputs.attacks[0].actions.is_empty() {
        inputs.remove_action(0, 0).unwrap();
    }
    assert!(inputs.apply(&mut definition).is_err());
    inputs.remove_attack(0).unwrap();
    assert!(inputs.apply(&mut definition).is_err());
    assert_eq!(definition, original);
    let mut valid = BossInputs::from_definition(&original);
    valid.fields[1] = u32::MAX.to_string();
    valid.fields[4] = "255".into();
    valid.fields[5] = "0".into();
    valid.selection = AttackSelection::Random;
    valid.apply(&mut definition).unwrap();
    assert_eq!(definition.health, u32::MAX);
    assert_eq!(definition.selection, AttackSelection::Random);
}

#[test]
fn boss_placement_dragging_and_deletion_validate_the_whole_body_atomically() {
    let definitions = BossDefinitions::from_definitions([boss()]).unwrap();
    let layout = RoomLayout::from_room(&test_rooms()[0]);
    let mut draft = RoomDraft::new_room(&layout);
    let corner = Vec2::new(layout.bounds.0 + 24.0, layout.floor_y - 24.0);
    assert!(draft.place_boss("missing", corner, &definitions).is_err());
    assert!(draft
        .place_boss("Ground Pound Example", Vec2::NAN, &definitions)
        .is_err());
    assert!(draft.boss.is_none());
    assert_eq!(
        draft
            .place_boss("Ground Pound Example", corner, &definitions)
            .unwrap(),
        Selection::Boss
    );
    let before = draft.boss.clone();
    assert!(draft
        .place_boss("Ground Pound Example", corner, &definitions)
        .is_err());
    for position in [corner - Vec2::X, corner + Vec2::Y, Vec2::INFINITY] {
        assert!(draft.move_boss(position, &definitions).is_err());
        assert_eq!(draft.boss, before);
    }
    draft
        .move_boss(corner + Vec2::new(30.0, -30.0), &definitions)
        .unwrap();
    assert_eq!(
        draft.boss.as_ref().unwrap().position,
        (corner.x + 30.0, corner.y - 30.0)
    );
    draft.delete(Selection::Boss).unwrap();
    assert!(draft.boss.is_none());
    assert!(draft.delete(Selection::Boss).is_err());
    assert!(draft.move_boss(corner, &definitions).is_err());
}
