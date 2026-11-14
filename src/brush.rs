//! Raster brush engine.
//!
//! A stroke is accumulated in an 8-bit coverage mask at full strength, and is
//! composited from an immutable snapshot of the layer. That means overlapping
//! dabs never build up opacity and an in-progress stroke can be re-composited
//! incrementally (only the freshly touched rectangle) without smearing.

use std::sync::Arc;

use egui::Color32;
use tiny_skia::Pixmap;

use crate::model::{Document, DrawingId, HistoryEntry, IRect};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrushPreset {
    Pencil,
    Ink,
    Marker,
    Airbrush,
    Soft,
}

impl BrushPreset {
    pub const ALL: [BrushPreset; 5] = [
        BrushPreset::Pencil,
        BrushPreset::Ink,
        BrushPreset::Marker,
        BrushPreset::Airbrush,
        BrushPreset::Soft,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BrushPreset::Pencil => "Pencil",
            BrushPreset::Ink => "Ink",
            BrushPreset::Marker => "Marker",
            BrushPreset::Airbrush => "Airbrush",
            BrushPreset::Soft => "Soft",
        }
    }

    pub fn apply(self, b: &mut BrushSettings) {
        match self {
            BrushPreset::Pencil => {
                b.size = 8.0;
                b.opacity = 1.0;
                b.hardness = 0.95;
                b.spacing = 0.08;
                b.smoothing = 0.15;
                b.pressure_size = 0.8;
            }
            BrushPreset::Ink => {
                b.size = 16.0;
                b.opacity = 1.0;
                b.hardness = 0.92;
                b.spacing = 0.05;
                b.smoothing = 0.55;
                b.pressure_size = 0.55;
            }
            BrushPreset::Marker => {
                b.size = 32.0;
                b.opacity = 0.72;
                b.hardness = 0.78;
                b.spacing = 0.10;
                b.smoothing = 0.40;
                b.pressure_size = 0.30;
            }
            BrushPreset::Airbrush => {
                b.size = 72.0;
                b.opacity = 0.22;
                b.hardness = 0.0;
                b.spacing = 0.05;
                b.smoothing = 0.30;
                b.pressure_size = 0.25;
            }
            BrushPreset::Soft => {
                b.size = 40.0;
                b.opacity = 0.65;
                b.hardness = 0.35;
                b.spacing = 0.08;
                b.smoothing = 0.45;
                b.pressure_size = 0.5;
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct BrushSettings {
    /// Diameter in canvas pixels.
    pub size: f32,
    pub opacity: f32,
    pub hardness: f32,
    /// Distance between dabs, as a fraction of the brush diameter.
    pub spacing: f32,
    /// 0 = raw input, 0.9 = very smooth / laggy.
    pub smoothing: f32,
    /// How strongly pressure affects width.
    pub pressure_size: f32,
    pub simulate_pressure: bool,
    pub color: Color32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            size: 12.0,
            opacity: 1.0,
            hardness: 0.85,
            spacing: 0.10,
            smoothing: 0.35,
            pressure_size: 0.6,
            simulate_pressure: false,
            color: Color32::from_rgb(24, 24, 28),
        }
    }
}

pub struct Stroke {
    pub drawing_id: DrawingId,
    pub erase: bool,
    mask: Vec<u8>,
    snapshot: Arc<Pixmap>,
    color: [u8; 4],
    opacity: f32,
    hardness: f32,
    smoothing: f32,
    pressure_size: f32,
    spacing: f32,
    carry: f32,
    last_input: (f32, f32),
    last_width: f32,
    smooth_pos: (f32, f32),
    started: bool,
    /// Total area touched by the whole stroke.
    pub bounds: IRect,
}

fn brush_radius(brush_size: f32, pressure_size: f32, pressure: f32) -> f32 {
    let p = pressure.clamp(0.02, 1.0);
    let scale = 1.0 - pressure_size + pressure_size * p;
    (brush_size * scale * 0.5).max(0.35)
}

impl Stroke {
    pub fn start(
        doc: &mut Document,
        brush: &BrushSettings,
        pos: (f32, f32),
        pressure: f32,
        erase: bool,
    ) -> Option<Self> {
        let layer_idx = doc.active_layer.min(doc.layers.len().saturating_sub(1));
        let layer = doc.layers.get(layer_idx)?;
        if layer.locked || !layer.visible {
            return None;
        }
        let drawing = doc.ensure_active_drawing()?;
        let drawing_id = drawing.id;

        // Keep the original bitmap for compositing this stroke and for undo,
        // then make the layer's bitmap unique (copy-on-write).
        let snapshot = drawing.pixmap.clone();
        let pixmap = Arc::make_mut(&mut drawing.pixmap);
        let (w, h) = (pixmap.width(), pixmap.height());

        let radius = brush_radius(brush.size, brush.pressure_size, pressure);
        let mut stroke = Self {
            drawing_id,
            erase,
            mask: vec![0; (w as usize) * (h as usize)],
            snapshot,
            color: [
                brush.color.r(),
                brush.color.g(),
                brush.color.b(),
                brush.color.a(),
            ],
            opacity: brush.opacity.clamp(0.0, 1.0),
            hardness: brush.hardness.clamp(0.0, 1.0),
            smoothing: brush.smoothing.clamp(0.0, 0.92),
            pressure_size: brush.pressure_size.clamp(0.0, 1.0),
            spacing: (brush.size * brush.spacing.clamp(0.02, 2.0)).max(0.55),
            carry: 0.0,
            last_input: pos,
            last_width: radius * 2.0,
            smooth_pos: pos,
            started: true,
            bounds: IRect::default(),
        };
        stroke.stamp(pos, radius, w, h);
        stroke.composite(doc, w, h);
        Some(stroke)
    }

    pub fn add_point(
        &mut self,
        doc: &mut Document,
        brush: &BrushSettings,
        pos: (f32, f32),
        pressure: f32,
    ) {
        if !self.started {
            return;
        }
        let smooth = self.smooth_pos;
        let k = 1.0 - self.smoothing;
        let target = (
            smooth.0 + (pos.0 - smooth.0) * k,
            smooth.1 + (pos.1 - smooth.1) * k,
        );

        let from = self.last_input;
        let dx = target.0 - from.0;
        let dy = target.1 - from.1;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 0.001 {
            return;
        }

        let brush_radius_now = brush_radius(brush.size, self.pressure_size, pressure);
        let spacing = self.spacing;
        let mut dist = spacing - self.carry;
        let mut any = false;
        let mut tmp_bounds = IRect::default();
        let (w, h) = (doc.width, doc.height);

        while dist <= len {
            let t = dist / len;
            let p = (from.0 + dx * t, from.1 + dy * t);
            let radius = self.last_width * 0.5 + (brush_radius_now - self.last_width * 0.5) * t;
            tmp_bounds = tmp_bounds.union(self.stamp(p, radius, w, h));
            dist += spacing;
            any = true;
        }
        if any {
            self.carry = len - (dist - spacing);
        } else {
            self.carry += len;
        }
        self.carry = self.carry.min(spacing);

        self.smooth_pos = target;
        self.last_input = target;
        self.last_width = brush_radius_now * 2.0;

        self.bounds = self.bounds.union(tmp_bounds);
        self.composite(doc, w, h);
        if let Some(d) = doc.find_drawing_mut(self.drawing_id) {
            d.touch();
        }
    }

    pub fn finish(
        mut self,
        doc: &mut Document,
        brush: &BrushSettings,
        pos: (f32, f32),
        pressure: f32,
    ) -> Option<HistoryEntry> {
        self.add_point(doc, brush, pos, pressure);
        // A final dab exactly at the pointer position.
        let (w, h) = (doc.width, doc.height);
        let r = brush_radius(brush.size, brush.pressure_size, pressure);
        let b = self.stamp(pos, r, w, h);
        self.bounds = self.bounds.union(b);
        self.composite(doc, w, h);
        if let Some(d) = doc.find_drawing_mut(self.drawing_id) {
            d.touch();
        }

        let rect = self.bounds.clamp(w, h);
        if rect.is_empty() {
            return None;
        }
        let before = crop_pixmap(&self.snapshot, rect);
        let after = doc
            .find_drawing(self.drawing_id)
            .map(|d| crop_pixmap(&d.pixmap, rect))
            .unwrap_or_default();
        Some(HistoryEntry::Pixels {
            drawing_id: self.drawing_id,
            rect,
            before,
            after,
        })
    }

    fn stamp(&mut self, pos: (f32, f32), radius: f32, w: u32, h: u32) -> IRect {
        let r = radius.max(0.35);
        let x0 = (pos.0 - r - 1.0).floor() as i32;
        let y0 = (pos.1 - r - 1.0).floor() as i32;
        let x1 = (pos.0 + r + 1.0).ceil() as i32;
        let y1 = (pos.1 + r + 1.0).ceil() as i32;
        let rect = IRect::new(x0, y0, x1, y1).clamp(w, h);
        if rect.is_empty() {
            return rect;
        }

        let inner = r * self.hardness.clamp(0.0, 1.0);
        let soft_range = (r - inner).max(0.0001);
        let hard = self.hardness >= 0.995;
        let mask = &mut self.mask;
        for py in rect.y0..rect.y1 {
            let dy = (py as f32 + 0.5) - pos.1;
            let row = py as usize * w as usize;
            for px in rect.x0..rect.x1 {
                let dx = (px as f32 + 0.5) - pos.0;
                let dist = (dx * dx + dy * dy).sqrt();
                let aa = (r + 0.5 - dist).clamp(0.0, 1.0);
                if aa <= 0.0 {
                    continue;
                }
                let cov = if hard {
                    aa
                } else {
                    let t = ((r - dist) / soft_range).clamp(0.0, 1.0);
                    aa * t * t * (3.0 - 2.0 * t)
                };
                let v = (cov * 255.0).round() as u8;
                let idx = row + px as usize;
                if v > mask[idx] {
                    mask[idx] = v;
                }
            }
        }
        rect
    }

    fn composite(&mut self, doc: &mut Document, w: u32, h: u32) {
        if self.bounds.is_empty() {
            return;
        }
        let rect = self.bounds.clamp(w, h);
        if rect.is_empty() {
            return;
        }
        let Some(drawing) = doc.find_drawing_mut(self.drawing_id) else {
            return;
        };
        let pm = Arc::make_mut(&mut drawing.pixmap);
        let dst = pm.data_mut();
        let src = self.snapshot.data();
        let mask = &self.mask;
        let brush_a = self.color[3] as f32 / 255.0;
        let opacity = self.opacity * brush_a;
        let cr = self.color[0] as f32;
        let cg = self.color[1] as f32;
        let cb = self.color[2] as f32;

        for y in rect.y0..rect.y1 {
            let row = y as usize * w as usize;
            for x in rect.x0..rect.x1 {
                let idx = row + x as usize;
                let m = mask[idx] as f32 / 255.0 * opacity;
                if m <= 0.0 {
                    continue;
                }
                let p = idx * 4;
                let da = src[p + 3] as f32;
                let dr = src[p] as f32;
                let dg = src[p + 1] as f32;
                let db = src[p + 2] as f32;
                if self.erase {
                    let f = 1.0 - m;
                    dst[p] = (dr * f).round().clamp(0.0, 255.0) as u8;
                    dst[p + 1] = (dg * f).round().clamp(0.0, 255.0) as u8;
                    dst[p + 2] = (db * f).round().clamp(0.0, 255.0) as u8;
                    dst[p + 3] = (da * f).round().clamp(0.0, 255.0) as u8;
                } else {
                    let f = 1.0 - m;
                    dst[p] = (cr * m + dr * f).round().clamp(0.0, 255.0) as u8;
                    dst[p + 1] = (cg * m + dg * f).round().clamp(0.0, 255.0) as u8;
                    dst[p + 2] = (cb * m + db * f).round().clamp(0.0, 255.0) as u8;
                    dst[p + 3] = (m * 255.0 + da * f).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

pub fn crop_pixmap(pm: &Pixmap, rect: IRect) -> Vec<u8> {
    let w = pm.width() as usize;
    let rect = rect.clamp(pm.width(), pm.height());
    let mut out = Vec::with_capacity((rect.width() * rect.height() * 4) as usize);
    if rect.is_empty() {
        return out;
    }
    let data = pm.data();
    for y in rect.y0..rect.y1 {
        let start = (y as usize * w + rect.x0 as usize) * 4;
        let end = start + rect.width() as usize * 4;
        out.extend_from_slice(&data[start..end]);
    }
    out
}

pub fn paste_pixmap(pm: &mut Pixmap, rect: IRect, data: &[u8]) {
    let w = pm.width() as usize;
    let rect = rect.clamp(pm.width(), pm.height());
    if rect.is_empty() {
        return;
    }
    let stride = rect.width() as usize * 4;
    let dst = pm.data_mut();
    for (row, y) in (rect.y0..rect.y1).enumerate() {
        let start = (y as usize * w + rect.x0 as usize) * 4;
        let src_start = row * stride;
        if src_start + stride <= data.len() {
            dst[start..start + stride].copy_from_slice(&data[src_start..src_start + stride]);
        }
    }
}
