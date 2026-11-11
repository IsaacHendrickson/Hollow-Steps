//! The terrain look's per-room palette through the public room API: defaults, loading,
//! validation, the editor round trip and the game's room loader.
mod support;

use hollow_steps::editor::{RoomDraft, RoomLayout};
use hollow_steps::enemy::EnemyDefinitions;
use hollow_steps::game::GameState;
use hollow_steps::room::{RoomEnemies, RoomId};
use hollow_steps::terrain_style::{
    build, ground_cover, Shape, TerrainStyle, DEFAULT_ACCENT, DEFAULT_EDGE, DEFAULT_FILL,
    DEFAULT_HIGHLIGHT,
};

fn layout(name: &str) -> RoomLayout {
    let path = support::fixture_assets().join("terrain").join(name);
    ron::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn styled() -> RoomLayout {
    layout("styled_layout.ron")
}

fn draft(layout: RoomLayout) -> RoomDraft {
    RoomDraft {
        enemies: Vec::new(),
        boss: None,
        layout,
    }
}

fn definitions() -> EnemyDefinitions {
    EnemyDefinitions::load(&support::fixture_assets()).expect("fixtures load")
}

fn palette(fill: (f32, f32, f32)) -> TerrainStyle {
    TerrainStyle {
        fill,
        ..TerrainStyle::default()
    }
}

#[test]
fn the_default_palette_is_the_part_one_look() {
    // These numbers are the look shipped with the first version of the terrain style. Changing
    // one changes every room without a palette, so it should be a deliberate act.
    let default = TerrainStyle::default();
    assert_eq!(default.fill, (0.133, 0.145, 0.227));
    assert_eq!(default.edge, (0.051, 0.055, 0.102));
    assert_eq!(default.highlight, (0.435, 0.478, 0.643));
    assert_eq!(default.accent, (0.286, 0.545, 0.443));
    assert_eq!(default.seed_offset, 0);
    assert!(default.moss);
    assert_eq!(
        default,
        TerrainStyle {
            fill: DEFAULT_FILL,
            edge: DEFAULT_EDGE,
            highlight: DEFAULT_HIGHLIGHT,
            accent: DEFAULT_ACCENT,
            seed_offset: 0,
            moss: true,
        }
    );
    assert!(default.is_default());
    default.validate().unwrap();
}

#[test]
fn a_room_without_a_palette_draws_exactly_the_default_look() {
    let layout = layout("terrain_layout.ron");
    assert_eq!(layout.terrain_style, TerrainStyle::default());

    let room = draft(layout)
        .build(RoomId::Custom(0), &definitions())
        .unwrap();
    assert_eq!(room.terrain_style, TerrainStyle::default());
    let ground = Some(ground_cover(room.bounds, room.floor_y));
    let from_room = build(room.id, room.terrain.solids(), ground, &room.terrain_style);
    let explicit = build(
        room.id,
        room.terrain.solids(),
        ground,
        &TerrainStyle {
            fill: DEFAULT_FILL,
            edge: DEFAULT_EDGE,
            highlight: DEFAULT_HIGHLIGHT,
            accent: DEFAULT_ACCENT,
            seed_offset: 0,
            moss: true,
        },
    );
    assert!(!from_room.is_empty());
    assert_eq!(from_room, explicit);
}

#[test]
fn old_room_files_load_without_a_palette_and_saving_adds_none() {
    for name in ["legacy_layout.ron", "terrain_layout.ron"] {
        let old = layout(name);
        assert_eq!(old.terrain_style, TerrainStyle::default(), "{name}");
        old.validate_geometry().unwrap();
        assert!(
            !ron::to_string(&old).unwrap().contains("terrain_style"),
            "{name}: saving an old room must not add a palette"
        );
    }
}

#[test]
fn a_palette_loads_validates_and_survives_a_ron_round_trip() {
    let layout = styled();
    layout.validate_geometry().unwrap();
    assert_eq!(layout.terrain_style.fill, (0.25, 0.5, 0.75));
    assert_eq!(layout.terrain_style.edge, (0.0, 0.1, 0.2));
    assert_eq!(layout.terrain_style.highlight, (1.0, 0.9, 0.8));
    assert_eq!(layout.terrain_style.accent, (0.3, 0.6, 0.9));
    assert_eq!(layout.terrain_style.seed_offset, 42);
    assert!(!layout.terrain_style.moss);
    assert!(!layout.terrain_style.is_default());

    let text = ron::to_string(&layout).unwrap();
    assert!(text.contains("terrain_style"));
    let back: RoomLayout = ron::from_str(&text).unwrap();
    assert_eq!(back, layout);
    let pretty = ron::ser::to_string_pretty(&layout, ron::ser::PrettyConfig::default()).unwrap();
    assert_eq!(ron::from_str::<RoomLayout>(&pretty).unwrap(), layout);
}

#[test]
fn a_palette_may_name_only_what_it_changes() {
    let text = std::fs::read_to_string(
        support::fixture_assets()
            .join("terrain")
            .join("legacy_layout.ron"),
    )
    .unwrap();
    let with = |style: &str| {
        let text = text.replace(
            "exits: [],",
            &format!("exits: [],\n    terrain_style: {style},"),
        );
        ron::from_str::<RoomLayout>(&text)
    };

    let moss_off = with("(moss: false)").unwrap().terrain_style;
    assert_eq!(
        moss_off,
        TerrainStyle {
            moss: false,
            ..TerrainStyle::default()
        }
    );
    let empty = with("()").unwrap().terrain_style;
    assert_eq!(empty, TerrainStyle::default());
    let error = with("(fil: (0.1, 0.1, 0.1))").unwrap_err().to_string();
    assert!(
        error.contains("fil"),
        "an unknown field is an error: {error}"
    );
}

#[test]
fn validation_rejects_not_a_number_and_out_of_range_colors_naming_the_color() {
    type Setter = fn(&mut TerrainStyle, (f32, f32, f32));
    let setters: [(&str, Setter); 4] = [
        ("fill", |s, c| s.fill = c),
        ("edge", |s, c| s.edge = c),
        ("highlight", |s, c| s.highlight = c),
        ("accent", |s, c| s.accent = c),
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
            let mut style = TerrainStyle::default();
            set(&mut style, bad);
            let error = style.validate().unwrap_err();
            assert!(error.contains(name), "{name} {bad:?}: {error}");

            // The same check blocks a room that carries it.
            let mut layout = styled();
            layout.terrain_style = style;
            let error = layout.validate_geometry().unwrap_err();
            assert!(error.contains("terrain_style"), "{error}");
            assert!(error.contains(name), "{error}");
            assert!(draft(layout)
                .build(RoomId::Custom(0), &definitions())
                .is_err());
        }
    }
}

#[test]
fn colors_on_the_range_limits_are_valid() {
    for edge in [0.0_f32, 1.0] {
        let style = TerrainStyle {
            fill: (edge, edge, edge),
            edge: (edge, edge, edge),
            highlight: (edge, edge, edge),
            accent: (edge, edge, edge),
            ..TerrainStyle::default()
        };
        style.validate().unwrap();
    }
}

#[test]
fn building_a_room_carries_the_palette_and_the_editor_save_keeps_it() {
    let definitions = definitions();
    let layout = styled();
    let room = draft(layout.clone())
        .build(RoomId::Custom(0), &definitions)
        .unwrap();
    assert_eq!(room.terrain_style, layout.terrain_style);

    // What the editor reads back from a live room still has the palette...
    assert_eq!(RoomLayout::from_room(&room), layout);

    // ...and so does the file it writes.
    let text = draft(layout.clone()).serialize(&definitions, &[]).unwrap();
    let saved: RoomEnemies = ron::from_str(&text).unwrap();
    assert_eq!(saved.layout.unwrap().terrain_style, layout.terrain_style);
}

#[test]
fn the_palette_changes_the_draw_list() {
    let definitions = definitions();
    let plain = draft(layout("terrain_layout.ron"))
        .build(RoomId::Custom(0), &definitions)
        .unwrap();
    let warm = draft(styled())
        .build(RoomId::Custom(0), &definitions)
        .unwrap();
    let look = |room: &hollow_steps::room::Room| {
        build(
            room.id,
            room.terrain.solids(),
            Some(ground_cover(room.bounds, room.floor_y)),
            &room.terrain_style,
        )
    };

    let (plain_list, warm_list) = (look(&plain), look(&warm));
    assert_ne!(plain_list, warm_list);
    // The first shape of a solid is its whole body in the palette's fill.
    let Shape::Rect { color, .. } = &warm_list[0] else {
        panic!("expected the fill rectangle first, got {:?}", warm_list[0]);
    };
    assert_eq!((color.r, color.g, color.b), (0.25, 0.5, 0.75));
    // Moss is off in this palette; the same solids with moss on do grow blades.
    let solids = warm.terrain.solids();
    assert_eq!(moss_blades(&warm_list, solids), 0, "moss is switched off");
    let mossy = TerrainStyle {
        moss: true,
        ..warm.terrain_style
    };
    let with_moss = build(warm.id, solids, None, &mossy);
    assert!(moss_blades(&with_moss, solids) > 0, "moss can grow here");
}

/// Blades of moss: triangles whose base sits one pixel inside a solid's top and whose tip
/// is above it (a stalactite's tip is below its base).
fn moss_blades(shapes: &[Shape], solids: &[macroquad::math::Rect]) -> usize {
    shapes
        .iter()
        .filter(|shape| match shape {
            Shape::Triangle { points, .. } => solids.iter().any(|solid| {
                let base = solid.y + 1.0;
                let on_base = points.iter().filter(|p| (p.1 - base).abs() < 1e-3).count();
                on_base == 2 && points.iter().any(|p| p.1 < solid.y)
            }),
            _ => false,
        })
        .count()
}

#[test]
fn a_different_seed_offset_reshuffles_the_look_and_a_different_palette_recolors_it() {
    let solids = [macroquad::math::Rect::new(100.0, 300.0, 300.0, 40.0)];
    let base = build(RoomId::First, &solids, None, &TerrainStyle::default());
    let offset = TerrainStyle {
        seed_offset: 1,
        ..TerrainStyle::default()
    };
    let recolored = palette((0.5, 0.5, 0.5));

    assert_ne!(base, build(RoomId::First, &solids, None, &offset));
    assert_ne!(base, build(RoomId::First, &solids, None, &recolored));
    assert_eq!(
        base.len(),
        build(RoomId::First, &solids, None, &recolored).len()
    );
}

#[test]
fn the_game_loads_a_rooms_palette() {
    let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    // Shipped content: loading and validating only, no assertions about its layout. The first
    // room ships with a palette of its own, which has to be valid for the game to start.
    let game = GameState::try_new(&assets).expect("shipped rooms must load and validate");
    game.room().terrain_style.validate().unwrap();
}
