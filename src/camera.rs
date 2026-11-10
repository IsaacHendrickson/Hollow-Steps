//! Follow camera, virtual-resolution letterboxing and HUD anchoring.
//!
//! Pure math only: the binary turns these results into a Macroquad `Camera2D`,
//! render target and draw calls.

use macroquad::math::{Rect, Vec2};
use serde::Deserialize;

/// Tunable camera behavior, loaded from the embedded `data/camera.ron`.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CameraConfig {
    /// How quickly the camera closes the gap to its target (per second; higher is snappier).
    pub follow_rate: f32,
    /// Horizontal distance the camera leads the player's facing direction, in pixels.
    pub look_ahead: f32,
    /// How quickly the look-ahead offset swings toward the facing direction (per second).
    pub look_ahead_rate: f32,
    /// A target farther than this from the camera is jumped to instead of followed.
    pub snap_distance: f32,
}

impl Default for CameraConfig {
    fn default() -> Self {
        let config: Self =
            ron::from_str(include_str!("../data/camera.ron")).expect("valid camera defaults");
        config.validate().expect("valid camera defaults");
        config
    }
}

impl CameraConfig {
    pub fn validate(&self) -> Result<(), String> {
        let positive = |name: &str, value: f32| {
            if value.is_finite() && value > 0.0 {
                Ok(())
            } else {
                Err(format!("camera {name} must be finite and positive"))
            }
        };
        positive("follow_rate", self.follow_rate)?;
        positive("look_ahead_rate", self.look_ahead_rate)?;
        positive("snap_distance", self.snap_distance)?;
        if !self.look_ahead.is_finite() || self.look_ahead < 0.0 {
            return Err("camera look_ahead must be finite and not negative".into());
        }
        Ok(())
    }
}

/// Keeps a view of size `view` inside `bounds`. On an axis where the room is no larger
/// than the view, the view is centered on the room instead.
pub fn clamp_center(center: Vec2, bounds: Rect, view: Vec2) -> Vec2 {
    let axis = |center: f32, start: f32, length: f32, view: f32| {
        if length <= view {
            start + length / 2.0
        } else {
            center.clamp(start + view / 2.0, start + length - view / 2.0)
        }
    };
    Vec2::new(
        axis(center.x, bounds.x, bounds.w, view.x),
        axis(center.y, bounds.y, bounds.h, view.y),
    )
}

/// Smoothed follow camera with horizontal look-ahead.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    config: CameraConfig,
    center: Vec2,
    look: f32,
}

impl Camera {
    /// A camera already resting on `target`.
    pub fn new(config: CameraConfig, target: Vec2, facing: f32, bounds: Rect, view: Vec2) -> Self {
        let mut camera = Self {
            config,
            center: Vec2::ZERO,
            look: 0.0,
        };
        camera.snap(target, facing, bounds, view);
        camera
    }

    /// World point at the middle of the view.
    pub fn center(&self) -> Vec2 {
        self.center
    }

    /// Current horizontal look-ahead offset (positive looks right).
    pub fn look_ahead(&self) -> f32 {
        self.look
    }

    /// The world rectangle that is visible.
    pub fn view_rect(&self, view: Vec2) -> Rect {
        Rect::new(
            self.center.x - view.x / 2.0,
            self.center.y - view.y / 2.0,
            view.x,
            view.y,
        )
    }

    /// Jumps straight to the settled position for `target`, e.g. after a room change.
    pub fn snap(&mut self, target: Vec2, facing: f32, bounds: Rect, view: Vec2) {
        if !target.is_finite() {
            return;
        }
        self.look = self.look_target(facing);
        self.center = clamp_center(target + Vec2::new(self.look, 0.0), bounds, view);
    }

    /// Advances the camera. Non-finite or non-positive `delta_seconds` and non-finite
    /// targets leave it unchanged.
    pub fn update(
        &mut self,
        target: Vec2,
        facing: f32,
        bounds: Rect,
        view: Vec2,
        delta_seconds: f32,
    ) {
        if !target.is_finite() || !delta_seconds.is_finite() || delta_seconds <= 0.0 {
            return;
        }
        let blend = |rate: f32| 1.0 - (-rate * delta_seconds).exp();
        self.look += (self.look_target(facing) - self.look) * blend(self.config.look_ahead_rate);
        let desired = clamp_center(target + Vec2::new(self.look, 0.0), bounds, view);
        if self.center.distance(desired) > self.config.snap_distance {
            self.center = desired;
        } else {
            self.center += (desired - self.center) * blend(self.config.follow_rate);
        }
        self.center = clamp_center(self.center, bounds, view);
    }

    fn look_target(&self, facing: f32) -> f32 {
        if facing.is_finite() && facing != 0.0 {
            facing.signum() * self.config.look_ahead
        } else {
            0.0
        }
    }
}

/// Where the virtual screen lands in the window: uniform scale, centered, with bars on
/// the unused sides so resizing never crops or stretches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Letterbox {
    pub offset: Vec2,
    pub size: Vec2,
    pub scale: f32,
}

impl Letterbox {
    /// Fits a `view`-sized screen into a `window`-sized one. A degenerate window gives a
    /// zero-sized box rather than NaN.
    pub fn fit(window: Vec2, view: Vec2) -> Self {
        let scale = (window.x / view.x).min(window.y / view.y);
        // `f32::min` ignores NaN, so check the inputs too.
        let scale = if window.is_finite() && view.is_finite() && scale.is_finite() && scale > 0.0 {
            scale
        } else {
            0.0
        };
        let size = view * scale;
        Self {
            offset: ((window - size) / 2.0).max(Vec2::ZERO),
            size,
            scale,
        }
    }

    /// Converts a window position (e.g. the mouse) to virtual-screen coordinates.
    pub fn window_to_virtual(&self, point: Vec2) -> Vec2 {
        if self.scale > 0.0 {
            (point - self.offset) / self.scale
        } else {
            Vec2::ZERO
        }
    }
}

/// Edge or corner of the virtual screen that a HUD element hangs from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    TopLeft,
    TopCenter,
    TopRight,
    CenterLeft,
    Center,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

/// Top-left corner for a box of `size` attached to `anchor` on a `view`-sized screen.
/// `offset` pushes the box inward from the anchored edges (down and right from top-left,
/// up and left from bottom-right); on a centered axis it is a plain shift.
pub fn anchored(view: Vec2, anchor: Anchor, size: Vec2, offset: Vec2) -> Vec2 {
    let (horizontal, vertical) = match anchor {
        Anchor::TopLeft => (-1.0, -1.0),
        Anchor::TopCenter => (0.0, -1.0),
        Anchor::TopRight => (1.0, -1.0),
        Anchor::CenterLeft => (-1.0, 0.0),
        Anchor::Center => (0.0, 0.0),
        Anchor::CenterRight => (1.0, 0.0),
        Anchor::BottomLeft => (-1.0, 1.0),
        Anchor::BottomCenter => (0.0, 1.0),
        Anchor::BottomRight => (1.0, 1.0),
    };
    let axis = |side: f32, view: f32, size: f32, offset: f32| match side {
        s if s < 0.0 => offset,
        s if s > 0.0 => view - size - offset,
        _ => (view - size) / 2.0 + offset,
    };
    Vec2::new(
        axis(horizontal, view.x, size.x, offset.x),
        axis(vertical, view.y, size.y, offset.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Vec2 = Vec2::new(960.0, 540.0);

    fn wide_room() -> Rect {
        Rect::new(0.0, 0.0, 2400.0, 1000.0)
    }

    fn camera_at(target: Vec2) -> Camera {
        Camera::new(CameraConfig::default(), target, 1.0, wide_room(), VIEW)
    }

    fn settle(camera: &mut Camera, target: Vec2, facing: f32, bounds: Rect) {
        for _ in 0..1200 {
            camera.update(target, facing, bounds, VIEW, 1.0 / 120.0);
        }
    }

    #[test]
    fn shipped_defaults_validate_and_bad_values_are_rejected() {
        let config = CameraConfig::default();
        assert!(config.validate().is_ok());
        for broken in [
            CameraConfig {
                follow_rate: 0.0,
                ..config
            },
            CameraConfig {
                look_ahead_rate: f32::NAN,
                ..config
            },
            CameraConfig {
                snap_distance: -1.0,
                ..config
            },
            CameraConfig {
                look_ahead: -5.0,
                ..config
            },
            CameraConfig {
                look_ahead: f32::INFINITY,
                ..config
            },
        ] {
            assert!(broken.validate().is_err());
        }
        assert!(ron::from_str::<CameraConfig>("(follow_rate: 1.0, extra: 2.0)").is_err());
    }

    #[test]
    fn view_is_clamped_to_room_bounds() {
        let room = wide_room();
        assert_eq!(
            clamp_center(Vec2::new(-500.0, -500.0), room, VIEW),
            Vec2::new(480.0, 270.0)
        );
        assert_eq!(
            clamp_center(Vec2::new(9000.0, 9000.0), room, VIEW),
            Vec2::new(1920.0, 730.0)
        );
        assert_eq!(
            clamp_center(Vec2::new(1000.0, 500.0), room, VIEW),
            Vec2::new(1000.0, 500.0)
        );
        let offset_room = Rect::new(100.0, 50.0, 2000.0, 800.0);
        assert_eq!(
            clamp_center(Vec2::ZERO, offset_room, VIEW),
            Vec2::new(580.0, 320.0)
        );
    }

    #[test]
    fn room_smaller_than_the_view_is_centered_per_axis() {
        let small = Rect::new(0.0, 0.0, 600.0, 2000.0);
        let center = clamp_center(Vec2::new(5.0, 1500.0), small, VIEW);
        assert_eq!(center.x, 300.0);
        assert_eq!(center.y, 1500.0);
        let exact = Rect::new(0.0, 0.0, 960.0, 540.0);
        assert_eq!(clamp_center(Vec2::new(1.0, 1.0), exact, VIEW), VIEW / 2.0);
    }

    #[test]
    fn following_converges_without_overshooting() {
        let mut camera = camera_at(Vec2::new(500.0, 400.0));
        let target = Vec2::new(1500.0, 600.0);
        let mut last_gap = f32::MAX;
        for _ in 0..600 {
            camera.update(target, 1.0, wide_room(), VIEW, 1.0 / 120.0);
            let gap = camera
                .center()
                .distance(target + Vec2::new(camera.look_ahead(), 0.0));
            assert!(gap <= last_gap + 0.5, "gap grew from {last_gap} to {gap}");
            last_gap = gap;
        }
        assert!(last_gap < 1.0);
    }

    #[test]
    fn follow_speed_does_not_depend_on_the_step_size() {
        let target = Vec2::new(1500.0, 500.0);
        let mut coarse = camera_at(Vec2::new(500.0, 500.0));
        let mut fine = coarse;
        for _ in 0..30 {
            coarse.update(target, 1.0, wide_room(), VIEW, 1.0 / 30.0);
        }
        for _ in 0..120 {
            fine.update(target, 1.0, wide_room(), VIEW, 1.0 / 120.0);
        }
        assert!((coarse.center().x - fine.center().x).abs() < 1.0);
    }

    #[test]
    fn look_ahead_follows_facing_and_flips_sign() {
        let config = CameraConfig::default();
        let target = Vec2::new(1200.0, 500.0);
        let mut camera = camera_at(target);
        settle(&mut camera, target, 1.0, wide_room());
        assert!((camera.look_ahead() - config.look_ahead).abs() < 0.1);
        assert!(camera.center().x > target.x);
        settle(&mut camera, target, -1.0, wide_room());
        assert!((camera.look_ahead() + config.look_ahead).abs() < 0.1);
        assert!(camera.center().x < target.x);
    }

    #[test]
    fn look_ahead_never_pushes_the_view_outside_the_room() {
        let mut camera = camera_at(Vec2::new(100.0, 500.0));
        settle(&mut camera, Vec2::new(100.0, 500.0), -1.0, wide_room());
        assert_eq!(camera.center().x, 480.0);
        let view = camera.view_rect(VIEW);
        assert!(view.x >= 0.0 && view.right() <= 2400.0);
    }

    #[test]
    fn far_targets_snap_and_snap_ignores_smoothing() {
        let mut camera = camera_at(Vec2::new(200.0, 500.0));
        camera.update(
            Vec2::new(2200.0, 500.0),
            1.0,
            wide_room(),
            VIEW,
            1.0 / 120.0,
        );
        assert!(camera.center().x > 1900.0);
        let mut other = camera_at(Vec2::new(200.0, 500.0));
        other.snap(Vec2::new(1000.0, 500.0), -1.0, wide_room(), VIEW);
        assert_eq!(other.look_ahead(), -CameraConfig::default().look_ahead);
        assert_eq!(
            other.center().x,
            1000.0 - CameraConfig::default().look_ahead
        );
    }

    #[test]
    fn zero_negative_and_non_finite_input_is_safe() {
        let mut camera = camera_at(Vec2::new(1000.0, 500.0));
        let before = camera;
        for delta in [0.0, -1.0, f32::NAN, f32::INFINITY] {
            camera.update(Vec2::new(1500.0, 500.0), 1.0, wide_room(), VIEW, delta);
            assert_eq!(camera, before);
        }
        camera.update(Vec2::new(f32::NAN, 0.0), 1.0, wide_room(), VIEW, 0.01);
        camera.snap(Vec2::new(0.0, f32::INFINITY), 1.0, wide_room(), VIEW);
        assert_eq!(camera, before);
        camera.update(Vec2::new(1500.0, 500.0), f32::NAN, wide_room(), VIEW, 0.01);
        assert!(camera.center().is_finite() && camera.look_ahead().is_finite());
        // Zero facing means no look-ahead target.
        let mut still = camera_at(Vec2::new(1000.0, 500.0));
        still.snap(Vec2::new(1000.0, 500.0), 0.0, wide_room(), VIEW);
        assert_eq!(still.look_ahead(), 0.0);
    }

    #[test]
    fn view_rect_is_centered_on_the_camera() {
        let camera = camera_at(Vec2::new(1000.0, 500.0));
        let rect = camera.view_rect(VIEW);
        assert_eq!(rect.w, 960.0);
        assert_eq!(rect.h, 540.0);
        assert_eq!(
            Vec2::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0),
            camera.center()
        );
    }

    #[test]
    fn letterbox_fits_without_cropping_or_stretching() {
        let same = Letterbox::fit(VIEW, VIEW);
        assert_eq!(
            (same.offset, same.size, same.scale),
            (Vec2::ZERO, VIEW, 1.0)
        );
        let wide = Letterbox::fit(Vec2::new(1920.0, 540.0), VIEW);
        assert_eq!(wide.scale, 1.0);
        assert_eq!(wide.offset, Vec2::new(480.0, 0.0));
        let tall = Letterbox::fit(Vec2::new(960.0, 1080.0), VIEW);
        assert_eq!(tall.scale, 1.0);
        assert_eq!(tall.offset, Vec2::new(0.0, 270.0));
        let small = Letterbox::fit(Vec2::new(480.0, 480.0), VIEW);
        assert_eq!(small.scale, 0.5);
        assert!((small.size.x / small.size.y - VIEW.x / VIEW.y).abs() < 1e-4);
        assert!(small.offset.x >= 0.0 && small.offset.y >= 0.0);
        assert!(small.size.x <= 480.0 && small.size.y <= 480.0);
    }

    #[test]
    fn letterbox_handles_degenerate_windows_and_maps_points_back() {
        for window in [
            Vec2::ZERO,
            Vec2::new(-5.0, 100.0),
            Vec2::new(f32::NAN, 100.0),
        ] {
            let boxed = Letterbox::fit(window, VIEW);
            assert_eq!(boxed.size, Vec2::ZERO);
            assert_eq!(boxed.window_to_virtual(Vec2::new(10.0, 10.0)), Vec2::ZERO);
        }
        let boxed = Letterbox::fit(Vec2::new(1920.0, 1200.0), VIEW);
        let corner = boxed.offset + boxed.size;
        assert!((boxed.window_to_virtual(boxed.offset)).length() < 1e-3);
        assert!((boxed.window_to_virtual(corner) - VIEW).length() < 1e-2);
    }

    #[test]
    fn anchors_place_boxes_on_edges_with_inward_offsets() {
        let size = Vec2::new(100.0, 40.0);
        let offset = Vec2::new(10.0, 6.0);
        let at = |anchor| anchored(VIEW, anchor, size, offset);
        assert_eq!(at(Anchor::TopLeft), Vec2::new(10.0, 6.0));
        assert_eq!(at(Anchor::TopRight), Vec2::new(850.0, 6.0));
        assert_eq!(at(Anchor::BottomLeft), Vec2::new(10.0, 494.0));
        assert_eq!(at(Anchor::BottomRight), Vec2::new(850.0, 494.0));
        assert_eq!(at(Anchor::TopCenter), Vec2::new(440.0, 6.0));
        assert_eq!(at(Anchor::BottomCenter), Vec2::new(440.0, 494.0));
        assert_eq!(at(Anchor::CenterLeft), Vec2::new(10.0, 256.0));
        assert_eq!(at(Anchor::CenterRight), Vec2::new(850.0, 256.0));
        assert_eq!(at(Anchor::Center), Vec2::new(440.0, 256.0));
    }

    #[test]
    fn anchored_boxes_follow_the_view_size() {
        let bigger = Vec2::new(1280.0, 720.0);
        let size = Vec2::new(360.0, 12.0);
        let centered = anchored(bigger, Anchor::TopCenter, size, Vec2::new(0.0, 166.0));
        assert_eq!(centered, Vec2::new(460.0, 166.0));
        let full = anchored(VIEW, Anchor::BottomLeft, VIEW, Vec2::ZERO);
        assert_eq!(full, Vec2::ZERO);
    }
}
