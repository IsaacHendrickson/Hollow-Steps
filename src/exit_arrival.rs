//! Where the player arrives next to an exit, and where a clicked exit goes. Pure geometry
//! on room layouts, used by the exit-linking flow (`exit_link`).
//!
//! An arrival point is a player center that is inside its room above the floor, not inside
//! a solid and not inside any exit rectangle (otherwise the player would walk straight back
//! out). "Next to" an exit is the free point nearest to the exit's inner face.
use macroquad::math::Vec2;

use crate::editor::RoomLayout;
use crate::room::RoomEdge;

/// The player's body, which every arrival point has to fit.
pub const PLAYER_SIZE: Vec2 = Vec2::new(32.0, 48.0);
/// Clear space left between an exit's inner face and the arriving body.
const GAP: f32 = 2.0;
/// How far candidate points move each time one is refused.
const STEP: f32 = 8.0;
/// Candidate rings: how many steps inward are tried before giving up.
const DEPTHS: usize = 16;
/// An exit placed by a click is this long, or the whole edge when that is shorter.
const CLICKED_EXIT_LENGTH: f32 = 96.0;

/// Why `center` is not a valid arrival point in `layout`, or `None` if it is one.
pub fn arrival_error(layout: &RoomLayout, center: Vec2) -> Option<&'static str> {
    if !layout.contains_body(center.into(), PLAYER_SIZE) {
        return Some("outside the room or below its floor");
    }
    let terrain = crate::terrain::Terrain::from_tuples(&layout.terrain);
    if terrain.overlaps_body(center, PLAYER_SIZE) {
        return Some("inside a solid");
    }
    let body = macroquad::math::Rect::new(
        center.x - PLAYER_SIZE.x / 2.0,
        center.y - PLAYER_SIZE.y / 2.0,
        PLAYER_SIZE.x,
        PLAYER_SIZE.y,
    );
    let rect = layout.rect();
    if layout
        .exits
        .iter()
        .any(|exit| exit.exit().zone(rect).overlaps(&body))
    {
        return Some("inside an exit");
    }
    None
}

/// The free arrival point nearest to exit `exit` of `layout`, just inside its inner face.
pub fn arrival_next_to(layout: &RoomLayout, exit: usize) -> Result<Vec2, String> {
    let placement = layout.exits.get(exit).ok_or("missing exit")?;
    let zone = placement.exit().zone(layout.rect());
    let half = PLAYER_SIZE / 2.0;
    let (inward, along, reach, span) = match placement.edge {
        RoomEdge::Left => (Vec2::X, Vec2::Y, zone.w / 2.0 + half.x, zone.h),
        RoomEdge::Right => (-Vec2::X, Vec2::Y, zone.w / 2.0 + half.x, zone.h),
        RoomEdge::Top => (Vec2::Y, Vec2::X, zone.h / 2.0 + half.y, zone.w),
        RoomEdge::Bottom => (-Vec2::Y, Vec2::X, zone.h / 2.0 + half.y, zone.w),
    };
    let center = zone.center();
    // Nearest first: each ring is one step deeper, and on each ring the point straight
    // across from the exit comes first, then points alternating to either side of it.
    let sideways = (0..)
        .map(|k: i32| ((k + 1) / 2) as f32 * STEP * if k % 2 == 0 { 1.0 } else { -1.0 })
        .take_while(|offset| offset.abs() <= span / 2.0 + 4.0 * STEP);
    for ring in 0..DEPTHS {
        let depth = reach + GAP + ring as f32 * STEP;
        for lateral in sideways.clone() {
            let point = center + inward * depth + along * lateral;
            let mut candidates = vec![point];
            if matches!(placement.edge, RoomEdge::Left | RoomEdge::Right) {
                // Standing on the floor is as good as hanging in the doorway.
                candidates.push(Vec2::new(point.x, layout.floor_y - half.y));
            }
            if let Some(found) = candidates
                .into_iter()
                .find(|point| arrival_error(layout, *point).is_none())
            {
                return Ok(found);
            }
        }
    }
    Err("there is no free spot for the player next to that exit".into())
}

/// The edge, offset and length of an exit placed by a click at `position` (room
/// coordinates): on the nearest edge, centered on the click and kept inside the edge.
pub fn exit_at_click(layout: &RoomLayout, position: Vec2) -> (RoomEdge, f32, f32) {
    let rect = layout.rect();
    let distances = [
        (RoomEdge::Left, position.x - rect.x),
        (RoomEdge::Right, rect.right() - position.x),
        (RoomEdge::Top, position.y - rect.y),
        (RoomEdge::Bottom, rect.bottom() - position.y),
    ];
    let edge = distances
        .iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(RoomEdge::Left, |(edge, _)| *edge);
    let (span, along) = match edge {
        RoomEdge::Left | RoomEdge::Right => (rect.h, position.y - rect.y),
        RoomEdge::Top | RoomEdge::Bottom => (rect.w, position.x - rect.x),
    };
    let length = span.min(CLICKED_EXIT_LENGTH);
    let offset = (along - length / 2.0).clamp(0.0, span - length);
    (edge, offset, length)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(exits: &str, terrain: &str) -> RoomLayout {
        ron::from_str(&format!(
            "(bounds: (0.0, 0.0, 960.0, 460.0), floor_y: 460.0, spawn: (480.0, 436.0), \
             items: [], exits: [{exits}], terrain: [{terrain}])"
        ))
        .unwrap()
    }

    fn exit(edge: &str, offset: f32, length: f32) -> String {
        format!(
            "(edge: {edge}, offset: {offset}, length: {length}, target_room: First, \
             spawn: (480.0, 436.0))"
        )
    }

    fn zones(layout: &RoomLayout) -> Vec<macroquad::math::Rect> {
        layout
            .exits
            .iter()
            .map(|exit| exit.exit().zone(layout.rect()))
            .collect()
    }

    fn body(center: Vec2) -> macroquad::math::Rect {
        macroquad::math::Rect::new(
            center.x - PLAYER_SIZE.x / 2.0,
            center.y - PLAYER_SIZE.y / 2.0,
            PLAYER_SIZE.x,
            PLAYER_SIZE.y,
        )
    }

    #[test]
    fn an_arrival_point_must_be_in_the_room_free_of_solids_and_exits() {
        let layout = layout(&exit("Left", 300.0, 96.0), "(400.0, 380.0, 100.0, 80.0)");
        assert_eq!(arrival_error(&layout, Vec2::new(480.0, 300.0)), None);
        assert!(
            arrival_error(&layout, Vec2::new(5.0, 300.0)).is_some(),
            "outside"
        );
        assert!(
            arrival_error(&layout, Vec2::new(480.0, 459.0)).is_some(),
            "below the floor"
        );
        assert_eq!(
            arrival_error(&layout, Vec2::new(450.0, 420.0)),
            Some("inside a solid")
        );
        assert_eq!(
            arrival_error(&layout, Vec2::new(30.0, 350.0)),
            Some("inside an exit")
        );
        assert!(arrival_error(&layout, Vec2::new(f32::NAN, 100.0)).is_some());
    }

    #[test]
    fn arrival_is_next_to_each_kind_of_exit_and_never_inside_one() {
        for edge in ["Left", "Right", "Top", "Bottom"] {
            let layout = layout(&exit(edge, 200.0, 96.0), "");
            let at = arrival_next_to(&layout, 0).unwrap();
            assert_eq!(arrival_error(&layout, at), None, "{edge}: {at:?}");
            let zone = zones(&layout)[0];
            // Close to the exit: within a body's width and a little more of its inner face.
            let gap = match edge {
                "Left" => body(at).x - zone.right(),
                "Right" => zone.x - body(at).right(),
                "Top" => body(at).y - zone.bottom(),
                _ => zone.y - body(at).bottom(),
            };
            assert!((0.0..=GAP + STEP).contains(&gap), "{edge}: gap {gap}");
        }
    }

    #[test]
    fn arrival_avoids_a_neighbouring_exit_and_solids_by_moving_away() {
        // Two exits on the left edge, one right above the other, and a block in front.
        let layout = layout(
            &format!(
                "{}, {}",
                exit("Left", 200.0, 96.0),
                exit("Left", 296.0, 96.0)
            ),
            "(32.0, 150.0, 40.0, 100.0)",
        );
        for index in 0..2 {
            let at = arrival_next_to(&layout, index).unwrap();
            assert_eq!(arrival_error(&layout, at), None, "exit {index}: {at:?}");
            for zone in zones(&layout) {
                assert!(!zone.overlaps(&body(at)), "exit {index}");
            }
        }
    }

    #[test]
    fn arrival_next_to_a_blocked_exit_is_refused() {
        // A wall of solid right in front of the exit, deeper than the search reaches.
        let layout = layout(&exit("Left", 0.0, 460.0), "(32.0, 0.0, 800.0, 460.0)");
        let error = arrival_next_to(&layout, 0).unwrap_err();
        assert!(error.contains("no free spot"), "{error}");
        assert!(arrival_next_to(&layout, 3).is_err());
    }

    #[test]
    fn a_side_exit_can_hang_in_the_doorway_or_stand_on_the_floor() {
        let high = layout(&exit("Left", 100.0, 96.0), "");
        let at = arrival_next_to(&high, 0).unwrap();
        assert!((at.y - 148.0).abs() <= 48.0, "{at:?}");
        let low = layout(&exit("Left", 364.0, 96.0), "");
        let at = arrival_next_to(&low, 0).unwrap();
        assert!(body(at).bottom() <= 460.0, "{at:?}");
    }

    #[test]
    fn a_click_places_the_exit_on_the_nearest_edge_inside_it() {
        let layout = layout("", "");
        let cases = [
            (Vec2::new(10.0, 200.0), RoomEdge::Left, 152.0, 96.0),
            (Vec2::new(950.0, 300.0), RoomEdge::Right, 252.0, 96.0),
            (Vec2::new(500.0, 6.0), RoomEdge::Top, 452.0, 96.0),
            (Vec2::new(300.0, 455.0), RoomEdge::Bottom, 252.0, 96.0),
            // Near a corner the exit is pushed back inside the edge.
            (Vec2::new(4.0, 5.0), RoomEdge::Left, 0.0, 96.0),
            (Vec2::new(4.0, 455.0), RoomEdge::Left, 364.0, 96.0),
            (Vec2::new(958.0, 40.0), RoomEdge::Right, 0.0, 96.0),
        ];
        for (click, edge, offset, length) in cases {
            assert_eq!(
                exit_at_click(&layout, click),
                (edge, offset, length),
                "{click:?}"
            );
        }
    }

    #[test]
    fn a_click_exit_is_never_longer_than_the_edge() {
        let mut small = layout("", "");
        small.bounds = (0.0, 0.0, 64.0, 60.0);
        small.floor_y = 60.0;
        let (edge, offset, length) = exit_at_click(&small, Vec2::new(2.0, 30.0));
        assert_eq!((edge, offset, length), (RoomEdge::Left, 0.0, 60.0));
    }
}
