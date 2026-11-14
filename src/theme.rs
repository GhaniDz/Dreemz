//! Dark charcoal + amber theme, inspired by animation suites.

use egui::{
    Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, ThemePreference, Visuals,
};

pub const BG_DEEP: Color32 = Color32::from_rgb(16, 16, 19);
pub const BG_PANEL: Color32 = Color32::from_rgb(24, 24, 28);
pub const BG_RAISED: Color32 = Color32::from_rgb(34, 34, 39);
pub const BG_HOVER: Color32 = Color32::from_rgb(44, 44, 50);
pub const BG_CANVAS: Color32 = Color32::from_rgb(13, 13, 16);
pub const STROKE: Color32 = Color32::from_rgb(48, 48, 55);
pub const STROKE_SOFT: Color32 = Color32::from_rgb(38, 38, 44);
pub const ACCENT: Color32 = Color32::from_rgb(238, 168, 66);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(120, 86, 38);
pub const TEXT: Color32 = Color32::from_rgb(228, 226, 222);
pub const TEXT_DIM: Color32 = Color32::from_rgb(146, 143, 138);
pub const TEXT_FAINT: Color32 = Color32::from_rgb(102, 100, 98);
pub const DANGER: Color32 = Color32::from_rgb(232, 106, 98);

pub fn apply(ctx: &egui::Context) {
    ctx.set_theme(ThemePreference::Dark);

    ctx.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(9.0, 4.0);
        style.spacing.interact_size = egui::vec2(28.0, 24.0);
        style.spacing.slider_width = 118.0;
        style.spacing.combo_width = 118.0;
        style.spacing.indent = 14.0;
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.menu_margin = egui::Margin::same(8);
        style.animation_time = 0.10;
        style.visuals.dark_mode = true;

        style.text_styles.insert(
            TextStyle::Heading,
            FontId::new(19.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Body,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Button,
            FontId::new(13.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Small,
            FontId::new(11.0, FontFamily::Proportional),
        );
        style.text_styles.insert(
            TextStyle::Monospace,
            FontId::new(12.0, FontFamily::Monospace),
        );

        let mut v = Visuals::dark();
        v.dark_mode = true;
        v.override_text_color = Some(TEXT);
        v.panel_fill = BG_PANEL;
        v.window_fill = BG_PANEL;
        v.window_stroke = Stroke::new(1.0, STROKE);
        v.window_corner_radius = CornerRadius::same(10);
        v.menu_corner_radius = CornerRadius::same(8);
        v.extreme_bg_color = BG_DEEP;
        v.faint_bg_color = BG_RAISED;
        v.code_bg_color = BG_DEEP;
        v.hyperlink_color = ACCENT;
        v.warn_fg_color = Color32::from_rgb(240, 190, 90);
        v.error_fg_color = Color32::from_rgb(232, 106, 98);
        v.selection.bg_fill = ACCENT_DIM;
        v.selection.stroke = Stroke::new(1.0, TEXT);
        v.text_cursor.stroke = Stroke::new(1.5, ACCENT);

        let radius = CornerRadius::same(6);
        v.widgets.noninteractive.bg_fill = BG_PANEL;
        v.widgets.noninteractive.weak_bg_fill = BG_PANEL;
        v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, STROKE_SOFT);
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT_DIM);
        v.widgets.noninteractive.corner_radius = radius;

        v.widgets.inactive.bg_fill = BG_RAISED;
        v.widgets.inactive.weak_bg_fill = BG_RAISED;
        v.widgets.inactive.bg_stroke = Stroke::new(1.0, STROKE_SOFT);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.inactive.corner_radius = radius;

        v.widgets.hovered.bg_fill = BG_HOVER;
        v.widgets.hovered.weak_bg_fill = BG_HOVER;
        v.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(74, 72, 78));
        v.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        v.widgets.hovered.corner_radius = radius;

        v.widgets.active.bg_fill = ACCENT_DIM;
        v.widgets.active.weak_bg_fill = ACCENT_DIM;
        v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
        v.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
        v.widgets.active.corner_radius = radius;

        v.widgets.open.bg_fill = BG_HOVER;
        v.widgets.open.weak_bg_fill = BG_HOVER;
        v.widgets.open.bg_stroke = Stroke::new(1.0, STROKE);
        v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.open.corner_radius = radius;

        style.visuals = v;
    });
}
