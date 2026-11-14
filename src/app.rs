//! Application state, menus, shortcuts, playback and file operations.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use egui::{Color32, Key, Modifiers, Pos2, Vec2};
use tiny_skia::Pixmap;

use crate::brush::{BrushPreset, BrushSettings, Stroke};
use crate::model::{Document, History, HistoryEntry, IRect};
use crate::paint::{self, OnionSettings};
use crate::project::{self, ExportFormat, ExportSettings};
use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Brush,
    Eraser,
    Fill,
    Eyedropper,
    Move,
    Select,
    Hand,
}

impl Tool {
    pub const ALL: [Tool; 7] = [
        Tool::Brush,
        Tool::Eraser,
        Tool::Fill,
        Tool::Eyedropper,
        Tool::Move,
        Tool::Select,
        Tool::Hand,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Brush => "Brush",
            Tool::Eraser => "Eraser",
            Tool::Fill => "Fill",
            Tool::Eyedropper => "Eyedropper",
            Tool::Move => "Move",
            Tool::Select => "Select",
            Tool::Hand => "Hand",
        }
    }

    pub fn shortcut(self) -> &'static str {
        match self {
            Tool::Brush => "B",
            Tool::Eraser => "E",
            Tool::Fill => "G",
            Tool::Eyedropper => "I",
            Tool::Move => "V",
            Tool::Select => "M",
            Tool::Hand => "H",
        }
    }

    pub fn icon(self) -> crate::icons::Icon {
        use crate::icons::Icon;
        match self {
            Tool::Brush => Icon::Brush,
            Tool::Eraser => Icon::Eraser,
            Tool::Fill => Icon::Fill,
            Tool::Eyedropper => Icon::Picker,
            Tool::Move => Icon::Move,
            Tool::Select => Icon::Select,
            Tool::Hand => Icon::Hand,
        }
    }
}

/// Optional composition overlays drawn on top of the canvas.
#[derive(Clone, Copy, Debug, Default)]
pub struct Guides {
    pub thirds: bool,
    pub center: bool,
    pub safe: bool,
}

#[derive(Clone, Debug)]
pub struct ViewState {
    pub zoom: f32,
    pub pan: Vec2,
    /// Canvas rotation in radians.
    pub rotation: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan: Vec2::ZERO,
            rotation: 0.0,
        }
    }
}

pub struct MoveState {
    drawing_id: u64,
    snapshot: Arc<Pixmap>,
    start: Pos2,
    offset: (i32, i32),
}

pub struct SelectionMove {
    drawing_id: u64,
    snapshot: Arc<Pixmap>,
    rect: IRect,
    start: Pos2,
    offset: (i32, i32),
    structural: bool,
}

pub struct ThumbEntry {
    version: u64,
    built_at: f64,
    texture: egui::TextureHandle,
}

pub struct KeyframeDrag {
    pub layer: usize,
    pub from: u32,
    pub target: u32,
    pub moved: bool,
}

#[derive(Default)]
pub struct TimelineState {
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub drag: Option<KeyframeDrag>,
    pub frame_w: f32,
    pub scrubbing: bool,
    pub auto_width: bool,
    /// Layer/frame under the last right-click, used by the keyframe menu.
    pub context: Option<(usize, u32)>,
}

pub struct Dialogs {
    pub new_project: bool,
    pub settings: bool,
    pub export: bool,
    pub shortcuts: bool,
    pub about: bool,
    pub brush_presets: bool,
}

impl Default for Dialogs {
    fn default() -> Self {
        Self {
            new_project: false,
            settings: false,
            export: false,
            shortcuts: false,
            about: false,
            brush_presets: false,
        }
    }
}

pub struct NewProjectForm {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub fps: f32,
    pub frames: i32,
    pub background: Color32,
}

impl Default for NewProjectForm {
    fn default() -> Self {
        Self {
            name: "Untitled".into(),
            width: 1280,
            height: 720,
            fps: 24.0,
            frames: 48,
            background: Color32::WHITE,
        }
    }
}

impl NewProjectForm {
    pub const PRESETS: [(&'static str, i32, i32); 5] = [
        ("HD 1280 x 720", 1280, 720),
        ("Full HD 1920 x 1080", 1920, 1080),
        ("Square 1080 x 1080", 1080, 1080),
        ("Social 1080 x 1350", 1080, 1350),
        ("Sketch 1600 x 900", 1600, 900),
    ];
}

pub struct ProjectSettingsForm {
    pub name: String,
    pub width: i32,
    pub height: i32,
    pub fps: f32,
    pub frames: i32,
    pub background: Color32,
}

impl ProjectSettingsForm {
    pub fn for_doc(doc: &Document) -> Self {
        Self {
            name: doc.name.clone(),
            width: doc.width as i32,
            height: doc.height as i32,
            fps: doc.fps,
            frames: doc.frame_count as i32,
            background: Color32::from_rgba_unmultiplied(
                doc.background[0],
                doc.background[1],
                doc.background[2],
                doc.background[3],
            ),
        }
    }
}

pub struct ExportForm {
    pub format: ExportFormat,
    pub path: String,
    pub start: i32,
    pub end: i32,
    pub scale: f32,
    pub background: bool,
    /// Grid width used by the sprite-sheet export.
    pub columns: i32,
    pub result: Option<String>,
}

impl ExportForm {
    fn for_doc(doc: &Document) -> Self {
        Self {
            format: ExportFormat::Gif,
            path: default_export_path(doc),
            start: 0,
            end: doc.frame_count.saturating_sub(1) as i32,
            scale: 1.0,
            background: true,
            columns: doc.frame_count.min(8) as i32,
            result: None,
        }
    }
}

/// Returns a tinted copy of `drawing`, reusing a cached one when the drawing
/// has not changed. `cache` is kept small because it only holds onion frames.
fn cached_tinted(
    cache: &mut Vec<((u64, u64), Arc<Pixmap>)>,
    drawing: &crate::model::Drawing,
    tint: [f32; 3],
) -> Arc<Pixmap> {
    let key = (drawing.id, drawing.version);
    if let Some((_, pixmap)) = cache.iter().find(|(k, _)| *k == key) {
        return pixmap.clone();
    }
    let pixmap = Arc::new(paint::tinted(&drawing.pixmap, tint, 1.0));
    cache.retain(|(k, _)| *k != key);
    cache.push((key, pixmap.clone()));
    if cache.len() > 24 {
        cache.remove(0);
    }
    pixmap
}

fn default_export_path(doc: &Document) -> String {
    let stem = if doc.name.trim().is_empty() {
        "dreemz"
    } else {
        doc.name.trim()
    };
    let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    base.join(format!("{stem}.gif")).to_string_lossy().to_string()
}

pub struct DreemzApp {
    pub doc: Document,
    pub history: History,
    pub tool: Tool,
    pub last_paint_tool: Tool,
    pub brush: BrushSettings,
    pub eraser: BrushSettings,
    pub onion: OnionSettings,
    pub view: ViewState,
    pub playing: bool,
    pub looping: bool,
    /// Perform mode: strokes are recorded onto successive frames as they are
    /// drawn. Playback stays off until the take is committed.
    pub perform: bool,
    record_accum: f32,
    /// Frame the current take started on, used for the elapsed-time read-out.
    pub perform_start: u32,
    last_stroke_pos: Option<(f32, f32, f32)>,
    pub play_accum: f32,
    pub stroke: Option<Stroke>,
    stroke_structural: bool,
    pub move_state: Option<MoveState>,
    /// Committed rectangular selection in canvas coordinates.
    pub selection: Option<IRect>,
    /// Marquee corners while dragging a new selection.
    pub selection_draft: Option<(Pos2, Pos2)>,
    pub selection_move: Option<SelectionMove>,
    /// Pixels captured from a selection, ready to paste.
    pub region_clipboard: Option<Arc<Pixmap>>,
    pub guides: Guides,
    pub layer_thumbs: HashMap<u64, ThumbEntry>,
    pub canvas_texture: Option<egui::TextureHandle>,
    pub canvas_key: Option<u64>,
    pub composite: Option<Arc<Pixmap>>,
    pub onion_before_cache: Vec<((u64, u64), Arc<Pixmap>)>,
    pub onion_after_cache: Vec<((u64, u64), Arc<Pixmap>)>,
    pub file_path: Option<PathBuf>,
    /// Bitmap copied from a frame, ready to paste.
    pub frame_clipboard: Option<Arc<Pixmap>>,
    /// Whole layer copied from the layer panel, ready to paste.
    pub layer_clipboard: Option<crate::model::Layer>,
    pub dirty: bool,
    pub now: f64,
    pub status: Option<(String, f64)>,
    pub fill_tolerance: u8,
    pub fill_contiguous: bool,
    pub dialogs: Dialogs,
    pub new_project: NewProjectForm,
    pub settings_form: ProjectSettingsForm,
    pub export: ExportForm,
    pub timeline: TimelineState,
    pub needs_fit: bool,
    pub view_size: Vec2,
    pub last_title: String,
    pub renaming_layer: Option<usize>,
    pub layer_name_edit: String,
    pub recent_colors: Vec<Color32>,
    pub hover_canvas: Option<Pos2>,
    pub pointer_pressure: f32,
}

impl DreemzApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        theme::apply(&cc.egui_ctx);
        let doc = Document::new("Untitled", 1280, 720, 24.0, 48);
        let export = ExportForm::for_doc(&doc);
        let settings_form = ProjectSettingsForm::for_doc(&doc);
        let app = Self {
            doc,
            history: History::default(),
            tool: Tool::Brush,
            last_paint_tool: Tool::Brush,
            brush: BrushSettings::default(),
            eraser: BrushSettings {
                size: 32.0,
                opacity: 1.0,
                hardness: 0.9,
                spacing: 0.08,
                smoothing: 0.3,
                pressure_size: 0.4,
                simulate_pressure: false,
                color: Color32::BLACK,
            },
            onion: OnionSettings::default(),
            view: ViewState::default(),
            playing: false,
            looping: true,
            perform: false,
            record_accum: 0.0,
            perform_start: 0,
            last_stroke_pos: None,
            play_accum: 0.0,
            stroke: None,
            stroke_structural: false,
            move_state: None,
            selection: None,
            selection_draft: None,
            selection_move: None,
            region_clipboard: None,
            guides: Guides::default(),
            layer_thumbs: HashMap::new(),
            canvas_texture: None,
            canvas_key: None,
            composite: None,
            onion_before_cache: Vec::new(),
            onion_after_cache: Vec::new(),
            file_path: None,
            frame_clipboard: None,
            layer_clipboard: None,
            dirty: false,
            now: 0.0,
            status: None,
            fill_tolerance: 24,
            fill_contiguous: true,
            dialogs: Dialogs::default(),
            new_project: NewProjectForm::default(),
            settings_form,
            export,
            timeline: TimelineState {
                scroll_x: 0.0,
                scroll_y: 0.0,
                drag: None,
                frame_w: 14.0,
                scrubbing: false,
                auto_width: true,
                context: None,
            },
            needs_fit: true,
            view_size: Vec2::ZERO,
            last_title: String::new(),
            renaming_layer: None,
            layer_name_edit: String::new(),
            recent_colors: vec![
                Color32::from_rgb(24, 24, 28),
                Color32::WHITE,
                Color32::from_rgb(224, 82, 82),
                Color32::from_rgb(238, 168, 66),
                Color32::from_rgb(94, 178, 128),
                Color32::from_rgb(74, 132, 214),
            ],
            hover_canvas: None,
            pointer_pressure: 1.0,
        };
        app
    }

    // ---------------------------------------------------------------- status

    pub fn notify(&mut self, message: impl Into<String>) {
        self.status = Some((message.into(), self.now + 6.0));
    }

    pub fn status_text(&mut self) {
        if let Some((_, until)) = &self.status {
            if self.now > *until {
                self.status = None;
            }
        }
    }

    // ------------------------------------------------------------- canvas

    pub fn invalidate(&mut self) {
        self.canvas_key = None;
    }

    pub fn brush_settings(&self) -> &BrushSettings {
        if self.tool == Tool::Eraser {
            &self.eraser
        } else {
            &self.brush
        }
    }

    pub fn brush_settings_mut(&mut self) -> &mut BrushSettings {
        if self.tool == Tool::Eraser {
            &mut self.eraser
        } else {
            &mut self.brush
        }
    }

    pub fn canvas_to_screen(&self, view: egui::Rect, p: Pos2) -> Pos2 {
        let center = view.center() + self.view.pan;
        let rel = Vec2::new(
            (p.x - self.doc.width as f32 * 0.5) * self.view.zoom,
            (p.y - self.doc.height as f32 * 0.5) * self.view.zoom,
        );
        let (sin, cos) = self.view.rotation.sin_cos();
        center + Vec2::new(rel.x * cos - rel.y * sin, rel.x * sin + rel.y * cos)
    }

    pub fn screen_to_canvas(&self, view: egui::Rect, p: Pos2) -> Pos2 {
        let center = view.center() + self.view.pan;
        let d = p - center;
        let (sin, cos) = self.view.rotation.sin_cos();
        let rx = d.x * cos + d.y * sin;
        let ry = -d.x * sin + d.y * cos;
        Pos2::new(
            rx / self.view.zoom + self.doc.width as f32 * 0.5,
            ry / self.view.zoom + self.doc.height as f32 * 0.5,
        )
    }

    pub fn fit_view(&mut self, view: egui::Rect) {
        let w = (view.width() - 80.0).max(40.0);
        let h = (view.height() - 80.0).max(40.0);
        let zoom = (w / self.doc.width as f32).min(h / self.doc.height as f32);
        self.view.zoom = zoom.clamp(0.02, 32.0);
        self.view.pan = Vec2::ZERO;
        self.view.rotation = 0.0;
        self.needs_fit = false;
    }

    /// Rebuilds the composite texture when anything changed.
    pub fn update_canvas(&mut self, ctx: &egui::Context) {
        let key = paint::fingerprint(&self.doc, self.doc.playhead, &self.onion);
        if self.canvas_key == Some(key) {
            return;
        }
        let frame = self.doc.playhead;

        let mut before_pm: Vec<Arc<Pixmap>> = Vec::new();
        let mut after_pm: Vec<Arc<Pixmap>> = Vec::new();
        if self.onion.enabled {
            let layer = self.doc.active_layer();
            let mut previous = Vec::new();
            let mut cursor = frame;
            while previous.len() < self.onion.before as usize {
                match layer.prev_keyframe(cursor) {
                    Some(f) => {
                        previous.push(f);
                        cursor = f;
                    }
                    None => break,
                }
            }
            // Farthest frame first so the nearest onion drawing sits on top.
            for f in previous.iter().rev() {
                if let Some(drawing) = layer.keyframe_at(*f) {
                    before_pm.push(cached_tinted(
                        &mut self.onion_before_cache,
                        drawing,
                        paint::ONION_BEFORE_TINT,
                    ));
                }
            }

            let mut following = Vec::new();
            let mut cursor = frame;
            while following.len() < self.onion.after as usize {
                match layer.next_keyframe(cursor) {
                    Some(f) => {
                        following.push(f);
                        cursor = f;
                    }
                    None => break,
                }
            }
            for f in following.iter().rev() {
                if let Some(drawing) = layer.keyframe_at(*f) {
                    after_pm.push(cached_tinted(
                        &mut self.onion_after_cache,
                        drawing,
                        paint::ONION_AFTER_TINT,
                    ));
                }
            }
        }

        let composite = paint::composite_with_onion(
            &self.doc,
            frame,
            &before_pm,
            &after_pm,
            self.onion.opacity,
        );
        let size = [composite.width() as usize, composite.height() as usize];
        let image = egui::ColorImage::from_rgba_premultiplied(size, composite.data());
        match &mut self.canvas_texture {
            Some(tex) if tex.size() == size => {
                tex.set(image, egui::TextureOptions::LINEAR);
            }
            _ => {
                self.canvas_texture = Some(ctx.load_texture(
                    "dreemz_canvas",
                    image,
                    egui::TextureOptions::LINEAR,
                ));
            }
        }
        self.composite = Some(Arc::new(composite));
        self.canvas_key = Some(key);
    }

    // -------------------------------------------------------------- editing

    pub fn begin_edit(&mut self) {
        self.history.begin_doc(&self.doc);
        self.dirty = true;
    }

    pub fn doc_edit(&mut self, edit: impl FnOnce(&mut Document)) {
        self.begin_edit();
        edit(&mut self.doc);
        self.invalidate();
    }

    pub fn doc_edit_bool(&mut self, edit: impl FnOnce(&mut Document) -> bool) {
        self.history.begin_doc(&self.doc);
        if !edit(&mut self.doc) {
            self.history.drop_last();
        } else {
            self.dirty = true;
            self.invalidate();
        }
    }

    pub fn undo(&mut self) {
        let Some(entry) = self.history.undo.pop() else {
            return;
        };
        match entry {
            HistoryEntry::Doc(snapshot) => {
                let current = std::mem::replace(&mut self.doc, *snapshot);
                self.history.redo.push(HistoryEntry::Doc(Box::new(current)));
            }
            HistoryEntry::Pixels {
                drawing_id,
                rect,
                before,
                after,
            } => {
                let Some(drawing) = self.doc.find_drawing_mut(drawing_id) else {
                    return;
                };
                let pm = Arc::make_mut(&mut drawing.pixmap);
                let current = crate::brush::crop_pixmap(pm, rect);
                crate::brush::paste_pixmap(pm, rect, &before);
                drawing.touch();
                self.history.redo.push(HistoryEntry::Pixels {
                    drawing_id,
                    rect,
                    before: current,
                    after,
                });
            }
        }
        self.dirty = true;
        self.invalidate();
    }

    pub fn redo(&mut self) {
        let Some(entry) = self.history.redo.pop() else {
            return;
        };
        match entry {
            HistoryEntry::Doc(snapshot) => {
                let current = std::mem::replace(&mut self.doc, *snapshot);
                self.history.undo.push(HistoryEntry::Doc(Box::new(current)));
            }
            HistoryEntry::Pixels {
                drawing_id,
                rect,
                before: _,
                after,
            } => {
                let Some(drawing) = self.doc.find_drawing_mut(drawing_id) else {
                    return;
                };
                let pm = Arc::make_mut(&mut drawing.pixmap);
                let current = crate::brush::crop_pixmap(pm, rect);
                crate::brush::paste_pixmap(pm, rect, &after);
                drawing.touch();
                self.history.undo.push(HistoryEntry::Pixels {
                    drawing_id,
                    rect,
                    before: current,
                    after,
                });
            }
        }
        self.dirty = true;
        self.invalidate();
    }

    pub fn add_layer(&mut self) {
        self.doc_edit(|doc| {
            doc.add_layer();
        });
    }

    pub fn duplicate_layer(&mut self) {
        let idx = self.doc.active_layer;
        self.doc_edit(|doc| {
            doc.duplicate_layer(idx);
        });
    }

    pub fn copy_layer(&mut self) {
        let idx = self.doc.active_layer.min(self.doc.layers.len() - 1);
        let layer = self.doc.layers[idx].clone();
        let name = layer.name.clone();
        self.layer_clipboard = Some(layer);
        self.notify(format!("Copied layer '{name}'."));
    }

    pub fn cut_layer(&mut self) {
        if self.doc.layers.len() <= 1 {
            self.notify("A project needs at least one layer.");
            return;
        }
        self.copy_layer();
        self.delete_layer();
    }

    pub fn paste_layer(&mut self) {
        let Some(layer) = self.layer_clipboard.clone() else {
            self.notify("The layer clipboard is empty.");
            return;
        };
        let name = layer.name.clone();
        self.doc_edit(|doc| {
            doc.paste_layer(layer);
        });
        self.notify(format!("Pasted layer '{name}'."));
    }

    pub fn delete_layer(&mut self) {
        let idx = self.doc.active_layer;
        self.doc_edit_bool(|doc| doc.delete_layer(idx));
    }

    pub fn move_layer_up(&mut self) {
        let idx = self.doc.active_layer;
        self.doc_edit_bool(|doc| doc.move_layer_up(idx));
    }

    pub fn move_layer_down(&mut self) {
        let idx = self.doc.active_layer;
        self.doc_edit_bool(|doc| doc.move_layer_down(idx));
    }

    pub fn merge_down(&mut self) {
        let idx = self.doc.active_layer;
        let before = self.doc.layers.len();
        self.doc_edit_bool(|doc| doc.merge_down(idx));
        if self.doc.layers.len() < before {
            self.notify("Merged layer down.");
        } else {
            self.notify("Can't merge down here (bottom layer or locked).");
        }
    }

    /// Texture for the layer's current drawing, rebuilt when the bitmap
    /// changes. Rebuilds are throttled while a stroke or drag is in progress.
    pub fn layer_thumbnail(
        &mut self,
        ctx: &egui::Context,
        layer_idx: usize,
    ) -> Option<egui::TextureId> {
        let layer = self.doc.layers.get(layer_idx)?;
        let drawing = layer.drawing_at(self.doc.playhead)?;
        let (id, version, pixmap) = (drawing.id, drawing.version, drawing.pixmap.clone());
        Some(self.drawing_texture(ctx, id, version, &pixmap))
    }

    /// Texture for one drawing, cached by drawing id and rebuilt when the
    /// bitmap changes. Shared by the layer list and the timeline clips.
    pub fn drawing_texture(
        &mut self,
        ctx: &egui::Context,
        id: u64,
        version: u64,
        pixmap: &Arc<Pixmap>,
    ) -> egui::TextureId {
        let time = ctx.input(|i| i.time);
        let busy = self.stroke.is_some()
            || self.move_state.is_some()
            || self.selection_move.is_some();
        let needs = match self.layer_thumbs.get(&id) {
            Some(entry) => entry.version != version && (!busy || time - entry.built_at > 0.4),
            None => true,
        };
        if needs {
            let pm = paint::thumbnail(pixmap, 40, 24, self.doc.background);
            let size = [pm.width() as usize, pm.height() as usize];
            let image = egui::ColorImage::from_rgba_premultiplied(size, pm.data());
            let texture = ctx.load_texture(
                format!("dreemz_thumb_{id}"),
                image,
                egui::TextureOptions::LINEAR,
            );
            self.layer_thumbs.insert(
                id,
                ThumbEntry {
                    version,
                    built_at: time,
                    texture,
                },
            );
        }
        self.layer_thumbs
            .get(&id)
            .expect("thumbnail was just inserted")
            .texture
            .id()
    }

    pub fn add_keyframe(&mut self, duplicate: bool) {
        let layer = self.doc.active_layer;
        let frame = self.doc.playhead;
        self.doc_edit_bool(|doc| doc.insert_keyframe(layer, frame, duplicate).is_some());
    }

    /// Duplicates the drawing at the playhead. When the playhead already sits
    /// on a keyframe, the copy is placed on the next free frame.
    pub fn duplicate_keyframe(&mut self) {
        let layer = self.doc.active_layer;
        let frame = self.doc.playhead;
        let target = if self.doc.layers[layer].frames.contains_key(&frame) {
            let mut t = frame + 1;
            while t < self.doc.frame_count && self.doc.layers[layer].frames.contains_key(&t) {
                t += 1;
            }
            if t >= self.doc.frame_count {
                self.notify("No free frame to duplicate into — extend the timeline.");
                return;
            }
            t
        } else {
            frame
        };
        self.history.begin_doc(&self.doc);
        if self.doc.insert_keyframe(layer, target, true).is_some() {
            self.dirty = true;
            self.doc.set_playhead(target);
            self.invalidate();
        } else {
            self.history.drop_last();
        }
    }

    pub fn copy_keyframe(&mut self) {
        match self.doc.active_layer().drawing_at(self.doc.playhead) {
            Some(drawing) => {
                self.frame_clipboard = Some(drawing.pixmap.clone());
                self.notify("Copied drawing.");
            }
            None => self.notify("Nothing to copy on this frame."),
        }
    }

    pub fn cut_frame(&mut self) {
        if self.frame_clipboard.is_none() {
            self.copy_keyframe();
        }
        if self.frame_clipboard.is_some() {
            self.clear_frame();
        }
    }

    pub fn paste_keyframe(&mut self) {
        let Some(pixmap) = self.frame_clipboard.clone() else {
            self.notify("The frame clipboard is empty.");
            return;
        };
        let layer_idx = self.doc.active_layer.min(self.doc.layers.len() - 1);
        if self.doc.layers[layer_idx].locked || !self.doc.layers[layer_idx].visible {
            self.notify("Layer is locked or hidden.");
            return;
        }
        self.history.begin_doc(&self.doc);
        let Some(drawing) = self.doc.ensure_active_drawing() else {
            self.history.drop_last();
            return;
        };
        drawing.pixmap = pixmap;
        drawing.touch();
        self.dirty = true;
        self.invalidate();
        self.notify("Pasted drawing.");
    }

    /// Mirrors the current drawing in place, creating a keyframe if the frame
    /// is currently held.
    pub fn flip_drawing(&mut self, horizontal: bool) {
        let layer_idx = self.doc.active_layer.min(self.doc.layers.len() - 1);
        if self.doc.layers[layer_idx].locked || !self.doc.layers[layer_idx].visible {
            self.notify("Layer is locked or hidden.");
            return;
        }
        let auto_key = !self.doc.layers[layer_idx]
            .frames
            .contains_key(&self.doc.playhead);
        if auto_key {
            self.begin_edit();
        }
        let Some(drawing) = self.doc.ensure_active_drawing() else {
            if auto_key {
                self.history.drop_last();
            }
            return;
        };
        let drawing_id = drawing.id;
        let before = drawing.pixmap.clone();
        let flipped = paint::flip(&before, horizontal);
        drawing.pixmap = Arc::new(flipped);
        drawing.touch();
        if !auto_key {
            let after = self
                .doc
                .find_drawing(drawing_id)
                .map(|d| d.pixmap.clone())
                .unwrap_or_else(|| before.clone());
            if let Some(entry) = paint::translate_history(drawing_id, &before, &after) {
                self.history.push_pixels(entry);
            }
        }
        self.dirty = true;
        self.invalidate();
        self.notify(if horizontal {
            "Flipped horizontally."
        } else {
            "Flipped vertically."
        });
    }

    pub fn delete_keyframe(&mut self) {
        let layer = self.doc.active_layer;
        let frame = self.doc.playhead;
        if !self.doc.layers[layer].frames.contains_key(&frame) {
            self.notify("No keyframe on this frame.");
            return;
        }
        if self.doc.layers[layer].frames.len() <= 1 {
            self.notify("A layer needs at least one keyframe.");
            return;
        }
        self.doc_edit_bool(|doc| doc.delete_keyframe(layer, frame));
    }

    pub fn clear_frame(&mut self) {
        self.doc_edit_bool(|doc| doc.clear_active_frame());
    }

    // ------------------------------------------------------------ selection

    pub fn select_all(&mut self) {
        self.selection = Some(IRect::new(
            0,
            0,
            self.doc.width as i32,
            self.doc.height as i32,
        ));
        self.selection_draft = None;
        self.set_tool(Tool::Select);
        self.notify("Selected the whole canvas.");
    }

    pub fn deselect(&mut self) {
        self.selection = None;
        self.selection_draft = None;
    }

    /// The rectangle the marching ants should follow, including a live drag.
    pub fn selection_preview(&self) -> Option<IRect> {
        let rect = self.selection?;
        match &self.selection_move {
            Some(mv) if mv.offset != (0, 0) => Some(rect.translated(mv.offset.0, mv.offset.1)),
            _ => Some(rect),
        }
    }

    pub fn begin_selection_draft(&mut self, pos: (f32, f32)) {
        self.selection_move = None;
        let p = Pos2::new(pos.0, pos.1);
        self.selection_draft = Some((p, p));
    }

    pub fn update_selection_draft(&mut self, pos: (f32, f32)) {
        if let Some((start, _)) = self.selection_draft {
            self.selection_draft = Some((start, Pos2::new(pos.0, pos.1)));
        }
    }

    pub fn finish_selection_draft(&mut self) {
        let Some((a, b)) = self.selection_draft.take() else {
            return;
        };
        let rect = IRect::new(
            a.x.min(b.x).floor() as i32,
            a.y.min(b.y).floor() as i32,
            a.x.max(b.x).ceil() as i32,
            a.y.max(b.y).ceil() as i32,
        )
        .clamp(self.doc.width, self.doc.height);
        self.selection = if rect.width() < 2 || rect.height() < 2 {
            None
        } else {
            Some(rect)
        };
    }

    fn writable_layer(&mut self) -> Option<usize> {
        let idx = self.doc.active_layer.min(self.doc.layers.len().saturating_sub(1));
        if self.doc.layers[idx].locked || !self.doc.layers[idx].visible {
            self.notify("Layer is locked or hidden.");
            return None;
        }
        Some(idx)
    }

    pub fn begin_selection_move(&mut self, pos: (f32, f32)) {
        let Some(rect) = self.selection else {
            return;
        };
        let Some(idx) = self.writable_layer() else {
            return;
        };
        let structural = !self.doc.layers[idx].frames.contains_key(&self.doc.playhead);
        if structural {
            self.begin_edit();
        }
        let Some(drawing) = self.doc.ensure_active_drawing() else {
            if structural {
                self.history.drop_last();
            }
            return;
        };
        self.selection_move = Some(SelectionMove {
            drawing_id: drawing.id,
            snapshot: drawing.pixmap.clone(),
            rect,
            start: Pos2::new(pos.0, pos.1),
            offset: (0, 0),
            structural,
        });
        self.dirty = true;
    }

    pub fn update_selection_move(&mut self, pos: (f32, f32)) {
        let Some(mv) = &mut self.selection_move else {
            return;
        };
        let dx = (pos.0 - mv.start.x).round() as i32;
        let dy = (pos.1 - mv.start.y).round() as i32;
        if (dx, dy) == mv.offset {
            return;
        }
        mv.offset = (dx, dy);
        let moved = paint::moved_region(&mv.snapshot, mv.rect, dx, dy);
        if let Some(drawing) = self.doc.find_drawing_mut(mv.drawing_id) {
            drawing.pixmap = Arc::new(moved);
            drawing.touch();
        }
        self.invalidate();
    }

    pub fn end_selection_move(&mut self) {
        let Some(mv) = self.selection_move.take() else {
            return;
        };
        if mv.offset == (0, 0) {
            if mv.structural {
                self.history.drop_last();
            }
            return;
        }
        let after = self
            .doc
            .find_drawing(mv.drawing_id)
            .map(|d| d.pixmap.clone())
            .unwrap_or_else(|| mv.snapshot.clone());
        if !mv.structural {
            let union = mv.rect.union(mv.rect.translated(mv.offset.0, mv.offset.1));
            if let Some(entry) = paint::region_history(mv.drawing_id, &mv.snapshot, &after, union) {
                self.history.push_pixels(entry);
            }
        }
        if let Some(rect) = self.selection {
            let moved = rect.translated(mv.offset.0, mv.offset.1);
            let clamped = moved.clamp(self.doc.width, self.doc.height);
            self.selection = if clamped.is_empty() { None } else { Some(clamped) };
        }
        self.dirty = true;
        self.invalidate();
    }

    /// Copies the selected pixels into the region clipboard. Returns `true`
    /// when something was captured.
    pub fn copy_selection(&mut self) -> bool {
        let Some(rect) = self.selection else {
            self.notify("Nothing selected.");
            return false;
        };
        let Some(drawing) = self.doc.active_layer().drawing_at(self.doc.playhead) else {
            self.notify("Nothing to copy on this frame.");
            return false;
        };
        match paint::copy_region(&drawing.pixmap, rect) {
            Some(pm) => {
                self.region_clipboard = Some(Arc::new(pm));
                self.notify(format!("Copied {} x {} pixels.", rect.width(), rect.height()));
                true
            }
            None => {
                self.notify("Nothing to copy.");
                false
            }
        }
    }

    pub fn cut_selection(&mut self) {
        if self.copy_selection() {
            self.delete_selection();
        }
    }

    pub fn delete_selection(&mut self) {
        let Some(rect) = self.selection else {
            self.notify("Nothing selected.");
            return;
        };
        let Some(idx) = self.writable_layer() else {
            return;
        };
        let structural = !self.doc.layers[idx].frames.contains_key(&self.doc.playhead);
        if structural {
            self.begin_edit();
        }
        let Some(drawing) = self.doc.ensure_active_drawing() else {
            if structural {
                self.history.drop_last();
            }
            return;
        };
        let drawing_id = drawing.id;
        let snapshot = drawing.pixmap.clone();
        paint::clear_region(Arc::make_mut(&mut drawing.pixmap), rect);
        drawing.touch();
        if !structural {
            let after = self
                .doc
                .find_drawing(drawing_id)
                .map(|d| d.pixmap.clone())
                .unwrap_or_else(|| snapshot.clone());
            if let Some(entry) = paint::region_history(drawing_id, &snapshot, &after, rect) {
                self.history.push_pixels(entry);
            }
        }
        self.dirty = true;
        self.invalidate();
        self.notify("Cleared the selection.");
    }

    pub fn paste_selection(&mut self) {
        let Some(clipboard) = self.region_clipboard.clone() else {
            self.notify("The region clipboard is empty.");
            return;
        };
        let Some(idx) = self.writable_layer() else {
            return;
        };
        let (w, h) = (clipboard.width() as i32, clipboard.height() as i32);
        let (x, y) = match self.selection {
            Some(rect) => (rect.x0, rect.y0),
            None => (
                (self.doc.width as i32 - w) / 2,
                (self.doc.height as i32 - h) / 2,
            ),
        };
        let target = IRect::new(x, y, x + w, y + h).clamp(self.doc.width, self.doc.height);
        if target.is_empty() {
            self.notify("The selection is outside the canvas.");
            return;
        }
        let structural = !self.doc.layers[idx].frames.contains_key(&self.doc.playhead);
        if structural {
            self.begin_edit();
        }
        let Some(drawing) = self.doc.ensure_active_drawing() else {
            if structural {
                self.history.drop_last();
            }
            return;
        };
        let drawing_id = drawing.id;
        let snapshot = drawing.pixmap.clone();
        paint::stamp_region(Arc::make_mut(&mut drawing.pixmap), &clipboard, x, y);
        drawing.touch();
        if !structural {
            let after = self
                .doc
                .find_drawing(drawing_id)
                .map(|d| d.pixmap.clone())
                .unwrap_or_else(|| snapshot.clone());
            if let Some(entry) = paint::region_history(drawing_id, &snapshot, &after, target) {
                self.history.push_pixels(entry);
            }
        }
        self.selection = Some(target);
        self.set_tool(Tool::Select);
        self.dirty = true;
        self.invalidate();
        self.notify(format!("Pasted {w} x {h} pixels."));
    }

    // -------------------------------------------------------------- strokes

    pub fn begin_stroke(&mut self, pos: (f32, f32), pressure: f32) {
        let erase = self.tool == Tool::Eraser;
        let layer_idx = self.doc.active_layer.min(self.doc.layers.len() - 1);
        if self.doc.layers[layer_idx].locked || !self.doc.layers[layer_idx].visible {
            self.notify("Layer is locked or hidden.");
            return;
        }
        let auto_key = !self.doc.layers[layer_idx]
            .frames
            .contains_key(&self.doc.playhead);
        self.stroke_structural = auto_key;
        if auto_key {
            self.begin_edit();
        }
        let brush = if erase { &self.eraser } else { &self.brush };
        let stroke = Stroke::start(&mut self.doc, brush, pos, pressure, erase);
        if stroke.is_none() && auto_key {
            self.history.drop_last();
        }
        self.last_stroke_pos = stroke.as_ref().map(|_| (pos.0, pos.1, pressure));
        self.stroke = stroke;
        self.dirty = true;
        self.invalidate();
    }

    pub fn extend_stroke(&mut self, pos: (f32, f32), pressure: f32) {
        let erase = self.tool == Tool::Eraser;
        let brush = if erase { &self.eraser } else { &self.brush };
        if let Some(stroke) = &mut self.stroke {
            stroke.add_point(&mut self.doc, brush, pos, pressure);
            self.last_stroke_pos = Some((pos.0, pos.1, pressure));
        }
        self.invalidate();
    }

    pub fn end_stroke(&mut self, pos: (f32, f32), pressure: f32) {
        let erase = self.tool == Tool::Eraser;
        let brush = if erase { &self.eraser } else { &self.brush };
        let mut created = false;
        if let Some(stroke) = self.stroke.take() {
            let entry = stroke.finish(&mut self.doc, brush, pos, pressure);
            created = entry.is_some();
            if !self.stroke_structural {
                if let Some(entry) = entry {
                    self.history.push_pixels(entry);
                }
            }
        }
        if self.stroke_structural && !created {
            // Nothing was painted; remove the structural snapshot.
            self.history.drop_last();
        }
        self.stroke_structural = false;
        self.last_stroke_pos = None;
        self.dirty = true;
        self.invalidate();
    }

    pub fn fill_at(&mut self, pos: (f32, f32)) {
        let layer_idx = self.doc.active_layer.min(self.doc.layers.len() - 1);
        if self.doc.layers[layer_idx].locked || !self.doc.layers[layer_idx].visible {
            self.notify("Layer is locked or hidden.");
            return;
        }
        let auto_key = !self.doc.layers[layer_idx]
            .frames
            .contains_key(&self.doc.playhead);
        if auto_key {
            self.begin_edit();
        }
        let entry = paint::flood_fill(
            &mut self.doc,
            (pos.0.floor() as i32, pos.1.floor() as i32),
            self.brush.color,
            self.fill_tolerance,
            self.fill_contiguous,
        );
        if auto_key {
            if entry.is_none() {
                self.history.drop_last();
            }
        } else if let Some(entry) = entry {
            self.history.push_pixels(entry);
        }
        self.dirty = true;
        self.invalidate();
    }

    pub fn pick_at(&mut self, pos: (f32, f32)) {
        let (x, y) = (pos.0.floor() as i32, pos.1.floor() as i32);
        if let Some(pm) = &self.composite {
            if let Some(color) = paint::pick(pm, x, y) {
                if color.a() > 0 {
                    self.brush.color = color;
                    self.tool = self.last_paint_tool;
                    self.notify(format!(
                        "Picked #{:02X}{:02X}{:02X}",
                        color.r(),
                        color.g(),
                        color.b()
                    ));
                }
            }
        }
    }

    pub fn begin_move(&mut self, pos: (f32, f32)) {
        let frame = self.doc.playhead;
        let layer_idx = self.doc.active_layer.min(self.doc.layers.len() - 1);
        if self.doc.layers[layer_idx].locked || !self.doc.layers[layer_idx].visible {
            self.notify("Layer is locked or hidden.");
            return;
        }
        let auto_key = !self.doc.layers[layer_idx].frames.contains_key(&frame);
        self.stroke_structural = auto_key;
        if auto_key {
            self.begin_edit();
        }
        let Some(drawing) = self.doc.ensure_active_drawing() else {
            if self.stroke_structural {
                self.history.drop_last();
            }
            self.stroke_structural = false;
            return;
        };
        self.move_state = Some(MoveState {
            drawing_id: drawing.id,
            snapshot: drawing.pixmap.clone(),
            start: Pos2::new(pos.0, pos.1),
            offset: (0, 0),
        });
        self.dirty = true;
    }

    pub fn update_move(&mut self, pos: (f32, f32)) {
        let Some(mv) = &mut self.move_state else {
            return;
        };
        let dx = (pos.0 - mv.start.x).round() as i32;
        let dy = (pos.1 - mv.start.y).round() as i32;
        if (dx, dy) == mv.offset {
            return;
        }
        mv.offset = (dx, dy);
        let shifted = paint::translated(&mv.snapshot, dx, dy);
        if let Some(d) = self.doc.find_drawing_mut(mv.drawing_id) {
            d.pixmap = Arc::new(shifted);
            d.touch();
        }
        self.invalidate();
    }

    pub fn end_move(&mut self) {
        let Some(mv) = self.move_state.take() else {
            return;
        };
        if mv.offset == (0, 0) {
            if self.stroke_structural {
                self.history.drop_last();
            }
            self.stroke_structural = false;
            return;
        }
        let after = self
            .doc
            .find_drawing(mv.drawing_id)
            .map(|d| d.pixmap.clone())
            .unwrap_or_else(|| mv.snapshot.clone());
        if !self.stroke_structural {
            if let Some(entry) =
                paint::translate_history(mv.drawing_id, &mv.snapshot, &after)
            {
                self.history.push_pixels(entry);
            }
        }
        self.stroke_structural = false;
        self.dirty = true;
        self.invalidate();
    }

    // -------------------------------------------------------------- playback

    pub fn playback_tick(&mut self, ctx: &egui::Context) {
        if self.perform {
            self.record_tick(ctx);
        } else if self.playing {
            self.advance_playback(ctx);
        }
    }

    fn advance_playback(&mut self, ctx: &egui::Context) {
        let dt = ctx.input(|i| i.stable_dt).clamp(0.0, 0.1);
        self.play_accum += dt;
        let frame_time = 1.0 / self.doc.fps.max(1.0);
        let mut guard = 0;
        while self.play_accum >= frame_time && guard < 240 {
            self.play_accum -= frame_time;
            guard += 1;
            if self.doc.playhead + 1 >= self.doc.frame_count {
                if self.looping {
                    self.doc.playhead = 0;
                } else {
                    self.playing = false;
                    self.doc.playhead = self.doc.frame_count.saturating_sub(1);
                    break;
                }
            } else {
                self.doc.playhead += 1;
            }
        }
        ctx.request_repaint();
    }

    /// Perform clock. The take runs in real time from the moment REC is armed
    /// until it is stopped, so pauses and rhythm are kept exactly as performed.
    /// The head never loops or replays — it just advances and the timeline
    /// grows — and playback only happens after the take is committed.
    fn record_tick(&mut self, ctx: &egui::Context) {
        // Real elapsed time, not the smoothed frame time, so the take keeps
        // pace with the wall clock even if a frame renders late.
        let dt = ctx.input(|i| i.unstable_dt).clamp(0.0, 0.25);
        self.record_accum += dt;
        let frame_time = 1.0 / self.doc.fps.max(1.0);
        let mut guard = 0;
        while self.record_accum >= frame_time && guard < 240 {
            self.record_accum -= frame_time;
            guard += 1;
            if !self.doc.advance_record_head() {
                break;
            }
            self.split_live_stroke();
            self.dirty = true;
            self.invalidate();
        }
        ctx.request_repaint();
    }

    /// When the record head crosses a frame boundary mid-stroke, commit the
    /// stroke to the old frame and reopen it on the new one so each frame gets
    /// its own auto-keyed segment.
    fn split_live_stroke(&mut self) {
        if self.stroke.is_none() {
            return;
        }
        let Some((x, y, pressure)) = self.last_stroke_pos else {
            return;
        };
        self.end_stroke((x, y), pressure);
        if self.perform {
            self.begin_stroke((x, y), pressure);
        }
    }

    pub fn toggle_perform(&mut self, ctx: &egui::Context) {
        if self.perform {
            let last = self.doc.playhead + 1;
            self.finish_perform();
            self.notify(format!(
                "Recording stopped at frame {last}. Press Space to play the take."
            ));
        } else {
            self.playing = false;
            self.perform = true;
            self.record_accum = 0.0;
            if self.doc.playhead >= self.doc.frame_count.saturating_sub(1) {
                self.doc.playhead = 0;
            }
            self.perform_start = self.doc.playhead;
            self.invalidate();
            self.notify("Recording — the clock runs in real time. P stops, Space stops and plays.");
            ctx.request_repaint();
        }
        self.invalidate();
    }

    /// Ends a take without starting playback. Whatever was recorded stays on
    /// the timeline; the user presses Play afterwards to watch it.
    fn finish_perform(&mut self) {
        self.perform = false;
        self.record_accum = 0.0;
        if let Some((x, y, pressure)) = self.last_stroke_pos {
            self.end_stroke((x, y), pressure);
        }
        self.dirty = true;
    }

    // ---------------------------------------------------------------- files

    pub fn new_document(&mut self) {
        let form = &self.new_project;
        let w = form.width.clamp(16, 8192) as u32;
        let h = form.height.clamp(16, 8192) as u32;
        let frames = form.frames.clamp(1, 10_000) as u32;
        let mut doc = Document::new(form.name.trim(), w, h, form.fps, frames);
        doc.background = [
            form.background.r(),
            form.background.g(),
            form.background.b(),
            255,
        ];
        self.doc = doc;
        self.history = History::default();
        self.file_path = None;
        self.dirty = false;
        self.stroke = None;
        self.perform = false;
        self.playing = false;
        self.record_accum = 0.0;
        self.last_stroke_pos = None;
        self.move_state = None;
        self.selection = None;
        self.selection_draft = None;
        self.selection_move = None;
        self.view = ViewState::default();
        self.needs_fit = true;
        self.canvas_key = None;
        self.onion_before_cache.clear();
        self.onion_after_cache.clear();
        self.layer_thumbs.clear();
        self.export = ExportForm::for_doc(&self.doc);
        self.renaming_layer = None;
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Dreemz project", &["dreemz"])
            .set_title("Open Dreemz project")
            .pick_file()
        {
            match project::load_project(&path) {
                Ok(doc) => {
                    self.doc = doc;
                    self.history = History::default();
                    self.file_path = Some(path.clone());
                    self.dirty = false;
                    self.stroke = None;
                    self.perform = false;
                    self.playing = false;
                    self.record_accum = 0.0;
                    self.last_stroke_pos = None;
                    self.selection = None;
                    self.selection_draft = None;
                    self.selection_move = None;
                    self.needs_fit = true;
                    self.canvas_key = None;
                    self.onion_before_cache.clear();
                    self.onion_after_cache.clear();
                    self.layer_thumbs.clear();
                    self.export = ExportForm::for_doc(&self.doc);
                    self.notify(format!("Opened {}", path.display()));
                }
                Err(e) => self.notify(format!("Could not open project: {e}")),
            }
        }
    }

    pub fn save_project(&mut self, save_as: bool) {
        let path = if save_as || self.file_path.is_none() {
            let suggested = self
                .file_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(format!("{}.dreemz", self.doc.name)));
            rfd::FileDialog::new()
                .add_filter("Dreemz project", &["dreemz"])
                .set_file_name(suggested.to_string_lossy())
                .set_title("Save Dreemz project")
                .save_file()
        } else {
            self.file_path.clone()
        };
        let Some(mut path) = path else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("dreemz");
        }
        match project::save_project(&path, &self.doc) {
            Ok(()) => {
                self.file_path = Some(path.clone());
                self.dirty = false;
                self.notify(format!("Saved {}", path.display()));
            }
            Err(e) => self.notify(format!("Save failed: {e}")),
        }
    }

    pub fn import_image_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp", "bmp"])
            .set_title("Import image as layer")
            .pick_file()
        {
            self.begin_edit();
            match project::import_image(&mut self.doc, &path) {
                Ok(name) => {
                    self.invalidate();
                    self.notify(format!("Imported layer '{name}'"));
                }
                Err(e) => {
                    self.history.drop_last();
                    self.notify(format!("Import failed: {e}"));
                }
            }
        }
    }

    /// Exports the current frame (with background) as a PNG, prompting for a path.
    pub fn export_current_frame(&mut self) {
        let suggested = format!("{}_frame_{:04}.png", self.doc.name, self.doc.playhead + 1);
        let Some(mut path) = rfd::FileDialog::new()
            .add_filter("PNG image", &["png"])
            .set_file_name(suggested)
            .set_title("Export current frame as PNG")
            .save_file()
        else {
            return;
        };
        if path.extension().is_none() {
            path.set_extension("png");
        }
        match project::save_frame_png(&self.doc, self.doc.playhead, &path, true, 1.0) {
            Ok(()) => self.notify(format!("Exported {}", path.display())),
            Err(e) => self.notify(format!("Frame export failed: {e}")),
        }
    }

    pub fn run_export(&mut self) {
        let start = self.export.start.clamp(0, self.doc.frame_count as i32 - 1) as u32;
        let end = self.export.end.clamp(0, self.doc.frame_count as i32 - 1) as u32;
        let settings = ExportSettings {
            format: self.export.format,
            path: PathBuf::from(&self.export.path),
            start,
            end,
            scale: self.export.scale,
            background: self.export.background,
            columns: self.export.columns.max(1) as u32,
        };
        match project::export(&self.doc, &settings) {
            Ok(count) => {
                self.export.result = Some(format!("Exported {count} frames."));
                self.notify(format!("Exported {count} frames."));
            }
            Err(e) => {
                self.export.result = Some(format!("Export failed: {e}"));
                self.notify(format!("Export failed: {e}"));
            }
        }
    }

    pub fn update_title(&mut self, ctx: &egui::Context) {
        let dirty = if self.dirty { "*" } else { "" };
        let path = self
            .file_path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "Unsaved project".into());
        let title = format!("{} - {} - Dreemz", self.doc.name, dirty);
        if title != self.last_title {
            self.last_title = title.clone();
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));
            let _ = path;
        }
    }

    // ------------------------------------------------------------- shortcuts

    pub fn handle_shortcuts(&mut self, ui: &egui::Ui) {
        if ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let ctrl = Modifiers::CTRL;
        let mut actions: Vec<Shortcut> = Vec::new();
        ui.input_mut(|i| {
            if i.consume_key(ctrl, Key::Z) {
                actions.push(Shortcut::Undo);
            }
            if i.consume_key(ctrl, Key::Y) || i.consume_key(ctrl | Modifiers::SHIFT, Key::Z) {
                actions.push(Shortcut::Redo);
            }
            if i.consume_key(ctrl, Key::S) {
                actions.push(Shortcut::Save);
            }
            if i.consume_key(ctrl | Modifiers::SHIFT, Key::S) {
                actions.push(Shortcut::SaveAs);
            }
            if i.consume_key(ctrl, Key::O) {
                actions.push(Shortcut::Open);
            }
            if i.consume_key(ctrl, Key::N) {
                actions.push(Shortcut::New);
            }
            if i.consume_key(ctrl, Key::E) {
                actions.push(Shortcut::Export);
            }
            if i.consume_key(ctrl | Modifiers::SHIFT, Key::H) {
                actions.push(Shortcut::FlipHorizontal);
            }
            if i.consume_key(ctrl | Modifiers::SHIFT, Key::V) {
                actions.push(Shortcut::FlipVertical);
            }
            if i.consume_key(ctrl, Key::C) {
                actions.push(Shortcut::CopyFrame);
            }
            if i.consume_key(ctrl, Key::X) {
                actions.push(Shortcut::CutFrame);
            }
            if i.consume_key(ctrl, Key::V) {
                actions.push(Shortcut::PasteFrame);
            }
            if i.consume_key(ctrl, Key::A) {
                actions.push(Shortcut::SelectAll);
            }
            if i.consume_key(Modifiers::NONE, Key::Escape) {
                actions.push(Shortcut::Deselect);
            }
            if i.consume_key(ctrl, Key::Num0) {
                actions.push(Shortcut::Fit);
            }
            if i.consume_key(ctrl, Key::Num1) {
                actions.push(Shortcut::ActualSize);
            }
            if i.consume_key(ctrl, Key::Num2) {
                actions.push(Shortcut::ZoomIn);
            }
            if i.consume_key(ctrl, Key::Num3) {
                actions.push(Shortcut::ZoomOut);
            }
            if i.consume_key(Modifiers::NONE, Key::Space) {
                actions.push(Shortcut::PlayPause);
            }
            if i.consume_key(Modifiers::NONE, Key::P) {
                actions.push(Shortcut::Perform);
            }
            if i.consume_key(Modifiers::NONE, Key::B) {
                actions.push(Shortcut::Tool(Tool::Brush));
            }
            if i.consume_key(Modifiers::NONE, Key::E) {
                actions.push(Shortcut::Tool(Tool::Eraser));
            }
            if i.consume_key(Modifiers::NONE, Key::G) {
                actions.push(Shortcut::Tool(Tool::Fill));
            }
            if i.consume_key(Modifiers::NONE, Key::I) {
                actions.push(Shortcut::Tool(Tool::Eyedropper));
            }
            if i.consume_key(Modifiers::NONE, Key::V) {
                actions.push(Shortcut::Tool(Tool::Move));
            }
            if i.consume_key(Modifiers::NONE, Key::M) {
                actions.push(Shortcut::Tool(Tool::Select));
            }
            if i.consume_key(Modifiers::NONE, Key::H) {
                actions.push(Shortcut::Tool(Tool::Hand));
            }
            if i.consume_key(Modifiers::NONE, Key::O) {
                actions.push(Shortcut::ToggleOnion);
            }
            if i.consume_key(Modifiers::NONE, Key::L) {
                actions.push(Shortcut::ToggleLoop);
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                actions.push(Shortcut::Step(-1));
            }
            if i.consume_key(Modifiers::NONE, Key::ArrowRight) {
                actions.push(Shortcut::Step(1));
            }
            if i.consume_key(Modifiers::NONE, Key::OpenBracket) {
                actions.push(Shortcut::BrushSize(-1.0));
            }
            if i.consume_key(Modifiers::NONE, Key::CloseBracket) {
                actions.push(Shortcut::BrushSize(1.0));
            }
            if i.consume_key(Modifiers::NONE, Key::Delete) {
                actions.push(Shortcut::DeleteKeyframe);
            }
            if i.consume_key(ctrl, Key::D) {
                actions.push(Shortcut::DuplicateKeyframe);
            }
        });
        for action in actions {
            self.apply_shortcut(ui.ctx(), action);
        }
    }

    fn apply_shortcut(&mut self, ctx: &egui::Context, action: Shortcut) {
        match action {
            Shortcut::Undo => self.undo(),
            Shortcut::Redo => self.redo(),
            Shortcut::Save => self.save_project(false),
            Shortcut::SaveAs => self.save_project(true),
            Shortcut::Open => self.open_project_dialog(),
            Shortcut::New => {
                self.new_project = NewProjectForm::default();
                self.dialogs.new_project = true;
            }
            Shortcut::Export => self.open_export_dialog(),
            Shortcut::FlipHorizontal => self.flip_drawing(true),
            Shortcut::FlipVertical => self.flip_drawing(false),
            Shortcut::CopyFrame => {
                if self.selection.is_some() {
                    self.copy_selection();
                } else {
                    self.copy_keyframe();
                }
            }
            Shortcut::CutFrame => {
                if self.selection.is_some() {
                    self.cut_selection();
                } else {
                    self.cut_frame();
                }
            }
            Shortcut::PasteFrame => {
                let region = self.selection.is_some()
                    || (self.tool == Tool::Select && self.region_clipboard.is_some());
                if region {
                    self.paste_selection();
                } else {
                    self.paste_keyframe();
                }
            }
            Shortcut::SelectAll => self.select_all(),
            Shortcut::Deselect => self.deselect(),
            Shortcut::Fit => self.needs_fit = true,
            Shortcut::ActualSize => {
                self.view.zoom = 1.0;
                self.view.pan = Vec2::ZERO;
            }
            Shortcut::ZoomIn => self.view.zoom = (self.view.zoom * 1.25).min(32.0),
            Shortcut::ZoomOut => self.view.zoom = (self.view.zoom / 1.25).max(0.02),
            Shortcut::PlayPause => self.toggle_playback(ctx),
            Shortcut::Perform => self.toggle_perform(ctx),
            Shortcut::Tool(tool) => {
                if self.tool == Tool::Brush || self.tool == Tool::Eraser {
                    self.last_paint_tool = self.tool;
                }
                self.tool = tool;
            }
            Shortcut::ToggleOnion => self.onion.enabled = !self.onion.enabled,
            Shortcut::ToggleLoop => self.looping = !self.looping,
            Shortcut::Step(delta) => {
                self.playing = false;
                let f = self.doc.playhead as i32 + delta;
                self.doc
                    .set_playhead(f.clamp(0, self.doc.frame_count as i32 - 1) as u32);
                self.invalidate();
            }
            Shortcut::BrushSize(dir) => {
                let b = self.brush_settings_mut();
                let factor = if dir > 0.0 { 1.15 } else { 1.0 / 1.15 };
                b.size = (b.size * factor).clamp(1.0, 400.0);
            }
            Shortcut::DeleteKeyframe => {
                if self.selection.is_some() {
                    self.delete_selection();
                } else {
                    self.delete_keyframe();
                }
            }
            Shortcut::DuplicateKeyframe => self.add_keyframe(true),
        }
    }

    pub fn toggle_playback(&mut self, ctx: &egui::Context) {
        // Space also commits an armed take, then plays the whole reel from the
        // top so the performance can be watched immediately.
        let committed_take = self.perform;
        if self.perform {
            self.finish_perform();
        }
        self.playing = !self.playing;
        self.play_accum = 0.0;
        if self.playing {
            if committed_take || self.doc.playhead >= self.doc.frame_count.saturating_sub(1) {
                self.doc.playhead = 0;
            }
            ctx.request_repaint();
        }
        self.invalidate();
    }

    pub fn open_settings_dialog(&mut self) {
        self.settings_form = ProjectSettingsForm::for_doc(&self.doc);
        self.dialogs.settings = true;
    }

    pub fn apply_settings(&mut self) {
        let mut name = self.settings_form.name.trim().to_string();
        if name.is_empty() {
            name = "Untitled".into();
        }
        let width = self.settings_form.width.clamp(16, 8192) as u32;
        let height = self.settings_form.height.clamp(16, 8192) as u32;
        let fps = self.settings_form.fps.clamp(1.0, 120.0);
        let frames = self.settings_form.frames.clamp(1, 100_000) as u32;
        let bg = self.settings_form.background;

        self.begin_edit();
        if width != self.doc.width || height != self.doc.height {
            for layer in &mut self.doc.layers {
                for drawing in layer.frames.values_mut() {
                    drawing.pixmap = Arc::new(paint::resampled(&drawing.pixmap, width, height));
                    drawing.touch();
                }
            }
        }
        self.doc.width = width;
        self.doc.height = height;
        self.doc.name = name;
        self.doc.fps = fps;
        self.doc.background = [bg.r(), bg.g(), bg.b(), bg.a()];
        self.doc.resize_frame_count(frames);
        if let Some(rect) = self.selection {
            let clamped = rect.clamp(self.doc.width, self.doc.height);
            self.selection = if clamped.is_empty() { None } else { Some(clamped) };
        }
        self.settings_form = ProjectSettingsForm::for_doc(&self.doc);
        self.dirty = true;
        self.needs_fit = true;
        self.invalidate();
        self.dialogs.settings = false;
        self.notify("Project settings updated.");
    }

    pub fn open_export_dialog(&mut self) {
        let start = self.export.start.min(self.doc.frame_count as i32 - 1);
        let mut form = ExportForm::for_doc(&self.doc);
        form.format = self.export.format;
        form.start = start;
        form.end = self.doc.frame_count as i32 - 1;
        self.export = form;
        self.dialogs.export = true;
    }

    pub fn set_tool(&mut self, tool: Tool) {
        if self.tool == Tool::Brush || self.tool == Tool::Eraser {
            self.last_paint_tool = self.tool;
        }
        self.tool = tool;
    }

    pub fn apply_preset(&mut self, preset: BrushPreset) {
        let color = self.brush.color;
        let simulate = self.brush.simulate_pressure;
        preset.apply(&mut self.brush);
        self.brush.color = color;
        self.brush.simulate_pressure = simulate;
        self.tool = Tool::Brush;
    }

    pub fn set_zoom_around(&mut self, view: egui::Rect, anchor: Pos2, factor: f32) {
        let target = self.screen_to_canvas(view, anchor);
        self.view.zoom = (self.view.zoom * factor).clamp(0.02, 32.0);
        let new_screen = self.canvas_to_screen(view, target);
        self.view.pan += anchor - new_screen;
    }

    pub fn reset_view_rotation(&mut self) {
        self.view.rotation = 0.0;
    }

    pub fn frame_label(&self) -> String {
        let total = (self.doc.playhead as f32 / self.doc.fps.max(1.0)) as u32;
        format!(
            "{} / {}  ·  {}:{:02}",
            self.doc.playhead + 1,
            self.doc.frame_count,
            total / 60,
            total % 60
        )
    }

    pub fn main_background(&self) -> Color32 {
        Color32::from_rgb(
            self.doc.background[0],
            self.doc.background[1],
            self.doc.background[2],
        )
    }
}

#[derive(Clone, Copy, Debug)]
enum Shortcut {
    Undo,
    Redo,
    Save,
    SaveAs,
    Open,
    New,
    Export,
    FlipHorizontal,
    FlipVertical,
    CopyFrame,
    CutFrame,
    PasteFrame,
    SelectAll,
    Deselect,
    Fit,
    ActualSize,
    ZoomIn,
    ZoomOut,
    PlayPause,
    Perform,
    Tool(Tool),
    ToggleOnion,
    ToggleLoop,
    Step(i32),
    BrushSize(f32),
    DeleteKeyframe,
    DuplicateKeyframe,
}

impl ExportForm {
    pub fn sync_path_extension(&mut self) {
        let ext = self.format.extension();
        let mut p = PathBuf::from(&self.path);
        if ext.is_empty() {
            // PNG sequence exports into a folder: drop a file extension.
            p.set_extension("");
            self.path = p.to_string_lossy().to_string();
        } else {
            p.set_extension(ext);
            self.path = p.to_string_lossy().to_string();
        }
    }
}
