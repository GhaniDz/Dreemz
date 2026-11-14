//! Dreemz — a frame-by-frame 2D animation app.
//!
//! The binary is intentionally thin: everything interesting lives in the
//! `app`, `model`, `brush`, `paint`, `project` and `ui` modules.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod brush;
mod icons;
mod model;
mod paint;
mod project;
mod theme;
mod ui;

#[cfg(test)]
mod tests;

use eframe::egui;

use app::DreemzApp;

impl eframe::App for DreemzApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.now = ui.input(|i| i.time);
        self.status_text();
        self.handle_shortcuts(ui);
        self.playback_tick(ui.ctx());
        self.update_title(ui.ctx());

        egui::Panel::top("dreemz_menu").show(ui, |ui| {
            ui.add_space(2.0);
            self.ui_menu_bar(ui);
            ui.add_space(1.0);
        });

        egui::Panel::top("dreemz_quick").show(ui, |ui| {
            ui.add_space(3.0);
            self.ui_quick_bar(ui);
            ui.add_space(3.0);
        });

        egui::Panel::left("dreemz_tools")
            .exact_size(60.0)
            .show(ui, |ui| {
                self.ui_tool_rail(ui);
            });

        egui::Panel::right("dreemz_inspector")
            .default_size(280.0)
            .min_size(238.0)
            .max_size(400.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(4.0);
                self.ui_inspector(ui);
            });

        egui::Panel::bottom("dreemz_timeline")
            .default_size(276.0)
            .min_size(170.0)
            .max_size(500.0)
            .resizable(true)
            .show(ui, |ui| {
                ui.add_space(4.0);
                self.ui_timeline(ui);
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.ui_canvas(ui);
            });

        self.ui_dialogs(ui.ctx());
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1024.0, 680.0])
            .with_title("Dreemz"),
        ..Default::default()
    };
    eframe::run_native(
        "Dreemz",
        options,
        Box::new(|cc| Ok(Box::new(DreemzApp::new(cc)))),
    )
}
