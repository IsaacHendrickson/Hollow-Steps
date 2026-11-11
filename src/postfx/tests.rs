use super::*;

fn config() -> PostFxConfig {
    PostFxConfig::default()
}

fn vignette(strength: f32, radius: f32, softness: f32) -> VignetteConfig {
    VignetteConfig {
        strength,
        radius,
        softness,
    }
}

fn temp_assets(name: &str, file: Option<&str>) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("hollow-steps-postfx-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    if let Some(text) = file {
        std::fs::write(dir.join(FILE_NAME), text).unwrap();
    }
    dir
}

#[test]
fn the_embedded_defaults_are_valid_and_switched_on() {
    let config = config();
    assert!(config.enabled);
    assert!(config.validate().is_ok());
    assert!(config.is_active());
}

#[test]
fn the_middle_of_the_picture_is_untouched_and_the_corners_are_darkest() {
    let v = config().vignette;
    assert_eq!(vignette_factor(Vec2::new(0.5, 0.5), &v), 1.0);
    let corner = vignette_factor(Vec2::new(0.0, 0.0), &v);
    assert!(corner < 1.0);
    // The fade is wide enough that a corner reaches the full strength within a hair.
    assert!((corner - (1.0 - v.strength)).abs() < 0.05, "{corner}");
    // Every corner is the same, and the edge middles are lighter than the corners.
    for corner_uv in [
        Vec2::new(1.0, 0.0),
        Vec2::new(0.0, 1.0),
        Vec2::new(1.0, 1.0),
    ] {
        assert!((vignette_factor(corner_uv, &v) - corner).abs() < 1e-6);
    }
    assert!(vignette_factor(Vec2::new(1.0, 0.5), &v) > corner);
}

#[test]
fn the_vignette_gets_darker_towards_the_edge_and_never_below_the_strength() {
    let v = vignette(0.6, 0.3, 0.5);
    let mut last = 1.0;
    for step in 0..=20 {
        let factor = vignette_factor(Vec2::new(0.5 + step as f32 * 0.025, 0.5), &v);
        assert!(factor <= last + 1e-6, "darkening never reverses");
        assert!(factor >= 1.0 - v.strength - 1e-6);
        last = factor;
    }
    // Inside the radius nothing changes.
    assert_eq!(vignette_factor(Vec2::new(0.55, 0.5), &v), 1.0);
}

#[test]
fn zero_strength_leaves_every_pixel_alone() {
    let v = vignette(0.0, 0.2, 0.3);
    for uv in [
        Vec2::new(0.0, 0.0),
        Vec2::new(0.3, 0.9),
        Vec2::new(1.0, 1.0),
    ] {
        assert_eq!(vignette_factor(uv, &v), 1.0);
    }
}

#[test]
fn glow_weight_is_zero_below_the_threshold_full_at_white_and_rises_between() {
    assert_eq!(glow_weight(0.0, 0.6), 0.0);
    assert_eq!(glow_weight(0.6, 0.6), 0.0);
    assert_eq!(glow_weight(1.0, 0.6), 1.0);
    assert!((glow_weight(0.8, 0.6) - 0.5).abs() < 1e-6);
    assert!(glow_weight(0.7, 0.6) < glow_weight(0.9, 0.6));
    // Brighter than white (an overbright color) is capped, never above 1.
    assert_eq!(glow_weight(3.0, 0.6), 1.0);
    // The highest allowed threshold still gives a finite weight.
    assert!(glow_weight(1.0, MAX_GLOW_THRESHOLD).is_finite());
}

#[test]
fn luma_weights_green_most_and_gives_white_one() {
    assert!((luma([1.0, 1.0, 1.0]) - 1.0).abs() < 1e-6);
    assert!(luma([0.0, 1.0, 0.0]) > luma([1.0, 0.0, 0.0]));
    assert!(luma([1.0, 0.0, 0.0]) > luma([0.0, 0.0, 1.0]));
    assert_eq!(luma([0.0, 0.0, 0.0]), 0.0);
}

#[test]
fn validation_rejects_each_out_of_range_or_non_finite_value_by_name() {
    let good = config();
    type Edit = Box<dyn Fn(&mut PostFxConfig)>;
    let cases: Vec<(&str, Edit)> = vec![
        ("vignette.strength", Box::new(|c| c.vignette.strength = 1.1)),
        (
            "vignette.strength",
            Box::new(|c| c.vignette.strength = -0.1),
        ),
        ("vignette.radius", Box::new(|c| c.vignette.radius = 1.5)),
        ("vignette.softness", Box::new(|c| c.vignette.softness = 0.0)),
        ("vignette.softness", Box::new(|c| c.vignette.softness = 9.0)),
        ("glow.strength", Box::new(|c| c.glow.strength = 5.0)),
        ("glow.threshold", Box::new(|c| c.glow.threshold = 1.0)),
        ("glow.radius", Box::new(|c| c.glow.radius = 100.0)),
        ("glow.radius", Box::new(|c| c.glow.radius = f32::NAN)),
        (
            "vignette.radius",
            Box::new(|c| c.vignette.radius = f32::INFINITY),
        ),
    ];
    for (name, edit) in cases {
        let mut bad = good;
        edit(&mut bad);
        let error = bad.validate().expect_err(name);
        assert!(error.contains(name), "{error} should name {name}");
    }
}

#[test]
fn ron_with_an_unknown_field_or_a_bad_value_is_an_error() {
    let text = include_str!("../../data/postfx.ron");
    assert!(PostFxConfig::from_ron(text).is_ok());
    let unknown = text.replace("enabled: true,", "enabled: true, bloom: 1.0,");
    assert!(PostFxConfig::from_ron(&unknown).is_err());
    let bad = text.replace("strength: 0.35", "strength: 4.0");
    assert!(PostFxConfig::from_ron(&bad)
        .unwrap_err()
        .contains("vignette.strength"));
    assert!(PostFxConfig::from_ron("(").is_err());
}

#[test]
fn the_toggle_and_zero_strengths_skip_the_pass() {
    let mut config = config();
    assert!(config.is_active());
    config.enabled = false;
    assert!(!config.is_active(), "the data toggle turns the pass off");

    let mut config = self::config();
    config.vignette.strength = 0.0;
    assert!(config.is_active(), "the glow alone still counts");
    config.glow.strength = 0.0;
    assert!(!config.is_active(), "nothing left to do");

    let mut config = self::config();
    config.vignette.strength = 0.0;
    config.glow.radius = 0.0;
    assert!(!config.is_active(), "a glow with no reach does nothing");
}

#[test]
fn uniforms_carry_the_settings_and_one_texel_of_the_view() {
    let uniforms = config().uniforms(Vec2::new(960.0, 540.0));
    let c = config();
    assert_eq!(
        uniforms.vignette,
        [c.vignette.strength, c.vignette.radius, c.vignette.softness]
    );
    assert_eq!(
        uniforms.glow,
        [c.glow.strength, c.glow.threshold, c.glow.radius]
    );
    assert_eq!(uniforms.texel, [1.0 / 960.0, 1.0 / 540.0]);
}

#[test]
fn a_degenerate_view_gives_zero_texels_not_infinity() {
    for view in [
        Vec2::ZERO,
        Vec2::new(-1.0, 540.0),
        Vec2::new(f32::NAN, 540.0),
        Vec2::new(960.0, f32::INFINITY),
    ] {
        assert_eq!(config().uniforms(view).texel, [0.0, 0.0], "{view:?}");
    }
}

#[test]
fn the_shader_reads_every_uniform_the_front_end_sets_and_the_glow_sample_count() {
    assert!(FRAGMENT_SHADER.contains("uniform sampler2D Texture;"));
    for (name, kind) in [
        (UNIFORM_VIGNETTE, "vec3"),
        (UNIFORM_GLOW, "vec3"),
        (UNIFORM_TEXEL, "vec2"),
    ] {
        assert!(
            FRAGMENT_SHADER.contains(&format!("uniform {kind} {name};")),
            "{name} should be a {kind}"
        );
    }
    assert!(FRAGMENT_SHADER.contains(&format!("i < {GLOW_DIRECTIONS}")));
    assert!(FRAGMENT_SHADER.contains(&format!("halo /= {}.0", GLOW_DIRECTIONS * GLOW_RINGS)));
    // No variable may hide a GLSL built-in: some drivers refuse that.
    for builtin in ["step", "distance", "length"] {
        assert!(
            !FRAGMENT_SHADER.contains(&format!("float {builtin} "))
                && !FRAGMENT_SHADER.contains(&format!("vec2 {builtin} ")),
            "{builtin}"
        );
    }
    // The vertex shader hands the fragment shader the names it reads.
    assert!(VERTEX_SHADER.contains("varying lowp vec2 uv;"));
    assert!(VERTEX_SHADER.contains("varying lowp vec4 color;"));
}

#[test]
fn load_uses_the_embedded_defaults_without_a_file() {
    let dir = temp_assets("none", None);
    assert_eq!(PostFxConfig::load(&dir).unwrap(), PostFxConfig::default());
}

#[test]
fn a_file_in_assets_replaces_the_defaults_whole() {
    let text = include_str!("../../data/postfx.ron").replace("enabled: true", "enabled: false");
    let dir = temp_assets("replace", Some(&text));
    let loaded = PostFxConfig::load(&dir).unwrap();
    assert!(!loaded.enabled);
    assert!(!loaded.is_active());
}

#[test]
fn an_invalid_file_is_an_error_naming_the_file_never_a_fallback() {
    let dir = temp_assets("invalid", Some("(enabled: true)"));
    let error = PostFxConfig::load(&dir).unwrap_err();
    assert!(error.contains(FILE_NAME), "{error}");
}
