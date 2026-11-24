//! The Generate row's logic with no window: what `set_generate_choice` does and what the label
//! says. The world is the F4 tests' scratch world with a three-piece pool added.
use super::super::tests::Scratch;
use super::*;
use hollow_steps::player::PlayerInput;

/// One piece file: a floor, one or two exits on the side walls (aimed at the hub, which is what
/// lets the authored world load) and the piece mark.
fn piece(role: &str, sides: &[&str], boss: bool) -> String {
    let exits: Vec<String> = sides
        .iter()
        .map(|side| {
            format!(
                "(edge: {side}, offset: 364.0, length: 96.0, target_room: Custom(0), \
                 spawn: (480.0, 436.0))"
            )
        })
        .collect();
    let boss = if boss {
        r#"Some((definition: "Ash Warden", position: (560.0, 436.0)))"#
    } else {
        "None"
    };
    format!(
        "(enemies: [], boss: {boss}, layout: Some((bounds: (0.0, 0.0, 960.0, 540.0), \
         floor_y: 460.0, spawn: (480.0, 436.0), items: [], exits: [{}], \
         terrain: [(0.0, 460.0, 960.0, 80.0)], piece: Some((role: {role}, weight: 1)), \
         slots: [])))",
        exits.join(", ")
    )
}

/// A scratch world with a Start, a Middle and a Boss piece that the generator can join, and
/// `variety.ron` with the generator `enabled` or not.
pub(super) fn world_with_pool(enabled: bool) -> Scratch {
    let scratch = Scratch::new(true);
    let rooms = scratch.0.join("rooms");
    let files = [
        ("room_1.ron", piece("Start", &["Right"], false)),
        ("room_2.ron", piece("Middle", &["Left", "Right"], false)),
        ("room_3.ron", piece("Boss", &["Left"], true)),
    ];
    for (name, text) in files {
        std::fs::write(rooms.join(name), text).unwrap();
    }
    std::fs::write(
        scratch.0.join("variety.ron"),
        format!(
            "(flip_percent: 0.0, generator: (enabled: {enabled}, rooms: (min: 3, max: 3), \
             spine: (min: 3, max: 3), keys: (min: 0, max: 0), mirror_percent: 0.0, attempts: 50))"
        ),
    )
    .unwrap();
    scratch
}

fn leave_the_hub(game: &mut GameState) {
    game.update(
        PlayerInput {
            move_axis: 1.0,
            ..Default::default()
        },
        10.0,
    );
}

#[test]
fn setting_the_choice_changes_it_and_nothing_else() {
    let scratch = world_with_pool(true);
    let mut game = scratch.game();
    let file = std::fs::read_to_string(scratch.0.join("variety.ron")).unwrap();
    assert_eq!(game.generate_choice(), GenerateChoice::Auto);
    for choice in [
        GenerateChoice::Always,
        GenerateChoice::Never,
        GenerateChoice::Auto,
    ] {
        assert_eq!(set_generate_choice(&mut game, choice), Ok(()));
        assert_eq!(game.generate_choice(), choice);
    }
    assert!(game.variety_rules().generator.enabled);
    // It does not edit the file, and it is not in the profile.
    assert_eq!(
        std::fs::read_to_string(scratch.0.join("variety.ron")).unwrap(),
        file
    );
    assert!(!scratch.profile_file().exists());
}

#[test]
fn the_label_says_what_the_next_run_does() {
    let scratch = world_with_pool(true);
    let mut game = scratch.game();
    assert!(
        generate_label(&game).starts_with("The next run is generated when the pool has a Start"),
        "{}",
        generate_label(&game)
    );
    set_generate_choice(&mut game, GenerateChoice::Never).unwrap();
    assert_eq!(
        generate_label(&game),
        "The next run plays the fixed dungeon"
    );
    set_generate_choice(&mut game, GenerateChoice::Always).unwrap();
    assert_eq!(
        generate_label(&game),
        "The next run is generated, or does not start and says why"
    );

    // With the rules off, Auto says so and Always is still Always.
    let off = world_with_pool(false);
    let mut game = off.game();
    assert_eq!(
        generate_label(&game),
        "The next run plays the fixed dungeon: generator.enabled is off in variety.ron"
    );
    set_generate_choice(&mut game, GenerateChoice::Always).unwrap();
    assert!(generate_label(&game).contains("generated"));
}

#[test]
fn during_a_run_the_label_also_says_whether_that_run_is_generated() {
    let scratch = world_with_pool(true);
    let mut game = scratch.game();
    leave_the_hub(&mut game);
    assert!(game.run_in_progress());
    assert!(game.run_generated());
    assert!(
        generate_label(&game).starts_with("This run is generated; "),
        "{}",
        generate_label(&game)
    );

    game.abandon_run();
    game.continue_after_result();
    set_generate_choice(&mut game, GenerateChoice::Never).unwrap();
    leave_the_hub(&mut game);
    assert!(game.run_in_progress());
    assert!(!game.run_generated());
    assert!(
        generate_label(&game).starts_with("This run is the fixed dungeon; "),
        "{}",
        generate_label(&game)
    );
}

#[test]
fn in_the_hub_the_label_is_only_about_the_next_run() {
    let scratch = world_with_pool(true);
    let game = scratch.game();
    assert!(!generate_label(&game).starts_with("This run"));
}
