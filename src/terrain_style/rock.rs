//! The shapes of one solid: a fill layer (fill, gradient bands, speckles, cracks) and a trim layer
//! (outline, lit top, rocky lip, moss, stalactites). Every random choice comes from a
//! generator seeded per solid, with one stream per feature (and per exposed stretch), so
//! switching one feature off never reshuffles another.

use macroquad::math::Rect;

use super::colors::Colors;
use super::edges::{EdgeSpan, Exposure};
use super::{
    stream, Shape, TerrainStyle, DRIP_LENGTH, HIGHLIGHT_HEIGHT, LIP_HEIGHT, MAX_CRACKS,
    MAX_SPECKLES, MOSS_HEIGHT, OUTLINE_WIDTH,
};

const TAG_SPECKLES: u64 = 1;
const TAG_CRACKS: u64 = 2;
const TAG_LIP: u64 = 3;
const TAG_MOSS: u64 = 4;
const TAG_DRIPS: u64 = 5;

/// Share of a solid's height taken by the lighter top band and by the darker bottom band, and
/// the most height either may take, so neighboring solids of different heights and tall walls
/// still show the same gradient.
const BAND_SHARE: f32 = 0.3;
const BAND_MAX: f32 = 14.0;
/// One speckle per this many square pixels of fill, and one crack per this many.
const SPECKLE_AREA: f32 = 350.0;
const CRACK_AREA: f32 = 3000.0;
/// Horizontal distance between the points of a rocky lip, and the most points per stretch.
const LIP_STEP: f32 = 8.0;
const MAX_LIP_SEGMENTS: usize = 512;
/// Gaps between moss tufts and between stalactites, and the chance a slot gets one.
const MOSS_GAP: (f32, f32) = (10.0, 26.0);
const MOSS_CHANCE: f32 = 0.6;
const DRIP_GAP: (f32, f32) = (14.0, 34.0);
const DRIP_CHANCE: f32 = 0.55;
/// Tufts and stalactites keep this far from the ends of an exposed stretch, so they never
/// reach past it.
const END_INSET: f32 = 4.0;
/// Most tufts or stalactites tried along one exposed stretch.
const MAX_SLOTS: usize = 512;

fn between(rng: &mut crate::particles::Rng, low: f32, high: f32) -> f32 {
    low + (high - low) * rng.unit()
}

/// The rectangle shrunk by `by` on every side, or `None` when nothing is left.
fn inset(rect: Rect, by: f32) -> Option<Rect> {
    (rect.w > 2.0 * by && rect.h > 2.0 * by).then(|| {
        Rect::new(
            rect.x + by,
            rect.y + by,
            rect.w - 2.0 * by,
            rect.h - 2.0 * by,
        )
    })
}

/// The fill, then speckles and cracks inside it. The whole solid is the palette fill; a
/// lighter band is laid under every open stretch of the top and a darker one over every open
/// stretch of the bottom, so a solid's gradient never shows a seam where another solid sits
/// against it.
pub(super) fn fill_layer(
    rect: Rect,
    exposure: &Exposure,
    seed: u64,
    colors: &Colors,
    out: &mut Vec<Shape>,
) {
    out.push(Shape::rect(rect.x, rect.y, rect.w, rect.h, colors.band_mid));
    let band = (rect.h * BAND_SHARE).min(BAND_MAX);
    for span in &exposure.top {
        out.push(Shape::rect(
            span.start,
            rect.y,
            span.length(),
            band,
            colors.band_top,
        ));
    }
    for span in &exposure.bottom {
        out.push(Shape::rect(
            span.start,
            rect.bottom() - band,
            span.length(),
            band,
            colors.band_low,
        ));
    }
    if let Some(inner) = inset(rect, OUTLINE_WIDTH + 1.0) {
        speckles(inner, seed, colors, out);
        cracks(inner, seed, colors, out);
    }
}

fn speckles(inner: Rect, seed: u64, colors: &Colors, out: &mut Vec<Shape>) {
    let count = ((inner.w * inner.h / SPECKLE_AREA) as usize).min(MAX_SPECKLES);
    let mut rng = stream(seed, TAG_SPECKLES, 0.0);
    for _ in 0..count {
        let size = if rng.unit() < 0.7 { 1.0 } else { 2.0 };
        let x = inner.x + rng.unit() * (inner.w - size).max(0.0);
        let y = inner.y + rng.unit() * (inner.h - size).max(0.0);
        let pick = rng.unit();
        let color = if pick < 0.4 {
            colors.speck_light
        } else if pick < 0.8 {
            colors.speck_dark
        } else {
            colors.speck_accent
        };
        out.push(Shape::rect(x, y, size, size, color));
    }
}

/// Short wandering cracks that start inside the fill and head downward.
fn cracks(inner: Rect, seed: u64, colors: &Colors, out: &mut Vec<Shape>) {
    let count = ((inner.w * inner.h / CRACK_AREA) as usize).min(MAX_CRACKS);
    let mut rng = stream(seed, TAG_CRACKS, 0.0);
    let keep_inside = |x: f32, y: f32| {
        (
            x.clamp(inner.x, inner.x + inner.w),
            y.clamp(inner.y, inner.y + inner.h),
        )
    };
    for _ in 0..count {
        let mut point = keep_inside(
            inner.x + rng.unit() * inner.w,
            inner.y + rng.unit() * inner.h * 0.6,
        );
        for _ in 0..2 + usize::from(rng.unit() < 0.5) {
            let next = keep_inside(
                point.0 + between(&mut rng, -6.0, 6.0),
                point.1 + between(&mut rng, 3.0, 9.0),
            );
            out.push(Shape::line(point, next, 1.0, colors.crack));
            point = next;
        }
    }
}

/// Outline along the open edges, a lit strip and a rocky lip on open tops, moss on open
/// tops and stalactites under open bottoms.
pub(super) fn trim_layer(
    rect: Rect,
    exposure: &Exposure,
    seed: u64,
    style: &TerrainStyle,
    colors: &Colors,
    out: &mut Vec<Shape>,
) {
    let width = OUTLINE_WIDTH.min(rect.w / 2.0).min(rect.h / 3.0);
    let lit = HIGHLIGHT_HEIGHT.min(rect.h / 3.0);
    // The top outline sits just under the lit strip, which gives the top a bevel.
    for span in &exposure.top {
        let (start, length) = (span.start, span.length());
        out.push(Shape::rect(
            start,
            rect.y + lit,
            length,
            width,
            colors.outline,
        ));
    }
    for span in &exposure.bottom {
        let (start, length) = (span.start, span.length());
        out.push(Shape::rect(
            start,
            rect.bottom() - width,
            length,
            width,
            colors.outline,
        ));
    }
    for span in &exposure.left {
        out.push(Shape::rect(
            rect.x,
            span.start,
            width,
            span.length(),
            colors.outline,
        ));
    }
    for span in &exposure.right {
        out.push(Shape::rect(
            rect.right() - width,
            span.start,
            width,
            span.length(),
            colors.outline,
        ));
    }
    for span in &exposure.top {
        out.push(Shape::rect(
            span.start,
            rect.y,
            span.length(),
            lit,
            colors.highlight,
        ));
        lip(rect, *span, seed, colors, out);
        if style.moss {
            moss(rect, *span, seed, colors, out);
        }
    }
    for span in &exposure.bottom {
        stalactites(rect, *span, seed, colors, out);
    }
}

/// A jittered stone lip along an open top: a row of trapezoids standing on the edge, rising
/// at most `LIP_HEIGHT` and tapering to nothing at both ends of the stretch.
fn lip(rect: Rect, span: EdgeSpan, seed: u64, colors: &Colors, out: &mut Vec<Shape>) {
    let mut rng = stream(seed, TAG_LIP, span.start);
    let segments = ((span.length() / LIP_STEP).round() as usize).clamp(2, MAX_LIP_SEGMENTS);
    let step = span.length() / segments as f32;
    let contour: Vec<(f32, f32)> = (0..=segments)
        .map(|i| match i {
            0 => (span.start, rect.y),
            last if last == segments => (span.end, rect.y),
            _ => {
                // Jitter stays under a quarter step, so the points never cross.
                let x = span.start + step * i as f32 + between(&mut rng, -0.25, 0.25) * step;
                (x, rect.y - between(&mut rng, 0.5, 1.0) * LIP_HEIGHT)
            }
        })
        .collect();
    // The trapezoids dip one pixel into the solid so no seam shows between lip and fill.
    let base = rect.y + 1.0;
    for pair in contour.windows(2) {
        let (left, right) = (pair[0], pair[1]);
        out.push(Shape::Polygon {
            points: vec![(left.0, base), left, right, (right.0, base)],
            color: colors.lip,
        });
    }
}

/// Little blade tufts standing on an open top.
fn moss(rect: Rect, span: EdgeSpan, seed: u64, colors: &Colors, out: &mut Vec<Shape>) {
    let mut rng = stream(seed, TAG_MOSS, span.start);
    let (first, last) = (span.start + END_INSET, span.end - END_INSET);
    let mut x = first;
    let mut slots = 0;
    while x <= last && slots < MAX_SLOTS {
        slots += 1;
        if rng.unit() < MOSS_CHANCE {
            for blade in 0..2 + usize::from(rng.unit() < 0.5) {
                let foot = x + between(&mut rng, -2.0, 2.0);
                let width = between(&mut rng, 2.0, 3.5);
                let tip = (
                    foot + between(&mut rng, -1.5, 1.5),
                    rect.y - between(&mut rng, 3.0, MOSS_HEIGHT),
                );
                let color = if blade % 2 == 0 {
                    colors.moss
                } else {
                    colors.moss_dark
                };
                out.push(Shape::triangle(
                    [
                        (foot - width / 2.0, rect.y + 1.0),
                        (foot + width / 2.0, rect.y + 1.0),
                        tip,
                    ],
                    color,
                ));
            }
        }
        x += between(&mut rng, MOSS_GAP.0, MOSS_GAP.1);
    }
}

/// Short stone points hanging under an open bottom.
fn stalactites(rect: Rect, span: EdgeSpan, seed: u64, colors: &Colors, out: &mut Vec<Shape>) {
    let mut rng = stream(seed, TAG_DRIPS, span.start);
    let (first, last) = (span.start + END_INSET, span.end - END_INSET);
    let mut x = first;
    let mut slots = 0;
    while x <= last && slots < MAX_SLOTS {
        slots += 1;
        if rng.unit() < DRIP_CHANCE {
            let width = between(&mut rng, 3.0, 7.0);
            let length = between(&mut rng, 3.0, DRIP_LENGTH);
            let tip = (x + between(&mut rng, -0.8, 0.8), rect.bottom() + length);
            let top = rect.bottom() - 1.0;
            out.push(Shape::triangle(
                [(x - width / 2.0, top), (x + width / 2.0, top), tip],
                colors.stalactite,
            ));
        }
        x += between(&mut rng, DRIP_GAP.0, DRIP_GAP.1);
    }
}
