use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "hollow-terrain-styles-{}-{}",
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

fn catalog() -> TerrainStyles {
    TerrainStyles::default()
}

/// A catalog file with one preset, where `edits` replace pieces of its text.
fn one_preset(edits: &[(&str, &str)]) -> String {
    let mut text = String::from(
        r#"(presets: [(
            name: "plain",
            fill: (0.2, 0.3, 0.4),
            edge: (0.1, 0.1, 0.1),
            highlight: (0.7, 0.7, 0.7),
            accent: (0.3, 0.6, 0.3),
            moss: true,
            speckle_density: 1.0,
            crack_density: 1.0,
            boundary_thickness: 40.0,
        )])"#,
    );
    for (from, to) in edits {
        assert!(text.contains(from), "template has no {from:?}");
        text = text.replace(from, to);
    }
    text
}

fn preset(name: &str) -> TerrainPreset {
    catalog().preset(name).unwrap().clone()
}

fn with_style(preset: &str, edit: impl FnOnce(&mut RoomStyle)) -> RoomStyle {
    let mut style = RoomStyle::named(preset);
    edit(&mut style);
    style
}

// ---------------------------------------------------------------------------------------
// The shipped presets
// ---------------------------------------------------------------------------------------

#[test]
fn the_embedded_presets_are_stone_mossy_cave_and_ruins() {
    let styles = catalog();

    assert_eq!(styles.names(), ["stone", "mossy", "cave", "ruins"]);
    assert_eq!(styles.presets().len(), 4);
    styles.validate().unwrap();
    assert!(styles.preset("stone").is_some());
    assert!(styles.preset("granite").is_none());
}

#[test]
fn stone_is_the_original_look_and_what_a_room_without_a_style_gets() {
    let stone = catalog().resolve(&RoomStyle::default()).unwrap();

    assert_eq!(stone, Look::default());
    assert_eq!(stone.palette, TerrainStyle::default());
    assert_eq!(RoomStyle::default().preset, DEFAULT_PRESET);
    assert_eq!(DEFAULT_PRESET, "stone");
}

#[test]
fn every_preset_resolves_to_its_own_numbers() {
    let styles = catalog();
    for preset in styles.presets() {
        let look = styles.resolve(&RoomStyle::named(&preset.name)).unwrap();

        assert_eq!(look.palette.fill, preset.fill, "{}", preset.name);
        assert_eq!(look.palette.edge, preset.edge);
        assert_eq!(look.palette.highlight, preset.highlight);
        assert_eq!(look.palette.accent, preset.accent);
        assert_eq!(look.palette.moss, preset.moss);
        assert_eq!(look.palette.seed_offset, 0);
        assert_eq!(look.speckle_density, preset.speckle_density);
        assert_eq!(look.crack_density, preset.crack_density);
        assert_eq!(look.boundary_thickness, preset.boundary_thickness);
        assert!(look.draw_boundary);
    }
}

#[test]
fn the_presets_differ_from_each_other() {
    let styles = catalog();
    let presets = styles.presets();
    for (index, a) in presets.iter().enumerate() {
        for b in &presets[index + 1..] {
            assert_ne!(a.fill, b.fill, "{} and {}", a.name, b.name);
            assert_ne!(a.highlight, b.highlight, "{} and {}", a.name, b.name);
        }
    }
    // They differ in the numbers too, not just the colors.
    assert!(presets
        .iter()
        .any(|p| p.boundary_thickness != presets[0].boundary_thickness));
    assert!(presets
        .iter()
        .any(|p| p.crack_density != presets[0].crack_density));
    assert!(presets.iter().any(|p| !p.moss));
}

// ---------------------------------------------------------------------------------------
// Resolving: preset plus overrides
// ---------------------------------------------------------------------------------------

#[test]
fn a_rooms_own_colors_and_moss_beat_the_preset() {
    let styles = catalog();
    let cave = preset("cave");
    let red = (0.9, 0.1, 0.1);

    let fill = styles
        .resolve(&with_style("cave", |s| s.fill = Some(red)))
        .unwrap();
    assert_eq!(fill.palette.fill, red);
    assert_eq!(fill.palette.edge, cave.edge, "the rest stays the preset's");
    assert_eq!(fill.palette.highlight, cave.highlight);
    assert_eq!(fill.palette.accent, cave.accent);

    let edge = styles
        .resolve(&with_style("cave", |s| s.edge = Some(red)))
        .unwrap();
    assert_eq!((edge.palette.edge, edge.palette.fill), (red, cave.fill));
    let highlight = styles
        .resolve(&with_style("cave", |s| s.highlight = Some(red)))
        .unwrap();
    assert_eq!(highlight.palette.highlight, red);
    assert_eq!(highlight.palette.edge, cave.edge);
    let accent = styles
        .resolve(&with_style("cave", |s| s.accent = Some(red)))
        .unwrap();
    assert_eq!(accent.palette.accent, red);
    assert_eq!(accent.palette.highlight, cave.highlight);

    // Moss can be switched either way against whatever the preset says.
    let ruins = preset("ruins");
    assert!(!ruins.moss && cave.moss);
    let grows = styles
        .resolve(&with_style("ruins", |s| s.moss = Some(true)))
        .unwrap();
    assert!(grows.palette.moss);
    let bare = styles
        .resolve(&with_style("cave", |s| s.moss = Some(false)))
        .unwrap();
    assert!(!bare.palette.moss);
    assert!(
        styles
            .resolve(&RoomStyle::named("cave"))
            .unwrap()
            .palette
            .moss
    );
}

#[test]
fn overrides_never_touch_the_preset_numbers() {
    let styles = catalog();
    let style = with_style("ruins", |s| {
        s.fill = Some((0.5, 0.5, 0.5));
        s.moss = Some(true);
        s.seed_offset = 9;
    });
    let look = styles.resolve(&style).unwrap();
    let ruins = preset("ruins");

    assert_eq!(look.speckle_density, ruins.speckle_density);
    assert_eq!(look.crack_density, ruins.crack_density);
    assert_eq!(look.boundary_thickness, ruins.boundary_thickness);
}

#[test]
fn the_seed_offset_and_the_boundary_switch_come_from_the_room() {
    let styles = catalog();
    let look = styles
        .resolve(&with_style("mossy", |s| {
            s.seed_offset = 1234;
            s.draw_boundary = false;
        }))
        .unwrap();

    assert_eq!(look.palette.seed_offset, 1234);
    assert!(!look.draw_boundary);
    assert_eq!(look.palette.fill, preset("mossy").fill);
}

#[test]
fn an_unknown_preset_is_an_error_that_lists_the_known_ones() {
    let error = catalog().resolve(&RoomStyle::named("granite")).unwrap_err();

    assert!(error.contains("terrain_style"), "{error}");
    assert!(error.contains("\"granite\""), "{error}");
    assert!(
        error.contains("known presets: stone, mossy, cave, ruins"),
        "{error}"
    );
    // The check is exact: case and spaces count.
    for name in ["Stone", "stone ", "", "cave\n"] {
        assert!(
            catalog().resolve(&RoomStyle::named(name)).is_err(),
            "{name:?}"
        );
    }
}

#[test]
fn a_replacement_catalog_decides_which_names_are_known() {
    let only_plain = TerrainStyles::from_ron(&one_preset(&[])).unwrap();

    assert_eq!(only_plain.names(), ["plain"]);
    assert!(only_plain.resolve(&RoomStyle::named("plain")).is_ok());
    // "stone" is the default, but a catalog without it simply does not know it.
    let error = only_plain.resolve(&RoomStyle::default()).unwrap_err();
    assert!(error.contains("known presets: plain"), "{error}");
}

#[test]
fn bad_override_colors_are_errors_naming_the_color() {
    type Setter = fn(&mut RoomStyle, Rgb);
    let setters: [(&str, Setter); 4] = [
        ("fill", |s, c| s.fill = Some(c)),
        ("edge", |s, c| s.edge = Some(c)),
        ("highlight", |s, c| s.highlight = Some(c)),
        ("accent", |s, c| s.accent = Some(c)),
    ];
    let bad_colors = [
        (f32::NAN, 0.5, 0.5),
        (0.5, f32::NAN, 0.5),
        (0.5, 0.5, f32::NAN),
        (f32::INFINITY, 0.5, 0.5),
        (0.5, f32::NEG_INFINITY, 0.5),
        (-0.01, 0.5, 0.5),
        (0.5, 1.01, 0.5),
        (0.5, 0.5, 255.0),
    ];
    for (name, set) in setters {
        for bad in bad_colors {
            let mut style = RoomStyle::named("cave");
            set(&mut style, bad);

            let error = style.validate().unwrap_err();
            assert!(error.contains("terrain_style"), "{error}");
            assert!(error.contains(name), "{name} {bad:?}: {error}");
            assert!(catalog().resolve(&style).is_err(), "{name} {bad:?}");
        }
    }
    // The limits themselves are fine.
    for edge in [0.0_f32, 1.0] {
        let style = RoomStyle {
            fill: Some((edge, edge, edge)),
            edge: Some((edge, edge, edge)),
            highlight: Some((edge, edge, edge)),
            accent: Some((edge, edge, edge)),
            ..RoomStyle::default()
        };
        style.validate().unwrap();
        assert!(catalog().resolve(&style).is_ok());
    }
}

#[test]
fn resolve_or_default_never_fails() {
    let styles = catalog();
    let known = with_style("cave", |s| s.seed_offset = 3);
    assert_eq!(
        styles.resolve_or_default(&known),
        styles.resolve(&known).unwrap()
    );

    let lost = with_style("granite", |s| {
        s.seed_offset = 5;
        s.draw_boundary = false;
    });
    let look = styles.resolve_or_default(&lost);
    assert_eq!(look.palette.seed_offset, 5);
    assert!(!look.draw_boundary);
    assert_eq!(look.palette.fill, TerrainStyle::default().fill);
    assert_eq!(look.boundary_thickness, Look::default().boundary_thickness);

    let broken = with_style("cave", |s| s.fill = Some((f32::NAN, 0.0, 0.0)));
    assert_eq!(
        styles.resolve_or_default(&broken).palette.fill,
        TerrainStyle::default().fill
    );
}

// ---------------------------------------------------------------------------------------
// Catalog validation
// ---------------------------------------------------------------------------------------

#[test]
fn a_valid_single_preset_catalog_loads() {
    let styles = TerrainStyles::from_ron(&one_preset(&[])).unwrap();

    assert_eq!(styles.names(), ["plain"]);
    assert_eq!(styles.preset("plain").unwrap().boundary_thickness, 40.0);
}

#[test]
fn an_empty_catalog_is_rejected() {
    let error = TerrainStyles::from_ron("(presets: [])").unwrap_err();

    assert!(error.contains("at least one preset"), "{error}");
}

#[test]
fn duplicate_names_are_rejected_naming_the_name() {
    let one = one_preset(&[]);
    let inner = &one["(presets: [".len()..one.len() - "])".len()];
    let error = TerrainStyles::from_ron(&format!("(presets: [{inner}, {inner}])")).unwrap_err();

    assert!(error.contains("\"plain\""), "{error}");
    assert!(error.contains("twice"), "{error}");
}

#[test]
fn a_blank_name_is_rejected() {
    for name in ["", "   "] {
        let text = one_preset(&[("\"plain\"", &format!("{name:?}"))]);
        let error = TerrainStyles::from_ron(&text).unwrap_err();
        assert!(error.contains("needs a name"), "{error}");
    }
}

#[test]
fn colors_outside_zero_to_one_are_rejected_naming_the_preset_and_the_color() {
    for (field, original) in [
        ("fill", "(0.2, 0.3, 0.4)"),
        ("edge", "(0.1, 0.1, 0.1)"),
        ("highlight", "(0.7, 0.7, 0.7)"),
        ("accent", "(0.3, 0.6, 0.3)"),
    ] {
        for bad in ["(1.5, 0.5, 0.5)", "(0.5, -0.1, 0.5)", "(0.5, 0.5, 2.0)"] {
            let text = one_preset(&[(&format!("{field}: {original}"), &format!("{field}: {bad}"))]);
            let error = TerrainStyles::from_ron(&text).unwrap_err();

            assert!(error.contains("\"plain\""), "{error}");
            assert!(error.contains(field), "{field} {bad}: {error}");
        }
    }
}

#[test]
fn non_finite_numbers_are_rejected_wherever_they_are() {
    let base = preset("stone");
    type Edit = fn(&mut TerrainPreset, f32);
    let edits: [(&str, Edit); 7] = [
        ("fill", |p, v| p.fill.1 = v),
        ("edge", |p, v| p.edge.0 = v),
        ("highlight", |p, v| p.highlight.2 = v),
        ("accent", |p, v| p.accent.0 = v),
        ("speckle_density", |p, v| p.speckle_density = v),
        ("crack_density", |p, v| p.crack_density = v),
        ("boundary_thickness", |p, v| p.boundary_thickness = v),
    ];
    for (field, set) in edits {
        for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut broken = base.clone();
            set(&mut broken, bad);
            let error = broken.validate().unwrap_err();
            assert!(error.contains(field), "{field} {bad}: {error}");
            assert!(error.contains("\"stone\""), "{error}");
        }
    }
}

#[test]
fn densities_must_be_between_zero_and_the_maximum() {
    for field in ["speckle_density", "crack_density"] {
        for bad in ["-0.5", "10.5", "100.0"] {
            let text = one_preset(&[(&format!("{field}: 1.0"), &format!("{field}: {bad}"))]);
            let error = TerrainStyles::from_ron(&text).unwrap_err();
            assert!(error.contains(field), "{field} {bad}: {error}");
            assert!(error.contains("between 0 and 10"), "{error}");
        }
        for good in ["0.0", "10.0", "0.001"] {
            let text = one_preset(&[(&format!("{field}: 1.0"), &format!("{field}: {good}"))]);
            TerrainStyles::from_ron(&text).unwrap();
        }
    }
}

#[test]
fn the_boundary_thickness_must_be_positive_and_at_most_the_maximum() {
    for bad in ["0.0", "-8.0", "512.5", "5000.0"] {
        let text = one_preset(&[(
            "boundary_thickness: 40.0",
            &format!("boundary_thickness: {bad}"),
        )]);
        let error = TerrainStyles::from_ron(&text).unwrap_err();
        assert!(error.contains("boundary_thickness"), "{bad}: {error}");
        assert!(error.contains("\"plain\""), "{error}");
    }
    for good in ["0.5", "48.0", "512.0"] {
        let text = one_preset(&[(
            "boundary_thickness: 40.0",
            &format!("boundary_thickness: {good}"),
        )]);
        TerrainStyles::from_ron(&text).unwrap();
    }
}

#[test]
fn unknown_and_missing_fields_and_bad_text_are_errors() {
    let unknown = one_preset(&[("moss: true,", "moss: true, mosss: false,")]);
    let error = TerrainStyles::from_ron(&unknown).unwrap_err();
    assert!(error.starts_with("terrain_styles:"), "{error}");
    assert!(error.contains("mosss"), "{error}");

    let missing = one_preset(&[("boundary_thickness: 40.0,", "")]);
    let error = TerrainStyles::from_ron(&missing).unwrap_err();
    assert!(error.contains("boundary_thickness"), "{error}");

    let error = TerrainStyles::from_ron("this is not ron").unwrap_err();
    assert!(error.starts_with("terrain_styles:"), "{error}");
    let error =
        TerrainStyles::from_ron(&format!("(extra: 1, {})", &one_preset(&[])[1..])).unwrap_err();
    assert!(error.contains("extra"), "{error}");
}

// ---------------------------------------------------------------------------------------
// Loading from the assets folder
// ---------------------------------------------------------------------------------------

#[test]
fn without_a_file_the_embedded_presets_load() {
    let assets = TempDir::new();

    assert_eq!(TerrainStyles::load(&assets.0).unwrap(), catalog());
}

#[test]
fn a_file_in_the_assets_folder_replaces_the_embedded_presets_whole() {
    let assets = TempDir::new();
    std::fs::write(assets.0.join(FILE_NAME), one_preset(&[])).unwrap();

    let styles = TerrainStyles::load(&assets.0).unwrap();

    assert_eq!(styles.names(), ["plain"]);
    assert!(styles.preset("stone").is_none(), "nothing is merged in");
}

#[test]
fn an_invalid_file_is_an_error_naming_the_file_and_never_a_fallback() {
    let assets = TempDir::new();
    let path = assets.0.join(FILE_NAME);
    std::fs::write(
        &path,
        one_preset(&[("boundary_thickness: 40.0", "boundary_thickness: 0.0")]),
    )
    .unwrap();

    let error = TerrainStyles::load(&assets.0).unwrap_err();
    assert!(error.contains(&path.display().to_string()), "{error}");
    assert!(error.contains("boundary_thickness"), "{error}");

    std::fs::write(&path, "(presets: [").unwrap();
    let error = TerrainStyles::load(&assets.0).unwrap_err();
    assert!(error.contains(&path.display().to_string()), "{error}");
}

#[test]
fn a_file_that_cannot_be_read_is_an_error_naming_it() {
    let assets = TempDir::new();
    std::fs::create_dir(assets.0.join(FILE_NAME)).unwrap();

    let error = TerrainStyles::load(&assets.0).unwrap_err();

    assert!(error.contains(FILE_NAME), "{error}");
}
