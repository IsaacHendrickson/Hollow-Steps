//! The map's layout on the screen: the panel, and how a block of cells is scaled and centered.
use macroquad::math::{vec2, Rect, Vec2};

use super::*;

const VIEW: Vec2 = Vec2::new(960.0, 540.0);
const EPSILON: f32 = 0.01;

fn inside(inner: Rect, outer: Rect) -> bool {
    inner.x >= outer.x - EPSILON
        && inner.y >= outer.y - EPSILON
        && inner.right() <= outer.right() + EPSILON
        && inner.bottom() <= outer.bottom() + EPSILON
}

/// `count` cells in rows of `per_row`.
fn block(count: i32, per_row: i32) -> Vec<Cell> {
    (0..count).map(|n| (n % per_row, n / per_row)).collect()
}

/// Every box is inside the area, no two touch, none is bigger than a capped cell, and the block
/// of cells is centered in the area.
fn assert_fits(area: Rect, cells: &[Cell]) {
    let grid = MapGrid::fit(area, cells);
    let rects: Vec<Rect> = cells.iter().map(|&cell| grid.room_rect(cell)).collect();
    for (cell, rect) in cells.iter().zip(&rects) {
        assert!(inside(*rect, area), "{cell:?} {rect:?} not inside {area:?}");
        assert!(
            rect.w <= MAX_PITCH * BOX_FRACTION + EPSILON,
            "{cell:?} too big"
        );
        assert!(rect.w > 0.0 && (rect.w - rect.h).abs() < EPSILON);
    }
    for (a, rect) in rects.iter().enumerate() {
        for other in &rects[a + 1..] {
            assert!(
                rect.intersect(*other).is_none(),
                "{rect:?} overlaps {other:?}"
            );
        }
    }
    let left_top = cells
        .iter()
        .fold((i32::MAX, i32::MAX), |m, c| (m.0.min(c.0), m.1.min(c.1)));
    let right_bottom = cells
        .iter()
        .fold((i32::MIN, i32::MIN), |m, c| (m.0.max(c.0), m.1.max(c.1)));
    let middle = (grid.center(left_top) + grid.center(right_bottom)) / 2.0;
    assert!(
        (middle - area.center()).length() < EPSILON,
        "block centered at {middle:?}, area at {:?}",
        area.center()
    );
}

// ---- the panel --------------------------------------------------------------------------------

#[test]
fn the_panel_is_inside_the_view_and_centered_and_the_cells_are_inside_the_panel() {
    for view in [
        VIEW,
        vec2(1920.0, 1080.0),
        vec2(640.0, 360.0),
        vec2(1280.0, 400.0),
    ] {
        let screen = screen_layout(view);
        let whole = Rect::new(0.0, 0.0, view.x, view.y);
        assert!(inside(screen.panel, whole), "{view:?}");
        assert!(inside(screen.area, screen.panel), "{view:?}");
        assert!((screen.panel.center() - whole.center()).length() < EPSILON);
        assert!(screen.area.w > 0.0 && screen.area.h > 0.0, "{view:?}");
        assert!(
            screen.title.y < screen.area.y,
            "the title is above the cells"
        );
        assert!(
            screen.footer.y > screen.area.bottom(),
            "the footer is below them"
        );
    }
}

#[test]
fn the_panel_follows_the_view_it_is_given() {
    let small = screen_layout(VIEW);
    let large = screen_layout(vec2(1920.0, 1080.0));
    assert!(large.panel.w > small.panel.w && large.panel.h > small.panel.h);
}

#[test]
fn a_view_too_small_for_the_panel_gives_an_empty_area_not_a_broken_one() {
    let screen = screen_layout(vec2(10.0, 10.0));
    assert!(screen.area.w >= 0.0 && screen.area.h >= 0.0);
    let grid = MapGrid::fit(screen.area, &[(0, 0), (3, 2)]);
    assert!(grid.pitch().is_finite() && grid.pitch() >= 0.0);
}

// ---- fitting cells ----------------------------------------------------------------------------

#[test]
fn one_room_two_rooms_and_forty_rooms_fit_and_are_centered() {
    let area = screen_layout(VIEW).area;
    assert_fits(area, &block(1, 1));
    assert_fits(area, &block(2, 2));
    assert_fits(area, &block(2, 1));
    assert_fits(area, &block(40, 8));
    assert_fits(area, &block(40, 40));
    assert_fits(area, &block(40, 1));
}

#[test]
fn a_few_rooms_are_capped_instead_of_filling_the_panel() {
    let area = screen_layout(VIEW).area;
    for cells in [block(1, 1), block(2, 2)] {
        let grid = MapGrid::fit(area, &cells);
        assert_eq!(grid.pitch(), MAX_PITCH);
        assert!(grid.box_size() < area.h);
    }
}

#[test]
fn a_big_map_is_scaled_down_until_it_fits_either_way() {
    let area = screen_layout(VIEW).area;
    let wide = MapGrid::fit(area, &block(40, 40));
    let tall = MapGrid::fit(area, &(0..40).map(|row| (0, row)).collect::<Vec<_>>());
    assert!(
        (wide.pitch() * 40.0 - area.w).abs() < EPSILON,
        "width decides"
    );
    assert!(
        (tall.pitch() * 40.0 - area.h).abs() < EPSILON,
        "height decides"
    );
    assert!(wide.pitch() < MAX_PITCH && tall.pitch() < wide.pitch());
}

#[test]
fn where_the_block_sits_in_the_grid_does_not_matter() {
    let area = screen_layout(VIEW).area;
    let shifted: Vec<Cell> = block(12, 4).iter().map(|c| (c.0 - 50, c.1 + 31)).collect();
    assert_fits(area, &shifted);
    let a = MapGrid::fit(area, &block(12, 4));
    let b = MapGrid::fit(area, &shifted);
    assert_eq!(a.pitch(), b.pitch());
    assert_eq!(a.room_rect((1, 1)), b.room_rect((-49, 32)));
}

#[test]
fn the_widest_legal_map_still_fits() {
    let area = screen_layout(VIEW).area;
    let corners = [(-CELL_LIMIT, -CELL_LIMIT), (CELL_LIMIT, CELL_LIMIT), (0, 0)];
    assert_fits(area, &corners);
    assert_fits(area, &[(i32::MIN, 0), (i32::MAX, 0)]);
}

#[test]
fn any_area_works_the_editor_window_has_its_own() {
    for area in [
        Rect::new(10.0, 20.0, 300.0, 200.0),
        Rect::new(-40.0, 5.0, 90.0, 600.0),
        Rect::new(0.0, 0.0, 2000.0, 80.0),
    ] {
        assert_fits(area, &block(1, 1));
        assert_fits(area, &block(7, 3));
        assert_fits(area, &block(40, 8));
    }
}

#[test]
fn nothing_to_draw_or_no_room_to_draw_in_gives_a_grid_of_size_zero() {
    let none = MapGrid::fit(screen_layout(VIEW).area, &[]);
    assert_eq!((none.pitch(), none.box_size()), (0.0, 0.0));
    let squeezed = MapGrid::fit(Rect::new(5.0, 5.0, 0.0, 0.0), &block(3, 3));
    assert_eq!(squeezed.room_rect((0, 0)).w, 0.0);
    assert!(squeezed.center((2, 0)).is_finite());
}
