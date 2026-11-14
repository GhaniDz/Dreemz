//! Headless tests for the document model, brush engine, painting helpers,
//! project persistence and export. These exercise the app's core without a
//! window, so regressions are caught quickly.

use std::path::PathBuf;

use egui::Color32;
use tiny_skia::Pixmap;

use crate::brush::{BrushSettings, Stroke};
use crate::model::{Document, Drawing, IRect};
use crate::paint;
use crate::project::{self, ExportFormat, ExportSettings};

fn blank_doc() -> Document {
    Document::new("Test", 48, 32, 24.0, 12)
}

fn paint_dot(doc: &mut Document, x: f32, y: f32, color: Color32) -> usize {
    let brush = BrushSettings {
        size: 6.0,
        opacity: 1.0,
        hardness: 1.0,
        spacing: 0.05,
        smoothing: 0.0,
        pressure_size: 0.0,
        simulate_pressure: false,
        color,
    };
    let mut stroke = Stroke::start(doc, &brush, (x, y), 1.0, false).expect("stroke should start");
    let _ = stroke.add_point(doc, &brush, (x + 6.0, y + 4.0), 1.0);
    let entry = stroke.finish(doc, &brush, (x + 8.0, y + 6.0), 1.0);
    entry.map(|e| e.pixel_count()).unwrap_or(0)
}

fn temp_path(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("dreemz_tests_{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir.join(name)
}

fn pixel(doc: &Document, x: u32, y: u32) -> [u8; 4] {
    let d = doc.layers[0].drawing_at(0).expect("drawing");
    let idx = (y as usize * doc.width as usize + x as usize) * 4;
    let p = &d.pixmap.data()[idx..idx + 4];
    [p[0], p[1], p[2], p[3]]
}

#[test]
fn first_stroke_paints_pixels() {
    let mut doc = blank_doc();
    let red = Color32::from_rgb(255, 0, 0);
    let touched = paint_dot(&mut doc, 10.0, 10.0, red);
    assert!(touched > 0, "stroke should produce an undo patch");
    let p = pixel(&doc, 10, 10);
    assert!(p[3] > 0, "painted pixel should be opaque");
    assert_eq!(p[0], 255, "red channel should be full strength");
    assert_eq!(p[1], 0, "green channel should be untouched");
}

#[test]
fn eraser_clears_painted_pixels() {
    let mut doc = blank_doc();
    let red = Color32::from_rgb(255, 0, 0);
    paint_dot(&mut doc, 12.0, 12.0, red);

    let eraser = BrushSettings {
        size: 12.0,
        opacity: 1.0,
        hardness: 1.0,
        spacing: 0.05,
        smoothing: 0.0,
        pressure_size: 0.0,
        simulate_pressure: false,
        color: Color32::BLACK,
    };
    let stroke = Stroke::start(&mut doc, &eraser, (12.0, 12.0), 1.0, true).unwrap();
    let _ = stroke.finish(&mut doc, &eraser, (12.0, 12.0), 1.0);

    assert_eq!(pixel(&doc, 12, 12)[3], 0, "eraser should clear alpha");
}

#[test]
fn keyframes_hold_and_auto_key_preserves_earlier_drawing() {
    let mut doc = blank_doc();
    assert!(doc.insert_keyframe(0, 4, false).is_some());
    assert!(doc.layers[0].drawing_at(6).is_some(), "keyframe holds");
    assert!(doc.layers[0].drawing_at(3).is_some(), "first keyframe holds back");
    assert!(doc.layers[0].drawing_at(11).is_some(), "last keyframe holds forward");

    let original = doc.layers[0].frames.get(&4).unwrap().id;
    doc.playhead = 6;
    let created = doc.ensure_active_drawing().expect("auto key").id;
    assert_ne!(created, original);
    assert!(doc.layers[0].frames.contains_key(&6));
    assert_eq!(doc.layers[0].frames.get(&4).unwrap().id, original);
}

#[test]
fn keyframe_move_and_delete_work() {
    let mut doc = blank_doc();
    assert!(doc.insert_keyframe(0, 5, false).is_some());
    assert!(doc.move_keyframe(0, 5, 7));
    assert!(doc.layers[0].frames.contains_key(&7));
    assert!(!doc.layers[0].frames.contains_key(&5));
    assert!(!doc.move_keyframe(0, 7, 7), "moving onto itself is a no-op");
    assert!(doc.delete_keyframe(0, 7));
    assert!(!doc.layers[0].frames.contains_key(&7));
}

#[test]
fn layer_duplicate_is_independent() {
    let mut doc = blank_doc();
    let red = Color32::from_rgb(255, 0, 0);
    paint_dot(&mut doc, 20.0, 16.0, red);
    let copy = doc.duplicate_layer(0).expect("duplicate");
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(copy, 1);

    doc.layers[1].frames.get_mut(&0).unwrap().pixmap = std::sync::Arc::new(Pixmap::new(
        doc.width,
        doc.height,
    ).unwrap());
    assert_eq!(pixel(&doc, 20, 16), [255, 0, 0, 255], "original stays intact");
}

#[test]
fn flood_fill_fills_blank_layer() {
    let mut doc = blank_doc();
    let entry = paint::flood_fill(
        &mut doc,
        (4, 4),
        Color32::from_rgb(0, 0, 255),
        8,
        true,
    )
    .expect("fill should change pixels");
    assert_eq!(entry.pixel_count(), doc.width as usize * doc.height as usize);
    let p = pixel(&doc, 20, 20);
    assert_eq!(p[0], 0);
    assert_eq!(p[1], 0);
    assert_eq!(p[2], 255);
    assert_eq!(p[3], 255);
}

#[test]
fn translation_moves_pixels() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 10.0, 10.0, Color32::WHITE);
    let snapshot = doc.layers[0].drawing_at(0).unwrap().pixmap.clone();
    let shifted = paint::translated(&snapshot, 5, 3);
    let before = paint::pick(&snapshot, 10, 10).unwrap().a();
    let after = paint::pick(&shifted, 15, 13).unwrap().a();
    assert!(before > 0, "source pixel exists");
    assert!(after > 0, "translated pixel exists at the new position");
    assert_eq!(paint::pick(&shifted, 10, 10).unwrap().a(), 0);
}

#[test]
fn composite_respects_layer_visibility() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 24.0, 16.0, Color32::from_rgb(255, 0, 0));
    let visible = paint::composite_frame(&doc, 0, false);
    assert!(paint::pick(&visible, 24, 16).unwrap().a() > 0);
    doc.layers[0].visible = false;
    let hidden = paint::composite_frame(&doc, 0, false);
    assert_eq!(paint::pick(&hidden, 24, 16).unwrap().a(), 0);
}

#[test]
fn project_roundtrip_preserves_document() {
    let mut doc = blank_doc();
    doc.name = "Round Trip".into();
    doc.background = [20, 30, 40, 255];
    paint_dot(&mut doc, 15.0, 12.0, Color32::from_rgb(30, 200, 90));
    doc.insert_keyframe(0, 5, true);
    doc.layers[0].opacity = 0.6;
    doc.layers[0].blend = crate::model::BlendKind::Multiply;

    let path = temp_path("roundtrip.dreemz");
    project::save_project(&path, &doc).expect("save");
    let loaded = project::load_project(&path).expect("load");
    let _ = std::fs::remove_file(&path);

    assert_eq!(loaded.name, doc.name);
    assert_eq!((loaded.width, loaded.height), (doc.width, doc.height));
    assert_eq!(loaded.frame_count, doc.frame_count);
    assert_eq!(loaded.layers.len(), doc.layers.len());
    assert_eq!(loaded.layers[0].name, doc.layers[0].name);
    assert_eq!(loaded.layers[0].blend, doc.layers[0].blend);
    assert!((loaded.layers[0].opacity - 0.6).abs() < 0.001);
    assert_eq!(loaded.layers[0].frames.len(), 2);
    assert!(loaded.layers[0].frames.contains_key(&5));
    assert_eq!(pixel(&loaded, 15, 12), [30, 200, 90, 255]);
}

#[test]
fn png_sequence_export_writes_frames() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 18.0, 14.0, Color32::from_rgb(200, 120, 40));
    let dir = temp_path("png_seq");
    let _ = std::fs::remove_dir_all(&dir);
    let settings = ExportSettings {
        format: ExportFormat::PngSequence,
        path: dir.clone(),
        start: 0,
        end: 3,
        scale: 1.0,
        background: true,
        columns: 4,
    };
    let count = project::export(&doc, &settings).expect("png export");
    assert_eq!(count, 4);
    let files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "png").unwrap_or(false))
        .collect();
    assert_eq!(files.len(), 4);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gif_export_writes_animation() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 14.0, 12.0, Color32::from_rgb(40, 120, 220));
    let path = temp_path("animation.gif");
    let _ = std::fs::remove_file(&path);
    let settings = ExportSettings {
        format: ExportFormat::Gif,
        path: path.clone(),
        start: 0,
        end: 2,
        scale: 1.0,
        background: true,
        columns: 4,
    };
    let count = project::export(&doc, &settings).expect("gif export");
    assert_eq!(count, 3);
    let meta = std::fs::metadata(&path).expect("gif exists");
    assert!(meta.len() > 32, "gif should contain data");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn import_image_adds_layer() {
    let doc_size = (48u32, 32u32);
    let mut img = image::RgbaImage::new(16, 16);
    for (_, _, px) in img.enumerate_pixels_mut() {
        *px = image::Rgba([220, 40, 90, 255]);
    }
    let path = temp_path("imported.png");
    img.save(&path).expect("write import png");

    let mut doc = Document::new("Import", doc_size.0, doc_size.1, 24.0, 12);
    let name = project::import_image(&mut doc, &path).expect("import");
    let _ = std::fs::remove_file(&path);
    assert_eq!(name, "imported");
    assert_eq!(doc.layers.len(), 2);
    assert_eq!(doc.active_layer, 1);
    let layer = &doc.layers[1];
    assert!(layer.frames.contains_key(&0));
    let pm = &layer.frames.get(&0).unwrap().pixmap;
    assert!(paint::pick(pm, 24, 16).unwrap().a() > 0, "centred image pixels");
}

#[test]
fn save_frame_png_writes_file() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 10.0, 10.0, Color32::WHITE);
    let path = temp_path("frame.png");
    project::save_frame_png(&doc, 0, &path, true, 1.0).expect("frame png");
    let meta = std::fs::metadata(&path).expect("file exists");
    assert!(meta.len() > 32);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn onion_tint_changes_pixels() {
    let pm = Pixmap::new(8, 8).unwrap();
    let mut painted = pm;
    painted.fill(tiny_skia::Color::from_rgba8(80, 120, 200, 255));
    let tinted = paint::tinted(&painted, paint::ONION_BEFORE_TINT, 0.5);
    let p = &tinted.data()[0..4];
    assert!(p[3] < 255, "opacity is applied");
    assert!(p[0] > p[1], "red tint dominates");
}

#[test]
fn resampling_changes_canvas_size_and_keeps_alpha() {
    let mut pm = Pixmap::new(8, 8).unwrap();
    pm.fill(tiny_skia::Color::from_rgba8(30, 90, 190, 128));
    let resized = paint::resampled(&pm, 16, 12);
    assert_eq!((resized.width(), resized.height()), (16, 12));
    assert!(resized.data()[3] > 0, "alpha survives resampling");
}

#[test]
fn pasted_layer_gets_fresh_ids_and_independent_bitmaps() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 8.0, 8.0, Color32::WHITE);
    let original = doc.layers[0].clone();
    let original_layer_id = original.id;
    let original_drawing_id = original.frames.values().next().unwrap().id;

    let at = doc.paste_layer(original.clone());
    assert_eq!(at, 1);
    assert_eq!(doc.layers.len(), 2);
    let pasted = &doc.layers[1];
    assert_ne!(pasted.id, original_layer_id);
    let copied_drawing = pasted.frames.values().next().unwrap();
    assert_ne!(copied_drawing.id, original_drawing_id);
    assert!(!std::sync::Arc::ptr_eq(
        &copied_drawing.pixmap,
        &original.frames.values().next().unwrap().pixmap
    ));
}

#[test]
fn flip_mirrors_pixels_on_both_axes() {
    let mut pm = Pixmap::new(4, 3).unwrap();
    // Paint a single distinctive pixel at (0, 0).
    let idx = 0;
    pm.data_mut()[idx..idx + 4].copy_from_slice(&[200, 40, 90, 255]);
    let horizontal = paint::flip(&pm, true);
    assert_eq!(&horizontal.data()[3 * 4..3 * 4 + 4], &[200, 40, 90, 255]);
    let vertical = paint::flip(&pm, false);
    assert_eq!(&vertical.data()[8 * 4..8 * 4 + 4], &[200, 40, 90, 255]);
}

#[test]
fn duplicate_drawing_objects_share_bitmaps_until_modified() {
    let d = Drawing::new(7, 4, 4);
    let clone = d.clone();
    assert!(std::sync::Arc::ptr_eq(&d.pixmap, &clone.pixmap));
    let mut owned = clone;
    std::sync::Arc::make_mut(&mut owned.pixmap).fill(tiny_skia::Color::BLACK);
    assert!(!std::sync::Arc::ptr_eq(&d.pixmap, &owned.pixmap));
}

#[test]
fn clearing_a_held_frame_keeps_the_earlier_keyframe() {
    let mut doc = blank_doc();
    let red = Color32::from_rgb(255, 0, 0);
    paint_dot(&mut doc, 12.0, 12.0, red);
    assert!(doc.insert_keyframe(0, 5, false).is_some());
    doc.playhead = 3;

    assert!(doc.clear_active_frame());
    assert!(doc.layers[0].frames.contains_key(&3), "auto-key is created");
    assert_eq!(pixel(&doc, 12, 12), [255, 0, 0, 255], "frame 0 is intact");
    let cleared = paint::pick(
        doc.layers[0].frames.get(&3).unwrap().pixmap.as_ref(),
        12,
        12,
    )
    .unwrap();
    assert_eq!(cleared.a(), 0, "the new keyframe is empty");
    assert!(doc.layers[0].frames.contains_key(&5));
}

#[test]
fn record_head_extends_the_timeline_instead_of_wrapping() {
    let mut doc = blank_doc();
    doc.set_playhead(10);
    assert!(doc.advance_record_head());
    assert_eq!(doc.playhead, 11, "the head moves forward");
    assert_eq!(doc.frame_count, 12, "inside the timeline nothing changes");
    assert!(doc.advance_record_head());
    assert_eq!(doc.playhead, 12, "the head passes the last frame");
    assert_eq!(doc.frame_count, 13, "recording grows the timeline");
    assert!(
        doc.layers[0].drawing_at(12).is_some(),
        "the last drawing holds over the new frame"
    );
}

#[test]
fn perform_recording_lands_each_segment_on_its_frame() {
    let mut doc = blank_doc();
    for frame in 1..4u32 {
        doc.set_playhead(frame);
        let before = doc.layers[0].frames.len();
        let painted = paint_dot(&mut doc, 8.0 + frame as f32 * 12.0, 16.0, Color32::BLACK);
        assert!(painted > 0, "frame {frame} should receive pixels");
        assert_eq!(
            doc.layers[0].frames.len(),
            before + 1,
            "frame {frame} gets its own auto-keyed keyframe"
        );
    }
    for frame in 0..4u32 {
        assert!(
            doc.layers[0].frames.contains_key(&frame),
            "recorded frame {frame} exists"
        );
    }
}

#[test]
fn merge_down_combines_layers_and_keeps_keyframes() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 10.0, 10.0, Color32::from_rgb(255, 0, 0));
    let top = doc.add_layer();
    assert!(doc.insert_keyframe(top, 5, false).is_some());
    doc.playhead = 5;
    paint_dot(&mut doc, 30.0, 20.0, Color32::from_rgb(0, 0, 255));

    assert!(doc.merge_down(top));
    assert_eq!(doc.layers.len(), 1);
    assert_eq!(doc.active_layer, 0);

    let layer = &doc.layers[0];
    assert_eq!(layer.frames.len(), 2, "both keyframe boundaries survive");
    let f0 = layer.drawing_at(0).unwrap().pixmap.clone();
    let f5 = layer.drawing_at(5).unwrap().pixmap.clone();
    assert!(paint::pick(&f0, 10, 10).unwrap().a() > 0);
    assert_eq!(paint::pick(&f0, 30, 20).unwrap().a(), 0, "top layer starts at 5");
    assert!(paint::pick(&f5, 10, 10).unwrap().a() > 0, "lower layer holds");
    assert!(paint::pick(&f5, 30, 20).unwrap().a() > 0, "upper is baked in");
}

#[test]
fn merge_down_refuses_the_bottom_layer() {
    let mut doc = blank_doc();
    assert!(!doc.merge_down(0));
    assert_eq!(doc.layers.len(), 1);
}

#[test]
fn region_copy_move_and_stamp_work() {
    let mut pm = Pixmap::new(8, 8).unwrap();
    let idx = (2 * 8 + 2) * 4;
    pm.data_mut()[idx..idx + 4].copy_from_slice(&[10, 20, 30, 255]);

    let rect = IRect::new(1, 1, 4, 4);
    let clip = paint::copy_region(&pm, rect).expect("copy region");
    assert_eq!((clip.width(), clip.height()), (3, 3));

    let moved = paint::moved_region(&pm, rect, 2, 1);
    assert_eq!(paint::pick(&moved, 2, 2).unwrap().a(), 0, "source is cleared");
    assert!(paint::pick(&moved, 4, 3).unwrap().a() > 0, "pixel moved");

    let mut cleared = pm.clone();
    paint::clear_region(&mut cleared, IRect::new(2, 2, 3, 3));
    assert_eq!(paint::pick(&cleared, 2, 2).unwrap().a(), 0);

    let mut stamped = Pixmap::new(8, 8).unwrap();
    paint::stamp_region(&mut stamped, &clip, 5, 5);
    assert!(
        paint::pick(&stamped, 6, 6).unwrap().a() > 0,
        "the clipped pixel lands at the stamped offset"
    );
}

#[test]
fn thumbnail_is_letterboxed_on_the_paper_colour() {
    let mut pm = Pixmap::new(40, 20).unwrap();
    pm.fill(tiny_skia::Color::from_rgba8(200, 60, 60, 255));
    let thumb = paint::thumbnail(&pm, 16, 16, [0, 0, 0, 255]);
    assert_eq!((thumb.width(), thumb.height()), (16, 16));
    assert!(paint::pick(&thumb, 8, 8).unwrap().r() > 100, "drawing is visible");
    assert_eq!(paint::pick(&thumb, 0, 0).unwrap().r(), 0, "paper shows at the edge");
}

#[test]
fn sprite_sheet_export_lays_out_a_grid() {
    let mut doc = blank_doc();
    paint_dot(&mut doc, 18.0, 14.0, Color32::from_rgb(200, 120, 40));
    let path = temp_path("sheet.png");
    let _ = std::fs::remove_file(&path);
    let settings = ExportSettings {
        format: ExportFormat::SpriteSheet,
        path: path.clone(),
        start: 0,
        end: 2,
        scale: 1.0,
        background: true,
        columns: 2,
    };
    let count = project::export(&doc, &settings).expect("sprite sheet export");
    assert_eq!(count, 3);
    let sheet = image::open(&path).expect("sheet exists").to_rgba8();
    assert_eq!(sheet.dimensions(), (96, 64), "3 frames in a 2-column grid");
    let _ = std::fs::remove_file(&path);
}
