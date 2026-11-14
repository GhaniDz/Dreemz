//! Core document model: drawings, layers, keyframes and undo.
//!
//! A `Drawing` owns a full-canvas premultiplied RGBA bitmap. Drawings are
//! reference counted (`Arc`) so that undo snapshots and layer duplication are
//! cheap: the bitmap is only copied when it is actually modified (copy-on-write).

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tiny_skia::{BlendMode, FilterQuality, Pixmap, PixmapPaint, Transform};

pub type DrawingId = u64;

/// Integer rectangle with exclusive `x1`/`y1`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IRect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IRect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Self {
        Self { x0, y0, x1, y1 }
    }

    pub fn is_empty(&self) -> bool {
        self.x1 <= self.x0 || self.y1 <= self.y0
    }

    pub fn width(&self) -> i32 {
        (self.x1 - self.x0).max(0)
    }

    pub fn height(&self) -> i32 {
        (self.y1 - self.y0).max(0)
    }

    pub fn union(&self, other: IRect) -> IRect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        IRect {
            x0: self.x0.min(other.x0),
            y0: self.y0.min(other.y0),
            x1: self.x1.max(other.x1),
            y1: self.y1.max(other.y1),
        }
    }

    pub fn clamp(&self, w: u32, h: u32) -> IRect {
        let r = IRect {
            x0: self.x0.max(0),
            y0: self.y0.max(0),
            x1: self.x1.min(w as i32),
            y1: self.y1.min(h as i32),
        };
        if r.is_empty() {
            IRect::default()
        } else {
            r
        }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        !self.is_empty()
            && x >= self.x0 as f32
            && y >= self.y0 as f32
            && x < self.x1 as f32
            && y < self.y1 as f32
    }

    pub fn translated(&self, dx: i32, dy: i32) -> IRect {
        IRect::new(self.x0 + dx, self.y0 + dy, self.x1 + dx, self.y1 + dy)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BlendKind {
    #[default]
    Normal,
    Multiply,
    Screen,
    Overlay,
    Darken,
    Lighten,
    Add,
    Difference,
    Hue,
    Saturation,
    Color,
    Luminosity,
}

impl BlendKind {
    pub const ALL: [BlendKind; 12] = [
        BlendKind::Normal,
        BlendKind::Multiply,
        BlendKind::Screen,
        BlendKind::Overlay,
        BlendKind::Darken,
        BlendKind::Lighten,
        BlendKind::Add,
        BlendKind::Difference,
        BlendKind::Hue,
        BlendKind::Saturation,
        BlendKind::Color,
        BlendKind::Luminosity,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BlendKind::Normal => "Normal",
            BlendKind::Multiply => "Multiply",
            BlendKind::Screen => "Screen",
            BlendKind::Overlay => "Overlay",
            BlendKind::Darken => "Darken",
            BlendKind::Lighten => "Lighten",
            BlendKind::Add => "Add",
            BlendKind::Difference => "Difference",
            BlendKind::Hue => "Hue",
            BlendKind::Saturation => "Saturation",
            BlendKind::Color => "Color",
            BlendKind::Luminosity => "Luminosity",
        }
    }
}

/// A single image on a layer. `pixmap` is copy-on-write.
#[derive(Clone)]
pub struct Drawing {
    pub id: DrawingId,
    pub pixmap: Arc<Pixmap>,
    pub version: u64,
}

impl Drawing {
    pub fn new(id: DrawingId, width: u32, height: u32) -> Self {
        Self {
            id,
            pixmap: Arc::new(Pixmap::new(width, height).expect("valid canvas size")),
            version: 0,
        }
    }

    pub fn from_pixmap(id: DrawingId, pixmap: Pixmap, version: u64) -> Self {
        Self {
            id,
            pixmap: Arc::new(pixmap),
            version,
        }
    }

    pub fn touch(&mut self) {
        self.version = self.version.wrapping_add(1);
    }
}

#[derive(Clone)]
pub struct Layer {
    pub id: u64,
    pub name: String,
    pub visible: bool,
    pub locked: bool,
    /// 0.0 ..= 1.0
    pub opacity: f32,
    pub blend: BlendKind,
    /// Keyframes keyed by their start frame. Each drawing is shown from its
    /// start frame until the next keyframe (i.e. keyframes "hold").
    pub frames: BTreeMap<u32, Drawing>,
}

impl Layer {
    pub fn new(id: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            visible: true,
            locked: false,
            opacity: 1.0,
            blend: BlendKind::Normal,
            frames: BTreeMap::new(),
        }
    }

    /// The drawing visible at `frame` (the latest keyframe at or before it).
    pub fn drawing_at(&self, frame: u32) -> Option<&Drawing> {
        self.frames.range(..=frame).next_back().map(|(_, d)| d)
    }

    pub fn drawing_at_mut(&mut self, frame: u32) -> Option<&mut Drawing> {
        let key = *self.frames.range(..=frame).next_back().map(|(k, _)| k)?;
        self.frames.get_mut(&key)
    }

    pub fn keyframe_at(&self, frame: u32) -> Option<&Drawing> {
        self.frames.get(&frame)
    }

    #[allow(dead_code)]
    pub fn keyframe_frames(&self) -> Vec<u32> {
        self.frames.keys().copied().collect()
    }

    pub fn prev_keyframe(&self, frame: u32) -> Option<u32> {
        self.frames.range(..frame).next_back().map(|(k, _)| *k)
    }

    pub fn next_keyframe(&self, frame: u32) -> Option<u32> {
        self.frames
            .range(frame.saturating_add(1)..)
            .next()
            .map(|(k, _)| *k)
    }

    /// End of the hold that starts at `frame` (exclusive), if any.
    pub fn hold_end(&self, frame: u32, frame_count: u32) -> u32 {
        self.frames
            .range(frame.saturating_add(1)..)
            .next()
            .map(|(k, _)| *k)
            .unwrap_or(frame_count)
    }

    #[allow(dead_code)]
    pub fn last_keyframe(&self) -> Option<u32> {
        self.frames.keys().next_back().copied()
    }
}

#[derive(Clone)]
pub struct Document {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
    pub frame_count: u32,
    pub playhead: u32,
    /// Straight (non-premultiplied) sRGB.
    pub background: [u8; 4],
    /// Bottom-to-top.
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    pub next_id: u64,
}

impl Document {
    pub fn new(name: impl Into<String>, width: u32, height: u32, fps: f32, frames: u32) -> Self {
        let mut doc = Self {
            name: name.into(),
            width,
            height,
            fps: fps.clamp(1.0, 120.0),
            frame_count: frames.max(1),
            playhead: 0,
            background: [255, 255, 255, 255],
            layers: Vec::new(),
            active_layer: 0,
            next_id: 1,
        };
        let id = doc.alloc_id();
        let mut layer = Layer::new(id, "Layer 1");
        let drawing_id = doc.next_id;
        doc.next_id += 1;
        layer
            .frames
            .insert(0, Drawing::new(drawing_id, width, height));
        doc.layers.push(layer);
        doc
    }

    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn active_layer(&self) -> &Layer {
        &self.layers[self.active_layer.min(self.layers.len() - 1)]
    }

    #[allow(dead_code)]
    pub fn active_layer_mut(&mut self) -> &mut Layer {
        let idx = self.active_layer.min(self.layers.len() - 1);
        &mut self.layers[idx]
    }

    pub fn set_active_layer(&mut self, idx: usize) {
        if !self.layers.is_empty() {
            self.active_layer = idx.min(self.layers.len() - 1);
        }
    }

    #[allow(dead_code)]
    pub fn active_drawing(&self) -> Option<&Drawing> {
        self.layers
            .get(self.active_layer)
            .and_then(|l| l.drawing_at(self.playhead))
    }

    /// The active drawing, or a freshly created keyframe at the playhead.
    ///
    /// Painting on a held frame auto-keys: the held drawing is duplicated at
    /// the playhead (sharing the bitmap copy-on-write) so the earlier frames
    /// stay intact.
    pub fn ensure_active_drawing(&mut self) -> Option<&mut Drawing> {
        let idx = self.active_layer.min(self.layers.len() - 1);
        let frame = self.playhead;
        let (w, h) = (self.width, self.height);
        if !self.layers[idx].frames.contains_key(&frame) {
            let source = self.layers[idx].drawing_at(frame).map(|d| d.pixmap.clone());
            let id = self.alloc_id();
            let mut drawing = Drawing::new(id, w, h);
            if let Some(pm) = source {
                drawing.pixmap = pm;
            }
            self.layers[idx].frames.insert(frame, drawing);
        }
        self.layers[idx].drawing_at_mut(frame)
    }

    pub fn add_layer(&mut self) -> usize {
        let id = self.alloc_id();
        let name = format!("Layer {}", self.layers.len() + 1);
        self.layers.push(Layer::new(id, name));
        self.active_layer = self.layers.len() - 1;
        self.active_layer
    }

    pub fn duplicate_layer(&mut self, idx: usize) -> Option<usize> {
        if idx >= self.layers.len() {
            return None;
        }
        let id = self.alloc_id();
        let mut copy = self.layers[idx].clone();
        copy.id = id;
        copy.name = format!("{} copy", copy.name);
        let mut frames = BTreeMap::new();
        for (frame, drawing) in copy.frames {
            let mut d = drawing;
            d.id = self.alloc_id();
            d.version = 0;
            d.pixmap = Arc::new((*d.pixmap).clone());
            frames.insert(frame, d);
        }
        copy.frames = frames;
        self.layers.insert(idx + 1, copy);
        self.active_layer = idx + 1;
        Some(self.active_layer)
    }

    /// Inserts a copy of `layer` above the active layer, giving the copy and
    /// all of its drawings fresh ids. Returns the new active layer index.
    pub fn paste_layer(&mut self, mut layer: Layer) -> usize {
        layer.id = self.alloc_id();
        layer.name = format!("{} copy", layer.name);
        let mut frames = BTreeMap::new();
        for (frame, mut drawing) in layer.frames {
            drawing.id = self.alloc_id();
            drawing.version = 0;
            drawing.pixmap = Arc::new((*drawing.pixmap).clone());
            frames.insert(frame, drawing);
        }
        layer.frames = frames;
        let insert_at = (self.active_layer + 1).min(self.layers.len());
        self.layers.insert(insert_at, layer);
        self.active_layer = insert_at;
        insert_at
    }

    pub fn delete_layer(&mut self, idx: usize) -> bool {
        if self.layers.len() <= 1 || idx >= self.layers.len() {
            return false;
        }
        self.layers.remove(idx);
        self.active_layer = self.active_layer.min(self.layers.len() - 1);
        true
    }

    /// Flattens layer `idx` into the layer below it, preserving every
    /// keyframe boundary of either layer. The upper layer's opacity is baked
    /// in; its blend mode is reduced to normal compositing.
    pub fn merge_down(&mut self, idx: usize) -> bool {
        if idx == 0 || idx >= self.layers.len() {
            return false;
        }
        if self.layers[idx].locked || self.layers[idx - 1].locked {
            return false;
        }
        let (w, h) = (self.width, self.height);
        let mut boundaries: BTreeSet<u32> =
            self.layers[idx - 1].frames.keys().copied().collect();
        boundaries.extend(self.layers[idx].frames.keys().copied());

        let opacity = self.layers[idx].opacity.clamp(0.0, 1.0);
        let mut merged: BTreeMap<u32, Pixmap> = BTreeMap::new();
        for frame in boundaries {
            let mut pm = match self.layers[idx - 1].drawing_at(frame) {
                Some(drawing) => (*drawing.pixmap).clone(),
                None => Pixmap::new(w, h).expect("valid canvas size"),
            };
            if let Some(upper) = self.layers[idx].drawing_at(frame) {
                let paint = PixmapPaint {
                    opacity,
                    blend_mode: BlendMode::SourceOver,
                    quality: FilterQuality::Nearest,
                };
                pm.draw_pixmap(
                    0,
                    0,
                    upper.pixmap.as_ref().as_ref(),
                    &paint,
                    Transform::identity(),
                    None,
                );
            }
            merged.insert(frame, pm);
        }

        let fresh: Vec<u32> = merged
            .keys()
            .copied()
            .filter(|frame| !self.layers[idx - 1].frames.contains_key(frame))
            .collect();
        let fresh_ids: Vec<u64> = fresh.iter().map(|_| self.alloc_id()).collect();

        let lower = &mut self.layers[idx - 1];
        for (frame, pixmap) in merged {
            if let Some(drawing) = lower.frames.get_mut(&frame) {
                drawing.pixmap = Arc::new(pixmap);
                drawing.touch();
            } else if let Some(pos) = fresh.iter().position(|f| *f == frame) {
                lower.frames.insert(
                    frame,
                    Drawing::from_pixmap(fresh_ids[pos], pixmap, 0),
                );
            }
        }

        self.layers.remove(idx);
        self.active_layer = idx - 1;
        true
    }

    pub fn move_layer_up(&mut self, idx: usize) -> bool {
        if idx + 1 >= self.layers.len() {
            return false;
        }
        self.layers.swap(idx, idx + 1);
        self.active_layer = idx + 1;
        true
    }

    pub fn move_layer_down(&mut self, idx: usize) -> bool {
        if idx == 0 || idx >= self.layers.len() {
            return false;
        }
        self.layers.swap(idx, idx - 1);
        self.active_layer = idx - 1;
        true
    }

    pub fn find_drawing(&self, id: DrawingId) -> Option<&Drawing> {
        for layer in &self.layers {
            for drawing in layer.frames.values() {
                if drawing.id == id {
                    return Some(drawing);
                }
            }
        }
        None
    }

    pub fn find_drawing_mut(&mut self, id: DrawingId) -> Option<&mut Drawing> {
        for layer in &mut self.layers {
            for drawing in layer.frames.values_mut() {
                if drawing.id == id {
                    return Some(drawing);
                }
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn drawing_at(&self, layer_idx: usize, frame: u32) -> Option<&Drawing> {
        self.layers.get(layer_idx).and_then(|l| l.drawing_at(frame))
    }

    pub fn insert_keyframe(&mut self, layer_idx: usize, frame: u32, duplicate: bool) -> Option<u32> {
        if layer_idx >= self.layers.len() || frame >= self.frame_count {
            return None;
        }
        if self.layers[layer_idx].frames.contains_key(&frame) {
            return None;
        }
        let id = self.alloc_id();
        let source = self.layers[layer_idx].drawing_at(frame).cloned();
        let drawing = match (duplicate, source) {
            (true, Some(src)) => {
                let mut d = Drawing::new(id, self.width, self.height);
                d.pixmap = Arc::new((*src.pixmap).clone());
                d
            }
            _ => Drawing::new(id, self.width, self.height),
        };
        self.layers[layer_idx].frames.insert(frame, drawing);
        Some(frame)
    }

    pub fn delete_keyframe(&mut self, layer_idx: usize, frame: u32) -> bool {
        self.layers
            .get_mut(layer_idx)
            .map(|l| l.frames.remove(&frame).is_some())
            .unwrap_or(false)
    }

    pub fn move_keyframe(&mut self, layer_idx: usize, from: u32, to: u32) -> bool {
        if layer_idx >= self.layers.len() || from == to || to >= self.frame_count {
            return false;
        }
        let layer = &mut self.layers[layer_idx];
        if layer.frames.contains_key(&to) {
            return false;
        }
        if let Some(d) = layer.frames.remove(&from) {
            layer.frames.insert(to, d);
            true
        } else {
            false
        }
    }

    pub fn clear_active_frame(&mut self) -> bool {
        let idx = self.active_layer;
        if idx >= self.layers.len() || self.layers[idx].frames.is_empty() {
            return false;
        }
        // Clearing a held frame must not wipe the earlier keyframe; auto-key
        // first, exactly like painting does.
        let (w, h) = (self.width, self.height);
        let Some(d) = self.ensure_active_drawing() else {
            return false;
        };
        let mut fresh = Pixmap::new(w, h).expect("canvas size");
        std::mem::swap(&mut fresh, Arc::make_mut(&mut d.pixmap));
        d.touch();
        true
    }

    pub fn set_playhead(&mut self, frame: u32) {
        self.playhead = frame.min(self.frame_count.saturating_sub(1));
    }

    /// Moves the record head one frame forward. Recording grows the timeline
    /// instead of stopping at the end, so a performance is never cut short.
    /// Returns false once the hard frame limit is reached.
    pub fn advance_record_head(&mut self) -> bool {
        if self.frame_count >= 100_000 && self.playhead + 1 >= self.frame_count {
            return false;
        }
        self.playhead += 1;
        if self.playhead >= self.frame_count {
            self.frame_count = self.playhead + 1;
        }
        true
    }

    pub fn step_frame(&mut self, delta: i32) {
        let f = self.playhead as i32 + delta;
        let max = self.frame_count.saturating_sub(1) as i32;
        self.playhead = f.clamp(0, max) as u32;
    }

    pub fn resize_frame_count(&mut self, frames: u32) {
        self.frame_count = frames.clamp(1, 100_000);
        self.playhead = self.playhead.min(self.frame_count - 1);
    }

    #[allow(dead_code)]
    pub fn active_layer_has_keyframe(&self) -> bool {
        self.layers
            .get(self.active_layer)
            .map(|l| l.frames.contains_key(&self.playhead))
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn any_visible_layer(&self) -> bool {
        self.layers.iter().any(|l| l.visible)
    }
}

/// One undoable step.
pub enum HistoryEntry {
    /// Full structural snapshot. Bitmaps are shared through `Arc`, so this is
    /// cheap unless a bitmap was modified afterwards.
    Doc(Box<Document>),
    /// A rectangular patch for a brush stroke.
    Pixels {
        drawing_id: DrawingId,
        rect: IRect,
        before: Vec<u8>,
        after: Vec<u8>,
    },
}

impl HistoryEntry {
    /// Number of pixels covered by a pixel patch (zero for structural snapshots).
    #[allow(dead_code)]
    pub fn pixel_count(&self) -> usize {
        match self {
            HistoryEntry::Pixels { rect, .. } => {
                (rect.width().max(0) * rect.height().max(0)) as usize
            }
            HistoryEntry::Doc(_) => 0,
        }
    }
}

pub struct History {
    pub undo: Vec<HistoryEntry>,
    pub redo: Vec<HistoryEntry>,
    limit: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            limit: 64,
        }
    }
}

impl History {
    pub fn begin_doc(&mut self, doc: &Document) {
        self.undo.push(HistoryEntry::Doc(Box::new(doc.clone())));
        self.redo.clear();
        self.trim();
    }

    pub fn push_pixels(&mut self, entry: HistoryEntry) {
        self.undo.push(entry);
        self.redo.clear();
        self.trim();
    }

    pub fn drop_last(&mut self) {
        self.undo.pop();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn trim(&mut self) {
        if self.undo.len() > self.limit {
            let excess = self.undo.len() - self.limit;
            self.undo.drain(0..excess);
        }
    }
}
