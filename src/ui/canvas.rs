//! The canvas view: presentation, pan/zoom/rotate and tool interaction.

use egui::{Color32, CursorIcon, PointerButton, Pos2, Rect, Sense, Stroke, Vec2};

use crate::app::{DreemzApp, Tool};
use crate::theme;

impl DreemzApp {
    pub fn ui_canvas(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        if avail.x < 8.0 || avail.y < 8.0 {
            return;
        }
        let (response, painter) = ui.allocate_painter(avail, Sense::click_and_drag());
        let rect = response.rect;
        painter.rect_filled(rect, 0, theme::BG_CANVAS);

        // Keep the composite texture current (playback, onion changes, undo…).
        self.update_canvas(ui.ctx());

        if self.needs_fit {
            self.fit_view(rect);
            self.view_size = rect.size();
        } else {
            self.view_size = rect.size();
        }

        let pointer = ui.input(|i| i.pointer.interact_pos());
        let hovered = response.hovered();
        let primary_pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
        let primary_released = ui.input(|i| i.pointer.button_released(PointerButton::Primary));
        let primary_down = ui.input(|i| i.pointer.button_down(PointerButton::Primary));
        let middle_down = ui.input(|i| i.pointer.middle_down());
        let pan_tool = self.tool == Tool::Hand;

        // ------------------------------------------------------- navigation
        if hovered {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta(), i.zoom_delta()));
            let shift = ui.input(|i| i.modifiers.shift);
            if shift && scroll.y.abs() > 0.01 {
                self.view.rotation += scroll.y * 0.0015;
            } else if scroll.y.abs() > 0.01 || (pinch - 1.0).abs() > 0.001 {
                let factor = (scroll.y * 0.0018).exp() * pinch;
                if let Some(p) = pointer {
                    self.set_zoom_around(rect, p, factor);
                }
            }
            if scroll.x.abs() > 0.01 {
                self.view.pan.x += scroll.x;
            }
        }

        if hovered && (middle_down || (pan_tool && primary_down)) {
            self.view.pan += ui.input(|i| i.pointer.delta());
        }

        // ------------------------------------------------------------ tools
        let speed = ui.input(|i| i.pointer.delta().length());
        let simulated = (1.0 - speed / 45.0).clamp(0.35, 1.0);
        self.pointer_pressure = simulated;
        // Real tablet / touch pressure wins over the speed simulation when the
        // backend reports it.
        let touch_force = ui.input(|i| {
            i.events.iter().rev().find_map(|e| match e {
                egui::Event::Touch { force, phase, .. }
                    if !matches!(
                        phase,
                        egui::TouchPhase::End | egui::TouchPhase::Cancel
                    ) =>
                {
                    force.filter(|f| *f > 0.0)
                }
                _ => None,
            })
        });
        let simulate = self.brush_settings().simulate_pressure;
        let pressure = if let Some(force) = touch_force {
            force.clamp(0.0, 1.0)
        } else if simulate {
            simulated
        } else {
            1.0
        };

        let canvas_pos = pointer.map(|p| self.screen_to_canvas(rect, p));
        self.hover_canvas = if hovered { canvas_pos } else { None };
        let doc_w = self.doc.width as f32;
        let doc_h = self.doc.height as f32;

        if primary_pressed && hovered && !pan_tool && !middle_down {
            if let Some(cp) = canvas_pos {
                let inside = cp.x >= 0.0 && cp.y >= 0.0 && cp.x < doc_w && cp.y < doc_h;
                match self.tool {
                    Tool::Select => {
                        let on_selection = self
                            .selection
                            .map(|r| r.contains(cp.x, cp.y))
                            .unwrap_or(false);
                        if on_selection {
                            self.begin_selection_move((cp.x, cp.y));
                        } else {
                            self.selection = None;
                            self.begin_selection_draft((cp.x, cp.y));
                        }
                    }
                    _ if !inside => {}
                    Tool::Brush | Tool::Eraser => self.begin_stroke((cp.x, cp.y), pressure),
                    Tool::Fill => self.fill_at((cp.x, cp.y)),
                    Tool::Eyedropper => self.pick_at((cp.x, cp.y)),
                    Tool::Move => self.begin_move((cp.x, cp.y)),
                    Tool::Hand => {}
                }
            }
        }

        if primary_down {
            if let Some(p) = pointer {
                let cp = self.screen_to_canvas(rect, p);
                match self.tool {
                    Tool::Brush | Tool::Eraser => {
                        if self.stroke.is_some() {
                            self.extend_stroke((cp.x, cp.y), pressure);
                        }
                    }
                    Tool::Move => {
                        if self.move_state.is_some() {
                            self.update_move((cp.x, cp.y));
                        }
                    }
                    Tool::Select => {
                        if self.selection_draft.is_some() {
                            self.update_selection_draft((cp.x, cp.y));
                        } else if self.selection_move.is_some() {
                            self.update_selection_move((cp.x, cp.y));
                        }
                    }
                    _ => {}
                }
            }
        }

        if primary_released {
            let end = canvas_pos.unwrap_or(Pos2::ZERO);
            if self.stroke.is_some() {
                self.end_stroke((end.x, end.y), pressure);
            }
            if self.move_state.is_some() {
                self.end_move();
            }
            if self.selection_draft.is_some() {
                self.finish_selection_draft();
            }
            if self.selection_move.is_some() {
                self.end_selection_move();
            }
        }

        // -------------------------------------------------------------- draw
        let tex_id = self.canvas_texture.as_ref().map(|t| t.id());
        let tl = self.canvas_to_screen(rect, Pos2::new(0.0, 0.0));
        let tr = self.canvas_to_screen(rect, Pos2::new(self.doc.width as f32, 0.0));
        let br = self.canvas_to_screen(
            rect,
            Pos2::new(self.doc.width as f32, self.doc.height as f32),
        );
        let bl = self.canvas_to_screen(rect, Pos2::new(0.0, self.doc.height as f32));
        let quad = [tl, tr, br, bl];

        if let Some(tex_id) = tex_id {
            let shadow_off = Vec2::new(7.0, 9.0);
            let shadow: Vec<Pos2> = quad.iter().map(|p| *p + shadow_off).collect();
            painter.add(egui::Shape::convex_polygon(
                shadow,
                Color32::from_black_alpha(120),
                Stroke::NONE,
            ));

            let mut mesh = egui::Mesh::with_texture(tex_id);
            let uvs = [
                egui::pos2(0.0, 0.0),
                egui::pos2(1.0, 0.0),
                egui::pos2(1.0, 1.0),
                egui::pos2(0.0, 1.0),
            ];
            for (pos, uv) in quad.iter().zip(uvs) {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: *pos,
                    uv,
                    color: Color32::WHITE,
                });
            }
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(egui::Shape::mesh(mesh));

            painter.add(egui::Shape::closed_line(
                quad.to_vec(),
                Stroke::new(1.0, theme::STROKE),
            ));
        } else {
            painter.add(egui::Shape::convex_polygon(
                quad.to_vec(),
                Color32::from_rgb(28, 28, 32),
                Stroke::new(1.0, theme::STROKE),
            ));
        }

        // Brush preview.
        if hovered && !pan_tool && matches!(self.tool, Tool::Brush | Tool::Eraser) {
            if let Some(p) = pointer {
                let radius = (self.brush_settings().size * 0.5 * self.view.zoom).max(2.0);
                painter.circle_stroke(
                    p,
                    radius,
                    Stroke::new(1.0, Color32::from_white_alpha(170)),
                );
                painter.circle_stroke(
                    p,
                    radius + 1.0,
                    Stroke::new(1.0, Color32::from_black_alpha(110)),
                );
            }
        }

        // View overlays.
        let frame_text = format!("Frame {} / {}", self.doc.playhead + 1, self.doc.frame_count);
        painter.text(
            rect.left_top() + Vec2::new(12.0, 10.0),
            egui::Align2::LEFT_TOP,
            frame_text,
            egui::FontId::proportional(11.0),
            theme::TEXT_FAINT,
        );
        painter.text(
            rect.right_bottom() + Vec2::new(-12.0, -10.0),
            egui::Align2::RIGHT_BOTTOM,
            format!("{:.0}%", self.view.zoom * 100.0),
            egui::FontId::proportional(11.0),
            theme::TEXT_FAINT,
        );
        if self.perform {
            let ink = Color32::from_rgb(28, 12, 12);
            let galley = painter.layout_no_wrap(
                "● REC".to_owned(),
                egui::FontId::proportional(10.5),
                ink,
            );
            let pad = Vec2::new(9.0, 5.0);
            let size = galley.size() + pad * 2.0;
            let corner = rect.right_top() + Vec2::new(-12.0, 10.0);
            let badge = Rect::from_min_size(Pos2::new(corner.x - size.x, corner.y), size);
            painter.rect_filled(badge, egui::CornerRadius::same(9), theme::DANGER);
            painter.galley(badge.min + pad, galley, ink);
        }
        // Composition guides.
        if self.guides.thirds || self.guides.center || self.guides.safe {
            let faint = Stroke::new(1.0, Color32::from_rgba_unmultiplied(238, 168, 66, 70));
            let bright = Stroke::new(1.0, Color32::from_rgba_unmultiplied(238, 168, 66, 130));
            if self.guides.thirds {
                for f in [1.0 / 3.0, 2.0 / 3.0] {
                    painter.line_segment(
                        [
                            self.canvas_to_screen(rect, Pos2::new(doc_w * f, 0.0)),
                            self.canvas_to_screen(rect, Pos2::new(doc_w * f, doc_h)),
                        ],
                        faint,
                    );
                    painter.line_segment(
                        [
                            self.canvas_to_screen(rect, Pos2::new(0.0, doc_h * f)),
                            self.canvas_to_screen(rect, Pos2::new(doc_w, doc_h * f)),
                        ],
                        faint,
                    );
                }
            }
            if self.guides.center {
                painter.line_segment(
                    [
                        self.canvas_to_screen(rect, Pos2::new(doc_w * 0.5, 0.0)),
                        self.canvas_to_screen(rect, Pos2::new(doc_w * 0.5, doc_h)),
                    ],
                    bright,
                );
                painter.line_segment(
                    [
                        self.canvas_to_screen(rect, Pos2::new(0.0, doc_h * 0.5)),
                        self.canvas_to_screen(rect, Pos2::new(doc_w, doc_h * 0.5)),
                    ],
                    bright,
                );
            }
            if self.guides.safe {
                let inset = 0.1;
                let corners = [
                    self.canvas_to_screen(rect, Pos2::new(doc_w * inset, doc_h * inset)),
                    self.canvas_to_screen(rect, Pos2::new(doc_w * (1.0 - inset), doc_h * inset)),
                    self.canvas_to_screen(rect, Pos2::new(doc_w * (1.0 - inset), doc_h * (1.0 - inset))),
                    self.canvas_to_screen(rect, Pos2::new(doc_w * inset, doc_h * (1.0 - inset))),
                ];
                painter.add(egui::Shape::closed_line(corners.to_vec(), bright));
            }
        }

        // Marching ants around the selection (or the live marquee).
        let phase = (ui.input(|i| i.time) * 10.0) as f32;
        if let Some(draft) = self.selection_draft {
            let corners = [
                self.canvas_to_screen(rect, draft.0),
                self.canvas_to_screen(rect, Pos2::new(draft.1.x, draft.0.y)),
                self.canvas_to_screen(rect, draft.1),
                self.canvas_to_screen(rect, Pos2::new(draft.0.x, draft.1.y)),
            ];
            marching_rect(&painter, corners, phase);
        } else if let Some(sel) = self.selection_preview() {
            let corners = [
                self.canvas_to_screen(rect, Pos2::new(sel.x0 as f32, sel.y0 as f32)),
                self.canvas_to_screen(rect, Pos2::new(sel.x1 as f32, sel.y0 as f32)),
                self.canvas_to_screen(rect, Pos2::new(sel.x1 as f32, sel.y1 as f32)),
                self.canvas_to_screen(rect, Pos2::new(sel.x0 as f32, sel.y1 as f32)),
            ];
            marching_rect(&painter, corners, phase);
        }

        let mut badges: Vec<String> = Vec::new();
        if self.onion.enabled {
            badges.push("ONION".into());
        }
        if let Some(sel) = self.selection {
            badges.push(format!("SEL {}x{}", sel.width(), sel.height()));
        }
        if (self.view.rotation).abs() > 0.001 {
            badges.push(format!("{:.0}°", self.view.rotation.to_degrees()));
        }
        if !badges.is_empty() {
            painter.text(
                rect.left_bottom() + Vec2::new(12.0, -10.0),
                egui::Align2::LEFT_BOTTOM,
                badges.join("  ·  "),
                egui::FontId::proportional(10.0),
                theme::ACCENT,
            );
        }

        // Cursor.
        if hovered {
            let cursor = match self.tool {
                Tool::Hand => {
                    if primary_down {
                        CursorIcon::Grabbing
                    } else {
                        CursorIcon::Grab
                    }
                }
                Tool::Move => CursorIcon::Move,
                Tool::Select => {
                    let over = self
                        .selection_preview()
                        .zip(self.hover_canvas)
                        .map(|(r, p)| r.contains(p.x, p.y))
                        .unwrap_or(false);
                    if over {
                        CursorIcon::Move
                    } else {
                        CursorIcon::Crosshair
                    }
                }
                Tool::Eyedropper => CursorIcon::Crosshair,
                _ => CursorIcon::Crosshair,
            };
            ui.ctx().set_cursor_icon(cursor);
        }

        // Keep the ants marching while a selection is visible.
        if self.selection.is_some() || self.selection_draft.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(60));
        }

        // Upload fresh pixels before the frame ends.
        if self.canvas_key.is_none() {
            self.update_canvas(ui.ctx());
        }
    }
}

fn dashed_segment(
    painter: &egui::Painter,
    a: Pos2,
    b: Pos2,
    dash: f32,
    phase: f32,
    color: Color32,
) {
    let delta = b - a;
    let len = delta.length();
    if len < 0.5 {
        return;
    }
    let dir = delta / len;
    let period = dash * 2.0;
    let mut t = -(phase % period);
    while t < len {
        let start = t.max(0.0);
        let end = (t + dash).min(len);
        if end > start {
            painter.line_segment(
                [a + dir * start, a + dir * end],
                Stroke::new(1.4, color),
            );
        }
        t += period;
    }
}

fn marching_rect(painter: &egui::Painter, corners: [Pos2; 4], phase: f32) {
    let dark = Color32::from_black_alpha(210);
    for i in 0..4 {
        let a = corners[i];
        let b = corners[(i + 1) % 4];
        painter.line_segment([a, b], Stroke::new(1.4, dark));
        dashed_segment(painter, a, b, 4.0, phase + i as f32 * 2.0, Color32::WHITE);
    }
}
