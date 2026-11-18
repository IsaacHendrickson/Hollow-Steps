use super::*;
use crate::player::PlayerInput;

fn input_with(binds: &Keybinds, held: &[KeyCode], pressed: &[KeyCode]) -> PlayerInput {
    player_input(
        binds,
        |key| held.contains(&key),
        |key| pressed.contains(&key),
    )
}

#[test]
fn the_shipped_keys_are_valid_each_action_has_a_primary_and_no_key_is_used_twice() {
    let binds = Keybinds::default();
    assert_eq!(binds.check(), Ok(()));
    assert!(binds.is_default());
    for action in Action::ALL {
        assert!(binds.key(action, Slot::Primary).is_some(), "{action:?}");
    }
}

#[test]
fn the_shipped_keys_do_what_the_fixed_keys_used_to() {
    let binds = Keybinds::default();
    for key in [KeyCode::A, KeyCode::Left] {
        assert_eq!(input_with(&binds, &[key], &[]).move_axis, -1.0);
    }
    for key in [KeyCode::D, KeyCode::Right] {
        assert_eq!(input_with(&binds, &[key], &[]).move_axis, 1.0);
    }
    assert_eq!(
        input_with(&binds, &[KeyCode::A, KeyCode::D], &[]).move_axis,
        0.0
    );
    for key in [KeyCode::Space, KeyCode::Z] {
        let input = input_with(&binds, &[key], &[key]);
        assert!(input.jump_pressed && input.jump_held, "{key:?}");
    }
    assert!(input_with(&binds, &[], &[KeyCode::X]).attack_pressed);
    let up = input_with(&binds, &[KeyCode::Up], &[KeyCode::Up]);
    assert!(up.pickup_pressed && up.up_held);
    assert!(input_with(&binds, &[KeyCode::Down], &[]).down_held);
    for key in [KeyCode::LeftShift, KeyCode::RightShift] {
        assert!(input_with(&binds, &[], &[key]).dash_pressed, "{key:?}");
    }
    assert!(input_with(&binds, &[KeyCode::C], &[]).heal_held);
    assert!(binds.triggered(Action::Inventory, |key| key == KeyCode::I));
    assert_eq!(input_with(&binds, &[], &[]), PlayerInput::default());
}

#[test]
fn edge_flags_come_only_from_pressed_keys_and_held_flags_only_from_held_keys() {
    let binds = Keybinds::default();
    let pressed_only = input_with(&binds, &[], &[KeyCode::Space]);
    assert!(pressed_only.jump_pressed && !pressed_only.jump_held);
    let held_only = input_with(&binds, &[KeyCode::Space], &[]);
    assert!(!held_only.jump_pressed && held_only.jump_held);
    // A held attack, dash or pickup key is not a new press.
    let held = input_with(&binds, &[KeyCode::X, KeyCode::LeftShift, KeyCode::Up], &[]);
    assert!(!held.attack_pressed && !held.dash_pressed && !held.pickup_pressed);
}

#[test]
fn a_rebound_action_answers_to_the_new_key_and_no_longer_to_the_old_one() {
    let mut binds = Keybinds::default();
    assert_eq!(
        binds.rebind(Action::Jump, Slot::Primary, KeyCode::Q),
        Ok(Rebound::default())
    );
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::Q));
    assert!(input_with(&binds, &[], &[KeyCode::Q]).jump_pressed);
    assert!(!input_with(&binds, &[], &[KeyCode::Space]).jump_pressed);
    // The alternate key is untouched.
    assert!(input_with(&binds, &[], &[KeyCode::Z]).jump_pressed);
    assert_eq!(binds.action_of(KeyCode::Space), None);
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn a_key_in_use_elsewhere_is_taken_and_the_two_slots_trade_keys() {
    let mut binds = Keybinds::default();
    let rebound = binds
        .rebind(Action::Jump, Slot::Primary, KeyCode::X)
        .unwrap();
    assert_eq!(
        rebound.swapped,
        Some(Swap {
            action: Action::Attack,
            slot: Slot::Primary,
            now: Some(KeyCode::Space),
        })
    );
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::X));
    assert_eq!(
        binds.key(Action::Attack, Slot::Primary),
        Some(KeyCode::Space)
    );
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn an_empty_alternate_that_takes_an_alternate_key_leaves_the_old_slot_empty() {
    let mut binds = Keybinds::default();
    let rebound = binds
        .rebind(Action::Attack, Slot::Alternate, KeyCode::Right)
        .unwrap();
    assert_eq!(
        rebound.swapped,
        Some(Swap {
            action: Action::MoveRight,
            slot: Slot::Alternate,
            now: None,
        })
    );
    assert_eq!(
        binds.key(Action::Attack, Slot::Alternate),
        Some(KeyCode::Right)
    );
    assert_eq!(binds.key(Action::MoveRight, Slot::Alternate), None);
    assert_eq!(
        binds.key(Action::MoveRight, Slot::Primary),
        Some(KeyCode::D)
    );
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn an_empty_alternate_cannot_take_a_primary_key_because_that_would_leave_it_without_one() {
    let mut binds = Keybinds::default();
    assert_eq!(
        binds.rebind(Action::Attack, Slot::Alternate, KeyCode::C),
        Err(RebindError::PrimaryOf(Action::Heal))
    );
    // Not even an action's own primary key goes into its empty alternate.
    assert_eq!(
        binds.rebind(Action::Attack, Slot::Alternate, KeyCode::X),
        Err(RebindError::PrimaryOf(Action::Attack))
    );
    assert!(binds.is_default());
}

#[test]
fn binding_the_key_a_slot_already_has_changes_nothing() {
    let mut binds = Keybinds::default();
    assert_eq!(
        binds.rebind(Action::Jump, Slot::Primary, KeyCode::Space),
        Ok(Rebound::default())
    );
    assert!(binds.is_default());
}

#[test]
fn the_two_slots_of_one_action_can_trade_keys() {
    let mut binds = Keybinds::default();
    binds
        .rebind(Action::Jump, Slot::Primary, KeyCode::Z)
        .unwrap();
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::Z));
    assert_eq!(
        binds.key(Action::Jump, Slot::Alternate),
        Some(KeyCode::Space)
    );
    binds
        .rebind(Action::Jump, Slot::Alternate, KeyCode::Z)
        .unwrap();
    assert_eq!(binds.key(Action::Jump, Slot::Primary), Some(KeyCode::Space));
    assert_eq!(binds.key(Action::Jump, Slot::Alternate), Some(KeyCode::Z));
    assert!(binds.is_default());
}

#[test]
fn keys_that_are_not_bindable_are_refused_and_nothing_changes() {
    let mut binds = Keybinds::default();
    for key in [
        KeyCode::Escape,
        KeyCode::Enter,
        KeyCode::KpEnter,
        KeyCode::Backspace,
        KeyCode::Delete,
        KeyCode::F2,
        KeyCode::F5,
        KeyCode::F12,
        KeyCode::LeftSuper,
        KeyCode::Unknown,
    ] {
        assert_eq!(
            binds.rebind(Action::Jump, Slot::Primary, key),
            Err(RebindError::NotBindable),
            "{key:?}"
        );
        assert!(!is_bindable(key));
    }
    assert!(binds.is_default());
}

#[test]
fn clearing_empties_an_alternate_but_never_a_primary() {
    let mut binds = Keybinds::default();
    assert_eq!(binds.clear(Action::Jump, Slot::Alternate), Ok(()));
    assert_eq!(
        binds.keys(Action::Jump).collect::<Vec<_>>(),
        [KeyCode::Space]
    );
    assert!(!input_with(&binds, &[], &[KeyCode::Z]).jump_pressed);
    // An alternate that is already empty stays empty without complaint.
    assert_eq!(binds.clear(Action::Attack, Slot::Alternate), Ok(()));
    let before = binds.clone();
    assert_eq!(
        binds.clear(Action::Jump, Slot::Primary),
        Err(RebindError::PrimaryRequired)
    );
    assert_eq!(binds, before);
    assert_eq!(binds.check(), Ok(()));
}

#[test]
fn reset_puts_every_action_back_on_its_shipped_keys() {
    let mut binds = Keybinds::default();
    binds
        .rebind(Action::Jump, Slot::Primary, KeyCode::Q)
        .unwrap();
    binds
        .rebind(Action::Dash, Slot::Primary, KeyCode::X)
        .unwrap();
    binds.clear(Action::Dash, Slot::Alternate).unwrap();
    assert!(!binds.is_default());
    binds.reset();
    assert!(binds.is_default());
    // Resetting shipped keys is harmless.
    binds.reset();
    assert!(binds.is_default());
}

#[test]
fn no_rebind_ever_leaves_a_key_bound_twice_or_an_action_without_a_primary() {
    let keys = [
        KeyCode::A,
        KeyCode::S,
        KeyCode::Space,
        KeyCode::Left,
        KeyCode::Z,
        KeyCode::X,
        KeyCode::Q,
        KeyCode::Up,
        KeyCode::RightShift,
        KeyCode::Escape,
    ];
    let mut starts = vec![Keybinds::default()];
    let mut moved = Keybinds::default();
    moved
        .rebind(Action::Attack, Slot::Alternate, KeyCode::Q)
        .unwrap();
    moved.clear(Action::Dash, Slot::Alternate).unwrap();
    starts.push(moved);
    for start in starts {
        for action in Action::ALL {
            for slot in Slot::ALL {
                for key in keys {
                    let mut binds = start.clone();
                    match binds.rebind(action, slot, key) {
                        Ok(_) => {
                            assert_eq!(binds.check(), Ok(()), "{action:?} {slot:?} {key:?}");
                            assert_eq!(binds.action_of(key), Some((action, slot)));
                        }
                        Err(_) => assert_eq!(binds, start, "{action:?} {slot:?} {key:?}"),
                    }
                }
            }
        }
    }
}

#[test]
fn check_names_the_problem() {
    let mut binds = Keybinds::default();
    binds.keys[Action::Attack.index()][Slot::Alternate.index()] = Some(KeyCode::Space);
    let error = binds.check().unwrap_err();
    assert!(error.contains("Space") && error.contains("Jump") && error.contains("Attack"));
    let mut binds = Keybinds::default();
    binds.keys[Action::Heal.index()][Slot::Primary.index()] = None;
    assert!(binds.check().unwrap_err().contains("Heal"));
    let mut binds = Keybinds::default();
    binds.keys[Action::Heal.index()][Slot::Primary.index()] = Some(KeyCode::Escape);
    assert!(binds.check().unwrap_err().contains("Heal"));
}

#[test]
fn every_action_and_slot_has_one_unique_name() {
    let mut names = Action::ALL.map(Action::name).to_vec();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), Action::ALL.len());
    let mut labels = Action::ALL.map(Action::label).to_vec();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), Action::ALL.len());
    for (index, action) in Action::ALL.into_iter().enumerate() {
        assert_eq!(Action::from_name(action.name()), Some(action));
        assert_eq!(action.index(), index);
    }
    assert_eq!(Action::from_name("fly"), None);
    assert_eq!(Slot::from_index(0), Slot::Primary);
    assert_eq!(Slot::from_index(1), Slot::Alternate);
    assert_eq!(Slot::from_index(99), Slot::Alternate);
}

#[test]
fn the_key_table_has_unique_keys_and_names_that_round_trip() {
    for (index, (key, name)) in BINDABLE.iter().enumerate() {
        assert_eq!(key_name(*key), Some(*name));
        assert_eq!(key_from_name(name), Some(*key));
        assert_eq!(key_from_name(&name.to_uppercase()), Some(*key));
        for (other_key, other_name) in &BINDABLE[index + 1..] {
            assert_ne!(key, other_key, "{name}");
            assert!(!name.eq_ignore_ascii_case(other_name), "{name}");
        }
    }
    assert_eq!(key_from_name("  left shift "), Some(KeyCode::LeftShift));
    assert_eq!(key_from_name("Escape"), None);
    assert_eq!(key_from_name(""), None);
    assert_eq!(key_name(KeyCode::Escape), None);
}

#[test]
fn first_pressed_picks_a_bindable_key_in_a_fixed_order_and_ignores_the_rest() {
    assert_eq!(first_pressed(|_| false), None);
    assert_eq!(
        first_pressed(|key| key == KeyCode::Space),
        Some(KeyCode::Space)
    );
    // Escape, Enter and the F keys never count as a pressed key to bind.
    assert_eq!(
        first_pressed(|key| matches!(key, KeyCode::Escape | KeyCode::Enter | KeyCode::F2)),
        None
    );
    // Two keys at once: the earlier one in the table, however they are asked.
    assert_eq!(
        first_pressed(|key| matches!(key, KeyCode::Z | KeyCode::A)),
        Some(KeyCode::A)
    );
}

#[test]
fn the_controls_hint_follows_the_primary_keys() {
    let mut binds = Keybinds::default();
    assert_eq!(
        hint_line(&binds),
        "A/D move   Space jump   X attack   Up pickup   Left Shift dash   I inventory   F5 reload"
    );
    binds
        .rebind(Action::Jump, Slot::Primary, KeyCode::W)
        .unwrap();
    binds
        .rebind(Action::Dash, Slot::Primary, KeyCode::RightControl)
        .unwrap();
    let hint = hint_line(&binds);
    assert!(
        hint.contains("W jump") && hint.contains("Right Ctrl dash"),
        "{hint}"
    );
}

#[test]
fn the_notes_name_the_key_the_action_and_the_slot() {
    let swap = Swap {
        action: Action::Dash,
        slot: Slot::Alternate,
        now: Some(KeyCode::Z),
    };
    assert_eq!(
        swap_note(KeyCode::LeftShift, swap),
        "Left Shift: swapped with Dash (alternate)"
    );
    assert_eq!(
        swap_note(KeyCode::Right, Swap { now: None, ..swap }),
        "Right: taken from Dash (alternate)"
    );
    let note = error_note(RebindError::PrimaryOf(Action::Heal), Some(KeyCode::C));
    assert!(note.contains("C is Heal's primary key"), "{note}");
    // Every message fits the settings screen's one-line message area (about 55 characters).
    let longest = Swap {
        action: Action::MoveRight,
        slot: Slot::Alternate,
        now: Some(KeyCode::Z),
    };
    for note in [
        swap_note(KeyCode::RightControl, longest),
        error_note(
            RebindError::PrimaryOf(Action::MoveRight),
            Some(KeyCode::RightControl),
        ),
        error_note(RebindError::PrimaryRequired, None),
        error_note(RebindError::NotBindable, None),
    ] {
        assert!(note.len() <= 60, "{note}");
    }
    assert!(error_note(RebindError::NotBindable, None).contains("cannot be used"));
    assert!(error_note(RebindError::PrimaryRequired, None).contains("primary"));
    assert_eq!(key_label(None), "-");
}
