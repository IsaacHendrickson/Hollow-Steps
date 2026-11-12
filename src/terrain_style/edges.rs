//! Which parts of a solid's four edges are open air.
//!
//! A stretch of an edge is "covered" when another solid (or a cover rectangle such as the
//! ground) fills the space just outside it. Solids that touch, stack or overlap hide the
//! seam between them; only the open stretches get an outline, a lit top, moss or drips.

use macroquad::math::Rect;

/// Positions closer than this count as the same line.
const TOL: f32 = 0.01;
/// Exposed stretches shorter than this are dropped (float crumbs from rounding).
const MIN_SPAN: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

impl Edge {
    pub const ALL: [Edge; 4] = [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right];
}

/// A stretch of an edge, in world coordinates measured along the edge: x for a top or
/// bottom edge, y for a left or right edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeSpan {
    pub start: f32,
    pub end: f32,
}

impl EdgeSpan {
    pub fn length(&self) -> f32 {
        self.end - self.start
    }
}

/// The open stretches of each edge of one solid, in increasing order along the edge.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Exposure {
    pub top: Vec<EdgeSpan>,
    pub bottom: Vec<EdgeSpan>,
    pub left: Vec<EdgeSpan>,
    pub right: Vec<EdgeSpan>,
}

impl Exposure {
    pub fn spans(&self, edge: Edge) -> &[EdgeSpan] {
        match edge {
            Edge::Top => &self.top,
            Edge::Bottom => &self.bottom,
            Edge::Left => &self.left,
            Edge::Right => &self.right,
        }
    }

    /// True when any stretch of `edge` is open.
    pub fn is_exposed(&self, edge: Edge) -> bool {
        !self.spans(edge).is_empty()
    }

    /// True when every edge is covered.
    pub fn is_buried(&self) -> bool {
        Edge::ALL.iter().all(|&edge| !self.is_exposed(edge))
    }
}

/// The open stretches of every edge of `solids[index]`. Other solids and `cover` rectangles
/// hide the parts of an edge they sit against or overlap. A solid that is not in `solids`
/// has no exposure.
///
/// When two solids share an edge line and overlap along it, exactly one keeps that stretch
/// exposed (the larger one; equal solids fall back to their order), so the same edge is
/// never drawn twice. A `cover` rectangle always wins such a tie.
pub fn exposure(solids: &[Rect], index: usize, cover: &[Rect]) -> Exposure {
    let Some(me) = solids.get(index) else {
        return Exposure::default();
    };
    let open = |edge: Edge| {
        let mut covered: Vec<EdgeSpan> = solids
            .iter()
            .enumerate()
            .filter(|&(other_index, _)| other_index != index)
            .filter_map(|(other_index, other)| {
                let wins = outranks(other, other_index, me, index);
                covered_by(me, other, edge, wins)
            })
            .collect();
        covered.extend(
            cover
                .iter()
                .filter_map(|other| covered_by(me, other, edge, true)),
        );
        subtract(along(me, edge), covered)
    };
    Exposure {
        top: open(Edge::Top),
        bottom: open(Edge::Bottom),
        left: open(Edge::Left),
        right: open(Edge::Right),
    }
}

/// Start and end of an edge along its own direction.
fn along(rect: &Rect, edge: Edge) -> (f32, f32) {
    match edge {
        Edge::Top | Edge::Bottom => (rect.x, rect.right()),
        Edge::Left | Edge::Right => (rect.y, rect.bottom()),
    }
}

/// Where the edge sits, across its own direction.
fn line(rect: &Rect, edge: Edge) -> f32 {
    match edge {
        Edge::Top => rect.y,
        Edge::Bottom => rect.bottom(),
        Edge::Left => rect.x,
        Edge::Right => rect.right(),
    }
}

/// The rectangle's extent across an edge's direction.
fn across(rect: &Rect, edge: Edge) -> (f32, f32) {
    match edge {
        Edge::Top | Edge::Bottom => (rect.y, rect.bottom()),
        Edge::Left | Edge::Right => (rect.x, rect.right()),
    }
}

/// Whether `other` takes a shared edge line from `me`: the larger solid wins, equal sizes
/// compare by position, and identical solids by order.
fn outranks(other: &Rect, other_index: usize, me: &Rect, me_index: usize) -> bool {
    let (other_area, my_area) = (other.w * other.h, me.w * me.h);
    if (other_area - my_area).abs() > TOL {
        return other_area > my_area;
    }
    match (other.x, other.y, other.w).partial_cmp(&(me.x, me.y, me.w)) {
        Some(std::cmp::Ordering::Greater) => true,
        Some(std::cmp::Ordering::Less) => false,
        _ => other_index < me_index,
    }
}

/// The part of `me`'s edge that `other` covers, if any. `wins` settles an edge line the two
/// share.
fn covered_by(me: &Rect, other: &Rect, edge: Edge, wins: bool) -> Option<EdgeSpan> {
    // `f32::max` and `f32::min` skip a NaN, so a broken neighbor would look like it covers
    // the whole edge; it covers nothing instead.
    if ![other.x, other.y, other.w, other.h]
        .iter()
        .all(|value| value.is_finite())
    {
        return None;
    }
    let (mine_start, mine_end) = along(me, edge);
    let (other_start, other_end) = along(other, edge);
    let (start, end) = (mine_start.max(other_start), mine_end.min(other_end));
    if end - start <= TOL {
        // Only touching at a corner, or apart.
        return None;
    }
    let edge_line = line(me, edge);
    let (low, high) = across(other, edge);
    let covers = match edge {
        // The outside is the smaller-coordinate side.
        Edge::Top | Edge::Left => {
            if low < edge_line - TOL {
                high > edge_line - TOL
            } else {
                (low - edge_line).abs() <= TOL && wins
            }
        }
        // The outside is the larger-coordinate side.
        Edge::Bottom | Edge::Right => {
            if high > edge_line + TOL {
                low < edge_line + TOL
            } else {
                (high - edge_line).abs() <= TOL && wins
            }
        }
    };
    covers.then_some(EdgeSpan { start, end })
}

/// What is left of `full` once the covered stretches are taken out.
fn subtract(full: (f32, f32), mut covered: Vec<EdgeSpan>) -> Vec<EdgeSpan> {
    covered.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut open = Vec::new();
    let mut cursor = full.0;
    for span in covered {
        if span.start > cursor {
            open.push(EdgeSpan {
                start: cursor,
                end: span.start,
            });
        }
        cursor = cursor.max(span.end);
    }
    if full.1 > cursor {
        open.push(EdgeSpan {
            start: cursor,
            end: full.1,
        });
    }
    open.retain(|span| span.length() >= MIN_SPAN);
    open
}
