//! The Flip row's logic with no window: what `set_flip_choice` does and what the label says.
use super::super::tests::Scratch;
use super::*;

fn game_with(percent: &str) -> (Scratch, GameState) {
    let scratch = Scratch::new(true);
    std::fs::write(
        scratch.0.join("variety.ron"),
        format!("(flip_percent: {percent})"),
    )
    .unwrap();
    let game = scratch.game();
    (scratch, game)
}

#[test]
fn setting_the_choice_changes_it_and_nothing_else() {
    let (scratch, mut game) = game_with("50.0");
    assert_eq!(game.flip_choice(), FlipChoice::Auto);
    for choice in [FlipChoice::Always, FlipChoice::Never, FlipChoice::Auto] {
        assert_eq!(set_flip_choice(&mut game, choice), Ok(()));
        assert_eq!(game.flip_choice(), choice);
    }
    assert_eq!(game.variety_rules().flip_percent, 50.0);
    // It does not edit the file.
    let file = std::fs::read_to_string(scratch.0.join("variety.ron")).unwrap();
    assert_eq!(file, "(flip_percent: 50.0)");
    assert!(!scratch.profile_file().exists(), "nor the profile");
}

#[test]
fn the_label_says_what_the_next_run_does() {
    let (_scratch, mut game) = game_with("25.0");
    assert!(
        flip_label(&game).contains("25% of seeds"),
        "{}",
        flip_label(&game)
    );
    set_flip_choice(&mut game, FlipChoice::Never).unwrap();
    assert_eq!(flip_label(&game), "The next run is not flipped");
    set_flip_choice(&mut game, FlipChoice::Always).unwrap();
    assert_eq!(flip_label(&game), "The next run is flipped");
}

#[test]
fn during_a_run_the_label_also_says_whether_that_run_is_flipped() {
    let (_scratch, mut game) = game_with("0.0");
    set_flip_choice(&mut game, FlipChoice::Always).unwrap();
    game.update(
        hollow_steps::player::PlayerInput {
            move_axis: 1.0,
            ..Default::default()
        },
        10.0,
    );
    assert!(game.run_in_progress());
    assert!(game.run_flipped());
    assert!(
        flip_label(&game).starts_with("This run is flipped;"),
        "{}",
        flip_label(&game)
    );
    set_flip_choice(&mut game, FlipChoice::Never).unwrap();
    assert!(flip_label(&game).starts_with("This run is flipped; the next run is not flipped"));
    game.abandon_run();
    game.continue_after_result();
    assert!(!flip_label(&game).contains("This run"));
}
