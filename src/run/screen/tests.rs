use super::*;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);

fn result(outcome: RunOutcome) -> RunResult {
    RunResult {
        outcome,
        coins_earned: 9,
        coins_banked: 4,
        tier_before: 1,
        tier_after: if outcome == RunOutcome::Cleared { 2 } else { 1 },
    }
}

fn profile() -> Profile {
    Profile {
        coins: 25,
        tier: 2,
        cleared: 1,
    }
}

#[test]
fn each_outcome_has_its_heading() {
    assert_eq!(result_title(RunOutcome::Cleared), "Dungeon cleared");
    assert_eq!(result_title(RunOutcome::Died), "You died");
    assert_eq!(result_title(RunOutcome::Abandoned), "Run abandoned");
}

#[test]
fn a_clear_shows_the_tier_going_up_and_nothing_else_does() {
    assert_eq!(
        result_lines(&result(RunOutcome::Cleared), &profile()),
        [
            "Coins earned: 9",
            "Coins banked: 4",
            "Total banked: 25",
            "Difficulty tier 1 -> 2"
        ]
    );
    for outcome in [RunOutcome::Died, RunOutcome::Abandoned] {
        let lines = result_lines(&result(outcome), &profile());
        assert_eq!(lines.len(), 3, "{outcome:?}");
        assert!(lines.iter().all(|line| !line.contains("tier")));
    }
}

#[test]
fn the_hint_says_how_to_leave() {
    assert!(RETURN_HINT.contains("Enter") && RETURN_HINT.contains("click"));
    assert!(RETURN_HINT.contains("hub"));
}

#[test]
fn the_layout_is_centered_with_every_line_inside_the_panel() {
    for lines in [0, 1, 3, 4] {
        let layout = result_layout(VIEW, lines);
        let panel = layout.panel;
        assert!((panel.center().x - VIEW.x / 2.0).abs() < 0.01);
        assert!((panel.center().y - VIEW.y / 2.0).abs() < 0.01);
        assert!(panel.x >= 0.0 && panel.right() <= VIEW.x);
        assert!(panel.y >= 0.0 && panel.bottom() <= VIEW.y);
        assert_eq!(layout.lines.len(), lines);
        for at in layout
            .lines
            .iter()
            .chain([&layout.title, &layout.message, &layout.hint])
        {
            assert!(at.y > panel.y && at.y < panel.bottom(), "{at:?}");
            assert_eq!(at.x, layout.text_x);
        }
    }
}

#[test]
fn text_runs_top_to_bottom_without_overlap_and_a_longer_screen_is_taller() {
    let short = result_layout(VIEW, 3);
    let long = result_layout(VIEW, 4);
    let order: Vec<f32> = [short.title.y]
        .into_iter()
        .chain(short.lines.iter().map(|at| at.y))
        .chain([short.message.y, short.hint.y])
        .collect();
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    assert!(long.panel.h > short.panel.h);
    assert!(short.text_width > 0.0 && short.text_width < short.panel.w);
}

#[test]
fn the_layout_follows_the_view_it_is_given() {
    let small = result_layout(vec2(640.0, 360.0), 4);
    let large = result_layout(vec2(1920.0, 1080.0), 4);
    assert_eq!(small.panel.size(), large.panel.size());
    assert!(large.panel.x > small.panel.x && large.panel.y > small.panel.y);
}

#[test]
fn with_no_hub_the_hud_counter_is_the_plain_coin_count() {
    let counter = hud_counter(None, 7, &profile());
    assert_eq!(
        counter,
        Counter {
            coins: 7,
            caption: None
        }
    );
}

#[test]
fn in_the_hub_the_counter_shows_banked_coins_and_the_tier() {
    let counter = hud_counter(Some(RunPhase::Hub), 0, &profile());
    assert_eq!(counter.coins, 25);
    assert_eq!(counter.caption.as_deref(), Some("banked  |  tier 2"));
}

#[test]
fn in_a_run_the_counter_shows_this_runs_coins() {
    let counter = hud_counter(Some(RunPhase::Running), 7, &profile());
    assert_eq!(counter.coins, 7);
    assert_eq!(counter.caption.as_deref(), Some("this run"));
    let after = hud_counter(
        Some(RunPhase::Result(result(RunOutcome::Died))),
        7,
        &profile(),
    );
    assert_eq!(after.coins, 7);
}

#[test]
fn the_inventory_shows_both_counts_when_there_is_a_hub() {
    assert_eq!(
        inventory_coins(None, 3, &profile()),
        ("Coins: 3".into(), None)
    );
    let (run, banked) = inventory_coins(Some(RunPhase::Running), 3, &profile());
    assert_eq!(run, "Coins this run: 3");
    assert_eq!(banked.as_deref(), Some("Banked: 25  |  Tier 2"));
    let (hub, banked) = inventory_coins(Some(RunPhase::Hub), 0, &profile());
    assert_eq!(hub, "Coins this run: 0");
    assert!(banked.is_some());
}
