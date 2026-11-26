//! The picture: one character per cell, y growing down. Terrain is drawn by how much of a cell it
//! covers (so a thin platform never vanishes); everything else by the cells it touches or mostly
//! covers, the more important mark winning where two share a cell.
use macroquad::math::Rect;

use super::scene::{Footprint, Scene, Thing};

/// Slack for rectangles that end exactly on a cell edge.
const EPS: f32 = 1e-3;
/// Width of the left margin that holds the y labels.
pub(super) const MARGIN: usize = 6;
/// The empty air of the room.
const AIR: char = '.';

/// How many characters wide a room `width` px wide is at `cell` px per character.
pub fn columns(width: f32, cell: f32) -> usize {
    ((width / cell - EPS).ceil().max(1.0)) as usize
}

/// Higher wins a shared cell. Equal marks keep the first one drawn, so the picture is stable.
/// The ends of a patrol range show over the enemy's body (it starts at one of them); the plain
/// line gives way to everything, terrain included, so a wall across a patrol breaks the line.
fn priority(mark: char) -> u8 {
    match mark {
        '@' => 100,
        '1'..='9' | 'a'..='z' | '?' => 90,
        'B' => 80,
        '<' | '>' => 76,
        'E' => 75,
        '^' => 70,
        'K' | 'D' | 'J' => 60,
        '$' => 55,
        'C' => 54,
        'S' => 53,
        '*' => 50,
        '/' => 45,
        '~' => 44,
        '%' => 43,
        '+' | '=' | '|' => 41,
        '#' => 40,
        '-' => 30,
        _ => 0,
    }
}

struct Canvas {
    origin: (f32, f32),
    cell: f32,
    cols: usize,
    rows: usize,
    marks: Vec<char>,
}

impl Canvas {
    fn put(&mut self, col: usize, row: usize, mark: char) {
        if col < self.cols && row < self.rows {
            let at = row * self.cols + col;
            if priority(mark) > priority(self.marks[at]) {
                self.marks[at] = mark;
            }
        }
    }

    /// The cell range of an interval `from..to` (px) on one axis, clamped to the picture.
    fn span(&self, from: f32, to: f32, origin: f32, count: usize) -> (usize, usize) {
        let first = ((from - origin) / self.cell + EPS).floor().max(0.0) as usize;
        let last = (((to - origin) / self.cell - EPS).ceil() - 1.0).max(first as f32) as usize;
        (first.min(count - 1), last.min(count - 1))
    }

    /// The cells a rectangle touches, or the cell of its center when it touches none.
    fn touched(&self, rect: Rect) -> Vec<(usize, usize)> {
        let (c0, c1) = self.span(rect.x, rect.right(), self.origin.0, self.cols);
        let (r0, r1) = self.span(rect.y, rect.bottom(), self.origin.1, self.rows);
        (r0..=r1)
            .flat_map(|row| (c0..=c1).map(move |col| (col, row)))
            .collect()
    }

    /// The cells a rectangle covers at least half of, or the cell of its center.
    fn mostly(&self, rect: Rect) -> Vec<(usize, usize)> {
        let covered: Vec<(usize, usize)> = self
            .touched(rect)
            .into_iter()
            .filter(|&(col, row)| self.overlap(rect, col, row) >= 0.5 - EPS)
            .collect();
        if !covered.is_empty() {
            return covered;
        }
        let (col, _) = self.span(rect.center().x, rect.center().x, self.origin.0, self.cols);
        let (row, _) = self.span(rect.center().y, rect.center().y, self.origin.1, self.rows);
        vec![(col, row)]
    }

    /// The share of cell (`col`, `row`) a rectangle covers, 0 to 1.
    fn overlap(&self, rect: Rect, col: usize, row: usize) -> f32 {
        let x0 = self.origin.0 + col as f32 * self.cell;
        let y0 = self.origin.1 + row as f32 * self.cell;
        let w = (rect.right().min(x0 + self.cell) - rect.x.max(x0)).max(0.0);
        let h = (rect.bottom().min(y0 + self.cell) - rect.y.max(y0)).max(0.0);
        w * h / (self.cell * self.cell)
    }

    fn terrain(&mut self, solids: &[Rect]) {
        let mut pieces: Vec<Vec<[f32; 4]>> = vec![Vec::new(); self.cols * self.rows];
        for solid in solids {
            for (col, row) in self.touched(*solid) {
                let x0 = self.origin.0 + col as f32 * self.cell;
                let y0 = self.origin.1 + row as f32 * self.cell;
                let clipped = [
                    solid.x.max(x0) - x0,
                    solid.y.max(y0) - y0,
                    solid.right().min(x0 + self.cell) - x0,
                    solid.bottom().min(y0 + self.cell) - y0,
                ];
                if clipped[2] > clipped[0] + EPS && clipped[3] > clipped[1] + EPS {
                    pieces[row * self.cols + col].push(clipped);
                }
            }
        }
        for (at, rects) in pieces.iter().enumerate() {
            if let Some(mark) = classify(rects, self.cell) {
                self.put(at % self.cols, at / self.cols, mark);
            }
        }
    }
}

/// The character of a cell holding these clipped rectangles (`[x0, y0, x1, y1]` inside the
/// cell): `#` when they cover at least half of it, else a thin mark for their shape.
fn classify(rects: &[[f32; 4]], cell: f32) -> Option<char> {
    if rects.is_empty() {
        return None;
    }
    let mut xs: Vec<f32> = rects.iter().flat_map(|r| [r[0], r[2]]).collect();
    let mut ys: Vec<f32> = rects.iter().flat_map(|r| [r[1], r[3]]).collect();
    xs.sort_by(f32::total_cmp);
    ys.sort_by(f32::total_cmp);
    let mut area = 0.0;
    for x in xs.windows(2) {
        for y in ys.windows(2) {
            let (cx, cy) = ((x[0] + x[1]) / 2.0, (y[0] + y[1]) / 2.0);
            if rects
                .iter()
                .any(|r| cx > r[0] && cx < r[2] && cy > r[1] && cy < r[3])
            {
                area += (x[1] - x[0]) * (y[1] - y[0]);
            }
        }
    }
    if area >= cell * cell * (0.5 - EPS) {
        return Some('#');
    }
    let wide = (xs[xs.len() - 1] - xs[0]) / cell >= 0.5 - EPS;
    let tall = (ys[ys.len() - 1] - ys[0]) / cell >= 0.5 - EPS;
    Some(match (wide, tall) {
        (true, false) => '=',
        (false, true) => '|',
        _ => '+',
    })
}

/// The picture, one string per row, with the y labels in the margin and an x axis under it.
pub(super) fn draw(scene: &Scene, things: &[Thing], cell: f32) -> Vec<String> {
    let bounds = scene.layout.rect();
    let cols = columns(bounds.w, cell);
    let rows = columns(bounds.h, cell);
    let mut canvas = Canvas {
        origin: (bounds.x, bounds.y),
        cell,
        cols,
        rows,
        marks: vec![AIR; cols * rows],
    };
    canvas.terrain(&scene.solids());
    for thing in things {
        let cells = match thing.footprint {
            Footprint::Terrain => continue,
            Footprint::Touch => canvas.touched(thing.rect),
            Footprint::Majority => canvas.mostly(thing.rect),
        };
        if let Some((left, right)) = thing.patrol {
            let row = canvas
                .span(thing.rect.center().y, thing.rect.center().y, bounds.y, rows)
                .0;
            let (from, _) = canvas.span(left.min(right), left.min(right), bounds.x, cols);
            let (to, _) = canvas.span(left.max(right), left.max(right), bounds.x, cols);
            for col in from..=to {
                canvas.put(col, row, '-');
            }
            canvas.put(from, row, '<');
            canvas.put(to, row, '>');
        }
        for (col, row) in cells {
            canvas.put(col, row, thing.mark);
        }
    }
    let mut lines: Vec<String> = (0..rows)
        .map(|row| {
            let label = if row % 10 == 0 {
                format!(
                    "{:>width$} ",
                    bounds.y + row as f32 * cell,
                    width = MARGIN - 1
                )
            } else {
                " ".repeat(MARGIN)
            };
            let picture: String = canvas.marks[row * cols..(row + 1) * cols].iter().collect();
            format!("{label}{picture}")
        })
        .collect();
    lines.push(axis(bounds.x, cell, cols));
    lines
}

/// The line under the picture: a tick and the x in px at every tenth column.
fn axis(x0: f32, cell: f32, cols: usize) -> String {
    let mut line: Vec<char> = " ".repeat(MARGIN + cols + 8).chars().collect();
    for col in (0..cols).step_by(10) {
        let label = format!("|{:.0}", x0 + col as f32 * cell);
        for (offset, ch) in label.chars().enumerate() {
            line[MARGIN + col + offset] = ch;
        }
    }
    line.into_iter().collect::<String>().trim_end().to_owned()
}
