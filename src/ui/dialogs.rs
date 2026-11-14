//! Application dialogs: new project, export, layer rename, brush presets,
//! keyboard shortcuts and about.

use egui::{Grid, Key, RichText, Vec2};

use crate::app::{DreemzApp, NewProjectForm};
use crate::brush::BrushPreset;
use crate::project::ExportFormat;
use crate::theme;
use crate::ui::divider;

impl DreemzApp {
    pub fn ui_dialogs(&mut self, ctx: &egui::Context) {
        self.dialog_new_project(ctx);
        self.dialog_project_settings(ctx);
        self.dialog_export(ctx);
        self.dialog_rename_layer(ctx);
        self.dialog_brush_presets(ctx);
        self.dialog_shortcuts(ctx);
        self.dialog_about(ctx);
    }

    // ---------------------------------------------------------- new project

    fn dialog_new_project(&mut self, ctx: &egui::Context) {
        if !self.dialogs.new_project {
            return;
        }
        let mut open = true;
        let mut create = false;
        let mut close = false;

        egui::Window::new("New project")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label(
                    RichText::new("Start a fresh animation")
                        .size(11.0)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(6.0);

                Grid::new("new_project_grid")
                    .num_columns(2)
                    .spacing(Vec2::new(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_project.name)
                                .desired_width(220.0),
                        );
                        ui.end_row();

                        ui.label("Size");
                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                for (label, w, h) in NewProjectForm::PRESETS {
                                    let selected =
                                        self.new_project.width == w && self.new_project.height == h;
                                    if ui
                                        .selectable_label(selected, RichText::new(label).size(10.5))
                                        .clicked()
                                    {
                                        self.new_project.width = w;
                                        self.new_project.height = h;
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.new_project.width)
                                        .range(64..=8192)
                                        .speed(1.0)
                                        .suffix(" px"),
                                );
                                ui.label(RichText::new("×").color(theme::TEXT_FAINT));
                                ui.add(
                                    egui::DragValue::new(&mut self.new_project.height)
                                        .range(64..=8192)
                                        .speed(1.0)
                                        .suffix(" px"),
                                );
                            });
                        });
                        ui.end_row();

                        ui.label("Timing");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.new_project.fps)
                                    .range(1.0..=120.0)
                                    .speed(0.25)
                                    .suffix(" fps"),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.new_project.frames)
                                    .range(1..=10_000)
                                    .speed(0.5)
                                    .suffix(" frames"),
                            );
                        });
                        ui.end_row();

                        ui.label("Paper");
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgba(&mut self.new_project.background);
                            ui.label(
                                RichText::new("Background colour")
                                    .size(11.0)
                                    .color(theme::TEXT_DIM),
                            );
                        });
                        ui.end_row();

                        ui.label("Duration");
                        let seconds =
                            self.new_project.frames as f32 / self.new_project.fps.max(1.0);
                        ui.label(
                            RichText::new(format!("{seconds:.2} seconds"))
                                .size(11.0)
                                .color(theme::TEXT_DIM),
                        );
                        ui.end_row();
                    });

                ui.add_space(10.0);
                divider(ui);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(
                                RichText::new("Create project").strong(),
                            ))
                            .clicked()
                        {
                            create = true;
                        }
                    });
                });
            });

        if create {
            if self.new_project.name.trim().is_empty() {
                self.new_project.name = "Untitled".into();
            }
            let name = self.new_project.name.trim().to_string();
            self.new_document();
            self.notify(format!("Created '{name}'"));
            close = true;
        }
        if close {
            open = false;
        }
        self.dialogs.new_project = open;
    }

    // ------------------------------------------------------ project settings

    fn dialog_project_settings(&mut self, ctx: &egui::Context) {
        if !self.dialogs.settings {
            return;
        }
        let mut open = true;
        let mut apply = false;
        let mut close = false;

        egui::Window::new("Project settings")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                ui.label(
                    RichText::new("Canvas, timing and paper")
                        .size(11.0)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(6.0);

                Grid::new("project_settings_grid")
                    .num_columns(2)
                    .spacing(Vec2::new(12.0, 8.0))
                    .show(ui, |ui| {
                        ui.label("Name");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.settings_form.name)
                                .desired_width(230.0),
                        );
                        ui.end_row();

                        ui.label("Canvas");
                        ui.vertical(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                for (label, w, h) in NewProjectForm::PRESETS {
                                    let selected = self.settings_form.width == w
                                        && self.settings_form.height == h;
                                    if ui
                                        .selectable_label(selected, RichText::new(label).size(10.5))
                                        .clicked()
                                    {
                                        self.settings_form.width = w;
                                        self.settings_form.height = h;
                                    }
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::DragValue::new(&mut self.settings_form.width)
                                        .range(16..=8192)
                                        .speed(1.0)
                                        .suffix(" px"),
                                );
                                ui.label(RichText::new("×").color(theme::TEXT_FAINT));
                                ui.add(
                                    egui::DragValue::new(&mut self.settings_form.height)
                                        .range(16..=8192)
                                        .speed(1.0)
                                        .suffix(" px"),
                                );
                            });
                        });
                        ui.end_row();

                        ui.label("Timing");
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.settings_form.fps)
                                    .range(1.0..=120.0)
                                    .speed(0.25)
                                    .suffix(" fps"),
                            );
                            ui.add(
                                egui::DragValue::new(&mut self.settings_form.frames)
                                    .range(1..=100_000)
                                    .speed(1.0)
                                    .suffix(" frames"),
                            );
                        });
                        ui.end_row();

                        ui.label("Paper");
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgba(&mut self.settings_form.background);
                            ui.label(
                                RichText::new("Background colour")
                                    .size(11.0)
                                    .color(theme::TEXT_DIM),
                            );
                        });
                        ui.end_row();

                        ui.label("Duration");
                        let seconds = self.settings_form.frames as f32
                            / self.settings_form.fps.max(1.0);
                        ui.label(
                            RichText::new(format!("{seconds:.2} seconds"))
                                .size(11.0)
                                .color(theme::TEXT_DIM),
                        );
                        ui.end_row();
                    });

                ui.add_space(4.0);
                ui.label(
                    RichText::new("Changing the canvas size resamples every drawing.")
                        .size(10.5)
                        .color(theme::TEXT_FAINT),
                );

                ui.add_space(10.0);
                divider(ui);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("Apply").strong()))
                            .clicked()
                        {
                            apply = true;
                        }
                    });
                });
            });

        if apply {
            self.apply_settings();
            close = true;
        }
        if close {
            open = false;
        }
        self.dialogs.settings = open;
    }

    // --------------------------------------------------------------- export

    fn dialog_export(&mut self, ctx: &egui::Context) {
        if !self.dialogs.export {
            return;
        }
        let mut open = true;
        let mut run = false;
        let mut browse = false;
        let mut close = false;
        let last_frame = self.doc.frame_count.saturating_sub(1) as i32;

        egui::Window::new("Export animation")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                ui.label(
                    RichText::new(format!(
                        "{} × {}  ·  {:.0} fps  ·  {} frames",
                        self.doc.width,
                        self.doc.height,
                        self.doc.fps,
                        self.doc.frame_count
                    ))
                    .size(11.0)
                    .color(theme::TEXT_FAINT),
                );
                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    ui.label("Format");
                    let current = self.export.format;
                    let mut format = current;
                    egui::ComboBox::from_id_salt("export_format")
                        .width(240.0)
                        .selected_text(current.label())
                        .show_ui(ui, |ui| {
                            for option in ExportFormat::ALL {
                                ui.selectable_value(&mut format, option, option.label());
                            }
                        });
                    if format != current {
                        self.export.format = format;
                        self.export.result = None;
                        self.export.sync_path_extension();
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Path");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.export.path)
                            .desired_width(300.0)
                            .hint_text(match self.export.format {
                                ExportFormat::PngSequence => "Folder for the PNG sequence",
                                ExportFormat::Gif => "Animation.gif",
                                ExportFormat::Mp4 => "Animation.mp4",
                                ExportFormat::SpriteSheet => "SpriteSheet.png",
                            }),
                    );
                    if ui.button("Browse…").clicked() {
                        browse = true;
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("Frames");
                    ui.add(
                        egui::DragValue::new(&mut self.export.start)
                            .range(0..=last_frame)
                            .speed(0.3),
                    );
                    ui.label(RichText::new("to").color(theme::TEXT_FAINT));
                    ui.add(
                        egui::DragValue::new(&mut self.export.end)
                            .range(0..=last_frame)
                            .speed(0.3),
                    );
                    ui.label(
                        RichText::new("(inclusive, 1-based in the timeline)")
                            .size(10.0)
                            .color(theme::TEXT_FAINT),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("Scale");
                    ui.add(
                        egui::Slider::new(&mut self.export.scale, 0.25..=4.0)
                            .logarithmic(true)
                            .suffix("×"),
                    );
                    ui.checkbox(&mut self.export.background, "Include paper background");
                });

                if self.export.format == ExportFormat::SpriteSheet {
                    ui.horizontal(|ui| {
                        ui.label("Columns");
                        ui.add(
                            egui::DragValue::new(&mut self.export.columns)
                                .range(1..=64)
                                .speed(0.1),
                        );
                        let count = (self.export.end - self.export.start).abs() + 1;
                        let columns = self.export.columns.max(1);
                        let rows = (count + columns - 1) / columns;
                        ui.label(
                            RichText::new(format!("{columns} x {rows} cells"))
                                .size(10.0)
                                .color(theme::TEXT_FAINT),
                        );
                    });
                }

                match self.export.format {
                    ExportFormat::PngSequence => {
                        ui.label(
                            RichText::new(
                                "Writes one PNG per frame into the chosen folder. \
                                 Transparent when the background is excluded.",
                            )
                            .size(10.5)
                            .color(theme::TEXT_FAINT),
                        );
                    }
                    ExportFormat::Gif => {
                        ui.label(
                            RichText::new(
                                "Animated GIF loops forever. Limited to 256 colours per frame.",
                            )
                            .size(10.5)
                            .color(theme::TEXT_FAINT),
                        );
                    }
                    ExportFormat::Mp4 => {
                        ui.label(
                            RichText::new(
                                "Requires ffmpeg on PATH. Encodes H.264 at the project frame rate.",
                            )
                            .size(10.5)
                            .color(theme::TEXT_FAINT),
                        );
                    }
                    ExportFormat::SpriteSheet => {
                        ui.label(
                            RichText::new(
                                "Lays every exported frame out in a grid on one PNG. \
                                 Rows are calculated from the column count.",
                            )
                            .size(10.5)
                            .color(theme::TEXT_FAINT),
                        );
                    }
                }

                if let Some(result) = &self.export.result {
                    ui.add_space(4.0);
                    let (color, text) = if result.starts_with("Export failed") {
                        (theme::DANGER, result.as_str())
                    } else {
                        (theme::ACCENT, result.as_str())
                    };
                    ui.label(RichText::new(text).size(11.0).color(color));
                }

                ui.add_space(8.0);
                divider(ui);
                ui.horizontal(|ui| {
                    if ui.button("Close").clicked() {
                        close = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new(RichText::new("Export").strong()))
                            .clicked()
                        {
                            run = true;
                        }
                    });
                });
            });

        if browse {
            self.browse_export_path();
        }
        if run {
            self.run_export();
        }
        if close {
            open = false;
        }
        self.dialogs.export = open;
    }

    fn browse_export_path(&mut self) {
        match self.export.format {
            ExportFormat::PngSequence => {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_title("Choose a folder for the PNG sequence")
                    .pick_folder()
                {
                    self.export.path = dir.to_string_lossy().to_string();
                }
            }
            ExportFormat::Gif | ExportFormat::Mp4 | ExportFormat::SpriteSheet => {
                let ext = self.export.format.extension();
                let mut dialog = rfd::FileDialog::new()
                    .add_filter("Output", &[ext])
                    .set_title("Export animation to…");
                if let Some(name) = std::path::Path::new(&self.export.path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                {
                    dialog = dialog.set_file_name(name);
                }
                if let Some(path) = dialog.save_file() {
                    self.export.path = path.to_string_lossy().to_string();
                    self.export.sync_path_extension();
                }
            }
        }
    }

    // ---------------------------------------------------------- rename layer

    fn dialog_rename_layer(&mut self, ctx: &egui::Context) {
        let Some(index) = self.renaming_layer else {
            return;
        };
        if index >= self.doc.layers.len() {
            self.renaming_layer = None;
            return;
        }

        let mut open = true;
        let mut commit = false;
        let mut cancel = false;

        egui::Window::new("Rename layer")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(280.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.layer_name_edit)
                        .desired_width(248.0)
                        .hint_text("Layer name"),
                );
                response.request_focus();
                if ui.input(|i| i.key_pressed(Key::Enter)) {
                    commit = true;
                }
                if ui.input(|i| i.key_pressed(Key::Escape)) {
                    cancel = true;
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        cancel = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Rename").clicked() {
                            commit = true;
                        }
                    });
                });
            });

        if commit {
            let trimmed = self.layer_name_edit.trim().to_string();
            if !trimmed.is_empty() {
                let fallback = self.doc.layers[index].name.clone();
                let name = if trimmed.is_empty() { fallback } else { trimmed };
                self.doc_edit(|doc| {
                    if let Some(layer) = doc.layers.get_mut(index) {
                        layer.name = name;
                    }
                });
            }
            self.renaming_layer = None;
        } else if cancel || !open {
            self.renaming_layer = None;
        }
    }

    // --------------------------------------------------------- brush presets

    fn dialog_brush_presets(&mut self, ctx: &egui::Context) {
        if !self.dialogs.brush_presets {
            return;
        }
        let mut open = true;
        let mut chosen: Option<BrushPreset> = None;
        egui::Window::new("Brush presets")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(280.0);
                ui.label(
                    RichText::new("Pick a starting point, then fine-tune in the inspector.")
                        .size(11.0)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(6.0);
                for preset in BrushPreset::ALL {
                    let selected =
                        self.brush.size == preset_size(preset) && self.tool != crate::app::Tool::Eraser;
                    if ui
                        .add_sized(
                            [ui.available_width(), 26.0],
                            egui::Button::new(RichText::new(preset.label()).size(12.0))
                                .selected(selected),
                        )
                        .clicked()
                    {
                        chosen = Some(preset);
                    }
                }
            });
        if let Some(preset) = chosen {
            self.apply_preset(preset);
        }
        self.dialogs.brush_presets = open;
    }

    // ------------------------------------------------------------- shortcuts

    fn dialog_shortcuts(&mut self, ctx: &egui::Context) {
        if !self.dialogs.shortcuts {
            return;
        }
        let mut open = true;
        egui::Window::new("Keyboard shortcuts")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                ui.label(
                    RichText::new("Everything is reachable without leaving the canvas.")
                        .size(11.0)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(6.0);

                let groups: &[(&str, &[(&str, &str)])] = &[
                    (
                        "Tools",
                        &[
                            ("B", "Brush"),
                            ("E", "Eraser"),
                            ("G", "Fill"),
                            ("I", "Eyedropper"),
                            ("V", "Move pixels"),
                            ("M", "Rectangular selection"),
                            ("H", "Hand / pan"),
                        ],
                    ),
                    (
                        "Playback",
                        &[
                            ("Space", "Play / pause"),
                            ("P", "Perform mode — record strokes onto the timeline over time"),
                            ("← / →", "Previous / next frame"),
                            ("O", "Toggle onion skin"),
                            ("L", "Toggle loop"),
                        ],
                    ),
                    (
                        "Editing",
                        &[
                            ("[ / ]", "Decrease / increase brush size"),
                            ("Ctrl+A", "Select all"),
                            ("Esc", "Deselect"),
                            ("Ctrl+C / Ctrl+X", "Copy / cut selection or drawing"),
                            ("Ctrl+V", "Paste selection or drawing"),
                            ("Ctrl+Shift+H / V", "Flip drawing horizontally / vertically"),
                            ("Ctrl+D", "Duplicate keyframe"),
                            ("Delete", "Delete selection or keyframe"),
                            ("Ctrl+Z", "Undo"),
                            ("Ctrl+Y / Ctrl+Shift+Z", "Redo"),
                        ],
                    ),
                    (
                        "Files",
                        &[
                            ("Ctrl+N", "New project"),
                            ("Ctrl+O", "Open project"),
                            ("Ctrl+S", "Save"),
                            ("Ctrl+Shift+S", "Save as"),
                            ("Ctrl+E", "Export"),
                        ],
                    ),
                    (
                        "View",
                        &[
                            ("Ctrl+0", "Fit canvas"),
                            ("Ctrl+1", "Actual size"),
                            ("Ctrl+2 / Ctrl+3", "Zoom in / out"),
                            ("Wheel", "Zoom canvas"),
                            ("Shift+Wheel", "Rotate canvas"),
                            ("Middle-drag / Hand tool", "Pan canvas"),
                            ("Ctrl+Wheel (timeline)", "Zoom timeline"),
                        ],
                    ),
                ];

                for (title, rows) in groups {
                    ui.label(RichText::new(*title).size(10.5).strong().color(theme::ACCENT));
                    Grid::new(("shortcuts", *title))
                        .num_columns(2)
                        .spacing(Vec2::new(18.0, 3.0))
                        .show(ui, |ui| {
                            for (keys, description) in rows.iter() {
                                ui.label(
                                    RichText::new(*keys)
                                        .monospace()
                                        .size(11.0)
                                        .color(theme::TEXT),
                                );
                                ui.label(
                                    RichText::new(*description)
                                        .size(11.0)
                                        .color(theme::TEXT_DIM),
                                );
                                ui.end_row();
                            }
                        });
                    ui.add_space(8.0);
                }
            });
        self.dialogs.shortcuts = open;
    }

    // ----------------------------------------------------------------- about

    fn dialog_about(&mut self, ctx: &egui::Context) {
        if !self.dialogs.about {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new("About Dreemz")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label(
                    RichText::new("DREEMZ")
                        .size(22.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                ui.label(
                    RichText::new(format!("Version {}", env!("CARGO_PKG_VERSION")))
                        .size(11.0)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "A frame-by-frame 2D animation studio for Windows and Linux, \
                         drawn with Rust and egui.",
                    )
                    .size(12.0)
                    .color(theme::TEXT),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(
                        "• Pressure-aware brush engine with pencils, ink, markers, \
                         airbrush and soft brushes\n\
                         • Perform mode records strokes onto successive frames as you draw\n\
                         • Procreate-style track lanes with draggable drawing clips\n\
                         • Layers with blend modes, opacity, thumbnails and merge down\n\
                         • Rectangular selection with move, copy, cut and paste\n\
                         • Keyframe timeline with holds, dragging and onion skinning\n\
                         • Composition guides and a rotatable, zoomable canvas\n\
                         • PNG sequence, animated GIF, sprite sheet and MP4 export\n\
                         • Native .dreemz project files with undo history",
                    )
                    .size(11.0)
                    .color(theme::TEXT_DIM),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Built with eframe, egui and tiny-skia.")
                        .size(10.5)
                        .color(theme::TEXT_FAINT),
                );
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
            });
        if close {
            open = false;
        }
        self.dialogs.about = open;
    }
}

fn preset_size(preset: BrushPreset) -> f32 {
    match preset {
        BrushPreset::Pencil => 8.0,
        BrushPreset::Ink => 16.0,
        BrushPreset::Marker => 32.0,
        BrushPreset::Airbrush => 72.0,
        BrushPreset::Soft => 40.0,
    }
}
