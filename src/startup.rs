//! Startup and frame presentation for the Macroquad front end.
//!
//! Everything is drawn at the virtual resolution into a render target, which
//! `present` scales into the window with letterboxing. If the game cannot load,
//! `load_game` shows the load error on that same screen and retries on F5.

use hollow_steps::camera::Letterbox;
use hollow_steps::game::GameState;
use macroquad::prelude::*;

use crate::hud;

pub fn screen_camera(rect: Rect, target: Option<RenderTarget>) -> Camera2D {
    Camera2D {
        render_target: target,
        ..Camera2D::from_display_rect(rect)
    }
}

/// Draws the render target into the window, letterboxed, and returns the fit used.
pub fn present(screen: &RenderTarget, view: Vec2) -> Letterbox {
    set_default_camera();
    clear_background(BLACK);
    let letterbox = Letterbox::fit(vec2(screen_width(), screen_height()), view);
    draw_texture_ex(
        &screen.texture,
        letterbox.offset.x,
        letterbox.offset.y,
        WHITE,
        DrawTextureParams {
            dest_size: Some(letterbox.size),
            // Render targets come out upside down otherwise.
            flip_y: true,
            ..Default::default()
        },
    );
    letterbox
}

/// Loads the game, or shows why it could not load until F5 succeeds.
pub async fn load_game(screen: &RenderTarget, view: Vec2) -> GameState {
    let mut failure = match GameState::new() {
        Ok(game) => return game,
        Err(error) => Err(error),
    };
    let mut scroll = 0;
    loop {
        set_camera(&screen_camera(
            Rect::new(0.0, 0.0, view.x, view.y),
            Some(screen.clone()),
        ));
        clear_background(Color::from_rgba(14, 10, 20, 255));
        hud::draw_reload_result(
            &failure,
            "Game failed to load - F5 to retry",
            &mut scroll,
            view,
        );
        present(screen, view);
        next_frame().await;
        if is_key_pressed(KeyCode::F5) {
            match GameState::new() {
                Ok(game) => return game,
                Err(error) => {
                    failure = Err(error);
                    scroll = 0;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_screen_camera_shows_exactly_the_given_rectangle() {
        // Offset, negative and room-sized rectangles: target is the center, zoom maps the
        // rectangle's width and height to the whole render target.
        for rect in [
            Rect::new(0.0, 0.0, 960.0, 540.0),
            Rect::new(-500.0, 300.0, 960.0, 540.0),
            Rect::new(3840.0, 2460.0, 960.0, 540.0),
        ] {
            let camera = screen_camera(rect, None);
            assert_eq!(camera.target, rect.center());
            assert!((camera.zoom.x.abs() - 2.0 / rect.w).abs() < 1e-6);
            assert!((camera.zoom.y.abs() - 2.0 / rect.h).abs() < 1e-6);
            assert_eq!(camera.offset, Vec2::ZERO);
            assert_eq!(camera.rotation, 0.0);
        }
    }
}
