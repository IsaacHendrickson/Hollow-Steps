//! Helpers shared by the editor panel's headless tests.
use egui_macroquad::egui;

/// All text drawn in one egui pass, one line per text shape.
pub(super) fn drawn_text(output: &egui::FullOutput) -> String {
    fn collect(shape: &egui::epaint::Shape, out: &mut String) {
        match shape {
            egui::epaint::Shape::Text(text) => {
                out.push_str(text.galley.text());
                out.push('\n');
            }
            egui::epaint::Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let mut text = String::new();
    for clipped in &output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

pub(super) fn click(at: egui::Pos2) -> [Vec<egui::Event>; 2] {
    let button = |pressed| {
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: Default::default(),
            },
        ]
    };
    [button(true), button(false)]
}
