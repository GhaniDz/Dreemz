//! Menu bar, quick toolbar, tool rail and the right-hand inspector.

use egui::{Color32, CornerRadius, FontId, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Vec2};

use crate::app::{DreemzApp, NewProjectForm, Tool};
use crate::brush::BrushPreset;
use crate::icons::{self, Icon};
use crate::model::BlendKind;
use crate::theme;
use crate::ui::{divider, icon_button};

const SWATCHES: [Color32; 16] = [
    Color32::from_rgb(16, 16, 18),
    Color32::from_rgb(72, 72, 78),
    Color32::from_rgb(146, 146, 150),
    Color32::WHITE,
    Color32::from_rgb(214, 72, 72),
    Color32::from_rgb(238, 138, 66),
    Color32::from_rgb(244, 205, 88),
    Color32::from_rgb(120, 196, 116),
    Color32::from_rgb(72, 186, 170),
    Color32::from_rgb(72, 132, 214),
    Color32::from_rgb(112, 100, 210),
    Color32::from_rgb(190, 104, 190),
    Color32::from_rgb(236, 168, 196),
    Color32::from_rgb(140, 96, 64),
    Color32::from_rgb(96, 64, 48),
    Color32::from_rgb(232, 224, 204),
];

enum LayerAction {
    Select(usize),
    ToggleVisible(usize),
    ToggleLock(usize),
    Rename(usize),
    Duplicate(usize),
    Copy(usize),
    Cut(usize),
    Paste(usize),
    MergeDown(usize),
    Delete(usize),
}

impl DreemzApp {
    // ------------------------------------------------------------- menu bar
    pub fn ui_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.label(
                RichText::new("DREEMZ")
                    .size(12.0)
                    .strong()
                    .color(theme::ACCENT),
            );
            ui.separator();

            ui.menu_button("File", |ui| {
                if ui.button("New project…   Ctrl+N").clicked() {
                    self.new_project = NewProjectForm::default();
                    self.dialogs.new_project = true;
                    ui.close();
                }
                if ui.button("Open project…   Ctrl+O").clicked() {
                    ui.close();
                    self.open_project_dialog();
                }
                if ui.button("Save   Ctrl+S").clicked() {
                    ui.close();
                    self.save_project(false);
                }
                if ui.button("Save as…   Ctrl+Shift+S").clicked() {
                    ui.close();
                    self.save_project(true);
                }
                ui.separator();
                if ui.button("Project settings…").clicked() {
                    ui.close();
                    self.open_settings_dialog();
                }
                ui.separator();
                if ui.button("Import image as layer…").clicked() {
                    ui.close();
                    self.import_image_dialog();
                }
                ui.separator();
                if ui.button("Export animation…   Ctrl+E").clicked() {
                    ui.close();
                    self.open_export_dialog();
                }
                if ui.button("Export current frame as PNG…").clicked() {
                    ui.close();
                    self.export_current_frame();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });

            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(self.history.can_undo(), egui::Button::new("Undo   Ctrl+Z"))
                    .clicked()
                {
                    self.undo();
                    ui.close();
                }
                if ui
                    .add_enabled(self.history.can_redo(), egui::Button::new("Redo   Ctrl+Y"))
                    .clicked()
                {
                    self.redo();
                    ui.close();
                }
                ui.separator();
                if ui.button("Copy drawing   Ctrl+C").clicked() {
                    self.copy_keyframe();
                    ui.close();
                }
                if ui.button("Cut drawing   Ctrl+X").clicked() {
                    self.cut_frame();
                    ui.close();
                }
                if ui.button("Paste drawing   Ctrl+V").clicked() {
                    self.paste_keyframe();
                    ui.close();
                }
                ui.separator();
                if ui.button("Flip horizontally   Ctrl+Shift+H").clicked() {
                    self.flip_drawing(true);
                    ui.close();
                }
                if ui.button("Flip vertically   Ctrl+Shift+V").clicked() {
                    self.flip_drawing(false);
                    ui.close();
                }
                ui.separator();
                if ui.button("Clear current drawing").clicked() {
                    self.clear_frame();
                    ui.close();
                }
                if ui.button("New keyframe (duplicate)").clicked() {
                    self.add_keyframe(true);
                    ui.close();
                }
                if ui.button("New empty keyframe").clicked() {
                    self.add_keyframe(false);
                    ui.close();
                }
                if ui.button("Duplicate keyframe to next free frame   Ctrl+D").clicked() {
                    self.duplicate_keyframe();
                    ui.close();
                }
                if ui.button("Delete keyframe").clicked() {
                    self.delete_keyframe();
                    ui.close();
                }
            });

            ui.menu_button("View", |ui| {
                if ui.button("Fit to view   Ctrl+0").clicked() {
                    self.needs_fit = true;
                    ui.close();
                }
                if ui.button("Actual size   Ctrl+1").clicked() {
                    self.view.zoom = 1.0;
                    self.view.pan = Vec2::ZERO;
                    ui.close();
                }
                if ui.button("Zoom in   Ctrl+2").clicked() {
                    self.view.zoom = (self.view.zoom * 1.25).min(32.0);
                    ui.close();
                }
                if ui.button("Zoom out   Ctrl+3").clicked() {
                    self.view.zoom = (self.view.zoom / 1.25).max(0.02);
                    ui.close();
                }
                if ui.button("Reset rotation").clicked() {
                    self.reset_view_rotation();
                    ui.close();
                }
                ui.separator();
                ui.checkbox(&mut self.onion.enabled, "Onion skin   O");
                ui.checkbox(&mut self.looping, "Loop playback   L");
                ui.separator();
                ui.checkbox(&mut self.guides.thirds, "Rule of thirds");
                ui.checkbox(&mut self.guides.center, "Centre guides");
                ui.checkbox(&mut self.guides.safe, "Safe area (10%)");
            });

            ui.menu_button("Layer", |ui| {
                if ui.button("New layer").clicked() {
                    self.add_layer();
                    ui.close();
                }
                if ui.button("Duplicate layer").clicked() {
                    self.duplicate_layer();
                    ui.close();
                }
                if ui.button("Copy layer").clicked() {
                    self.copy_layer();
                    ui.close();
                }
                if ui.button("Cut layer").clicked() {
                    self.cut_layer();
                    ui.close();
                }
                if ui.button("Paste layer").clicked() {
                    self.paste_layer();
                    ui.close();
                }
                if ui.button("Delete layer").clicked() {
                    self.delete_layer();
                    ui.close();
                }
                ui.separator();
                if ui.button("Move layer up").clicked() {
                    self.move_layer_up();
                    ui.close();
                }
                if ui.button("Move layer down").clicked() {
                    self.move_layer_down();
                    ui.close();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.doc.active_layer > 0,
                        egui::Button::new("Merge down"),
                    )
                    .clicked()
                {
                    self.merge_down();
                    ui.close();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("Keyboard shortcuts").clicked() {
                    self.dialogs.shortcuts = true;
                    ui.close();
                }
                if ui.button("About Dreemz").clicked() {
                    self.dialogs.about = true;
                    ui.close();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!(
                        "{} x {}  ·  {:.0} fps",
                        self.doc.width, self.doc.height, self.doc.fps
                    ))
                    .size(10.0)
                    .color(theme::TEXT_FAINT),
                );
            });
        });
    }

    // ---------------------------------------------------------- quick toolbar
    pub fn ui_quick_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(2.0);
            let mut title = self.doc.name.clone();
            if self.dirty {
                title.push_str(" *");
            }
            ui.label(RichText::new(title).size(13.0).strong().color(theme::TEXT));
            ui.separator();

            let can_undo = self.history.can_undo();
            if ui
                .add_enabled_ui(can_undo, |ui| {
                    icon_button(ui, Icon::Undo, 26.0, false, "Undo (Ctrl+Z)")
                })
                .inner
                .clicked()
            {
                self.undo();
            }
            let can_redo = self.history.can_redo();
            if ui
                .add_enabled_ui(can_redo, |ui| {
                    icon_button(ui, Icon::Redo, 26.0, false, "Redo (Ctrl+Y)")
                })
                .inner
                .clicked()
            {
                self.redo();
            }

            ui.separator();
            if icon_button(ui, Icon::Minus, 24.0, false, "Zoom out").clicked() {
                self.view.zoom = (self.view.zoom / 1.25).max(0.02);
            }
            ui.label(
                RichText::new(format!("{:.0}%", self.view.zoom * 100.0))
                    .monospace()
                    .size(11.0)
                    .color(theme::TEXT_DIM),
            );
            if icon_button(ui, Icon::Plus, 24.0, false, "Zoom in").clicked() {
                self.view.zoom = (self.view.zoom * 1.25).min(32.0);
            }
            if icon_button(ui, Icon::Fit, 24.0, false, "Fit to view (Ctrl+0)").clicked() {
                self.needs_fit = true;
            }
            if icon_button(
                ui,
                Icon::Rotate,
                24.0,
                (self.view.rotation).abs() > 0.001,
                "Reset canvas rotation",
            )
            .clicked()
            {
                self.view.rotation = 0.0;
            }

            ui.separator();
            if icon_button(
                ui,
                Icon::Onion,
                24.0,
                self.onion.enabled,
                "Onion skin (O)",
            )
            .clicked()
            {
                self.onion.enabled = !self.onion.enabled;
            }
            if icon_button(ui, Icon::Loop, 24.0, self.looping, "Loop (L)").clicked() {
                self.looping = !self.looping;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some((message, _)) = &self.status {
                    ui.label(
                        RichText::new(message)
                            .size(10.5)
                            .color(theme::ACCENT),
                    );
                } else {
                    ui.label(
                        RichText::new(self.tool.label())
                            .size(10.5)
                            .color(theme::TEXT_FAINT),
                    );
                }
                if icon_button(ui, Icon::Export, 24.0, false, "Export (Ctrl+E)").clicked() {
                    self.open_export_dialog();
                }
                if icon_button(ui, Icon::Save, 24.0, false, "Save (Ctrl+S)").clicked() {
                    self.save_project(false);
                }
                if icon_button(ui, Icon::Folder, 24.0, false, "Open project (Ctrl+O)").clicked() {
                    self.open_project_dialog();
                }
            });
        });
    }

    // -------------------------------------------------------------- tool rail
    pub fn ui_tool_rail(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(10.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(9), theme::ACCENT_DIM);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "R",
                FontId::proportional(19.0),
                theme::ACCENT,
            );
            ui.add_space(12.0);

            for tool in Tool::ALL {
                let tip = format!("{} ({})", tool.label(), tool.shortcut());
                if icon_button(ui, tool.icon(), 40.0, self.tool == tool, &tip).clicked() {
                    self.set_tool(tool);
                }
                ui.add_space(3.0);
            }

            ui.add_space(10.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::new(30.0, 1.0), Sense::hover());
            ui.painter().rect_filled(rect, 0, theme::STROKE);

            ui.add_space(10.0);
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(34.0), Sense::click());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(8), self.brush.color);
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(8),
                Stroke::new(1.0, theme::STROKE),
                StrokeKind::Inside,
            );
            if resp.hovered() {
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(8),
                    Stroke::new(1.0, theme::ACCENT),
                    StrokeKind::Inside,
                );
            }
            if resp
                .on_hover_text("Current colour — click to open the brush")
                .clicked()
            {
                self.set_tool(Tool::Brush);
            }
        });
    }

    // --------------------------------------------------------------- inspector
    pub fn ui_inspector(&mut self, ui: &mut egui::Ui) {
        let full_h = ui.available_height();
        let top_h = (full_h * 0.58).clamp(210.0, (full_h - 150.0).max(210.0));
        egui::ScrollArea::vertical()
            .id_salt("inspector_top")
            .max_height(top_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.brush_section(ui);
                ui.add_space(8.0);
                divider(ui);
                self.color_section(ui);
                ui.add_space(8.0);
                divider(ui);
                self.onion_section(ui);
            });
        ui.add_space(4.0);
        divider(ui);
        self.layers_section(ui);
    }

    fn brush_section(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("BRUSH").size(10.0).strong().color(theme::TEXT_FAINT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(self.tool.label())
                        .size(10.0)
                        .color(theme::ACCENT),
                );
            });
        });
        ui.add_space(4.0);
        let gap = 4.0;
        let cell = ((ui.available_width() - gap * 2.0) / 3.0).max(40.0);
        egui::Grid::new("brush_presets")
            .num_columns(3)
            .spacing(Vec2::splat(gap))
            .show(ui, |ui| {
                let count = BrushPreset::ALL.len();
                for (i, preset) in BrushPreset::ALL.into_iter().enumerate() {
                    let resp = ui.add_sized(
                        [cell, 22.0],
                        egui::Button::new(RichText::new(preset.label()).size(10.5)),
                    );
                    if resp.clicked() {
                        self.apply_preset(preset);
                    }
                    if i % 3 == 2 || i + 1 == count {
                        ui.end_row();
                    }
                }
            });
        ui.add_space(4.0);

        match self.tool {
            Tool::Fill => {
                let mut tolerance = self.fill_tolerance as i32;
                let text = format!("{tolerance}");
                let resp = control_row(ui, "Tolerance", &text, |ui| {
                    ui.add(egui::Slider::new(&mut tolerance, 0..=255).show_value(false))
                });
                if resp.changed() {
                    self.fill_tolerance = tolerance.clamp(0, 255) as u8;
                }
                ui.checkbox(
                    &mut self.fill_contiguous,
                    RichText::new("Contiguous region").size(11.0),
                );
                ui.label(
                    RichText::new(
                        "Click a colour region to paint it with the current colour.",
                    )
                    .size(10.5)
                    .color(theme::TEXT_FAINT),
                );
            }
            Tool::Eyedropper => {
                ui.label(
                    RichText::new("Click the canvas to sample a colour.")
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
            }
            Tool::Move => {
                ui.label(
                    RichText::new("Drag the current drawing to move its pixels.")
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
                ui.label(
                    RichText::new("Tip: moving on a held frame creates a keyframe.")
                        .size(10.5)
                        .color(theme::TEXT_FAINT),
                );
            }
            Tool::Select => {
                match self.selection {
                    Some(rect) => {
                        ui.label(
                            RichText::new(format!(
                                "Selection: {} x {} px at {}, {}",
                                rect.width(),
                                rect.height(),
                                rect.x0,
                                rect.y0
                            ))
                            .size(11.0)
                            .color(theme::TEXT_DIM),
                        );
                    }
                    None => {
                        ui.label(
                            RichText::new("Drag on the canvas to select a rectangle.")
                                .size(11.0)
                                .color(theme::TEXT_DIM),
                        );
                    }
                }
                ui.label(
                    RichText::new("Drag inside the selection to move its pixels.")
                        .size(10.5)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(2.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Select all").clicked() {
                        self.select_all();
                    }
                    if ui
                        .add_enabled(self.selection.is_some(), egui::Button::new("Deselect"))
                        .clicked()
                    {
                        self.deselect();
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(self.selection.is_some(), egui::Button::new("Copy"))
                        .clicked()
                    {
                        self.copy_selection();
                    }
                    if ui
                        .add_enabled(self.selection.is_some(), egui::Button::new("Cut"))
                        .clicked()
                    {
                        self.cut_selection();
                    }
                    if ui
                        .add_enabled(
                            self.region_clipboard.is_some(),
                            egui::Button::new("Paste"),
                        )
                        .clicked()
                    {
                        self.paste_selection();
                    }
                    if ui
                        .add_enabled(self.selection.is_some(), egui::Button::new("Clear"))
                        .clicked()
                    {
                        self.delete_selection();
                    }
                });
            }
            Tool::Hand => {
                ui.label(
                    RichText::new("Drag to pan. Hold Space with any tool to pan.")
                        .size(11.0)
                        .color(theme::TEXT_DIM),
                );
            }
            Tool::Brush | Tool::Eraser => {
                let eraser = self.tool == Tool::Eraser;
                let b = if eraser { &mut self.eraser } else { &mut self.brush };

                let text = format!("{:.0} px", b.size);
                let _ = control_row(ui, "Size", &text, |ui| {
                    ui.add(
                        egui::Slider::new(&mut b.size, 1.0..=400.0)
                            .logarithmic(true)
                            .show_value(false),
                    )
                });

                let mut opacity = (b.opacity * 100.0).round() as i32;
                let text = format!("{opacity} %");
                let resp = control_row(ui, "Opacity", &text, |ui| {
                    ui.add(egui::Slider::new(&mut opacity, 1..=100).show_value(false))
                });
                if resp.changed() {
                    b.opacity = opacity as f32 / 100.0;
                }

                let mut hardness = (b.hardness * 100.0).round() as i32;
                let text = format!("{hardness} %");
                let resp = control_row(ui, "Hardness", &text, |ui| {
                    ui.add(egui::Slider::new(&mut hardness, 0..=100).show_value(false))
                });
                if resp.changed() {
                    b.hardness = hardness as f32 / 100.0;
                }

                let mut spacing = (b.spacing * 100.0).round() as i32;
                let text = format!("{spacing} %");
                let resp = control_row(ui, "Spacing", &text, |ui| {
                    ui.add(egui::Slider::new(&mut spacing, 2..=200).show_value(false))
                });
                if resp.changed() {
                    b.spacing = spacing as f32 / 100.0;
                }

                let mut smoothing = (b.smoothing * 100.0).round() as i32;
                let text = format!("{smoothing} %");
                let resp = control_row(ui, "Smoothing", &text, |ui| {
                    ui.add(egui::Slider::new(&mut smoothing, 0..=92).show_value(false))
                });
                if resp.changed() {
                    b.smoothing = smoothing as f32 / 100.0;
                }

                let mut pressure = (b.pressure_size * 100.0).round() as i32;
                let text = format!("{pressure} %");
                let resp = control_row(ui, "Pressure", &text, |ui| {
                    ui.add(egui::Slider::new(&mut pressure, 0..=100).show_value(false))
                });
                if resp.changed() {
                    b.pressure_size = pressure as f32 / 100.0;
                }

                ui.add_space(2.0);
                ui.checkbox(
                    &mut b.simulate_pressure,
                    RichText::new("Simulate pressure from speed").size(11.0),
                );
                if b.simulate_pressure {
                    ui.label(
                        RichText::new("Mouse strokes get thinner when drawn quickly.")
                            .size(10.0)
                            .color(theme::TEXT_FAINT),
                    );
                }
                ui.add_space(4.0);
                let width = ui.available_width();
                if ui
                    .add_sized(
                        [width, 22.0],
                        egui::Button::new(RichText::new("Brush presets…").size(11.0)),
                    )
                    .clicked()
                {
                    self.dialogs.brush_presets = true;
                }
            }
        }
    }

    fn color_section(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("COLOUR")
                    .size(10.0)
                    .strong()
                    .color(theme::TEXT_FAINT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("Pick")
                    .on_hover_text("Eyedropper (I)")
                    .clicked()
                {
                    self.set_tool(Tool::Eyedropper);
                }
            });
        });
        ui.add_space(3.0);

        ui.horizontal(|ui| {
            let before = self.brush.color;
            ui.color_edit_button_srgba(&mut self.brush.color);
            if self.brush.color != before {
                let c = self.brush.color;
                self.remember_color(c);
            }
            ui.label(
                RichText::new(format!(
                    "#{:02X}{:02X}{:02X}",
                    self.brush.color.r(),
                    self.brush.color.g(),
                    self.brush.color.b()
                ))
                .monospace()
                .size(10.5)
                .color(theme::TEXT_DIM),
            );
        });

        ui.add_space(4.0);
        let mut clicked: Option<Color32> = None;
        egui::Grid::new("swatch_grid")
            .spacing(Vec2::splat(5.0))
            .show(ui, |ui| {
                for (i, c) in SWATCHES.iter().enumerate() {
                    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(19.0), Sense::click());
                    let selected = self.brush.color == *c;
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(4), *c);
                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius::same(4),
                        Stroke::new(
                            if selected { 1.5 } else { 1.0 },
                            if selected { theme::ACCENT } else { theme::STROKE },
                        ),
                        StrokeKind::Inside,
                    );
                    if resp.clicked() {
                        clicked = Some(*c);
                    }
                    if i % 8 == 7 {
                        ui.end_row();
                    }
                }
            });
        if let Some(c) = clicked {
            self.brush.color = c;
            self.remember_color(c);
        }

        ui.add_space(4.0);
        ui.label(RichText::new("RECENT").size(9.5).color(theme::TEXT_FAINT));
        let recent = self.recent_colors.clone();
        ui.horizontal_wrapped(|ui| {
            for c in recent.iter().take(8) {
                let (rect, resp) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::click());
                ui.painter().rect_filled(rect, CornerRadius::same(3), *c);
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(3),
                    Stroke::new(1.0, theme::STROKE),
                    StrokeKind::Inside,
                );
                if resp.clicked() {
                    self.brush.color = *c;
                }
            }
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(LABEL_W, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new("PAPER").size(9.5).color(theme::TEXT_FAINT));
                },
            );
            let mut bg = self.main_background();
            let before = bg;
            ui.color_edit_button_srgba(&mut bg);
            if bg != before {
                self.doc.background = [bg.r(), bg.g(), bg.b(), 255];
                self.dirty = true;
                self.invalidate();
            }
            ui.label(
                RichText::new(format!("#{:02X}{:02X}{:02X}", bg.r(), bg.g(), bg.b()))
                    .monospace()
                    .size(10.0)
                    .color(theme::TEXT_FAINT),
            );
        });
    }

    fn onion_section(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("ONION SKIN")
                    .size(10.0)
                    .strong()
                    .color(theme::TEXT_FAINT),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut self.onion.enabled, "");
            });
        });
        ui.add_space(3.0);

        let mut before = self.onion.before as i32;
        let text = format!("{before}");
        let resp = control_row(ui, "Before", &text, |ui| {
            ui.add(egui::Slider::new(&mut before, 0..=5).show_value(false))
        });
        if resp.changed() {
            self.onion.before = before.max(0) as u32;
        }

        let mut after = self.onion.after as i32;
        let text = format!("{after}");
        let resp = control_row(ui, "After", &text, |ui| {
            ui.add(egui::Slider::new(&mut after, 0..=5).show_value(false))
        });
        if resp.changed() {
            self.onion.after = after.max(0) as u32;
        }

        let mut opacity = (self.onion.opacity * 100.0).round() as i32;
        let text = format!("{opacity} %");
        let resp = control_row(ui, "Opacity", &text, |ui| {
            ui.add(egui::Slider::new(&mut opacity, 5..=100).show_value(false))
        });
        if resp.changed() {
            self.onion.opacity = opacity as f32 / 100.0;
        }

        ui.add_space(2.0);
        ui.label(
            RichText::new("Previous frames are tinted red, following frames blue.")
                .size(10.0)
                .color(theme::TEXT_FAINT),
        );
    }

    fn remember_color(&mut self, c: Color32) {
        self.recent_colors.retain(|x| *x != c);
        self.recent_colors.insert(0, c);
        self.recent_colors.truncate(12);
    }

    fn layers_section(&mut self, ui: &mut Ui) {
        let mut actions: Vec<LayerAction> = Vec::new();
        ui.horizontal(|ui| {
            ui.label(RichText::new("LAYERS").size(10.5).strong().color(theme::TEXT_FAINT));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icon_button(ui, Icon::Trash, 22.0, false, "Delete layer").clicked() {
                    self.delete_layer();
                }
                if icon_button(ui, Icon::ArrowDown, 22.0, false, "Move layer down").clicked() {
                    self.move_layer_down();
                }
                if icon_button(ui, Icon::ArrowUp, 22.0, false, "Move layer up").clicked() {
                    self.move_layer_up();
                }
                if icon_button(ui, Icon::Layers, 22.0, false, "Merge down").clicked() {
                    self.merge_down();
                }
                if icon_button(ui, Icon::Copy, 22.0, false, "Duplicate layer").clicked() {
                    self.duplicate_layer();
                }
                if icon_button(ui, Icon::Plus, 22.0, false, "New layer").clicked() {
                    self.add_layer();
                }
            });
        });

        // Blend mode and opacity for the active layer.
        let active = self.doc.active_layer.min(self.doc.layers.len() - 1);
        let current_blend = self.doc.layers[active].blend;
        let mut blend = current_blend;
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(LABEL_W, 20.0),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.label(RichText::new("BLEND").size(9.5).color(theme::TEXT_FAINT));
                },
            );
            let gap = ui.spacing().item_spacing.x;
            let width = (ui.available_width() - gap).max(60.0);
            egui::ComboBox::from_id_salt("layer_blend")
                .width(width)
                .selected_text(current_blend.label())
                .show_ui(ui, |ui| {
                    for kind in BlendKind::ALL {
                        ui.selectable_value(&mut blend, kind, kind.label());
                    }
                });
        });
        if blend != current_blend {
            self.begin_edit();
            self.doc.layers[active].blend = blend;
            self.invalidate();
        }

        let mut opacity = (self.doc.layers[active].opacity * 100.0).round() as i32;
        let text = format!("{opacity} %");
        let opacity_resp = control_row(ui, "Opacity", &text, |ui| {
            ui.add(egui::Slider::new(&mut opacity, 0..=100).show_value(false))
        });
        if opacity_resp.drag_started() {
            self.history.begin_doc(&self.doc);
        }
        if opacity_resp.changed() {
            self.doc.layers[active].opacity = opacity as f32 / 100.0;
            self.dirty = true;
            self.invalidate();
        }
        ui.add_space(2.0);

        let layer_count = self.doc.layers.len();
        let list_h = (ui.available_height() - 4.0).max(70.0);
        egui::ScrollArea::vertical()
            .id_salt("layer_list")
            .max_height(list_h)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = ui.available_width();
                for idx in (0..layer_count).rev() {
                    let thumb = self.layer_thumbnail(ui.ctx(), idx);
                    let (rect, resp) =
                        ui.allocate_exact_size(Vec2::new(width, 36.0), Sense::click());
                    let active = idx == self.doc.active_layer;
                    let layer = &self.doc.layers[idx];
                    let bg = if active {
                        Color32::from_rgb(42, 37, 28)
                    } else if resp.hovered() {
                        theme::BG_HOVER
                    } else {
                        Color32::TRANSPARENT
                    };
                    if bg != Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
                    }
                    if active {
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(4),
                            Stroke::new(1.0, theme::ACCENT_DIM),
                            StrokeKind::Inside,
                        );
                    }

                    let eye_rect = Rect::from_min_size(
                        rect.left_top() + Vec2::new(5.0, 9.0),
                        Vec2::splat(18.0),
                    );
                    icons::paint_centered(
                        ui.painter(),
                        if layer.visible { Icon::Eye } else { Icon::EyeOff },
                        eye_rect,
                        if layer.visible {
                            theme::TEXT_DIM
                        } else {
                            theme::TEXT_FAINT
                        },
                    );

                    let thumb_rect = Rect::from_min_size(
                        rect.left_top() + Vec2::new(27.0, 6.0),
                        Vec2::new(40.0, 24.0),
                    );
                    if let Some(texture) = thumb {
                        ui.painter().image(
                            texture,
                            thumb_rect,
                            Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    } else {
                        ui.painter()
                            .rect_filled(thumb_rect, CornerRadius::same(3), theme::BG_DEEP);
                    }
                    ui.painter().rect_stroke(
                        thumb_rect,
                        CornerRadius::same(3),
                        Stroke::new(1.0, theme::STROKE_SOFT),
                        StrokeKind::Inside,
                    );

                    let label_x = 74.0;
                    let lock_rect = Rect::from_min_size(
                        rect.right_top() + Vec2::new(-74.0, 11.0),
                        Vec2::splat(16.0),
                    );
                    icons::paint_centered(
                        ui.painter(),
                        if layer.locked { Icon::Lock } else { Icon::Unlock },
                        lock_rect,
                        if layer.locked {
                            theme::ACCENT
                        } else {
                            theme::TEXT_FAINT
                        },
                    );

                    let name_color = if !layer.visible {
                        theme::TEXT_FAINT
                    } else if active {
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    };
                    let mut name = layer.name.clone();
                    let max_chars = ((rect.width() - label_x - 86.0) / 6.2).max(3.0) as usize;
                    if name.chars().count() > max_chars {
                        name = name.chars().take(max_chars).collect::<String>() + "…";
                    }
                    ui.painter().text(
                        rect.left_center() + Vec2::new(label_x, 0.0),
                        egui::Align2::LEFT_CENTER,
                        name,
                        FontId::proportional(11.5),
                        name_color,
                    );
                    ui.painter().text(
                        rect.right_center() + Vec2::new(-8.0, 0.0),
                        egui::Align2::RIGHT_CENTER,
                        format!("{:.0}%", layer.opacity * 100.0),
                        FontId::monospace(9.5),
                        theme::TEXT_FAINT,
                    );

                    if resp.double_clicked() {
                        actions.push(LayerAction::Rename(idx));
                    } else if resp.clicked() {
                        if let Some(p) = resp.interact_pointer_pos() {
                            if eye_rect.contains(p) {
                                actions.push(LayerAction::ToggleVisible(idx));
                            } else if lock_rect.contains(p) {
                                actions.push(LayerAction::ToggleLock(idx));
                            } else {
                                actions.push(LayerAction::Select(idx));
                            }
                        }
                    }

                    resp.context_menu(|ui| {
                        if ui.button("Rename layer").clicked() {
                            actions.push(LayerAction::Rename(idx));
                            ui.close();
                        }
                        if ui.button("Duplicate layer").clicked() {
                            actions.push(LayerAction::Duplicate(idx));
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Copy layer").clicked() {
                            actions.push(LayerAction::Copy(idx));
                            ui.close();
                        }
                        if ui.button("Cut layer").clicked() {
                            actions.push(LayerAction::Cut(idx));
                            ui.close();
                        }
                        if ui.button("Paste layer").clicked() {
                            actions.push(LayerAction::Paste(idx));
                            ui.close();
                        }
                        ui.separator();
                        if ui
                            .add_enabled(idx > 0, egui::Button::new("Merge down"))
                            .clicked()
                        {
                            actions.push(LayerAction::MergeDown(idx));
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Delete layer").clicked() {
                            actions.push(LayerAction::Delete(idx));
                            ui.close();
                        }
                    });
                    ui.add_space(2.0);
                }
            });

        for action in actions {
            match action {
                LayerAction::Select(i) => {
                    self.doc.active_layer = i;
                    self.invalidate();
                }
                LayerAction::ToggleVisible(i) => {
                    self.doc_edit_bool(|doc| {
                        if let Some(layer) = doc.layers.get_mut(i) {
                            layer.visible = !layer.visible;
                            true
                        } else {
                            false
                        }
                    });
                }
                LayerAction::ToggleLock(i) => {
                    self.doc_edit_bool(|doc| {
                        if let Some(layer) = doc.layers.get_mut(i) {
                            layer.locked = !layer.locked;
                            true
                        } else {
                            false
                        }
                    });
                }
                LayerAction::Rename(i) => {
                    self.layer_name_edit = self.doc.layers[i].name.clone();
                    self.renaming_layer = Some(i);
                }
                LayerAction::Duplicate(i) => {
                    self.doc.active_layer = i;
                    self.duplicate_layer();
                }
                LayerAction::Copy(i) => {
                    self.doc.active_layer = i;
                    self.copy_layer();
                }
                LayerAction::Cut(i) => {
                    self.doc.active_layer = i;
                    self.cut_layer();
                }
                LayerAction::Paste(i) => {
                    self.doc.active_layer = i;
                    self.paste_layer();
                }
                LayerAction::MergeDown(i) => {
                    self.doc.active_layer = i;
                    self.merge_down();
                }
                LayerAction::Delete(i) => {
                    self.doc.active_layer = i;
                    self.delete_layer();
                }
            }
        }

        if self.layer_thumbs.len() > 64 {
            let live: std::collections::HashSet<u64> = self
                .doc
                .layers
                .iter()
                .flat_map(|l| l.frames.values().map(|d| d.id))
                .collect();
            self.layer_thumbs.retain(|id, _| live.contains(id));
        }

        ui.add_space(2.0);
        ui.horizontal(|ui| {
            if icon_button(ui, Icon::Export, 22.0, false, "Export animation (Ctrl+E)").clicked() {
                self.open_export_dialog();
            }
            ui.label(
                RichText::new(format!(
                    "{} layer{}  ·  {}×{}",
                    layer_count,
                    if layer_count == 1 { "" } else { "s" },
                    self.doc.width,
                    self.doc.height
                ))
                .size(10.0)
                .color(theme::TEXT_FAINT),
            );
        });
    }
}

/// Width of the label column in inspector rows.
const LABEL_W: f32 = 58.0;
/// Width of the numeric read-out column in inspector rows.
const VALUE_W: f32 = 46.0;

/// One consistently aligned inspector row: label, slider, value read-out.
fn control_row(
    ui: &mut Ui,
    label: &str,
    value: &str,
    add: impl FnOnce(&mut Ui) -> egui::Response,
) -> egui::Response {
    let gap = ui.spacing().item_spacing.x;
    let slider_w = (ui.available_width() - LABEL_W - VALUE_W - gap * 2.0 - 6.0).max(36.0);
    ui.horizontal(|ui| {
        ui.allocate_ui_with_layout(
            Vec2::new(LABEL_W, 18.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(9.5).color(theme::TEXT_FAINT));
            },
        );
        ui.spacing_mut().slider_width = slider_w;
        let resp = add(ui);
        ui.allocate_ui_with_layout(
            Vec2::new(VALUE_W, 18.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.label(
                    RichText::new(value)
                        .monospace()
                        .size(9.5)
                        .color(theme::TEXT_DIM),
                );
            },
        );
        resp
    })
    .inner
}
