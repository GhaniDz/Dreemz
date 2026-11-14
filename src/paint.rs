//! Compositing, onion skinning, flood fill and other pixel helpers.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use egui::Color32;
use tiny_skia::{BlendMode, Color, FilterQuality, Pixmap, PixmapPaint, Transform};

use crate::model::{BlendKind, Document, Drawing, DrawingId, HistoryEntry, IRect};

pub fn blend_mode(kind: BlendKind) -> BlendMode {
    match kind {
        BlendKind::Normal => BlendMode::SourceOver,
        BlendKind::Multiply => BlendMode::Multiply,
        BlendKind::Screen => BlendMode::Screen,
        BlendKind::Overlay => BlendMode::Overlay,
        BlendKind::Darken => BlendMode::Darken,
        BlendKind::Lighten => BlendMode::Lighten,
        BlendKind::Add => BlendMode::Plus,
        BlendKind::Difference => BlendMode::Difference,
        BlendKind::Hue => BlendMode::Hue,
        BlendKind::Saturation => BlendMode::Saturation,
        BlendKind::Color => BlendMode::Color,
        BlendKind::Luminosity => BlendMode::Luminosity,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OnionSettings {
    pub enabled: bool,
    pub before: u32,
    pub after: u32,
    /// 0.0 ..= 1.0
    pub opacity: f32,
}

impl Default for OnionSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            before: 1,
            after: 1,
            opacity: 0.35,
        }
    }
}

pub const ONION_BEFORE_TINT: [f32; 3] = [1.0, 0.32, 0.36];
pub const ONION_AFTER_TINT: [f32; 3] = [0.32, 0.62, 1.0];

/// Draws a layer pixmap onto the accumulator.
fn draw_layer(acc: &mut Pixmap, drawing: &Drawing, opacity: f32, blend: BlendKind) {
    if opacity <= 0.001 {
        return;
    }
    let paint = PixmapPaint {
        opacity: opacity.clamp(0.0, 1.0),
        blend_mode: blend_mode(blend),
        quality: FilterQuality::Nearest,
    };
    acc.draw_pixmap(
        0,
        0,
        drawing.pixmap.as_ref().as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
}

fn background_pixmap(width: u32, height: u32, background: [u8; 4]) -> Pixmap {
    let mut pm = Pixmap::new(width, height).expect("valid canvas size");
    pm.fill(Color::from_rgba8(
        background[0],
        background[1],
        background[2],
        background[3],
    ));
    pm
}

/// Composite one frame of the document. `with_background` includes the paper
/// colour; without it the result keeps transparency (used for export).
pub fn composite_frame(doc: &Document, frame: u32, with_background: bool) -> Pixmap {
    let mut acc = if with_background {
        background_pixmap(doc.width, doc.height, doc.background)
    } else {
        Pixmap::new(doc.width, doc.height).expect("valid canvas size")
    };
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        if let Some(drawing) = layer.drawing_at(frame) {
            draw_layer(&mut acc, drawing, layer.opacity, layer.blend);
        }
    }
    acc
}

/// Draws a stack of tinted onion frames, farthest first. The nearest frame
/// gets the full onion opacity; older frames fade out.
fn draw_onion(acc: &mut Pixmap, frames: &[Arc<Pixmap>], opacity: f32) {
    let count = frames.len();
    for (index, pm) in frames.iter().enumerate() {
        let rank = count.saturating_sub(1 + index);
        let alpha = (opacity / (1.0 + rank as f32 * 0.65)).clamp(0.0, 1.0);
        let paint = PixmapPaint {
            opacity: alpha,
            blend_mode: BlendMode::SourceOver,
            quality: FilterQuality::Nearest,
        };
        acc.draw_pixmap(
            0,
            0,
            pm.as_ref().as_ref(),
            &paint,
            Transform::identity(),
            None,
        );
    }
}

/// Composite the current frame with optional onion skin frames.
pub fn composite_with_onion(
    doc: &Document,
    frame: u32,
    before: &[Arc<Pixmap>],
    after: &[Arc<Pixmap>],
    opacity: f32,
) -> Pixmap {
    let mut acc = background_pixmap(doc.width, doc.height, doc.background);
    draw_onion(&mut acc, before, opacity);
    draw_onion(&mut acc, after, opacity);
    for layer in &doc.layers {
        if !layer.visible {
            continue;
        }
        if let Some(drawing) = layer.drawing_at(frame) {
            draw_layer(&mut acc, drawing, layer.opacity, layer.blend);
        }
    }
    acc
}

/// Multiplies the (premultiplied) channels by a tint and alpha factor.
pub fn tinted(pm: &Pixmap, tint: [f32; 3], opacity: f32) -> Pixmap {
    let mut out = pm.clone();
    let opacity = opacity.clamp(0.0, 1.0);
    for px in out.data_mut().chunks_exact_mut(4) {
        px[0] = (px[0] as f32 * tint[0]).clamp(0.0, 255.0) as u8;
        px[1] = (px[1] as f32 * tint[1]).clamp(0.0, 255.0) as u8;
        px[2] = (px[2] as f32 * tint[2]).clamp(0.0, 255.0) as u8;
        px[3] = (px[3] as f32 * opacity).clamp(0.0, 255.0) as u8;
    }
    out
}

/// Structural fingerprint used to know when the cached composite is stale.
pub fn fingerprint(doc: &Document, frame: u32, onion: &OnionSettings) -> u64 {
    let mut h = DefaultHasher::new();
    doc.width.hash(&mut h);
    doc.height.hash(&mut h);
    doc.background.hash(&mut h);
    frame.hash(&mut h);
    onion.enabled.hash(&mut h);
    onion.before.hash(&mut h);
    onion.after.hash(&mut h);
    ((onion.opacity * 1000.0) as i32).hash(&mut h);
    for layer in &doc.layers {
        layer.id.hash(&mut h);
        layer.visible.hash(&mut h);
        layer.locked.hash(&mut h);
        ((layer.opacity * 1000.0) as i32).hash(&mut h);
        layer.blend.label().hash(&mut h);
        for (k, d) in &layer.frames {
            k.hash(&mut h);
            d.id.hash(&mut h);
            d.version.hash(&mut h);
            // The bitmap itself can change without the version changing when a
            // stroke is in progress; include a cheap content probe.
            let data = d.pixmap.data();
            let step = (data.len() / 256).max(4);
            let mut sum: u64 = 0;
            let mut i = 0;
            while i < data.len() {
                sum = sum.wrapping_mul(31).wrapping_add(data[i] as u64);
                i += step;
            }
            sum.hash(&mut h);
        }
    }
    h.finish()
}

/// Straight (un-premultiplied) RGBA bytes for export / textures.
pub fn demultiplied(pm: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(pm.data().len());
    for px in pm.data().chunks_exact(4) {
        let a = px[3];
        if a == 0 {
            out.extend_from_slice(&[0, 0, 0, 0]);
        } else if a == 255 {
            out.extend_from_slice(px);
        } else {
            let inv = 255.0 / a as f32;
            out.push((px[0] as f32 * inv).clamp(0.0, 255.0) as u8);
            out.push((px[1] as f32 * inv).clamp(0.0, 255.0) as u8);
            out.push((px[2] as f32 * inv).clamp(0.0, 255.0) as u8);
            out.push(a);
        }
    }
    out
}

pub fn premultiplied(width: u32, height: u32, straight: &[u8]) -> Option<Pixmap> {
    let mut data = straight.to_vec();
    if data.len() != (width as usize) * (height as usize) * 4 {
        return None;
    }
    for px in data.chunks_exact_mut(4) {
        let a = px[3] as u32;
        px[0] = ((px[0] as u32 * a + 127) / 255) as u8;
        px[1] = ((px[1] as u32 * a + 127) / 255) as u8;
        px[2] = ((px[2] as u32 * a + 127) / 255) as u8;
    }
    Pixmap::from_vec(data, tiny_skia::IntSize::from_wh(width, height)?)
}

/// Straight sRGB colour of a single pixel.
pub fn pick(pm: &Pixmap, x: i32, y: i32) -> Option<Color32> {
    if x < 0 || y < 0 || x >= pm.width() as i32 || y >= pm.height() as i32 {
        return None;
    }
    let idx = (y as usize * pm.width() as usize + x as usize) * 4;
    let px = &pm.data()[idx..idx + 4];
    let a = px[3];
    if a == 0 {
        return Some(Color32::TRANSPARENT);
    }
    let inv = 255.0 / a as f32;
    Some(Color32::from_rgba_unmultiplied(
        (px[0] as f32 * inv) as u8,
        (px[1] as f32 * inv) as u8,
        (px[2] as f32 * inv) as u8,
        a,
    ))
}

fn history_for(
    drawing_id: DrawingId,
    snapshot: &Pixmap,
    after: &Pixmap,
    rect: IRect,
) -> Option<HistoryEntry> {
    let rect = rect.clamp(snapshot.width(), snapshot.height());
    if rect.is_empty() {
        return None;
    }
    Some(HistoryEntry::Pixels {
        drawing_id,
        rect,
        before: crate::brush::crop_pixmap(snapshot, rect),
        after: crate::brush::crop_pixmap(after, rect),
    })
}

/// Paint-bucket fill. `contiguous` limits the fill to the connected region
/// around the seed, otherwise every matching pixel in the drawing is filled.
pub fn flood_fill(
    doc: &mut Document,
    seed: (i32, i32),
    color: Color32,
    tolerance: u8,
    contiguous: bool,
) -> Option<HistoryEntry> {
    let layer_idx = doc.active_layer.min(doc.layers.len().saturating_sub(1));
    {
        let layer = doc.layers.get(layer_idx)?;
        if layer.locked || !layer.visible {
            return None;
        }
    }
    let drawing = doc.ensure_active_drawing()?;
    let drawing_id = drawing.id;
    let snapshot = drawing.pixmap.clone();
    let pm = std::sync::Arc::make_mut(&mut drawing.pixmap);
    let (w, h) = (pm.width() as i32, pm.height() as i32);
    let (sx, sy) = seed;
    if sx < 0 || sy < 0 || sx >= w || sy >= h {
        return None;
    }

    let data = snapshot.data();
    let seed_idx = (sy as usize * w as usize + sx as usize) * 4;
    let seed_px = &data[seed_idx..seed_idx + 4];
    let seed_straight = demultiply_px(seed_px);
    let tol = tolerance as i32;

    let matches = |x: i32, y: i32| -> bool {
        let idx = (y as usize * w as usize + x as usize) * 4;
        let s = demultiply_px(&data[idx..idx + 4]);
        let d0 = (s[0] as i32 - seed_straight[0] as i32).abs();
        let d1 = (s[1] as i32 - seed_straight[1] as i32).abs();
        let d2 = (s[2] as i32 - seed_straight[2] as i32).abs();
        let d3 = (s[3] as i32 - seed_straight[3] as i32).abs();
        d0.max(d1).max(d2).max(d3) <= tol
    };

    // Source-over blend of the fill colour, in premultiplied space.
    let sa = color.a() as f32 / 255.0;
    let sr = color.r() as f32 * sa;
    let sg = color.g() as f32 * sa;
    let sb = color.b() as f32 * sa;

    let mut bounds = IRect::new(w, h, 0, 0);
    let fill_pixel = |pm: &mut Pixmap, x: i32, y: i32, bounds: &mut IRect| {
        let idx = (y as usize * w as usize + x as usize) * 4;
        let dst = &mut pm.data_mut()[idx..idx + 4];
        if sa >= 0.999 {
            dst[0] = sr as u8;
            dst[1] = sg as u8;
            dst[2] = sb as u8;
            dst[3] = 255;
        } else {
            let inv = 1.0 - sa;
            dst[0] = (sr + dst[0] as f32 * inv).clamp(0.0, 255.0) as u8;
            dst[1] = (sg + dst[1] as f32 * inv).clamp(0.0, 255.0) as u8;
            dst[2] = (sb + dst[2] as f32 * inv).clamp(0.0, 255.0) as u8;
            dst[3] = (sa * 255.0 + dst[3] as f32 * inv).clamp(0.0, 255.0) as u8;
        }
        *bounds = bounds.union(IRect::new(x, y, x + 1, y + 1));
    };

    if contiguous {
        let mut visited = vec![false; (w * h) as usize];
        let mut stack = vec![(sx, sy)];
        while let Some((x, y)) = stack.pop() {
            if x < 0 || y < 0 || x >= w || y >= h {
                continue;
            }
            let vi = y as usize * w as usize + x as usize;
            if visited[vi] {
                continue;
            }
            if !matches(x, y) {
                continue;
            }
            visited[vi] = true;
            fill_pixel(pm, x, y, &mut bounds);
            stack.push((x + 1, y));
            stack.push((x - 1, y));
            stack.push((x, y + 1));
            stack.push((x, y - 1));
        }
    } else {
        for y in 0..h {
            for x in 0..w {
                if matches(x, y) {
                    fill_pixel(pm, x, y, &mut bounds);
                }
            }
        }
    }

    if bounds.is_empty() {
        return None;
    }
    history_for(drawing_id, &snapshot, pm, bounds)
}

fn demultiply_px(px: &[u8]) -> [u8; 4] {
    let a = px[3] as u32;
    if a == 0 {
        [0, 0, 0, 0]
    } else if a == 255 {
        [px[0], px[1], px[2], 255]
    } else {
        [
            ((px[0] as u32 * 255 + a / 2) / a).min(255) as u8,
            ((px[1] as u32 * 255 + a / 2) / a).min(255) as u8,
            ((px[2] as u32 * 255 + a / 2) / a).min(255) as u8,
            a as u8,
        ]
    }
}

/// Returns a copy of `src` shifted by `(dx, dy)` on a same-sized canvas.
pub fn translated(src: &Pixmap, dx: i32, dy: i32) -> Pixmap {
    let mut out = Pixmap::new(src.width(), src.height()).expect("valid canvas size");
    out.draw_pixmap(
        dx,
        dy,
        src.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    out
}

/// Returns a mirrored copy of `src` (horizontal or vertical axis).
pub fn flip(src: &Pixmap, horizontal: bool) -> Pixmap {
    let (w, h) = (src.width() as usize, src.height() as usize);
    let mut out = Pixmap::new(src.width(), src.height()).expect("valid canvas size");
    let src_data = src.data();
    let dst_data = out.data_mut();
    for y in 0..h {
        for x in 0..w {
            let sx = if horizontal { w - 1 - x } else { x };
            let sy = if horizontal { y } else { h - 1 - y };
            let si = (sy * w + sx) * 4;
            let di = (y * w + x) * 4;
            dst_data[di..di + 4].copy_from_slice(&src_data[si..si + 4]);
        }
    }
    out
}

/// Resamples a drawing to a new pixel size, preserving transparency.
pub fn resampled(src: &Pixmap, width: u32, height: u32) -> Pixmap {
    let width = width.max(1);
    let height = height.max(1);
    if width == src.width() && height == src.height() {
        return src.clone();
    }
    let straight = demultiplied(src);
    let Some(img) = image::RgbaImage::from_raw(src.width(), src.height(), straight) else {
        return Pixmap::new(width, height).expect("valid canvas size");
    };
    let resized = image::imageops::resize(&img, width, height, image::imageops::FilterType::Lanczos3);
    premultiplied(width, height, resized.as_raw())
        .unwrap_or_else(|| Pixmap::new(width, height).expect("valid canvas size"))
}

/// Extracts `rect` from `src` into a new pixmap sized to the rectangle.
pub fn copy_region(src: &Pixmap, rect: IRect) -> Option<Pixmap> {
    let rect = rect.clamp(src.width(), src.height());
    if rect.is_empty() {
        return None;
    }
    let data = crate::brush::crop_pixmap(src, rect);
    Pixmap::from_vec(
        data,
        tiny_skia::IntSize::from_wh(rect.width() as u32, rect.height() as u32)?,
    )
}

/// Clears `rect` to transparent in place.
pub fn clear_region(pm: &mut Pixmap, rect: IRect) {
    let rect = rect.clamp(pm.width(), pm.height());
    if rect.is_empty() {
        return;
    }
    let w = pm.width() as usize;
    let data = pm.data_mut();
    for y in rect.y0..rect.y1 {
        let start = (y as usize * w + rect.x0 as usize) * 4;
        let end = start + rect.width() as usize * 4;
        data[start..end].fill(0);
    }
}

/// Moves the pixels inside `rect` by `(dx, dy)`, clearing the source area.
/// Pixels shifted past the canvas edge are clipped.
pub fn moved_region(src: &Pixmap, rect: IRect, dx: i32, dy: i32) -> Pixmap {
    let mut out = src.clone();
    let rect = rect.clamp(src.width(), src.height());
    if rect.is_empty() {
        return out;
    }
    clear_region(&mut out, rect);
    let w = src.width() as i32;
    let h = src.height() as i32;
    let data = src.data();
    let dst = out.data_mut();
    for y in rect.y0..rect.y1 {
        let ty = y + dy;
        if ty < 0 || ty >= h {
            continue;
        }
        for x in rect.x0..rect.x1 {
            let tx = x + dx;
            if tx < 0 || tx >= w {
                continue;
            }
            let si = (y as usize * w as usize + x as usize) * 4;
            let di = (ty as usize * w as usize + tx as usize) * 4;
            dst[di..di + 4].copy_from_slice(&data[si..si + 4]);
        }
    }
    out
}

/// Stamps `clipboard` over `pm` with its top-left corner at `(x, y)`.
pub fn stamp_region(pm: &mut Pixmap, clipboard: &Pixmap, x: i32, y: i32) {
    let paint = PixmapPaint {
        quality: FilterQuality::Nearest,
        ..Default::default()
    };
    pm.draw_pixmap(
        x,
        y,
        clipboard.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
}

/// Builds an undo entry covering the whole canvas for a translated drawing.
pub fn translate_history(
    drawing_id: DrawingId,
    before: &Pixmap,
    after: &Pixmap,
) -> Option<HistoryEntry> {
    let rect = IRect::new(0, 0, before.width() as i32, before.height() as i32);
    history_for(drawing_id, before, after, rect)
}

/// Undo entry for a change confined to `rect`.
pub fn region_history(
    drawing_id: DrawingId,
    before: &Pixmap,
    after: &Pixmap,
    rect: IRect,
) -> Option<HistoryEntry> {
    history_for(drawing_id, before, after, rect)
}

/// Builds a small preview of a drawing on the paper colour, letterboxed into
/// `width` x `height` pixels.
pub fn thumbnail(pm: &Pixmap, width: u32, height: u32, background: [u8; 4]) -> Pixmap {
    let width = width.max(1);
    let height = height.max(1);
    let mut out = background_pixmap(width, height, background);
    let scale = (width as f32 / pm.width() as f32).min(height as f32 / pm.height() as f32);
    let dx = (width as f32 - pm.width() as f32 * scale) * 0.5;
    let dy = (height as f32 - pm.height() as f32 * scale) * 0.5;
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..Default::default()
    };
    out.draw_pixmap(
        0,
        0,
        pm.as_ref(),
        &paint,
        Transform::from_row(scale, 0.0, 0.0, scale, dx, dy),
        None,
    );
    out
}

/// Rescales an image to fit inside the canvas, preserving the aspect ratio.
pub fn fit_into(width: u32, height: u32, src_w: u32, src_h: u32) -> (u32, u32, u32, u32) {
    if src_w == 0 || src_h == 0 {
        return (0, 0, width, height);
    }
    let scale = (width as f32 / src_w as f32).min(height as f32 / src_h as f32);
    let w = ((src_w as f32 * scale).round() as u32).clamp(1, width);
    let h = ((src_h as f32 * scale).round() as u32).clamp(1, height);
    ((width - w) / 2, (height - h) / 2, w, h)
}
