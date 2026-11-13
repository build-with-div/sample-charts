//! Probe 6: is an app-side Skia raster, shown through Div, pixel-equal to
//! the same geometry as SVG in pinned Chromium? Writes, for each scene and
//! scale, `out/fidelity/<scene>.html` (the Chromium side) and
//! `out/fidelity/<scene>@<scale>x-div.png` (the Div side). `compare.py`
//! (next to this file's crate) renders the HTML with Chromium 147 in WSL
//! and counts differing pixels.
//!
//! `cargo test --release --test fidelity_sample -- --nocapture`

use chart_probes::*;
use div::prelude::*;

const W: u32 = 400;
const H: u32 = 200;

struct Scene {
    name: &'static str,
    /// SVG body, user units = CSS px.
    svg: String,
    /// The same marks on a Skia canvas in CSS px.
    draw: Box<dyn Fn(&skia_safe::Canvas)>,
}

fn rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("rgb({r},{g},{b})")
}

fn scenes() -> Vec<Scene> {
    let pts = to_pixels(&series(40), W as f32, H as f32, 12.0);
    let d: String = pts
        .iter()
        .enumerate()
        .map(|(i, (x, y))| format!("{}{x},{y}", if i == 0 { "M" } else { "L" }))
        .collect();
    let line_pts = pts.clone();
    let area_pts = pts.clone();
    let area_d = format!(
        "{d}L{},{}L{},{}Z",
        pts[pts.len() - 1].0,
        H - 12,
        pts[0].0,
        H - 12
    );
    vec![
        Scene {
            name: "line",
            svg: format!(
                r#"<path d="{d}" fill="none" stroke="{}" stroke-width="2"/>"#,
                rgb(CHART_1)
            ),
            draw: Box::new(move |c| draw_polyline(c, &line_pts, CHART_1, 2.0)),
        },
        Scene {
            name: "grid-dashed",
            svg: (0..5)
                .map(|i| {
                    let y = 12.0 + i as f32 * 44.0;
                    format!(
                        r#"<line x1="12" y1="{y}" x2="{}" y2="{y}" stroke="rgb(228,228,231)" stroke-dasharray="3 3"/>"#,
                        W - 12
                    )
                })
                .collect(),
            draw: Box::new(|c| {
                let mut p = skia_safe::Paint::default();
                p.set_anti_alias(true);
                p.set_style(skia_safe::paint::Style::Stroke);
                p.set_stroke_width(1.0);
                p.set_color(skia_safe::Color::from_rgb(228, 228, 231));
                p.set_path_effect(skia_safe::PathEffect::dash(&[3.0, 3.0], 0.0));
                for i in 0..5 {
                    let y = 12.0 + i as f32 * 44.0;
                    c.draw_line((12.0, y), ((W - 12) as f32, y), &p);
                }
            }),
        },
        Scene {
            name: "area",
            svg: format!(
                r#"<path d="{area_d}" fill="{}" fill-opacity="0.4" stroke="none"/>"#,
                rgb(CHART_2)
            ),
            draw: Box::new(move |c| {
                let mut b = skia_safe::PathBuilder::new();
                b.move_to(area_pts[0]);
                for &p in &area_pts[1..] {
                    b.line_to(p);
                }
                b.line_to((area_pts[area_pts.len() - 1].0, (H - 12) as f32));
                b.line_to((area_pts[0].0, (H - 12) as f32));
                b.close();
                let mut p = skia_safe::Paint::default();
                p.set_anti_alias(true);
                p.set_color(skia_safe::Color::from_argb(
                    (0.4f32 * 255.0).round() as u8,
                    CHART_2.0,
                    CHART_2.1,
                    CHART_2.2,
                ));
                c.draw_path(&b.detach(), &p);
            }),
        },
        Scene {
            name: "bars-rounded",
            svg: (0..6)
                .map(|i| {
                    let x = 20.0 + i as f32 * 62.0;
                    let h = 40.0 + (i * 23 % 120) as f32;
                    // Recharts' rounded-top bar path (radius 4).
                    format!(
                        r#"<path d="M{x},{y0}L{x},{yt}A4,4,0,0,1,{x4},{t}L{xw4},{t}A4,4,0,0,1,{xw},{yt}L{xw},{y0}Z" fill="{}"/>"#,
                        rgb(CHART_1),
                        y0 = H - 12,
                        yt = (H - 12) as f32 - h + 4.0,
                        t = (H - 12) as f32 - h,
                        x4 = x + 4.0,
                        xw4 = x + 44.0,
                        xw = x + 48.0,
                    )
                })
                .collect(),
            draw: Box::new(|c| {
                let mut p = skia_safe::Paint::default();
                p.set_anti_alias(true);
                p.set_color(skia_safe::Color::from_rgb(CHART_1.0, CHART_1.1, CHART_1.2));
                for i in 0..6 {
                    let x = 20.0 + i as f32 * 62.0;
                    let h = 40.0 + (i * 23 % 120) as f32;
                    let t = (H - 12) as f32 - h;
                    let rect = skia_safe::Rect::from_xywh(x, t, 48.0, h);
                    let rr = skia_safe::RRect::new_rect_radii(
                        rect,
                        &[
                            (4.0, 4.0).into(),
                            (4.0, 4.0).into(),
                            (0.0, 0.0).into(),
                            (0.0, 0.0).into(),
                        ],
                    );
                    c.draw_rrect(rr, &p);
                }
            }),
        },
    ]
}

/// `opaque`: fill the plot's known background (white) before drawing, so
/// marks blend once, as Chromium blends SVG into the page.
fn div_render(scene: &Scene, scale: f32, opaque: bool) -> Vec<u8> {
    let (pw, ph) = ((W as f32 * scale) as i32, (H as f32 * scale) as i32);
    let mut bytes = vec![0u8; 16 + (pw * ph * 4) as usize];
    bytes[..4].copy_from_slice(b"OUIR");
    bytes[4] = 1;
    bytes[8..12].copy_from_slice(&(pw as u32).to_le_bytes());
    bytes[12..16].copy_from_slice(&(ph as u32).to_le_bytes());
    {
        let info = skia_safe::ImageInfo::new(
            (pw, ph),
            skia_safe::ColorType::RGBA8888,
            skia_safe::AlphaType::Unpremul,
            None,
        );
        let mut surface =
            skia_safe::surfaces::wrap_pixels(&info, &mut bytes[16..], (pw * 4) as usize, None)
                .unwrap();
        if opaque {
            surface.canvas().clear(skia_safe::Color::WHITE);
        }
        surface.canvas().scale((scale, scale));
        (scene.draw)(surface.canvas());
        if opaque {
            // The raster itself, before Div shows it, to split Skia-vs-Chromium
            // differences from Div's image path.
            let png = surface
                .image_snapshot()
                .encode(None, skia_safe::EncodedImageFormat::PNG, None)
                .unwrap();
            std::fs::write(
                out_dir().join(format!("fidelity/{}@{scale}x-raw.png", scene.name)),
                png.as_bytes(),
            )
            .unwrap();
        }
    }
    let viewport = ViewportMetrics::from_logical_size(W as f64, H as f64, scale as f64).unwrap();
    let document = Document::with_viewport_metrics(viewport).unwrap();
    css(&document.body(), "margin", "0px");
    css(&document.body(), "background-color", "#ffffff");
    let img = Element::create(&document, "img").unwrap();
    img.set_image_resource(register_rgba8(&document, bytes), Some((W as f32, H as f32)))
        .unwrap();
    css(&img, "display", "block");
    css(&img, "width", &format!("{W}px"));
    css(&img, "height", &format!("{H}px"));
    document.body().append_child(&img).unwrap();
    document.render_to_png_buffer().unwrap()
}

#[test]
fn fidelity_sample() {
    let dir = out_dir().join("fidelity");
    std::fs::create_dir_all(&dir).unwrap();
    for scene in scenes() {
        let html = format!(
            "<!doctype html><html><body style=\"margin:0;background:#fff\">\
             <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{W}\" height=\"{H}\" \
             style=\"display:block\">{}</svg></body></html>",
            scene.svg
        );
        std::fs::write(dir.join(format!("{}.html", scene.name)), html).unwrap();
        for scale in [1.0f32, 2.0] {
            for (suffix, opaque) in [("div", false), ("div-opaque", true)] {
                std::fs::write(
                    dir.join(format!("{}@{scale}x-{suffix}.png", scene.name)),
                    div_render(&scene, scale, opaque),
                )
                .unwrap();
            }
        }
    }
    println!("wrote {}", dir.display());
}
