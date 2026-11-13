//! Probe 1: draw a 2 px polyline (40 points) with what Div has today.
//!
//! A. one rotated `<div>` per segment (CSS transform);
//! B. an `<img>` showing an `image/svg+xml` resource with `<polyline>` and
//!    `<path>` strokes (Div's built-in static SVG rasterizer);
//! C. an `<img>` showing an `image/x-openui-rgba8` resource the app
//!    rasterized with Skia.
//!
//! `cargo test --release --test polyline_today -- --nocapture`
//! writes `out/polyline_today.png` and prints ink counts and timings.

use chart_probes::*;
use div::prelude::*;

const W: u32 = 360;
const H: u32 = 160;
const STROKE: f32 = 2.0;

fn panel(document: &Document, top: u32) -> Element {
    let panel = Element::create(document, "div").unwrap();
    css(&panel, "position", "absolute");
    css(&panel, "left", "20px");
    css(&panel, "top", &format!("{top}px"));
    css(&panel, "width", &format!("{W}px"));
    css(&panel, "height", &format!("{H}px"));
    document.body().append_child(&panel).unwrap();
    panel
}

fn points() -> Vec<(f32, f32)> {
    to_pixels(&series(40), W as f32, H as f32, 8.0)
}

fn hex((r, g, b): (u8, u8, u8)) -> String {
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// A: rotated divs, one per segment, rotated about their centre.
fn segments(document: &Document, panel: &Element) -> usize {
    let pts = points();
    for pair in pts.windows(2) {
        let ((x0, y0), (x1, y1)) = (pair[0], pair[1]);
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        let (mx, my) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
        let seg = Element::create(document, "div").unwrap();
        css(&seg, "position", "absolute");
        css(&seg, "left", &format!("{}px", mx - len / 2.0));
        css(&seg, "top", &format!("{}px", my - STROKE / 2.0));
        css(&seg, "width", &format!("{len}px"));
        css(&seg, "height", &format!("{STROKE}px"));
        css(&seg, "background-color", &hex(CHART_1));
        css(&seg, "transform", &format!("rotate({}rad)", dy.atan2(dx)));
        panel.append_child(&seg).unwrap();
    }
    pts.len() - 1
}

/// B: Div's static SVG image rasterizer with stroked shapes.
fn svg_image(document: &Document, panel: &Element) {
    let pts = points();
    let list: Vec<String> = pts.iter().map(|(x, y)| format!("{x:.2},{y:.2}")).collect();
    let d: Vec<String> = pts
        .iter()
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x:.2} {y:.2}", if i == 0 { "M" } else { "L" }))
        .collect();
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{W}" height="{H}" viewBox="0 0 {W} {H}">
<polyline points="{}" fill="none" stroke="{c}" stroke-width="{STROKE}"/>
<path d="{}" fill="none" stroke="{c}" stroke-width="{STROKE}" transform="translate(0,4)"/>
<line x1="0" y1="{H}" x2="{W}" y2="0" stroke="{c}" stroke-width="1"/>
</svg>"#,
        list.join(" "),
        d.join(" "),
        c = hex(CHART_2),
    );
    let id = document
        .register_image_resource("polyline.svg", "image/svg+xml", "probe-polyline-svg", svg)
        .unwrap();
    let img = Element::create(document, "img").unwrap();
    img.set_image_resource(id, Some((W as f32, H as f32)))
        .unwrap();
    css(&img, "width", &format!("{W}px"));
    css(&img, "height", &format!("{H}px"));
    css(&img, "display", "block");
    panel.append_child(&img).unwrap();
}

/// C: app-side Skia raster handed to Div as decoded RGBA.
fn raster_image(document: &Document, panel: &Element) {
    let bytes = skia_polyline_rgba8(W, H, 1.0, &points(), CHART_1, STROKE);
    let id = register_rgba8(document, bytes);
    let img = Element::create(document, "img").unwrap();
    img.set_image_resource(id, Some((W as f32, H as f32)))
        .unwrap();
    css(&img, "width", &format!("{W}px"));
    css(&img, "height", &format!("{H}px"));
    css(&img, "display", "block");
    panel.append_child(&img).unwrap();
}

#[test]
fn polyline_today() {
    let document = Document::new(400, 3 * (H as i32 + 20) + 20).unwrap();
    css(&document.body(), "margin", "0px");
    css(&document.body(), "background-color", "#ffffff");

    let tops = [20, 20 + H + 20, 20 + 2 * (H + 20)];
    let (n, t_a) = timed(|| segments(&document, &panel(&document, tops[0])));
    let ((), t_b) = timed(|| svg_image(&document, &panel(&document, tops[1])));
    let ((), t_c) = timed(|| raster_image(&document, &panel(&document, tops[2])));
    let (bitmap, t_render) = timed(|| document.render_to_bitmap().unwrap());

    let ink_a = ink(&bitmap, (20, tops[0], W, H), 8);
    let ink_b = ink(&bitmap, (20, tops[1], W, H), 8);
    let ink_c = ink(&bitmap, (20, tops[2], W, H), 8);
    println!(
        "A rotated divs ({n} segments): build {} ms, ink {ink_a} px",
        ms(t_a)
    );
    println!(
        "B svg image (polyline+path+line): build {} ms, ink {ink_b} px",
        ms(t_b)
    );
    println!(
        "C skia rgba8 image: build+raster {} ms, ink {ink_c} px",
        ms(t_c)
    );
    println!("first render of all three: {} ms", ms(t_render));

    // Hit testing on a rotated segment: Div maps the point through the
    // transform (or not); report which.
    let pts = points();
    let (x0, y0) = pts[10];
    let (x1, y1) = pts[11];
    let probe = (20.0 + (x0 + x1) / 2.0, tops[0] as f32 + (y0 + y1) / 2.0);
    let hit = document.hit_test(probe.0, probe.1).unwrap();
    println!(
        "hit_test at the middle of segment 10 -> {:?}",
        hit.map(|e| e.kind().unwrap())
    );

    std::fs::write(
        out_dir().join("polyline_today.png"),
        document.render_to_png_buffer().unwrap(),
    )
    .unwrap();
    assert!(ink_a > 0 && ink_c > 0);
}
