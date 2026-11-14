//! Small vector icons drawn with egui shapes, so the app has no asset files.

use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Icon {
    Brush,
    Eraser,
    Fill,
    Picker,
    Move,
    Select,
    Hand,
    Plus,
    Minus,
    Copy,
    Trash,
    Eye,
    EyeOff,
    Lock,
    Unlock,
    ArrowUp,
    ArrowDown,
    Play,
    Pause,
    Loop,
    SkipStart,
    SkipEnd,
    StepBack,
    StepForward,
    Onion,
    Fit,
    Undo,
    Redo,
    Save,
    Folder,
    Export,
    Image,
    Settings,
    Key,
    Help,
    Layers,
    Grid,
    Close,
    Rotate,
}

pub fn paint(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    let r = rect.shrink(rect.width() * 0.22);
    let p = |x: f32, y: f32| Pos2::new(r.min.x + x * r.width(), r.min.y + y * r.height());
    let w = (rect.width() * 0.085).clamp(1.1, 2.0);
    let stroke = Stroke::new(w, color);
    let line = |a: Pos2, b: Pos2| painter.line_segment([a, b], stroke);
    let poly = |points: Vec<Pos2>, fill: Color32| {
        painter.add(Shape::convex_polygon(points, fill, Stroke::NONE));
    };
    let outline = |points: Vec<Pos2>| {
        painter.add(Shape::closed_line(points, stroke));
    };

    match icon {
        Icon::Brush => {
            line(p(0.12, 0.88), p(0.62, 0.38));
            line(p(0.55, 0.3), p(0.7, 0.45));
            line(p(0.55, 0.3), p(0.3, 0.5));
            poly(
                vec![p(0.12, 0.88), p(0.30, 0.72), p(0.30, 0.72), p(0.06, 0.94)],
                color,
            );
            painter.line_segment([p(0.55, 0.3), p(0.78, 0.12)], Stroke::new(w * 2.0, color));
        }
        Icon::Eraser => {
            poly(
                vec![p(0.18, 0.72), p(0.5, 0.2), p(0.86, 0.42), p(0.55, 0.94)],
                Color32::TRANSPARENT,
            );
            line(p(0.34, 0.46), p(0.7, 0.68));
            line(p(0.18, 0.72), p(0.5, 0.94));
            line(p(0.5, 0.94), p(0.9, 0.94));
        }
        Icon::Fill => {
            poly(
                vec![p(0.18, 0.52), p(0.5, 0.14), p(0.82, 0.5), p(0.5, 0.82)],
                Color32::TRANSPARENT,
            );
            line(p(0.5, 0.12), p(0.5, 0.3));
            line(p(0.9, 0.62), p(0.9, 0.62));
            painter.circle_filled(p(0.88, 0.72), r.width() * 0.13, color);
            line(p(0.88, 0.86), p(0.88, 0.9));
        }
        Icon::Picker => {
            line(p(0.2, 0.82), p(0.66, 0.34));
            painter.circle_stroke(p(0.76, 0.24), r.width() * 0.14, stroke);
            poly(
                vec![p(0.18, 0.86), p(0.32, 0.78), p(0.22, 0.7)],
                color,
            );
            line(p(0.38, 0.5), p(0.56, 0.66));
        }
        Icon::Move => {
            line(p(0.5, 0.1), p(0.5, 0.9));
            line(p(0.1, 0.5), p(0.9, 0.5));
            line(p(0.5, 0.1), p(0.36, 0.26));
            line(p(0.5, 0.1), p(0.64, 0.26));
            line(p(0.5, 0.9), p(0.36, 0.74));
            line(p(0.5, 0.9), p(0.64, 0.74));
            line(p(0.1, 0.5), p(0.26, 0.36));
            line(p(0.1, 0.5), p(0.26, 0.64));
            line(p(0.9, 0.5), p(0.74, 0.36));
            line(p(0.9, 0.5), p(0.74, 0.64));
        }
        Icon::Select => {
            // Corner brackets, like a marquee.
            line(p(0.12, 0.36), p(0.12, 0.12));
            line(p(0.12, 0.12), p(0.36, 0.12));
            line(p(0.88, 0.36), p(0.88, 0.12));
            line(p(0.88, 0.12), p(0.64, 0.12));
            line(p(0.12, 0.64), p(0.12, 0.88));
            line(p(0.12, 0.88), p(0.36, 0.88));
            line(p(0.88, 0.64), p(0.88, 0.88));
            line(p(0.88, 0.88), p(0.64, 0.88));
            painter.circle_filled(p(0.5, 0.5), r.width() * 0.055, color);
        }
        Icon::Hand => {
            painter.add(Shape::closed_line(
                vec![
                    p(0.18, 0.62),
                    p(0.26, 0.34),
                    p(0.34, 0.38),
                    p(0.36, 0.16),
                    p(0.46, 0.2),
                    p(0.5, 0.12),
                    p(0.6, 0.2),
                    p(0.64, 0.28),
                    p(0.72, 0.26),
                    p(0.78, 0.4),
                    p(0.8, 0.58),
                    p(0.74, 0.82),
                    p(0.6, 0.92),
                    p(0.34, 0.92),
                    p(0.2, 0.78),
                ],
                stroke,
            ));
        }
        Icon::Plus => {
            line(p(0.15, 0.5), p(0.85, 0.5));
            line(p(0.5, 0.15), p(0.5, 0.85));
        }
        Icon::Minus => {
            line(p(0.15, 0.5), p(0.85, 0.5));
        }
        Icon::Copy => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.1, 0.3), p(0.68, 0.92)),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(0.3, 0.08), p(0.9, 0.7)),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Trash => {
            line(p(0.15, 0.24), p(0.85, 0.24));
            line(p(0.36, 0.24), p(0.42, 0.1));
            line(p(0.58, 0.24), p(0.64, 0.1));
            line(p(0.42, 0.1), p(0.64, 0.1));
            painter.add(Shape::closed_line(
                vec![p(0.24, 0.34), p(0.3, 0.9), p(0.7, 0.9), p(0.76, 0.34)],
                stroke,
            ));
            line(p(0.44, 0.46), p(0.45, 0.78));
            line(p(0.58, 0.46), p(0.57, 0.78));
        }
        Icon::Eye | Icon::EyeOff => {
            let mut pts = Vec::new();
            for i in 0..=16 {
                let t = i as f32 / 16.0;
                let x = 0.08 + t * 0.84;
                let y = 0.5 - (t * std::f32::consts::PI).sin() * 0.3;
                pts.push(p(x, y));
            }
            for i in 0..=16 {
                let t = i as f32 / 16.0;
                let x = 0.92 - t * 0.84;
                let y = 0.5 + (t * std::f32::consts::PI).sin() * 0.3;
                pts.push(p(x, y));
            }
            painter.add(Shape::closed_line(pts, stroke));
            painter.circle_filled(p(0.5, 0.5), r.width() * 0.14, color);
            if icon == Icon::EyeOff {
                line(p(0.14, 0.88), p(0.86, 0.12));
            }
        }
        Icon::Lock | Icon::Unlock => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.24, 0.46), p(0.76, 0.9)),
                egui::CornerRadius::same(2),
                stroke,
                egui::StrokeKind::Inside,
            );
            let arc = if icon == Icon::Lock {
                vec![p(0.34, 0.46), p(0.34, 0.28), p(0.5, 0.14), p(0.66, 0.28), p(0.66, 0.46)]
            } else {
                vec![p(0.34, 0.46), p(0.34, 0.28), p(0.5, 0.14), p(0.72, 0.2)]
            };
            painter.add(Shape::line(arc, stroke));
        }
        Icon::ArrowUp => {
            line(p(0.5, 0.8), p(0.5, 0.2));
            line(p(0.5, 0.2), p(0.3, 0.44));
            line(p(0.5, 0.2), p(0.7, 0.44));
        }
        Icon::ArrowDown => {
            line(p(0.5, 0.2), p(0.5, 0.8));
            line(p(0.5, 0.8), p(0.3, 0.56));
            line(p(0.5, 0.8), p(0.7, 0.56));
        }
        Icon::Play => {
            poly(vec![p(0.3, 0.14), p(0.86, 0.5), p(0.3, 0.86)], color);
        }
        Icon::Pause => {
            painter.rect_filled(Rect::from_min_max(p(0.26, 0.12), p(0.42, 0.88)), 1.0, color);
            painter.rect_filled(Rect::from_min_max(p(0.58, 0.12), p(0.74, 0.88)), 1.0, color);
        }
        Icon::Loop => {
            line(p(0.18, 0.36), p(0.18, 0.62));
            line(p(0.18, 0.36), p(0.42, 0.36));
            line(p(0.58, 0.64), p(0.82, 0.64));
            line(p(0.82, 0.64), p(0.82, 0.38));
            line(p(0.82, 0.38), p(0.58, 0.38));
            line(p(0.42, 0.64), p(0.18, 0.64));
            line(p(0.42, 0.64), p(0.3, 0.5));
            line(p(0.58, 0.36), p(0.7, 0.5));
        }
        Icon::SkipStart => {
            poly(vec![p(0.66, 0.14), p(0.66, 0.86), p(0.2, 0.5)], color);
            line(p(0.8, 0.14), p(0.8, 0.86));
        }
        Icon::SkipEnd => {
            poly(vec![p(0.34, 0.14), p(0.34, 0.86), p(0.8, 0.5)], color);
            line(p(0.2, 0.14), p(0.2, 0.86));
        }
        Icon::StepBack => {
            poly(vec![p(0.68, 0.14), p(0.68, 0.86), p(0.24, 0.5)], color);
        }
        Icon::StepForward => {
            poly(vec![p(0.32, 0.14), p(0.32, 0.86), p(0.76, 0.5)], color);
        }
        Icon::Onion => {
            painter.circle_stroke(p(0.32, 0.5), r.width() * 0.2, Stroke::new(w * 0.8, color.gamma_multiply(0.45)));
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.2, Stroke::new(w * 0.8, color.gamma_multiply(0.7)));
            painter.circle_stroke(p(0.68, 0.5), r.width() * 0.2, stroke);
        }
        Icon::Fit => {
            line(p(0.12, 0.34), p(0.12, 0.12));
            line(p(0.12, 0.12), p(0.34, 0.12));
            line(p(0.88, 0.34), p(0.88, 0.12));
            line(p(0.88, 0.12), p(0.66, 0.12));
            line(p(0.12, 0.66), p(0.12, 0.88));
            line(p(0.12, 0.88), p(0.34, 0.88));
            line(p(0.88, 0.66), p(0.88, 0.88));
            line(p(0.88, 0.88), p(0.66, 0.88));
        }
        Icon::Undo | Icon::Redo => {
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let angle = std::f32::consts::PI * (0.15 + t * 0.85);
                let x = 0.5 + angle.cos() * 0.34;
                let y = 0.46 + angle.sin() * 0.34;
                pts.push(if icon == Icon::Undo {
                    p(1.0 - x, y)
                } else {
                    p(x, y)
                });
            }
            painter.add(Shape::line(pts, stroke));
            if icon == Icon::Undo {
                line(p(0.16, 0.62), p(0.16, 0.32));
                line(p(0.16, 0.32), p(0.44, 0.36));
            } else {
                line(p(0.84, 0.62), p(0.84, 0.32));
                line(p(0.84, 0.32), p(0.56, 0.36));
            }
        }
        Icon::Save => {
            outline(vec![p(0.16, 0.16), p(0.72, 0.16), p(0.84, 0.28), p(0.84, 0.84), p(0.16, 0.84)]);
            painter.rect_stroke(
                Rect::from_min_max(p(0.32, 0.16), p(0.66, 0.42)),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_stroke(
                Rect::from_min_max(p(0.3, 0.58), p(0.7, 0.84)),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        Icon::Folder => {
            painter.add(Shape::closed_line(
                vec![
                    p(0.1, 0.28),
                    p(0.44, 0.28),
                    p(0.52, 0.4),
                    p(0.9, 0.4),
                    p(0.9, 0.86),
                    p(0.1, 0.86),
                ],
                stroke,
            ));
        }
        Icon::Export => {
            line(p(0.5, 0.66), p(0.5, 0.12));
            line(p(0.5, 0.12), p(0.3, 0.34));
            line(p(0.5, 0.12), p(0.7, 0.34));
            painter.add(Shape::line(
                vec![p(0.14, 0.52), p(0.14, 0.88), p(0.86, 0.88), p(0.86, 0.52)],
                stroke,
            ));
        }
        Icon::Image => {
            painter.rect_stroke(
                Rect::from_min_max(p(0.1, 0.16), p(0.9, 0.84)),
                egui::CornerRadius::same(1),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.circle_filled(p(0.3, 0.34), r.width() * 0.08, color);
            painter.add(Shape::line(
                vec![p(0.14, 0.76), p(0.4, 0.46), p(0.58, 0.64), p(0.7, 0.52), p(0.88, 0.72)],
                stroke,
            ));
        }
        Icon::Settings => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.2, stroke);
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let (s, c) = a.sin_cos();
                line(
                    p(0.5 + c * 0.3, 0.5 + s * 0.3),
                    p(0.5 + c * 0.44, 0.5 + s * 0.44),
                );
            }
        }
        Icon::Key => {
            painter.circle_stroke(p(0.3, 0.5), r.width() * 0.18, stroke);
            line(p(0.46, 0.5), p(0.9, 0.5));
            line(p(0.76, 0.5), p(0.76, 0.68));
            line(p(0.86, 0.5), p(0.86, 0.64));
        }
        Icon::Help => {
            painter.circle_stroke(p(0.5, 0.5), r.width() * 0.42, stroke);
            painter.text(
                p(0.5, 0.48),
                egui::Align2::CENTER_CENTER,
                "?",
                egui::FontId::proportional(rect.height() * 0.46),
                color,
            );
        }
        Icon::Layers => {
            poly(
                vec![p(0.5, 0.14), p(0.9, 0.36), p(0.5, 0.58), p(0.1, 0.36)],
                Color32::TRANSPARENT,
            );
            line(p(0.1, 0.54), p(0.5, 0.76));
            line(p(0.5, 0.76), p(0.9, 0.54));
            line(p(0.1, 0.72), p(0.5, 0.94));
            line(p(0.5, 0.94), p(0.9, 0.72));
        }
        Icon::Grid => {
            line(p(0.16, 0.16), p(0.16, 0.84));
            line(p(0.5, 0.16), p(0.5, 0.84));
            line(p(0.84, 0.16), p(0.84, 0.84));
            line(p(0.16, 0.16), p(0.84, 0.16));
            line(p(0.16, 0.5), p(0.84, 0.5));
            line(p(0.16, 0.84), p(0.84, 0.84));
        }
        Icon::Close => {
            line(p(0.2, 0.2), p(0.8, 0.8));
            line(p(0.8, 0.2), p(0.2, 0.8));
        }
        Icon::Rotate => {
            let mut pts = Vec::new();
            for i in 0..=12 {
                let t = i as f32 / 12.0;
                let angle = std::f32::consts::PI * (0.1 + t * 0.9);
                pts.push(p(0.5 + angle.cos() * 0.34, 0.5 + angle.sin() * 0.34));
            }
            painter.add(Shape::line(pts, stroke));
            line(p(0.84, 0.5), p(0.84, 0.24));
            line(p(0.84, 0.24), p(0.6, 0.3));
        }
    }
}

/// Paints an icon centred on a rect (used by small buttons).
pub fn paint_centered(painter: &egui::Painter, icon: Icon, rect: Rect, color: Color32) {
    let side: Vec2 = Vec2::splat(rect.width().min(rect.height()));
    let square = Rect::from_center_size(rect.center(), side);
    paint(painter, icon, square, color);
}
