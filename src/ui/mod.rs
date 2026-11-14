//! UI modules, implemented as `impl DreemzApp` blocks to keep the app state
//! and its views together.

pub mod canvas;
pub mod dialogs;
pub mod panels;
pub mod timeline;

use egui::{Color32, CornerRadius, Response, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::icons::{self, Icon};
use crate::theme;

pub fn icon_button(ui: &mut Ui, icon: Icon, size: f32, selected: bool, tip: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::click());
    let bg = if selected {
        theme::ACCENT_DIM
    } else if resp.hovered() {
        theme::BG_HOVER
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), bg);
    }
    if selected {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(6),
            Stroke::new(1.0, theme::ACCENT),
            StrokeKind::Inside,
        );
    }
    let color = if selected {
        theme::ACCENT
    } else if resp.hovered() {
        Color32::WHITE
    } else {
        theme::TEXT_DIM
    };
    icons::paint_centered(ui.painter(), icon, rect.shrink(size * 0.2), color);
    resp.on_hover_text(tip)
}

pub fn divider(ui: &mut Ui) {
    ui.add_space(2.0);
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0, theme::STROKE_SOFT);
    ui.add_space(2.0);
}
