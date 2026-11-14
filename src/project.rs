//! Project persistence (.dreemz) and image / video export.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tiny_skia::{IntSize, Pixmap};

use crate::model::{BlendKind, Document, Drawing, DrawingId, Layer};
use crate::paint;

const MAGIC: &[u8; 7] = b"DREEMZ\x01";

#[derive(Serialize, Deserialize)]
struct ProjectMeta {
    format: u32,
    name: String,
    width: u32,
    height: u32,
    fps: f32,
    frame_count: u32,
    playhead: u32,
    background: [u8; 4],
    active_layer: usize,
    next_id: u64,
    layers: Vec<LayerMeta>,
}

#[derive(Serialize, Deserialize)]
struct LayerMeta {
    id: u64,
    name: String,
    visible: bool,
    locked: bool,
    opacity: f32,
    blend: BlendKind,
    frames: Vec<FrameMeta>,
}

#[derive(Serialize, Deserialize)]
struct FrameMeta {
    frame: u32,
    id: DrawingId,
    version: u64,
    len: u64,
}

pub fn save_project(path: &Path, doc: &Document) -> Result<(), String> {
    let mut meta = ProjectMeta {
        format: 1,
        name: doc.name.clone(),
        width: doc.width,
        height: doc.height,
        fps: doc.fps,
        frame_count: doc.frame_count,
        playhead: doc.playhead,
        background: doc.background,
        active_layer: doc.active_layer,
        next_id: doc.next_id,
        layers: Vec::with_capacity(doc.layers.len()),
    };
    let mut blobs: Vec<Vec<u8>> = Vec::new();
    for layer in &doc.layers {
        let mut frames = Vec::with_capacity(layer.frames.len());
        for (frame, drawing) in &layer.frames {
            let png = drawing
                .pixmap
                .encode_png()
                .map_err(|e| format!("PNG encode failed: {e}"))?;
            frames.push(FrameMeta {
                frame: *frame,
                id: drawing.id,
                version: drawing.version,
                len: png.len() as u64,
            });
            blobs.push(png);
        }
        meta.layers.push(LayerMeta {
            id: layer.id,
            name: layer.name.clone(),
            visible: layer.visible,
            locked: layer.locked,
            opacity: layer.opacity,
            blend: layer.blend,
            frames,
        });
    }

    let json = serde_json::to_vec(&meta).map_err(|e| format!("JSON encode failed: {e}"))?;

    // Write to a temporary file first, then rename, so a failed save never
    // destroys a previous good project.
    let tmp = path.with_extension("dreemz.tmp");
    {
        let file = File::create(&tmp).map_err(|e| format!("Could not create file: {e}"))?;
        let mut w = BufWriter::new(file);
        w.write_all(MAGIC).map_err(io_err)?;
        w.write_all(&(json.len() as u32).to_le_bytes())
            .map_err(io_err)?;
        w.write_all(&json).map_err(io_err)?;
        for blob in &blobs {
            w.write_all(blob).map_err(io_err)?;
        }
        w.flush().map_err(io_err)?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("Could not finish save: {e}"))?;
    Ok(())
}

fn io_err(e: std::io::Error) -> String {
    format!("IO error: {e}")
}

pub fn load_project(path: &Path) -> Result<Document, String> {
    let mut file = File::open(path).map_err(|e| format!("Could not open file: {e}"))?;
    let mut magic = [0u8; 7];
    file.read_exact(&mut magic)
        .map_err(|_| "Not a Dreemz project file.".to_string())?;
    if &magic != MAGIC {
        return Err("Not a Dreemz project file (bad magic header).".into());
    }
    let mut len_buf = [0u8; 4];
    file.read_exact(&mut len_buf).map_err(io_err)?;
    let json_len = u32::from_le_bytes(len_buf) as usize;
    if json_len > 64 * 1024 * 1024 {
        return Err("Project file is corrupt (metadata too large).".into());
    }
    let mut json = vec![0u8; json_len];
    file.read_exact(&mut json).map_err(io_err)?;
    let meta: ProjectMeta =
        serde_json::from_slice(&json).map_err(|e| format!("Project metadata is corrupt: {e}"))?;
    if meta.width == 0 || meta.height == 0 || meta.width > 16_384 || meta.height > 16_384 {
        return Err("Project has an invalid canvas size.".into());
    }
    if meta.format != 1 {
        return Err(format!("Unsupported project version {}.", meta.format));
    }

    let size = IntSize::from_wh(meta.width, meta.height).ok_or("Invalid canvas size.")?;
    let expected = (size.width() as usize) * (size.height() as usize) * 4;

    let mut layers: Vec<Layer> = Vec::with_capacity(meta.layers.len());
    for lm in meta.layers {
        let mut frames: BTreeMap<u32, Drawing> = BTreeMap::new();
        for fm in lm.frames {
            if fm.len > 512 * 1024 * 1024 {
                return Err("Project contains an unreasonably large drawing.".into());
            }
            let mut png = vec![0u8; fm.len as usize];
            file.read_exact(&mut png)
                .map_err(|_| "Project file is truncated.".to_string())?;
            let pm = Pixmap::decode_png(&png).map_err(|e| format!("Drawing decode failed: {e}"))?;
            if pm.width() != meta.width || pm.height() != meta.height {
                return Err("Project contains a drawing with the wrong size.".into());
            }
            debug_assert_eq!(pm.data().len(), expected);
            frames.insert(
                fm.frame,
                Drawing::from_pixmap(fm.id, pm, fm.version),
            );
        }
        layers.push(Layer {
            id: lm.id,
            name: lm.name,
            visible: lm.visible,
            locked: lm.locked,
            opacity: lm.opacity.clamp(0.0, 1.0),
            blend: lm.blend,
            frames,
        });
    }
    if layers.is_empty() {
        return Err("Project has no layers.".into());
    }

    let mut doc = Document {
        name: meta.name,
        width: meta.width,
        height: meta.height,
        fps: meta.fps.clamp(1.0, 120.0),
        frame_count: meta.frame_count.max(1),
        playhead: meta.playhead,
        background: meta.background,
        layers,
        active_layer: meta.active_layer,
        next_id: meta.next_id,
    };
    doc.set_active_layer(doc.active_layer);
    doc.set_playhead(doc.playhead);
    let max_id = doc
        .layers
        .iter()
        .flat_map(|l| l.frames.values().map(|d| d.id))
        .chain(doc.layers.iter().map(|l| l.id))
        .max()
        .unwrap_or(0);
    doc.next_id = doc.next_id.max(max_id + 1);
    Ok(doc)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    PngSequence,
    Gif,
    Mp4,
    SpriteSheet,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 4] = [
        ExportFormat::PngSequence,
        ExportFormat::Gif,
        ExportFormat::Mp4,
        ExportFormat::SpriteSheet,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::PngSequence => "PNG sequence",
            ExportFormat::Gif => "Animated GIF",
            ExportFormat::Mp4 => "MP4 video (needs ffmpeg)",
            ExportFormat::SpriteSheet => "Sprite sheet (PNG)",
        }
    }

    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::PngSequence => "",
            ExportFormat::Gif => "gif",
            ExportFormat::Mp4 => "mp4",
            ExportFormat::SpriteSheet => "png",
        }
    }
}

#[derive(Clone)]
pub struct ExportSettings {
    pub format: ExportFormat,
    pub path: PathBuf,
    pub start: u32,
    pub end: u32,
    pub scale: f32,
    pub background: bool,
    /// Number of columns in a sprite sheet.
    pub columns: u32,
}

/// Renders a frame to straight RGBA, optionally scaled.
fn render_frame(doc: &Document, frame: u32, background: bool, scale: f32) -> (Vec<u8>, u32, u32) {
    let pm = paint::composite_frame(doc, frame, background);
    let mut w = pm.width();
    let mut h = pm.height();
    let mut rgba = paint::demultiplied(&pm);
    let scale = scale.clamp(0.05, 8.0);
    if (scale - 1.0).abs() > 0.001 {
        w = ((w as f32 * scale).round() as u32).max(1);
        h = ((h as f32 * scale).round() as u32).max(1);
        if let Some(img) = image::RgbaImage::from_raw(
            pm.width(),
            pm.height(),
            std::mem::take(&mut rgba),
        ) {
            let scaled = image::imageops::resize(&img, w, h, image::imageops::FilterType::Lanczos3);
            rgba = scaled.into_raw();
        }
    }
    (rgba, w, h)
}

fn write_png(path: &Path, rgba: &[u8], w: u32, h: u32) -> Result<(), String> {
    image::save_buffer_with_format(
        path,
        rgba,
        w,
        h,
        image::ColorType::Rgba8,
        image::ImageFormat::Png,
    )
    .map_err(|e| format!("Could not write {}: {e}", path.display()))
}

pub fn export(doc: &Document, settings: &ExportSettings) -> Result<usize, String> {
    let start = settings.start.min(settings.end);
    let end = settings.start.max(settings.end).min(doc.frame_count.saturating_sub(1));
    let count = (end - start + 1) as usize;
    if count == 0 {
        return Err("Nothing to export.".into());
    }
    if doc.fps <= 0.0 {
        return Err("Frame rate must be positive.".into());
    }

    match settings.format {
        ExportFormat::PngSequence => {
            let dir = if settings.path.is_dir() {
                settings.path.clone()
            } else {
                settings.path.clone()
            };
            std::fs::create_dir_all(&dir).map_err(|e| format!("Could not create folder: {e}"))?;
            let stem = sanitize(&doc.name);
            for frame in start..=end {
                let (rgba, w, h) = render_frame(doc, frame, settings.background, settings.scale);
                let path = dir.join(format!("{stem}_{frame:05}.png"));
                write_png(&path, &rgba, w, h)?;
            }
            Ok(count)
        }
        ExportFormat::Gif => {
            let path = with_extension(&settings.path, "gif");
            let file = File::create(&path).map_err(|e| format!("Could not create file: {e}"))?;
            let mut encoder = image::codecs::gif::GifEncoder::new_with_speed(
                BufWriter::new(file),
                10,
            );
            encoder
                .set_repeat(image::codecs::gif::Repeat::Infinite)
                .map_err(|e| format!("GIF setup failed: {e}"))?;
            let delay_ms = (1000.0 / doc.fps).round().max(10.0) as u32;
            for frame in start..=end {
                let (rgba, w, h) = render_frame(doc, frame, settings.background, settings.scale);
                let buffer = image::RgbaImage::from_raw(w, h, rgba)
                    .ok_or("Could not build GIF frame.")?;
                let gif_frame = image::Frame::from_parts(
                    buffer,
                    0,
                    0,
                    image::Delay::from_numer_denom_ms(delay_ms, 1),
                );
                encoder
                    .encode_frame(gif_frame)
                    .map_err(|e| format!("GIF encode failed: {e}"))?;
            }
            Ok(count)
        }
        ExportFormat::Mp4 => {
            let path = with_extension(&settings.path, "mp4");
            let temp = std::env::temp_dir().join(format!(
                "dreemz_mp4_{}_{}",
                std::process::id(),
                start
            ));
            std::fs::create_dir_all(&temp).map_err(|e| format!("Could not create temp dir: {e}"))?;
            for (i, frame) in (start..=end).enumerate() {
                let (rgba, w, h) = render_frame(doc, frame, settings.background, settings.scale);
                let p = temp.join(format!("frame_{i:05}.png"));
                write_png(&p, &rgba, w, h)?;
            }
            let status = std::process::Command::new("ffmpeg")
                .arg("-y")
                .arg("-loglevel")
                .arg("error")
                .arg("-framerate")
                .arg(format!("{}", doc.fps))
                .arg("-i")
                .arg(temp.join("frame_%05d.png"))
                .arg("-vf")
                .arg("scale=trunc(iw/2)*2:trunc(ih/2)*2")
                .arg("-c:v")
                .arg("libx264")
                .arg("-pix_fmt")
                .arg("yuv420p")
                .arg("-movflags")
                .arg("+faststart")
                .arg(&path)
                .status();
            let _ = std::fs::remove_dir_all(&temp);
            match status {
                Ok(s) if s.success() => Ok(count),
                Ok(s) => Err(format!(
                    "ffmpeg failed with status {s}. Is it installed and on PATH?"
                )),
                Err(e) => Err(format!(
                    "Could not run ffmpeg ({e}). Install ffmpeg to export MP4."
                )),
            }
        }
        ExportFormat::SpriteSheet => {
            let path = with_extension(&settings.path, "png");
            let columns = settings.columns.clamp(1, 256) as usize;
            let rows = (count + columns - 1) / columns;
            let (_, cell_w, cell_h) = render_frame(doc, start, settings.background, settings.scale);
            let sheet_w = cell_w
                .checked_mul(columns as u32)
                .ok_or("Sprite sheet is too wide.")?;
            let sheet_h = cell_h
                .checked_mul(rows as u32)
                .ok_or("Sprite sheet is too tall.")?;
            let bytes = (sheet_w as u64) * (sheet_h as u64) * 4;
            if bytes > 512 * 1024 * 1024 {
                return Err(
                    "Sprite sheet is too large. Reduce the scale, frame range or column count."
                        .into(),
                );
            }
            let mut sheet = vec![0u8; bytes as usize];
            for (i, frame) in (start..=end).enumerate() {
                let (rgba, w, h) = render_frame(doc, frame, settings.background, settings.scale);
                let cx = (i % columns) as u32 * cell_w;
                let cy = (i / columns) as u32 * cell_h;
                let copy_w = w.min(cell_w) as usize;
                for row in 0..h.min(cell_h) as usize {
                    let src = (row * w as usize) * 4;
                    let dst = ((cy as usize + row) * sheet_w as usize + cx as usize) * 4;
                    let len = copy_w * 4;
                    if src + len <= rgba.len() && dst + len <= sheet.len() {
                        sheet[dst..dst + len].copy_from_slice(&rgba[src..src + len]);
                    }
                }
            }
            write_png(&path, &sheet, sheet_w, sheet_h)?;
            Ok(count)
        }
    }
}

fn with_extension(path: &Path, ext: &str) -> PathBuf {
    if path
        .extension()
        .map(|e| e.eq_ignore_ascii_case(ext))
        .unwrap_or(false)
    {
        path.to_path_buf()
    } else {
        path.with_extension(ext)
    }
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    if trimmed.is_empty() {
        "dreemz".into()
    } else {
        trimmed.into()
    }
}

/// Saves the visible composite of one frame as a PNG.
pub fn save_frame_png(
    doc: &Document,
    frame: u32,
    path: &Path,
    background: bool,
    scale: f32,
) -> Result<(), String> {
    let (rgba, w, h) = render_frame(doc, frame, background, scale);
    write_png(&with_extension(path, "png"), &rgba, w, h)
}

/// Imports an image as a new layer (keyframe on frame 0).
pub fn import_image(doc: &mut Document, path: &Path) -> Result<String, String> {
    let img = image::open(path).map_err(|e| format!("Could not read image: {e}"))?;
    let rgba = img.to_rgba8();
    let (x, y, w, h) = paint::fit_into(
        doc.width,
        doc.height,
        rgba.width(),
        rgba.height(),
    );
    let resized = image::imageops::resize(&rgba, w.max(1), h.max(1), image::imageops::FilterType::Lanczos3);

    let mut canvas = vec![0u8; (doc.width as usize) * (doc.height as usize) * 4];
    let canvas_w = doc.width as usize;
    for row in 0..h as usize {
        let dst = ((y as usize + row) * canvas_w + x as usize) * 4;
        let src = row * w as usize * 4;
        let len = w as usize * 4;
        canvas[dst..dst + len].copy_from_slice(&resized.as_raw()[src..src + len]);
    }
    let pm = paint::premultiplied(doc.width, doc.height, &canvas)
        .ok_or("Could not build layer bitmap.")?;

    let name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "Imported".into());
    let layer_id = doc.alloc_id();
    let drawing_id = doc.alloc_id();
    let mut layer = Layer::new(layer_id, name.clone());
    layer
        .frames
        .insert(0, Drawing::from_pixmap(drawing_id, pm, 0));
    doc.layers.push(layer);
    doc.active_layer = doc.layers.len() - 1;
    Ok(name)
}
