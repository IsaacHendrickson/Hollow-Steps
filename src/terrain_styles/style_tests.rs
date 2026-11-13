//! `RoomStyle`, the style a room file carries: its defaults, RON forms, round trips and errors.
use super::*;

fn catalog() -> TerrainStyles {
    TerrainStyles::default()
}

// ---------------------------------------------------------------------------------------
// A room's style in RON
// ---------------------------------------------------------------------------------------

fn rich_style() -> RoomStyle {
    RoomStyle {
        preset: "cave".into(),
        fill: Some((0.25, 0.5, 0.75)),
        edge: Some((0.0, 0.1, 0.2)),
        highlight: Some((1.0, 0.9, 0.8)),
        accent: Some((0.3, 0.6, 0.9)),
        moss: Some(false),
        seed_offset: 42,
        draw_boundary: false,
    }
}

#[test]
fn the_default_style_is_stone_with_nothing_changed() {
    let style = RoomStyle::default();

    assert_eq!(style.preset, "stone");
    assert!(style.fill.is_none() && style.edge.is_none());
    assert!(style.highlight.is_none() && style.accent.is_none() && style.moss.is_none());
    assert_eq!(style.seed_offset, 0);
    assert!(style.draw_boundary);
    assert!(style.is_default());
    assert_eq!(RoomStyle::named("stone"), style);
    assert!(!RoomStyle::named("cave").is_default());
}

#[test]
fn changing_any_one_thing_makes_a_style_non_default() {
    let edits: [fn(&mut RoomStyle); 8] = [
        |s| s.preset = "cave".into(),
        |s| s.fill = Some((0.1, 0.1, 0.1)),
        |s| s.edge = Some((0.1, 0.1, 0.1)),
        |s| s.highlight = Some((0.1, 0.1, 0.1)),
        |s| s.accent = Some((0.1, 0.1, 0.1)),
        |s| s.moss = Some(true),
        |s| s.seed_offset = 1,
        |s| s.draw_boundary = false,
    ];
    for (index, edit) in edits.iter().enumerate() {
        let mut style = RoomStyle::default();
        edit(&mut style);
        assert!(!style.is_default(), "edit {index}");
    }
}

#[test]
fn a_style_round_trips_through_ron_compact_and_pretty() {
    for style in [
        rich_style(),
        RoomStyle::default(),
        RoomStyle::named("ruins"),
    ] {
        let compact = ron::to_string(&style).unwrap();
        assert_eq!(
            ron::from_str::<RoomStyle>(&compact).unwrap(),
            style,
            "{compact}"
        );
        let pretty = ron::ser::to_string_pretty(&style, ron::ser::PrettyConfig::default()).unwrap();
        assert_eq!(
            ron::from_str::<RoomStyle>(&pretty).unwrap(),
            style,
            "{pretty}"
        );
    }
}

#[test]
fn only_what_a_room_changes_is_written_and_overrides_are_written_bare() {
    assert_eq!(
        ron::to_string(&RoomStyle::named("cave")).unwrap(),
        r#"(preset:"cave")"#
    );
    let mut style = RoomStyle::named("cave");
    style.seed_offset = 7;
    assert_eq!(
        ron::to_string(&style).unwrap(),
        r#"(preset:"cave",seed_offset:7)"#
    );
    style.draw_boundary = false;
    assert!(ron::to_string(&style)
        .unwrap()
        .ends_with("seed_offset:7,draw_boundary:false)"));

    let text = ron::to_string(&rich_style()).unwrap();
    assert!(text.contains("fill:(0.25,0.5,0.75)"), "{text}");
    assert!(text.contains("moss:false"), "{text}");
    assert!(!text.contains("Some"), "{text}");
}

#[test]
fn every_field_is_optional() {
    assert_eq!(
        ron::from_str::<RoomStyle>("()").unwrap(),
        RoomStyle::default()
    );
    assert_eq!(
        ron::from_str::<RoomStyle>(r#"(preset: "cave")"#).unwrap(),
        RoomStyle::named("cave")
    );
    let style = ron::from_str::<RoomStyle>("(moss: false)").unwrap();
    assert_eq!(style.moss, Some(false));
    assert_eq!(
        style.preset, "stone",
        "naming only moss keeps the default preset"
    );
    let style = ron::from_str::<RoomStyle>("(draw_boundary: false)").unwrap();
    assert!(!style.draw_boundary);
}

#[test]
fn the_old_palette_format_still_loads_as_overrides() {
    // How a room file spelled its palette before presets existed.
    let style: RoomStyle = ron::from_str(
        "(fill: (0.25, 0.5, 0.75), edge: (0.0, 0.1, 0.2), highlight: (1.0, 0.9, 0.8), \
         accent: (0.3, 0.6, 0.9), seed_offset: 42, moss: false)",
    )
    .unwrap();

    assert_eq!(
        style,
        RoomStyle {
            preset: "stone".into(),
            draw_boundary: true,
            ..rich_style()
        }
    );
    // Over stone, which is the original look, that is exactly the palette it described.
    let look = catalog().resolve(&style).unwrap();
    assert_eq!(look.palette.fill, (0.25, 0.5, 0.75));
    assert_eq!(look.speckle_density, 1.0);
}

#[test]
fn rons_own_some_and_none_are_accepted_too() {
    let style: RoomStyle =
        ron::from_str("(fill: Some((0.1, 0.2, 0.3)), edge: None, moss: Some(true), accent: None)")
            .unwrap();

    assert_eq!(style.fill, Some((0.1, 0.2, 0.3)));
    assert_eq!(style.edge, None);
    assert_eq!(style.moss, Some(true));
    assert_eq!(style.accent, None);
}

#[test]
fn bad_text_in_a_style_is_an_error_that_says_what_was_expected() {
    let error = ron::from_str::<RoomStyle>("(fil: (0.1, 0.1, 0.1))")
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("fil"),
        "an unknown field is an error: {error}"
    );

    let error = ron::from_str::<RoomStyle>(r#"(fill: "red")"#)
        .unwrap_err()
        .to_string();
    assert!(error.contains("expected a color"), "{error}");
    let error = ron::from_str::<RoomStyle>("(fill: (0.1, 0.2))")
        .unwrap_err()
        .to_string();
    assert!(error.contains("expected a color"), "{error}");
    let error = ron::from_str::<RoomStyle>("(moss: 3)")
        .unwrap_err()
        .to_string();
    assert!(error.contains("expected true or false"), "{error}");
    assert!(ron::from_str::<RoomStyle>("(preset: 3)").is_err());
    assert!(ron::from_str::<RoomStyle>("(seed_offset: -1)").is_err());
}
