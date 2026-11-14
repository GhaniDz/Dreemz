# Dreemz

A frame-by-frame 2D animation app for Windows and Linux, written in Rust with
[egui/eframe](https://github.com/emilk/egui) and [tiny-skia](https://github.com/RazrFalcon/tiny-skia).

Dreemz is built around a simple idea: draw on frames, watch the reel, and record
performances in real time without leaving the canvas.

## Features

- **Perform mode** — a real-time recorder. Press `P`, draw while the clock runs,
  and every stroke lands on the frame that is live at that moment. The reel does
  not play back while you record, the timeline grows to fit the take, and
  `Space` stops recording and plays the result from the top.
- **Brush engine** — pressure-aware strokes with five presets (Pencil, Ink,
  Marker, Airbrush, Soft), plus eraser, flood fill, eyedropper, pixel move and
  hand tools.
- **Procreate Dreams-style timeline** — one track lane per layer, drawings shown
  as clips that span their holds, thumbnail headers, a zoomable ruler, and a
  frame-width playhead band that is always clear about the current frame.
- **Keyframes and holds** — insert, duplicate, delete, and drag drawings between
  frames. Empty frames hold the previous drawing, and onion skinning shows the
  neighbouring frames while you work.
- **Layers** — add, duplicate and merge down; visibility, locking, opacity and
  twelve blend modes (normal, multiply, screen, overlay, darken, lighten, add,
  difference, hue, saturation, colour, luminosity).
- **Rectangular selection** — move, copy, cut and paste regions; select all,
  deselect, and flip drawings horizontally or vertically.
- **Canvas** — pan, zoom and rotate; fit or actual size; composition guides
  (thirds, centre cross, safe area).
- **Export** — PNG image sequence, animated GIF, sprite sheet and MP4 video
  (MP4 requires `ffmpeg` on `PATH`), with frame range, scale and optional
  background.
- **Native project files** — compact `.dreemz` files with full undo/redo
  history.

## Getting started

### Prerequisites

- A stable Rust toolchain (install from [rustup.rs](https://rustup.rs)).
- On Linux, the usual eframe/winit build dependencies for X11 or Wayland.
- Optional: `ffmpeg` for MP4 export.

### Build and run

```sh
cargo run --release
```

The release binary is written to `target/release/` (`dreemz.exe` on Windows).

Run the test suite headlessly with:

```sh
cargo test
```

## Keyboard shortcuts

| Key | Action |
| --- | --- |
| `B` / `E` / `G` / `I` / `V` / `M` / `H` | Brush, eraser, fill, eyedropper, move, rectangular selection, hand |
| `Space` | Play / pause (also stops a performance and plays it) |
| `P` | Perform mode — record strokes onto the timeline in real time |
| `←` / `→` | Previous / next frame |
| `O` | Toggle onion skin |
| `L` | Toggle loop |
| `[` / `]` | Decrease / increase brush size |
| `Ctrl+A` / `Esc` | Select all / deselect |
| `Ctrl+C` / `Ctrl+X` / `Ctrl+V` | Copy / cut / paste selection or drawing |
| `Ctrl+Shift+H` / `Ctrl+Shift+V` | Flip drawing horizontally / vertically |
| `Ctrl+D` | Duplicate keyframe |
| `Delete` | Delete selection or keyframe |
| `Ctrl+Z` / `Ctrl+Y` | Undo / redo |
| `Ctrl+N` / `Ctrl+O` / `Ctrl+S` / `Ctrl+Shift+S` / `Ctrl+E` | New, open, save, save as, export |
| `Ctrl+0` / `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | Fit, actual size, zoom in, zoom out |
| Wheel | Zoom canvas |
| `Shift`+wheel | Rotate canvas |
| Middle-drag / `H` | Pan canvas |
| `Ctrl`+wheel (timeline) | Zoom timeline |
| `Shift`+wheel (timeline) | Scroll tracks |

## Project files

Dreemz projects use the `.dreemz` extension. A project stores the canvas size,
frame rate, frame count, layers, blend settings and every drawing, plus undo
history while the app is open. Saves are written to a temporary file and then
renamed, so an interrupted save never destroys the previous version.

## Repository layout

```
src/
  main.rs        eframe entry point and panel layout
  app.rs         application state, tools, playback and recording logic
  model.rs       document, layers, drawings, keyframes and undo history
  brush.rs       pressure-aware brush engine
  paint.rs       compositing, thumbnails, flood fill and pixel operations
  project.rs     .dreemz persistence and PNG/GIF/sprite-sheet/MP4 export
  theme.rs       colours and visual styling
  icons.rs       vector icons
  ui/            canvas, timeline, side panels and dialogs
  tests.rs       headless model, painting and export tests
```

## License

MIT
