//! The canvas marker of a pickup slot: a dashed ring with the kind's letter in it (K, A or ?).
//! An item is a filled gold box, so nothing here is filled and a slot cannot be mistaken for one.
use hollow_steps::piece::SlotKind;
use macroquad::prelude::*;

/// How many dashes make up the ring.
const DASHES: usize = 12;
/// The part of each dash's share of the ring that is drawn; the rest is the gap.
const DASH_FILL: f32 = 0.6;
/// The letter's height as a share of the marker's height.
const LETTER_SHARE: f32 = 0.9;

/// The dashes of a ring around `center`, each as its two end points.
pub(super) fn dashes(center: Vec2, radius: f32) -> Vec<(Vec2, Vec2)> {
    let step = std::f32::consts::TAU / DASHES as f32;
    (0..DASHES)
        .map(|index| {
            let from = step * index as f32;
            let to = from + step * DASH_FILL;
            (
                center + Vec2::from_angle(from) * radius,
                center + Vec2::from_angle(to) * radius,
            )
        })
        .collect()
}

/// Draws the marker for a slot whose box is `rect` (on screen): a ring that just holds the box,
/// and the letter of `kind` in the middle. The selected one is white and thicker.
pub(super) fn draw(rect: Rect, kind: SlotKind, color: Color, selected: bool) {
    let center = rect.center();
    let radius = rect.size().length() / 2.0;
    let (ink, width) = if selected { (WHITE, 3.0) } else { (color, 2.0) };
    for (from, to) in dashes(center, radius) {
        draw_line(from.x, from.y, to.x, to.y, width, ink);
    }
    let font_size = rect.h * LETTER_SHARE;
    let letter = kind.letter();
    let size = measure_text(letter, None, font_size as u16, 1.0);
    draw_text(
        letter,
        center.x - size.width / 2.0,
        center.y - size.height / 2.0 + size.offset_y,
        font_size,
        ink,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_is_dashes_on_one_circle_with_gaps_between() {
        let center = vec2(100.0, 50.0);
        let ring = dashes(center, 13.0);
        assert_eq!(ring.len(), DASHES);
        for (from, to) in &ring {
            assert!(((*from - center).length() - 13.0).abs() < 1e-3);
            assert!(((*to - center).length() - 13.0).abs() < 1e-3);
            assert!(from.distance(*to) > 0.0, "a dash has length");
        }
        for pair in ring.windows(2) {
            assert!(pair[0].1.distance(pair[1].0) > 0.0, "a gap between dashes");
        }
    }

    #[test]
    fn the_ring_scales_with_the_radius() {
        let small = dashes(Vec2::ZERO, 5.0);
        let large = dashes(Vec2::ZERO, 10.0);
        assert!(
            (large[0].1.distance(large[0].0) - 2.0 * small[0].1.distance(small[0].0)).abs() < 1e-3
        );
    }
}
