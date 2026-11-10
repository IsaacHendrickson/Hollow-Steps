//! Pure room-editor canvas math: the world/screen transform with zoom and pan, grid
//! snapping, the window regions around the canvas, and footer text wrapping. The
//! Macroquad front end only feeds it mouse and window values and draws the result.
use macroquad::math::{Rect, Vec2};

/// Screen pixels per world unit, at the extremes.
pub const MIN_ZOOM: f32 = 0.1;
pub const MAX_ZOOM: f32 = 8.0;
/// Scale multiplier for one mouse-wheel step.
pub const ZOOM_PER_STEP: f32 = 1.15;
/// Wheel steps accepted in one frame; larger spins are clamped.
pub const MAX_WHEEL_STEPS: f32 = 3.0;
/// At least this much of the room (in pixels) stays inside the viewport after panning.
pub const KEEP_VISIBLE: f32 = 64.0;

pub const DEFAULT_SNAP: f32 = 8.0;
pub const MIN_SNAP: f32 = 1.0;
pub const MAX_SNAP: f32 = 128.0;
/// Grid lines closer than this on screen are skipped when drawing.
pub const MIN_GRID_PIXELS: f32 = 6.0;
const MAX_GRID_LINES: usize = 1000;

pub const HEADER_HEIGHT: f32 = 80.0;
pub const FOOTER_LINE_HEIGHT: f32 = 19.0;
/// The footer always reserves this many lines so a message appearing never resizes the
/// canvas; longer messages grow it.
pub const FOOTER_RESERVED_LINES: usize = 3;
const FOOTER_PADDING: f32 = 6.0;
pub const CANVAS_MARGIN: f32 = 20.0;

/// Maps world coordinates to screen pixels: `screen = offset + world * scale`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasView {
    /// Screen position of world point (0, 0).
    pub offset: Vec2,
    pub scale: f32,
}

impl CanvasView {
    /// Scales `room` to fit `area` with its top-left corner at the area's top-left.
    pub fn fit(room: Rect, area: Rect) -> Self {
        let scale = (area.w.max(1.0) / room.w.max(1.0))
            .min(area.h.max(1.0) / room.h.max(1.0))
            .clamp(MIN_ZOOM, MAX_ZOOM);
        Self {
            offset: area.point() - room.point() * scale,
            scale,
        }
    }

    pub fn screen(&self, world: Vec2) -> Vec2 {
        self.offset + world * self.scale
    }

    pub fn world(&self, screen: Vec2) -> Vec2 {
        (screen - self.offset) / self.scale
    }

    pub fn rect(&self, rect: Rect) -> Rect {
        let p = self.screen(rect.point());
        Rect::new(p.x, p.y, rect.w * self.scale, rect.h * self.scale)
    }

    /// Zooms by `steps` wheel steps (positive zooms in) keeping the world point under
    /// `cursor` fixed. The scale is clamped to `MIN_ZOOM..=MAX_ZOOM`; non-finite input
    /// is ignored.
    pub fn zoom_at(&mut self, cursor: Vec2, steps: f32) {
        if !steps.is_finite() || !cursor.is_finite() {
            return;
        }
        let steps = steps.clamp(-MAX_WHEEL_STEPS, MAX_WHEEL_STEPS);
        let anchor = self.world(cursor);
        self.scale = (self.scale * ZOOM_PER_STEP.powf(steps)).clamp(MIN_ZOOM, MAX_ZOOM);
        self.offset = cursor - anchor * self.scale;
    }

    pub fn pan_by(&mut self, delta: Vec2) {
        if delta.is_finite() {
            self.offset += delta;
        }
    }

    /// Slides the view back so a corner of `room` (at least `KEEP_VISIBLE` pixels, or the
    /// whole room if it is smaller) stays inside `viewport`.
    pub fn keep_visible(&mut self, room: Rect, viewport: Rect) {
        let r = self.rect(room);
        let keep = KEEP_VISIBLE.min(r.w).min(r.h);
        let slide = |low: f32, high: f32, view_low: f32, view_high: f32| {
            if high < view_low + keep {
                view_low + keep - high
            } else if low > view_high - keep {
                view_high - keep - low
            } else {
                0.0
            }
        };
        self.offset += Vec2::new(
            slide(r.x, r.right(), viewport.x, viewport.right()),
            slide(r.y, r.bottom(), viewport.y, viewport.bottom()),
        );
    }
}

/// Grid overlay and snapping. Grid lines are measured from the room's top-left corner so
/// they line up with the room edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub enabled: bool,
    pub size: f32,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            enabled: false,
            size: DEFAULT_SNAP,
        }
    }
}

impl Grid {
    /// The snap size, clamped to a usable range (also rejects NaN).
    pub fn step(&self) -> f32 {
        if self.size.is_finite() {
            self.size.clamp(MIN_SNAP, MAX_SNAP)
        } else {
            DEFAULT_SNAP
        }
    }

    /// Rounds `point` to the nearest grid line measured from `origin`. Returns it
    /// unchanged when the grid is off.
    pub fn snap(&self, point: Vec2, origin: Vec2) -> Vec2 {
        if !self.enabled || !point.is_finite() || !origin.is_finite() {
            return point;
        }
        let step = self.step();
        origin + ((point - origin) / step).round() * step
    }

    /// Spacing in world units between drawn lines: the snap size, doubled until the lines
    /// are at least `MIN_GRID_PIXELS` apart on screen. `None` when the grid is off.
    pub fn line_step(&self, scale: f32) -> Option<f32> {
        if !self.enabled || !scale.is_finite() || scale <= 0.0 {
            return None;
        }
        let mut step = self.step();
        while step * scale < MIN_GRID_PIXELS {
            step *= 2.0;
        }
        Some(step)
    }
}

/// World coordinates of grid lines in `start..=end` that sit on `origin + k * step`.
pub fn grid_lines(start: f32, end: f32, origin: f32, step: f32) -> impl Iterator<Item = f32> {
    let valid = step > 0.0 && step.is_finite() && start.is_finite() && end.is_finite();
    let first = origin + ((start - origin) / step).ceil() * step;
    let count = if valid && end >= first {
        (((end - first) / step).floor() as usize + 1).min(MAX_GRID_LINES)
    } else {
        0
    };
    (0..count).map(move |i| first + i as f32 * step)
}

/// Window space around the canvas: a header above, a footer below and the egui panel to
/// the right of `panel_left`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CanvasRegions {
    pub header: Rect,
    /// Everything between header and footer, left of the panel. The canvas draws and
    /// takes the wheel and pan input here.
    pub viewport: Rect,
    pub footer: Rect,
}

impl CanvasRegions {
    pub fn new(window: Vec2, panel_left: f32, footer_lines: usize) -> Self {
        let width = panel_left.clamp(1.0, window.x.max(1.0));
        let lines = footer_lines.max(FOOTER_RESERVED_LINES) as f32;
        let footer_height = (lines * FOOTER_LINE_HEIGHT + FOOTER_PADDING).min(window.y.max(1.0));
        let footer_top = (window.y - footer_height).max(HEADER_HEIGHT.min(window.y));
        Self {
            header: Rect::new(0.0, 0.0, width, HEADER_HEIGHT.min(window.y.max(1.0))),
            viewport: Rect::new(
                0.0,
                HEADER_HEIGHT,
                width,
                (footer_top - HEADER_HEIGHT).max(1.0),
            ),
            footer: Rect::new(0.0, footer_top, width, footer_height),
        }
    }

    /// The viewport shrunk by `CANVAS_MARGIN`, where a freshly fitted room goes.
    pub fn fit_area(&self) -> Rect {
        let v = self.viewport;
        Rect::new(
            v.x + CANVAS_MARGIN,
            v.y + CANVAS_MARGIN,
            (v.w - 2.0 * CANVAS_MARGIN).max(1.0),
            (v.h - 2.0 * CANVAS_MARGIN).max(1.0),
        )
    }

    /// Maximum pixel width of a footer text line.
    pub fn footer_wrap_width(&self) -> f32 {
        (self.footer.w - 2.0 * CANVAS_MARGIN).max(1.0)
    }

    /// Text baseline of footer line `index`.
    pub fn footer_baseline(&self, index: usize) -> f32 {
        self.footer.y + FOOTER_PADDING + FOOTER_LINE_HEIGHT * (index as f32 + 1.0) - 4.0
    }
}

/// Splits `text` into lines no wider than `max_width` according to `measure`. Words
/// wider than a whole line (long file paths) are broken at the limit.
pub fn wrap_words(text: &str, max_width: f32, measure: impl Fn(&str) -> f32) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let joined = if line.is_empty() {
            word.to_owned()
        } else {
            format!("{line} {word}")
        };
        if measure(&joined) <= max_width {
            line = joined;
            continue;
        }
        if !line.is_empty() {
            lines.push(std::mem::take(&mut line));
        }
        for c in word.chars() {
            line.push(c);
            if measure(&line) > max_width && line.chars().count() > 1 {
                line.pop();
                lines.push(std::mem::replace(&mut line, c.to_string()));
            }
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// Tracks a pan drag across frames and reports how far the mouse moved each frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct PanDrag {
    last: Option<Vec2>,
}

impl PanDrag {
    /// Call once per frame. `active` is true while pan input is held and allowed (the
    /// caller only starts a pan over the canvas). Returns the mouse movement since the
    /// previous active frame; the first active frame returns zero.
    pub fn update(&mut self, active: bool, mouse: Vec2) -> Vec2 {
        let delta = match self.last {
            Some(last) if active => mouse - last,
            _ => Vec2::ZERO,
        };
        self.last = active.then_some(mouse);
        delta
    }

    pub fn is_panning(&self) -> bool {
        self.last.is_some()
    }
}
