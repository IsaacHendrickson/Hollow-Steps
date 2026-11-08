use std::sync::Arc;

use hollow_steps::behavior::{
    Behavior, BehaviorContext, BehaviorDefinition, Condition, ConditionContext,
};
use hollow_steps::player::WorldBounds;
use macroquad::math::Vec2;

fn context() -> BehaviorContext {
    BehaviorContext {
        player_position: Vec2::new(400.0, 442.0),
        health: 3,
        size: Vec2::new(36.0, 36.0),
        bounds: WorldBounds::new(0.0, 960.0, 460.0),
        patrol_left: 100.0,
        patrol_right: 200.0,
    }
}

fn machine(actions: &str) -> Behavior {
    let definition = BehaviorDefinition::from_ron(&format!(
        "(initial_state: \"run\", states: [(name: \"run\", actions: [{actions}], transitions: [])])"
    ))
    .unwrap();
    Behavior::new(Arc::new(definition)).unwrap()
}

fn conditions() -> ConditionContext {
    ConditionContext {
        state_elapsed: 0.5,
        player_distance: 50.0,
        health: 2,
        action_finished: false,
    }
}

#[test]
fn move_toward_player_moves_both_ways_without_overshooting() {
    for (player_x, expected) in [(400.0, 125.0), (50.0, 75.0)] {
        let mut behavior = machine("move_toward_player(speed: 100.0)");
        let mut position = Vec2::new(100.0, 442.0);
        let input = BehaviorContext {
            player_position: Vec2::new(player_x, 100.0),
            ..context()
        };
        behavior.update(&mut position, input, 0.25);
        assert_eq!(position, Vec2::new(expected, 442.0));
        assert!(!behavior.action_finished());
        behavior.update(&mut position, input, 100.0);
        assert_eq!(position.x, player_x);
        assert!(behavior.action_finished());
    }
}

#[test]
fn move_toward_player_clamps_to_walls_and_zero_speed_does_not_finish() {
    let mut behavior = machine("move_toward_player(speed: 100.0)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(
        &mut position,
        BehaviorContext {
            player_position: Vec2::new(2000.0, 442.0),
            ..context()
        },
        f32::MAX,
    );
    assert_eq!(position.x, 942.0);
    assert!(behavior.action_finished());
    let mut stopped = machine("move_toward_player(speed: 0.0)");
    stopped.update(&mut position, context(), 10.0);
    assert_eq!(position.x, 942.0);
    assert!(!stopped.action_finished());
}

#[test]
fn move_away_from_player_moves_both_ways_and_finishes_at_walls() {
    for (player_x, expected, wall) in [(50.0, 125.0, 942.0), (200.0, 75.0, 18.0)] {
        let mut behavior = machine("move_away_from_player(speed: 100.0)");
        let mut position = Vec2::new(100.0, 442.0);
        let input = BehaviorContext {
            player_position: Vec2::new(player_x, 442.0),
            ..context()
        };
        behavior.update(&mut position, input, 0.25);
        assert_eq!(position.x, expected);
        assert!(!behavior.action_finished());
        behavior.update(&mut position, input, f32::MAX);
        assert_eq!(position.x, wall);
        assert!(behavior.action_finished());
    }
}

#[test]
fn move_away_uses_facing_when_player_overlaps_and_respects_zero_speed() {
    let input = BehaviorContext {
        player_position: Vec2::new(100.0, 442.0),
        ..context()
    };
    let mut position = input.player_position;
    let mut behavior = machine("move_away_from_player(speed: 100.0)");
    behavior.update(&mut position, input, 0.25);
    assert_eq!(position.x, 125.0);
    let mut stopped = machine("move_away_from_player(speed: 0.0)");
    stopped.update(&mut position, input, 100.0);
    assert_eq!(position.x, 125.0);
    assert!(!stopped.action_finished());
}

#[test]
fn patrol_reflects_overshoot_and_returns_from_outside_without_teleporting() {
    let mut behavior = machine("patrol(speed: 100.0)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 4.25);
    assert_eq!(position.x, 125.0);
    behavior.update(&mut position, context(), 1.0);
    assert_eq!(position.x, 175.0);
    position.x = 50.0;
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(position.x, 75.0);
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position.x, 125.0);
    assert!(!behavior.action_finished());
}

#[test]
fn patrol_handles_stationary_intervals_zero_speed_and_extreme_time() {
    let mut behavior = machine("patrol(speed: 100.0)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(
        &mut position,
        BehaviorContext {
            patrol_right: 100.0,
            ..context()
        },
        f32::MAX,
    );
    assert_eq!(position.x, 100.0);
    behavior.update(&mut position, context(), f32::MAX);
    assert!(position.x >= 100.0 && position.x <= 200.0);
    let before = position;
    machine("patrol(speed: 0.0)").update(&mut position, context(), 10.0);
    assert_eq!(position, before);
}

#[test]
fn wait_blocks_actions_then_passes_remaining_time_in_order() {
    let mut behavior =
        machine("wait(seconds: 0.5), move_toward_player(speed: 100.0), wait(seconds: 0.5)");
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(position.x, 100.0);
    behavior.update(&mut position, context(), 0.5);
    assert_eq!(position.x, 125.0);
    assert!(!behavior.action_finished());
    behavior.update(&mut position, context(), 3.25);
    assert_eq!(position.x, 400.0);
    assert!(behavior.action_finished());
}

#[test]
fn telegraph_exposes_a_flag_without_moving_and_expires_at_its_duration() {
    let mut behavior = machine("telegraph(seconds: 0.5), wait(seconds: 0.25)");
    let mut position = Vec2::new(100.0, 442.0);
    assert!(behavior.is_telegraphing());
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.is_telegraphing());
    assert_eq!(position, Vec2::new(100.0, 442.0));
    behavior.update(&mut position, context(), 0.25);
    assert!(!behavior.is_telegraphing());
    assert!(!behavior.action_finished());
    behavior.update(&mut position, context(), 0.25);
    assert!(behavior.action_finished());
}

#[test]
fn zero_duration_actions_finish_and_invalid_time_never_advances_any_action() {
    for action in [
        "wait(seconds: 0.0)",
        "telegraph(seconds: 0.0)",
        "patrol(speed: 100.0)",
        "move_toward_player(speed: 100.0)",
        "move_away_from_player(speed: 100.0)",
    ] {
        let mut behavior = machine(action);
        let mut position = Vec2::new(100.0, 442.0);
        for dt in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            behavior.update(&mut position, context(), dt);
            assert_eq!(position, Vec2::new(100.0, 442.0));
            assert_eq!(behavior.state_elapsed(), 0.0);
            assert!(!behavior.action_finished());
        }
    }
    let mut behavior = machine("wait(seconds: 0.0), telegraph(seconds: 0.0)");
    behavior.update(&mut Vec2::new(100.0, 442.0), context(), f32::MAX);
    assert!(behavior.action_finished());
    assert!(!behavior.is_telegraphing());
}

#[test]
fn timer_elapsed_includes_equality() {
    assert!(Condition::TimerElapsed(0.5).matches(conditions()));
    assert!(Condition::TimerElapsed(0.0).matches(conditions()));
    assert!(!Condition::TimerElapsed(0.75).matches(conditions()));
}

#[test]
fn player_distance_less_than_is_strict() {
    assert!(Condition::PlayerDistanceLessThan(51.0).matches(conditions()));
    assert!(!Condition::PlayerDistanceLessThan(50.0).matches(conditions()));
    assert!(!Condition::PlayerDistanceLessThan(49.0).matches(conditions()));
}

#[test]
fn player_distance_greater_than_is_strict() {
    assert!(Condition::PlayerDistanceGreaterThan(49.0).matches(conditions()));
    assert!(!Condition::PlayerDistanceGreaterThan(50.0).matches(conditions()));
    assert!(!Condition::PlayerDistanceGreaterThan(51.0).matches(conditions()));
}

#[test]
fn health_below_uses_absolute_health_and_is_strict() {
    assert!(Condition::HealthBelow(3).matches(conditions()));
    assert!(!Condition::HealthBelow(2).matches(conditions()));
    assert!(!Condition::HealthBelow(0).matches(conditions()));
}

#[test]
fn action_finished_requires_the_whole_sequence_to_finish() {
    assert!(!Condition::ActionFinished.matches(conditions()));
    assert!(Condition::ActionFinished.matches(ConditionContext {
        action_finished: true,
        ..conditions()
    }));
    assert!(machine("").action_finished());
}

#[test]
fn transitions_reset_timers_and_actions_and_wait_for_the_sequence() {
    let definition = BehaviorDefinition::from_ron(
        r#"(
        initial_state: "wait", states: [
            (name: "wait", actions: [wait(seconds: 0.25), wait(seconds: 0.25)],
             transitions: [(target: "warn", condition: action_finished)]),
            (name: "warn", actions: [telegraph(seconds: 1.0)],
             transitions: [(target: "wait", condition: timer_elapsed(0.25))]),
        ])"#,
    )
    .unwrap();
    let mut behavior = Behavior::new(Arc::new(definition)).unwrap();
    let mut position = Vec2::new(100.0, 442.0);
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.state_name(), "wait");
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.state_name(), "warn");
    assert_eq!(behavior.state_elapsed(), 0.0);
    assert!(behavior.is_telegraphing());
    behavior.update(&mut position, context(), 0.25);
    assert_eq!(behavior.state_name(), "wait");
    assert_eq!(behavior.state_elapsed(), 0.0);
    assert!(!behavior.action_finished());
}

#[test]
fn first_matching_transition_wins_and_uses_center_distance_on_both_axes() {
    let definition = BehaviorDefinition::from_ron(
        r#"(
        initial_state: "start", states: [
            (name: "start", actions: [], transitions: [
                (target: "near", condition: player_distance_less_than(50.0)),
                (target: "far", condition: player_distance_greater_than(50.0)),
                (target: "hurt", condition: health_below(4)),
            ]),
            (name: "near", actions: [], transitions: []),
            (name: "far", actions: [], transitions: []),
            (name: "hurt", actions: [], transitions: []),
        ])"#,
    )
    .unwrap();
    for (player, expected) in [
        (Vec2::new(100.0, 342.0), "far"),
        (Vec2::new(100.0, 442.0), "near"),
        (Vec2::new(100.0, 392.0), "hurt"),
    ] {
        let mut behavior = Behavior::new(Arc::new(definition.clone())).unwrap();
        behavior.update(
            &mut Vec2::new(100.0, 442.0),
            BehaviorContext {
                player_position: player,
                ..context()
            },
            0.01,
        );
        assert_eq!(behavior.state_name(), expected);
    }
}

#[test]
fn self_transitions_are_bounded_even_for_instant_actions_and_large_time() {
    let definition = BehaviorDefinition::from_ron(
        r#"(
        initial_state: "loop", states: [(name: "loop", actions: [wait(seconds: 0.0)],
        transitions: [(target: "loop", condition: action_finished)])])"#,
    )
    .unwrap();
    let mut behavior = Behavior::new(Arc::new(definition)).unwrap();
    behavior.update(&mut Vec2::new(100.0, 442.0), context(), f32::MAX);
    assert_eq!(behavior.state_name(), "loop");
    assert_eq!(behavior.state_elapsed(), 0.0);
    assert!(!behavior.action_finished());
}

#[test]
fn validation_rejects_missing_initial_target_duplicate_and_empty_states() {
    for (source, expected) in [
        (
            r#"(initial_state: "absent", states: [])"#,
            "missing initial state",
        ),
        (
            r#"(initial_state: "run", states: [(name: "run", actions: [], transitions: [(target: "gone", condition: action_finished)])])"#,
            "transition to missing state \"gone\"",
        ),
        (
            r#"(initial_state: "run", states: [(name: "run", actions: [], transitions: []), (name: "run", actions: [], transitions: [])])"#,
            "duplicate state name",
        ),
        (
            r#"(initial_state: "", states: [(name: "", actions: [], transitions: [])])"#,
            "empty or duplicate state",
        ),
    ] {
        let error = BehaviorDefinition::from_ron(source).unwrap_err();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn validation_rejects_invalid_action_and_condition_parameters() {
    for action in [
        "move_toward_player(speed: -1.0)",
        "move_away_from_player(speed: inf)",
        "patrol(speed: NaN)",
        "wait(seconds: -1.0)",
        "telegraph(seconds: inf)",
    ] {
        let source = format!("(initial_state: \"run\", states: [(name: \"run\", actions: [{action}], transitions: [])])");
        let error = BehaviorDefinition::from_ron(&source).unwrap_err();
        assert!(error.contains("state \"run\", action 0"), "{error}");
    }
    for condition in [
        "timer_elapsed(-1.0)",
        "player_distance_less_than(NaN)",
        "player_distance_greater_than(inf)",
    ] {
        let source = format!("(initial_state: \"run\", states: [(name: \"run\", actions: [], transitions: [(target: \"run\", condition: {condition})])])");
        assert!(BehaviorDefinition::from_ron(&source)
            .unwrap_err()
            .contains("transition condition"));
    }
}

#[test]
fn validation_reports_ron_syntax_unknown_actions_conditions_and_fields() {
    for source in [
        "this is not RON",
        r#"(initial_state: "run", states: [], typo: 1)"#,
        r#"(initial_state: "run", states: [(name: "run", actions: [fly(speed: 2.0)], transitions: [])])"#,
        r#"(initial_state: "run", states: [(name: "run", actions: [wait(seconds: 1.0, typo: 1)], transitions: [])])"#,
        r#"(initial_state: "run", states: [(name: "run", actions: [], transitions: [(target: "run", condition: unknown)])])"#,
    ] {
        assert!(BehaviorDefinition::from_ron(source).is_err(), "{source}");
    }
}
