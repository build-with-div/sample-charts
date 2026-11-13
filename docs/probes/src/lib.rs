//! Shared helpers for the chart probes. Everything here uses Div's public
//! API (`divui`) plus `skia-safe`, the same Skia build Div links.

use div::prelude::*;
use div::typed_style::{parse_literal, StyleProperty};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// shadcn `--chart-1` (light), `oklch(0.646 0.222 41.116)`, as sRGB.
pub const CHART_1: (u8, u8, u8) = (0xf5, 0x4a, 0x00);
/// shadcn `--chart-2` (light), `oklch(0.6 0.118 184.704)`, as sRGB.
pub const CHART_2: (u8, u8, u8) = (0x00, 0x9e, 0x89);

/// Set one CSS property from text at run time, the way a chart layer would
/// position a mark. Panics on a literal Div rejects, so probes show it.
pub fn css(element: &Element, name: &str, value: &str) {
    let property = StyleProperty::from_css_name(name)
        .unwrap_or_else(|| panic!("unknown Div style property {name}"));
    let parsed = parse_literal(property, value)
        .unwrap_or_else(|error| panic!("Div rejects `{name}: {value}`: {error:?}"));
    element.set_property(property, parsed).unwrap();
}

/// A deterministic test series: a sum of sines with a slow trend.
pub fn series(n: usize) -> Vec<(f64, f64)> {
    (0..n)
        .map(|i| {
            let x = i as f64;
            let t = x / n.max(1) as f64;
            let y = 50.0
                + 20.0 * (t * 12.0).sin()
                + 8.0 * (t * 97.0).sin()
                + 3.0 * (x * 0.7).sin()
                + 15.0 * t;
            (x, y)
        })
        .collect()
}

/// Largest-Triangle-Three-Buckets decimation (Steinarsson 2013). Keeps the
/// first and last point and one point per bucket, `threshold` points total.
pub fn lttb(data: &[(f64, f64)], threshold: usize) -> Vec<(f64, f64)> {
    if threshold >= data.len() || threshold < 3 {
        return data.to_vec();
    }
    let mut out = Vec::with_capacity(threshold);
    let every = (data.len() - 2) as f64 / (threshold - 2) as f64;
    let mut a = 0usize;
    out.push(data[0]);
    for i in 0..threshold - 2 {
        let avg_start = ((i + 1) as f64 * every) as usize + 1;
        let avg_end = (((i + 2) as f64 * every) as usize + 1).min(data.len());
        let (mut ax, mut ay) = (0.0, 0.0);
        let count = (avg_end - avg_start).max(1) as f64;
        for p in &data[avg_start..avg_end] {
            ax += p.0;
            ay += p.1;
        }
        ax /= count;
        ay /= count;
        let start = (i as f64 * every) as usize + 1;
        let end = ((i + 1) as f64 * every) as usize + 1;
        let (px, py) = data[a];
        let mut best = start;
        let mut best_area = -1.0;
        for (j, p) in data[start..end].iter().enumerate() {
            let area = ((px - ax) * (p.1 - py) - (px - p.0) * (ay - py)).abs();
            if area > best_area {
                best_area = area;
                best = start + j;
            }
        }
        out.push(data[best]);
        a = best;
    }
    out.push(data[data.len() - 1]);
    out
}

/// Map data points into a `width` x `height` plot box (y up), with `pad`.
pub fn to_pixels(data: &[(f64, f64)], width: f32, height: f32, pad: f32) -> Vec<(f32, f32)> {
    let (mut x0, mut x1, mut y0, mut y1) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for &(x, y) in data {
        x0 = x0.min(x);
        x1 = x1.max(x);
        y0 = y0.min(y);
        y1 = y1.max(y);
    }
    let sx = (width - 2.0 * pad) as f64 / (x1 - x0).max(f64::EPSILON);
    let sy = (height - 2.0 * pad) as f64 / (y1 - y0).max(f64::EPSILON);
    data.iter()
        .map(|&(x, y)| {
            (
                pad + ((x - x0) * sx) as f32,
                height - pad - ((y - y0) * sy) as f32,
            )
        })
        .collect()
}

// div-gap: 1, 12 (docs/design.md work list). Scaffolding: Div has no inline
// SVG shapes and no Canvas 2D, so the probes draw marks with Skia and show
// them as an image. Not a proposed solution.
/// Rasterize a stroked polyline with Skia into Div's decoded-RGBA image
/// format (`image/x-openui-rgba8`: "OUIR", version 1, three zero bytes,
/// little-endian width and height, then unpremultiplied RGBA rows).
/// `scale` is the device pixel ratio.
pub fn skia_polyline_rgba8(
    width: u32,
    height: u32,
    scale: f32,
    points: &[(f32, f32)],
    color: (u8, u8, u8),
    stroke_width: f32,
) -> Vec<u8> {
    let (pw, ph) = (
        (width as f32 * scale).round() as i32,
        (height as f32 * scale).round() as i32,
    );
    let info = skia_safe::ImageInfo::new(
        (pw, ph),
        skia_safe::ColorType::RGBA8888,
        skia_safe::AlphaType::Unpremul,
        None,
    );
    let mut bytes = vec![0u8; 16 + (pw * ph * 4) as usize];
    bytes[..4].copy_from_slice(b"OUIR");
    bytes[4] = 1;
    bytes[8..12].copy_from_slice(&(pw as u32).to_le_bytes());
    bytes[12..16].copy_from_slice(&(ph as u32).to_le_bytes());
    {
        let mut surface =
            skia_safe::surfaces::wrap_pixels(&info, &mut bytes[16..], (pw * 4) as usize, None)
                .expect("a raster surface over the resource bytes");
        let canvas = surface.canvas();
        canvas.scale((scale, scale));
        draw_polyline(canvas, points, color, stroke_width);
    }
    bytes
}

/// Stroke `points` on a Skia canvas the way an SVG `<path fill="none">`
/// with `stroke-linejoin`/`stroke-linecap` defaults is stroked.
pub fn draw_polyline(
    canvas: &skia_safe::Canvas,
    points: &[(f32, f32)],
    color: (u8, u8, u8),
    stroke_width: f32,
) {
    let mut path = skia_safe::PathBuilder::new();
    if let Some(&(x, y)) = points.first() {
        path.move_to((x, y));
        for &(x, y) in &points[1..] {
            path.line_to((x, y));
        }
    }
    let mut paint = skia_safe::Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(skia_safe::paint::Style::Stroke);
    paint.set_stroke_width(stroke_width);
    paint.set_color(skia_safe::Color::from_rgb(color.0, color.1, color.2));
    canvas.draw_path(&path.detach(), &paint);
}

static RESOURCE_SERIAL: AtomicU64 = AtomicU64::new(0);

/// Register `rgba8` bytes as a new image resource. The key must be unique
/// per content: Div caches decoded images by the sha256 string for the
/// thread's lifetime and never unregisters a resource.
pub fn register_rgba8(document: &Document, rgba8: Vec<u8>) -> div::typed_style::ImageResourceId {
    let serial = RESOURCE_SERIAL.fetch_add(1, Ordering::Relaxed);
    document
        .register_image_resource(
            format!("chart-raster-{serial}"),
            "image/x-openui-rgba8",
            format!("{serial:064x}"),
            rgba8,
        )
        .unwrap()
}

/// Count pixels in `rect` (x, y, w, h) whose color differs from white by
/// more than `tolerance` in any channel. Used to tell "nothing drawn".
pub fn ink(bitmap: &Bitmap, rect: (u32, u32, u32, u32), tolerance: u8) -> usize {
    let (x0, y0, w, h) = rect;
    let mut count = 0;
    for y in y0..(y0 + h).min(bitmap.height()) {
        for x in x0..(x0 + w).min(bitmap.width()) {
            let i = y as usize * bitmap.stride() + x as usize * 4;
            let p = &bitmap.pixels()[i..i + 3];
            if p.iter().any(|&c| 255 - c > tolerance) {
                count += 1;
            }
        }
    }
    count
}

/// Run `f` and return its result with the wall time it took.
pub fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let value = f();
    (value, start.elapsed())
}

/// Milliseconds with two decimals, for the probe tables.
pub fn ms(d: Duration) -> String {
    format!("{:.2}", d.as_secs_f64() * 1000.0)
}

/// Directory for probe output images (ignored by git).
pub fn out_dir() -> std::path::PathBuf {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("out");
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
