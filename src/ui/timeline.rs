//! Timeline panel: transport, ruler and track lanes with drawing clips.
//!
//! The lane layout follows the Procreate Dreams timeline: one rounded track per
//! layer with a thumbnail header, and drawings shown as rounded clips that span
//! their hold. The playhead is a frame-width band across every track.

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, CornerRadius, FontId, PointerButton, Pos2, Rect, Sense, Stroke, StrokeKind, Vec2};
use tiny_skia::Pixmap;

use crate::app::{DreemzApp, KeyframeDrag};
use crate::icons::{self, Icon};
use crate::theme;
use crate::ui::icon_button;

/// One drawing clip, collected before painting so thumbnails can be built
/// without borrowing the document while it is drawn.
struct Clip {
    layer: usize,
    start: u32,
    end: u32,
    drawing_id: u64,
    version: u64,
    pixmap: Arc<Pixmap>,
    visible: bool,
    active: bool,
}

impl DreemzApp {
    pub fn ui_timeline(&mut self, ui: &mut egui::Ui) {
        self.ui_transport(ui);
        ui.add_space(4.0);
        self.ui_timeline_body(ui);
    }

    fn ui_transport(&mut self, ui: &mut egui::Ui) {
        ui.spacing_mut().item_spacing.x = 5.0;
        ui.horizontal(|ui| {
            let btn = 26.0;
            if icon_button(ui, Icon::SkipStart, btn, false, "Go to first frame").clicked() {
                self.playing = false;
                self.doc.set_playhead(0);
                self.invalidate();
            }
            if icon_button(ui, Icon::StepBack, btn, false, "Previous frame (←)").clicked() {
                self.playing = false;
                self.doc.step_frame(-1);
                self.invalidate();
            }
            let play_icon = if self.playing { Icon::Pause } else { Icon::Play };
            if icon_button(
                ui,
                play_icon,
                btn + 6.0,
                self.playing,
                "Play / pause (Space)",
            )
            .clicked()
            {
                self.toggle_playback(ui.ctx());
            }
            if icon_button(ui, Icon::StepForward, btn, false, "Next frame (→)").clicked() {
                self.playing = false;
                self.doc.step_frame(1);
                self.invalidate();
            }
            if icon_button(ui, Icon::SkipEnd, btn, false, "Go to last frame").clicked() {
                self.playing = false;
                let last = self.doc.frame_count.saturating_sub(1);
                self.doc.set_playhead(last);
                self.invalidate();
            }

            ui.separator();
            let perform_label = if self.perform { "● REC" } else { "● PERFORM" };
            let (fill, fg, outline) = if self.perform {
                (
                    theme::DANGER,
                    Color32::from_rgb(30, 12, 12),
                    Stroke::new(1.0, theme::DANGER),
                )
            } else {
                (
                    Color32::TRANSPARENT,
                    theme::DANGER,
                    Stroke::new(1.0, theme::DANGER.gamma_multiply(0.5)),
                )
            };
            let perform = egui::Button::new(
                egui::RichText::new(perform_label)
                    .size(10.5)
                    .strong()
                    .color(fg),
            )
            .fill(fill)
            .stroke(outline)
            .corner_radius(CornerRadius::same(13))
            .min_size(Vec2::new(88.0, 24.0));
            if ui
                .add(perform)
                .on_hover_text(
                    "Perform mode: record a take in real time while you draw. The reel only \
                     plays back after you stop (P); Space stops and plays it",
                )
                .clicked()
            {
                self.toggle_perform(ui.ctx());
            }

            ui.separator();
            ui.label(
                egui::RichText::new(self.frame_label())
                    .monospace()
                    .size(12.0)
                    .color(theme::TEXT),
            );

            ui.separator();
            if icon_button(ui, Icon::Loop, btn, self.looping, "Loop playback (L)").clicked() {
                self.looping = !self.looping;
            }
            if icon_button(
                ui,
                Icon::Onion,
                btn,
                self.onion.enabled,
                "Onion skin (O)",
            )
            .clicked()
            {
                self.onion.enabled = !self.onion.enabled;
            }

            ui.separator();
            let mut fps = self.doc.fps;
            let resp = ui.add(
                egui::DragValue::new(&mut fps)
                    .range(1.0..=120.0)
                    .speed(0.25)
                    .suffix(" fps"),
            );
            if resp.changed() {
                self.doc.fps = fps.clamp(1.0, 120.0);
                self.dirty = true;
            }
            let mut frames = self.doc.frame_count as i32;
            let resp = ui.add(
                egui::DragValue::new(&mut frames)
                    .range(1..=10_000)
                    .speed(0.4)
                    .suffix(" frames"),
            );
            if resp.changed() {
                self.doc.resize_frame_count(frames.max(1) as u32);
                self.dirty = true;
                self.invalidate();
            }

            ui.separator();
            if icon_button(ui, Icon::Plus, btn, false, "New empty drawing at the playhead").clicked() {
                self.add_keyframe(false);
            }
            if icon_button(ui, Icon::Copy, btn, false, "Duplicate drawing (Ctrl+D)").clicked() {
                self.duplicate_keyframe();
            }
            if icon_button(ui, Icon::Trash, btn, false, "Delete drawing (Del)").clicked() {
                self.delete_keyframe();
            }

            ui.separator();
            if icon_button(ui, Icon::Minus, btn, false, "Zoom the timeline out").clicked() {
                self.timeline.auto_width = false;
                self.timeline.frame_w = (self.timeline.frame_w * 0.8).max(4.0);
            }
            if icon_button(ui, Icon::Plus, btn, false, "Zoom the timeline in").clicked() {
                self.timeline.auto_width = false;
                self.timeline.frame_w = (self.timeline.frame_w * 1.25).min(48.0);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let layer = self.doc.active_layer();
                let count = layer.frames.len();
                ui.label(
                    egui::RichText::new(format!("{}  ·  {} drawings", layer.name, count))
                        .size(10.5)
                        .color(theme::TEXT_FAINT),
                );
                if self.dirty {
                    ui.label(
                        egui::RichText::new("unsaved")
                            .size(10.0)
                            .color(theme::ACCENT),
                    );
                }
            });
        });
    }

    fn ui_timeline_body(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        if avail.x < 60.0 || avail.y < 36.0 {
            return;
        }
        let (rect, response) = ui.allocate_exact_size(avail, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, CornerRadius::same(4), theme::BG_PANEL);

        let header_w = 172.0_f32.min(rect.width() * 0.36);
        let ruler_h = 24.0;
        let row_h = 40.0;
        let row_gap = 4.0;
        let row_step = row_h + row_gap;
        let grid_left = rect.left() + header_w;
        let grid_top = rect.top() + ruler_h;
        let grid_w = (rect.width() - header_w).max(10.0);
        let tracks_h = (rect.bottom() - grid_top).max(1.0);
        let rows = self.doc.layers.len();
        let frames = self.doc.frame_count.max(1) as f32;

        // ------------------------------------------------------- zoom / scroll
        let fit_w = (grid_w / frames).clamp(4.0, 26.0);
        let mut fw = if self.timeline.auto_width {
            fit_w
        } else {
            self.timeline.frame_w.clamp(4.0, 48.0)
        };
        if response.hovered() {
            let (scroll, ctrl, shift) =
                ui.input(|i| (i.smooth_scroll_delta(), i.modifiers.ctrl, i.modifiers.shift));
            if ctrl && scroll.y.abs() > 0.01 {
                self.timeline.auto_width = false;
                fw = (fw * (scroll.y * 0.0025).exp()).clamp(4.0, 48.0);
                self.timeline.frame_w = fw;
            } else if shift && scroll.y.abs() > 0.01 {
                self.timeline.scroll_y -= scroll.y;
            } else {
                if scroll.y.abs() > 0.01 {
                    self.timeline.scroll_x -= scroll.y * 0.7;
                }
                if scroll.x.abs() > 0.01 {
                    self.timeline.scroll_x -= scroll.x;
                }
            }
        }
        if self.timeline.auto_width {
            self.timeline.frame_w = fw;
        }

        let content_w = frames * fw;
        let max_scroll = (content_w - grid_w).max(0.0);
        let content_h = rows as f32 * row_step;
        let max_scroll_y = (content_h - tracks_h).max(0.0);

        // Keep the playhead in view while the reel plays or records.
        if self.playing || self.perform {
            let head = self.doc.playhead as f32 * fw;
            let margin = fw * 2.0;
            if head < self.timeline.scroll_x + margin {
                self.timeline.scroll_x = (head - margin).max(0.0);
            } else if head + fw + margin > self.timeline.scroll_x + grid_w {
                self.timeline.scroll_x = head + fw + margin - grid_w;
            }
        }

        // Middle-drag pans both axes.
        if response.hovered() && ui.input(|i| i.pointer.middle_down()) {
            let delta = ui.input(|i| i.pointer.delta());
            self.timeline.scroll_x -= delta.x;
            self.timeline.scroll_y -= delta.y;
        }
        self.timeline.scroll_x = self.timeline.scroll_x.clamp(0.0, max_scroll);
        self.timeline.scroll_y = self.timeline.scroll_y.clamp(0.0, max_scroll_y);
        let scroll_x = self.timeline.scroll_x;
        let scroll_y = self.timeline.scroll_y;

        // ------------------------------------------------------------ geometry
        let frame_from_x = |x: f32| -> u32 {
            (((x - grid_left + scroll_x) / fw).floor()).clamp(0.0, frames - 1.0) as u32
        };
        let row_y = |layer: usize| -> f32 {
            grid_top + (rows - 1 - layer) as f32 * row_step - scroll_y
        };
        let layer_at_y = |y: f32| -> Option<usize> {
            if y < grid_top || y > rect.bottom() {
                return None;
            }
            let local = y - grid_top + scroll_y;
            let idx = (local / row_step).floor();
            if idx < 0.0 {
                return None;
            }
            let idx = idx as usize;
            if idx >= rows || local - idx as f32 * row_step > row_h {
                return None;
            }
            Some(rows - 1 - idx)
        };
        let frame_x = |f: f32| -> f32 { grid_left + f * fw - scroll_x };
        let eye_center = |layer: usize| Pos2::new(rect.left() + 18.0, row_y(layer) + row_h * 0.5);

        // --------------------------------------------------------- interaction
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
        let down = ui.input(|i| i.pointer.button_down(PointerButton::Primary));
        let released = ui.input(|i| i.pointer.button_released(PointerButton::Primary));
        let double = response.double_clicked();

        if pressed {
            if let Some(p) = pointer {
                if rect.contains(p) {
                    if p.y < grid_top {
                        if p.x >= grid_left {
                            self.timeline.scrubbing = true;
                            self.doc.set_playhead(frame_from_x(p.x));
                            self.invalidate();
                        }
                    } else if let Some(layer) = layer_at_y(p.y) {
                        self.doc.set_active_layer(layer);
                        let eye =
                            Rect::from_center_size(eye_center(layer), Vec2::splat(22.0));
                        if p.x < grid_left && eye.contains(p) {
                            self.doc_edit_bool(|doc| {
                                doc.layers[layer].visible = !doc.layers[layer].visible;
                                true
                            });
                        } else if p.x >= grid_left {
                            let f = frame_from_x(p.x);
                            self.doc.set_playhead(f);
                            if self.doc.layers[layer].frames.contains_key(&f) {
                                self.timeline.drag = Some(KeyframeDrag {
                                    layer,
                                    from: f,
                                    target: f,
                                    moved: false,
                                });
                            } else {
                                // Dragging an empty lane scrubs the playhead.
                                self.timeline.scrubbing = true;
                            }
                        }
                        self.invalidate();
                    }
                }
            }
        }

        if down {
            if self.timeline.scrubbing {
                if let Some(p) = pointer {
                    if p.x >= grid_left {
                        let f = frame_from_x(p.x);
                        if f != self.doc.playhead {
                            self.doc.set_playhead(f);
                            self.invalidate();
                        }
                    }
                }
            }
            if let Some(drag) = &mut self.timeline.drag {
                if let Some(p) = pointer {
                    let f = frame_from_x(p.x);
                    if f != drag.target {
                        drag.target = f;
                        drag.moved = true;
                    }
                }
            }
        }

        if released {
            self.timeline.scrubbing = false;
            if let Some(drag) = self.timeline.drag.take() {
                if drag.moved && drag.from != drag.target {
                    let (layer, from, to) = (drag.layer, drag.from, drag.target);
                    self.history.begin_doc(&self.doc);
                    if self.doc.move_keyframe(layer, from, to) {
                        self.dirty = true;
                        self.doc.set_playhead(to);
                        self.invalidate();
                    } else {
                        self.history.drop_last();
                        self.notify("A drawing already exists on that frame.");
                    }
                }
            }
        }

        if double {
            if let Some(p) = pointer {
                if p.x >= grid_left {
                    if let Some(layer) = layer_at_y(p.y) {
                        let f = frame_from_x(p.x);
                        if !self.doc.layers[layer].frames.contains_key(&f) {
                            self.doc.active_layer = layer;
                            self.doc_edit_bool(|doc| {
                                doc.insert_keyframe(layer, f, true).is_some()
                            });
                            self.doc.set_playhead(f);
                        }
                    }
                }
            }
        }

        // Right-click a clip or an empty cell for drawing actions.
        if response.secondary_clicked() {
            if let Some(p) = pointer {
                if p.x >= grid_left && p.y >= grid_top {
                    if let Some(layer) = layer_at_y(p.y) {
                        let frame = frame_from_x(p.x);
                        self.doc.active_layer = layer;
                        self.doc.set_playhead(frame);
                        self.timeline.context = Some((layer, frame));
                        self.invalidate();
                    }
                }
            }
        }

        response.context_menu(|ui| {
            let Some((layer, frame)) = self.timeline.context else {
                ui.close();
                return;
            };
            let has_key = self
                .doc
                .layers
                .get(layer)
                .map(|l| l.frames.contains_key(&frame))
                .unwrap_or(false);
            ui.label(
                egui::RichText::new(format!("Frame {}", frame + 1))
                    .size(11.0)
                    .color(theme::TEXT_FAINT),
            );
            ui.separator();
            if has_key {
                if ui.button("Copy drawing   Ctrl+C").clicked() {
                    self.copy_keyframe();
                    ui.close();
                }
                if ui.button("Cut drawing   Ctrl+X").clicked() {
                    self.cut_frame();
                    ui.close();
                }
                if ui.button("Duplicate to next free frame").clicked() {
                    self.duplicate_keyframe();
                    ui.close();
                }
                if ui.button("Delete drawing   Del").clicked() {
                    self.delete_keyframe();
                    ui.close();
                }
            } else {
                if ui.button("Insert empty drawing").clicked() {
                    self.add_keyframe(false);
                    ui.close();
                }
                if ui.button("Insert drawing from hold").clicked() {
                    self.add_keyframe(true);
                    ui.close();
                }
            }
            if ui.button("Paste drawing   Ctrl+V").clicked() {
                self.paste_keyframe();
                ui.close();
            }
            ui.separator();
            if ui.button("Flip horizontally").clicked() {
                self.flip_drawing(true);
                ui.close();
            }
            if ui.button("Flip vertically").clicked() {
                self.flip_drawing(false);
                ui.close();
            }
        });

        // --------------------------------------------------------- collect data
        let mut clips: Vec<Clip> = Vec::new();
        let mut header_thumbs: Vec<Option<egui::TextureId>> = Vec::with_capacity(rows);
        for i in 0..rows {
            let layer_idx = rows - 1 - i;
            header_thumbs.push(self.layer_thumbnail(ui.ctx(), layer_idx));
            let layer = &self.doc.layers[layer_idx];
            for (&start, drawing) in &layer.frames {
                let y = row_y(layer_idx);
                if y + row_h < grid_top || y > rect.bottom() {
                    continue;
                }
                clips.push(Clip {
                    layer: layer_idx,
                    start,
                    end: layer.hold_end(start, self.doc.frame_count),
                    drawing_id: drawing.id,
                    version: drawing.version,
                    pixmap: drawing.pixmap.clone(),
                    visible: layer.visible,
                    active: layer_idx == self.doc.active_layer,
                });
            }
        }
        let mut thumbnails: HashMap<u64, egui::TextureId> = HashMap::new();
        for clip in &clips {
            if (clip.end - clip.start) as f32 * fw < 44.0
                || thumbnails.contains_key(&clip.drawing_id)
            {
                continue;
            }
            let texture = self.drawing_texture(
                ui.ctx(),
                clip.drawing_id,
                clip.version,
                &clip.pixmap,
            );
            thumbnails.insert(clip.drawing_id, texture);
        }

        // -------------------------------------------------------------- paint
        let track_clip = Rect::from_min_max(
            Pos2::new(rect.left() + 1.0, grid_top),
            Pos2::new(rect.right() - 1.0, rect.bottom()),
        );
        let tp = painter.with_clip_rect(track_clip);

        // Ruler and header backgrounds.
        let ruler_rect = Rect::from_min_max(rect.min, Pos2::new(rect.right(), grid_top));
        painter.rect_filled(
            ruler_rect,
            CornerRadius {
                nw: 4,
                ne: 4,
                sw: 0,
                se: 0,
            },
            Color32::from_rgb(19, 19, 23),
        );
        painter.line_segment(
            [
                Pos2::new(rect.left(), grid_top),
                Pos2::new(rect.right(), grid_top),
            ],
            Stroke::new(1.0, theme::STROKE_SOFT),
        );
        let header_rect = Rect::from_min_max(
            Pos2::new(rect.left(), grid_top),
            Pos2::new(grid_left, rect.bottom()),
        );
        tp.rect_filled(header_rect, 0, Color32::from_rgb(21, 21, 25));
        tp.line_segment(
            [
                Pos2::new(grid_left, grid_top),
                Pos2::new(grid_left, rect.bottom()),
            ],
            Stroke::new(1.0, theme::STROKE),
        );
        painter.text(
            Pos2::new(rect.left() + 12.0, rect.top() + 12.0),
            egui::Align2::LEFT_CENTER,
            format!("{rows} TRACK{}", if rows == 1 { "" } else { "S" }),
            FontId::proportional(9.0),
            theme::TEXT_FAINT,
        );
        if self.perform {
            let fps = self.doc.fps.max(1.0);
            let elapsed =
                (self.doc.playhead.saturating_sub(self.perform_start)) as f32 / fps;
            let secs = elapsed.floor() as u32;
            painter.text(
                Pos2::new(rect.right() - 12.0, rect.top() + 12.0),
                egui::Align2::RIGHT_CENTER,
                format!("● REC  {}:{:02}", secs / 60, secs % 60),
                FontId::proportional(10.0),
                theme::DANGER,
            );
        }

        // Track lanes.
        for i in 0..rows {
            let layer_idx = rows - 1 - i;
            let y = grid_top + i as f32 * row_step - scroll_y;
            if y + row_h < grid_top || y > rect.bottom() {
                continue;
            }
            let active = layer_idx == self.doc.active_layer;
            let hovered = pointer
                .map(|p| layer_at_y(p.y) == Some(layer_idx))
                .unwrap_or(false);
            let lane = Rect::from_min_max(
                Pos2::new(grid_left + 2.0, y + 2.0),
                Pos2::new(rect.right() - 3.0, y + row_h - 2.0),
            );
            let lane_bg = if active {
                Color32::from_rgb(33, 30, 23)
            } else if hovered {
                Color32::from_rgb(28, 28, 33)
            } else {
                Color32::from_rgb(23, 23, 27)
            };
            tp.rect_filled(lane, CornerRadius::same(6), lane_bg);
            if active {
                tp.rect_stroke(
                    lane,
                    CornerRadius::same(6),
                    Stroke::new(1.0, theme::ACCENT_DIM),
                    StrokeKind::Inside,
                );
            }
        }

        // Frame grid, drawn over the lanes but under the clips.
        let mut step = 1u32;
        for candidate in [1u32, 2, 5, 10, 20, 25, 50, 100, 200, 500, 1000] {
            step = candidate;
            if candidate as f32 * fw >= 46.0 {
                break;
            }
        }
        let fps = self.doc.fps.max(1.0);
        let label_time = fps >= 8.0 && step as f32 >= fps;
        let step_i = step as i32;
        let first = (((scroll_x / fw) / step as f32).floor() * step as f32) as i32;
        let last = ((scroll_x + grid_w) / fw).ceil() as i32 + 1;
        let advance = if fw < 6.0 { step_i } else { 1 };
        let mut f = first.max(0);
        while f <= last && (f as f32) < frames {
            let x = frame_x(f as f32);
            if x >= grid_left - 0.5 && x <= rect.right() {
                let major = f % step_i == 0;
                tp.line_segment(
                    [Pos2::new(x, grid_top), Pos2::new(x, rect.bottom())],
                    Stroke::new(
                        1.0,
                        if major {
                            Color32::from_rgb(36, 36, 42)
                        } else {
                            Color32::from_rgb(28, 28, 33)
                        },
                    ),
                );
            }
            f += advance;
        }

        // Ruler ticks and labels.
        let mut f = first.max(0);
        while f <= last && (f as f32) < frames {
            let x = frame_x(f as f32);
            if x >= grid_left - 0.5 && x <= rect.right() {
                let major = f % step_i == 0;
                if major {
                    painter.line_segment(
                        [
                            Pos2::new(x, rect.top() + ruler_h - 7.0),
                            Pos2::new(x, grid_top),
                        ],
                        Stroke::new(1.0, theme::TEXT_FAINT.gamma_multiply(0.7)),
                    );
                    let label = if label_time {
                        let total = (f as f32 / fps).floor() as u32;
                        format!("{}:{:02}", total / 60, total % 60)
                    } else {
                        (f as u32 + 1).to_string()
                    };
                    painter.text(
                        Pos2::new(x + 4.0, rect.top() + 9.0),
                        egui::Align2::LEFT_CENTER,
                        label,
                        FontId::monospace(9.0),
                        theme::TEXT_FAINT,
                    );
                } else if fw >= 7.0 {
                    painter.line_segment(
                        [Pos2::new(x, grid_top - 4.0), Pos2::new(x, grid_top)],
                        Stroke::new(1.0, Color32::from_rgb(46, 46, 53)),
                    );
                }
            }
            f += advance;
        }

        // Drawing clips.
        for clip in &clips {
            let y = row_y(clip.layer);
            let x0 = frame_x(clip.start as f32);
            let x1 = frame_x(clip.end as f32);
            if x1 < grid_left || x0 > rect.right() {
                continue;
            }
            let left = x0.max(grid_left + 1.0);
            let right = x1.min(rect.right() - 1.0);
            if right - left < 2.0 {
                continue;
            }
            let block = Rect::from_min_max(
                Pos2::new(left + 1.0, y + 6.0),
                Pos2::new(right - 1.0, y + row_h - 6.0),
            );
            let (base, highlight) = if clip.active {
                (
                    Color32::from_rgb(122, 88, 42),
                    Color32::from_rgb(178, 132, 62),
                )
            } else {
                (
                    Color32::from_rgb(62, 61, 68),
                    Color32::from_rgb(86, 85, 93),
                )
            };
            let (base, highlight) = if clip.visible {
                (base, highlight)
            } else {
                (base.gamma_multiply(0.45), highlight.gamma_multiply(0.45))
            };
            tp.rect_filled(block, CornerRadius::same(5), base);
            tp.line_segment(
                [
                    Pos2::new(block.left() + 4.0, block.top() + 1.0),
                    Pos2::new(block.right() - 4.0, block.top() + 1.0),
                ],
                Stroke::new(1.0, highlight),
            );
            // Frame divisions inside the clip.
            if fw >= 8.0 && clip.end - clip.start <= 240 {
                let mut k = clip.start + 1;
                while k < clip.end {
                    let x = frame_x(k as f32);
                    if x > block.left() + 2.0 && x < block.right() - 2.0 {
                        tp.line_segment(
                            [
                                Pos2::new(x, block.top() + 2.0),
                                Pos2::new(x, block.bottom() - 2.0),
                            ],
                            Stroke::new(1.0, Color32::from_white_alpha(12)),
                        );
                    }
                    k += 1;
                }
            }
            // Drawing thumbnail at the clip start.
            if block.width() > 40.0 {
                if let Some(texture) = thumbnails.get(&clip.drawing_id) {
                    let thumb = Rect::from_min_size(
                        Pos2::new(block.left() + 5.0, block.center().y - 9.0),
                        Vec2::new(30.0, 18.0),
                    );
                    tp.image(
                        *texture,
                        thumb,
                        Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        if clip.visible {
                            Color32::WHITE
                        } else {
                            Color32::from_white_alpha(110)
                        },
                    );
                    tp.rect_stroke(
                        thumb,
                        CornerRadius::same(2),
                        Stroke::new(1.0, Color32::from_black_alpha(80)),
                        StrokeKind::Inside,
                    );
                }
            }
        }

        // Layer headers.
        for i in 0..rows {
            let layer_idx = rows - 1 - i;
            let layer = &self.doc.layers[layer_idx];
            let y = grid_top + i as f32 * row_step - scroll_y;
            if y + row_h < grid_top || y > rect.bottom() {
                continue;
            }
            let active = layer_idx == self.doc.active_layer;
            if active {
                let row_rect = Rect::from_min_max(
                    Pos2::new(rect.left(), y + 1.0),
                    Pos2::new(grid_left, y + row_h - 1.0),
                );
                tp.rect_filled(row_rect, 0, Color32::from_rgb(33, 30, 24));
                tp.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.left(), y + 6.0),
                        Pos2::new(rect.left() + 3.0, y + row_h - 6.0),
                    ),
                    CornerRadius::same(2),
                    theme::ACCENT,
                );
            }

            let eye = Rect::from_center_size(eye_center(layer_idx), Vec2::splat(18.0));
            icons::paint_centered(
                &tp,
                if layer.visible { Icon::Eye } else { Icon::EyeOff },
                eye,
                if layer.visible {
                    theme::TEXT_DIM
                } else {
                    theme::TEXT_FAINT
                },
            );

            let thumb = Rect::from_min_size(
                Pos2::new(rect.left() + 32.0, y + row_h * 0.5 - 12.0),
                Vec2::new(40.0, 24.0),
            );
            let texture = header_thumbs.get(i).copied().flatten();
            if let Some(texture) = texture {
                tp.image(
                    texture,
                    thumb,
                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    if layer.visible {
                        Color32::WHITE
                    } else {
                        Color32::from_white_alpha(110)
                    },
                );
            } else {
                tp.rect_filled(thumb, CornerRadius::same(3), theme::BG_DEEP);
            }
            tp.rect_stroke(
                thumb,
                CornerRadius::same(3),
                Stroke::new(1.0, theme::STROKE_SOFT),
                StrokeKind::Inside,
            );

            if layer.locked {
                icons::paint_centered(
                    &tp,
                    Icon::Lock,
                    Rect::from_min_size(
                        Pos2::new(rect.left() + header_w - 22.0, y + row_h * 0.5 - 8.0),
                        Vec2::splat(16.0),
                    ),
                    theme::ACCENT,
                );
            }

            let name_x = rect.left() + 80.0;
            let name_color = if !layer.visible {
                theme::TEXT_FAINT
            } else if active {
                theme::TEXT
            } else {
                theme::TEXT_DIM
            };
            let space = if layer.locked {
                (header_w - 80.0 - 26.0).max(20.0)
            } else {
                (header_w - 80.0 - 10.0).max(20.0)
            };
            let max_chars = (space / 6.1) as usize;
            let mut name = layer.name.clone();
            if name.chars().count() > max_chars {
                name = name.chars().take(max_chars.saturating_sub(1)).collect::<String>() + "…";
            }
            tp.text(
                Pos2::new(name_x, y + 14.0),
                egui::Align2::LEFT_CENTER,
                name,
                FontId::proportional(11.5),
                name_color,
            );
            let meta = if layer.frames.is_empty() {
                "empty".to_string()
            } else {
                format!(
                    "{} drawing{}",
                    layer.frames.len(),
                    if layer.frames.len() == 1 { "" } else { "s" }
                )
            };
            tp.text(
                Pos2::new(name_x, y + 27.0),
                egui::Align2::LEFT_CENTER,
                meta,
                FontId::proportional(9.0),
                theme::TEXT_FAINT,
            );

            if layer.frames.is_empty() {
                tp.text(
                    Pos2::new(grid_left + 14.0, y + row_h * 0.5),
                    egui::Align2::LEFT_CENTER,
                    "Draw here to create the first drawing",
                    FontId::proportional(10.0),
                    theme::TEXT_FAINT.gamma_multiply(0.8),
                );
            }
        }

        // Hovered cell.
        if let Some(p) = pointer {
            if p.x >= grid_left && p.y >= grid_top && rect.contains(p) {
                if let Some(layer) = layer_at_y(p.y) {
                    let y = row_y(layer);
                    let left = frame_x(frame_from_x(p.x) as f32).max(grid_left);
                    let right = (left + fw).min(rect.right());
                    if right - left > 2.0 {
                        tp.rect_filled(
                            Rect::from_min_max(
                                Pos2::new(left, y + 3.0),
                                Pos2::new(right, y + row_h - 3.0),
                            ),
                            CornerRadius::same(4),
                            Color32::from_white_alpha(10),
                        );
                    }
                }
            }
        }

        // Current frame: a band exactly one frame cell wide across every track,
        // plus a filled ruler cell, so the playhead position is unambiguous.
        let band_color = if self.perform {
            theme::DANGER
        } else {
            theme::ACCENT
        };
        let band_left = frame_x(self.doc.playhead as f32);
        let band_right = band_left + fw;
        if band_right > grid_left && band_left < rect.right() {
            let l = band_left.max(grid_left);
            let r = band_right.min(rect.right());
            let band =
                Rect::from_min_max(Pos2::new(l, grid_top), Pos2::new(r, rect.bottom()));
            painter.rect_filled(band, 0, band_color.gamma_multiply(0.16));
            let edge = band_color.gamma_multiply(0.65);
            painter.line_segment(
                [Pos2::new(l, grid_top), Pos2::new(l, rect.bottom())],
                Stroke::new(1.0, edge),
            );
            painter.line_segment(
                [Pos2::new(r, grid_top), Pos2::new(r, rect.bottom())],
                Stroke::new(1.0, edge),
            );

            let cell = Rect::from_min_max(
                Pos2::new(l + 1.0, rect.top() + 1.0),
                Pos2::new((r - 1.0).max(l + 3.0), grid_top - 1.0),
            );
            painter.rect_filled(cell, CornerRadius::same(3), band_color);
            if r - l >= 14.0 {
                painter.text(
                    cell.center(),
                    egui::Align2::CENTER_CENTER,
                    format!("{}", self.doc.playhead + 1),
                    FontId::monospace(9.5),
                    Color32::from_rgb(30, 20, 8),
                );
            }
        }

        // Dragged clip ghost.
        if let Some(drag) = &self.timeline.drag {
            let y = row_y(drag.layer);
            let x = frame_x(drag.target as f32);
            let ghost = Rect::from_min_max(
                Pos2::new(x.max(grid_left) + 1.0, y + 6.0),
                Pos2::new((x + fw - 1.0).max(x + 8.0), y + row_h - 6.0),
            );
            tp.rect_filled(
                ghost,
                CornerRadius::same(5),
                Color32::from_rgba_unmultiplied(238, 168, 66, 40),
            );
            tp.rect_stroke(
                ghost,
                CornerRadius::same(5),
                Stroke::new(1.5, theme::ACCENT),
                StrokeKind::Inside,
            );
        }

        // Track scrollbar and overflow hint.
        if max_scroll_y > 0.0 {
            let bar_h = (tracks_h / content_h * tracks_h).max(18.0);
            let bar_y = grid_top + (scroll_y / max_scroll_y) * (tracks_h - bar_h);
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.right() - 4.0, bar_y),
                    Vec2::new(3.0, bar_h),
                ),
                CornerRadius::same(2),
                theme::STROKE,
            );
            painter.text(
                Pos2::new(rect.right() - 10.0, rect.bottom() - 6.0),
                egui::Align2::RIGHT_BOTTOM,
                "⇕ scroll for more tracks",
                FontId::proportional(9.0),
                theme::TEXT_FAINT,
            );
        }

        // Recording frame.
        if self.perform {
            painter.rect_stroke(
                rect.shrink(1.0),
                CornerRadius::same(4),
                Stroke::new(1.5, theme::DANGER.gamma_multiply(0.8)),
                StrokeKind::Inside,
            );
        }
    }
}
