//! Probe 3: a large line series drawn by the app with Skia and shown in Div
//! as an `image/x-openui-rgba8` resource (the only anti-aliased line path
//! Div's public API has today), with and without LTTB decimation, and the
//! cost of replacing the image every frame (streaming).
//!
//! `cargo test --release --test raster_cost -- --nocapture --test-threads 1`

use chart_probes::*;
use div::prelude::*;

const W: u32 = 800;
const H: u32 = 400;

fn host() -> (Document, Element) {
    let document = Document::new(W as i32, H as i32).unwrap();
    css(&document.body(), "margin", "0px");
    let img = Element::create(&document, "img").unwrap();
    css(&img, "display", "block");
    css(&img, "width", &format!("{W}px"));
    css(&img, "height", &format!("{H}px"));
    document.body().append_child(&img).unwrap();
    (document, img)
}

#[test]
fn raster_cost() {
    println!(
        "| N | scale | LTTB to | lttb ms | map ms | raster ms | handoff+render ms | total ms |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    for n in [1_000usize, 10_000, 100_000, 1_000_000] {
        let data = series(n);
        for scale in [1.0f32, 2.0] {
            for target in [None, Some(2 * W as usize)] {
                let (document, img) = host();
                // Warm the first decode path once so the row measures steady state.
                let (decimated, t_lttb) = timed(|| match target {
                    Some(t) => lttb(&data, t),
                    None => data.clone(),
                });
                let (pts, t_map) = timed(|| to_pixels(&decimated, W as f32, H as f32, 8.0));
                let (bytes, t_raster) =
                    timed(|| skia_polyline_rgba8(W, H, scale, &pts, CHART_1, 2.0));
                let ((), t_handoff) = timed(|| {
                    let id = register_rgba8(&document, bytes);
                    img.set_image_resource(id, Some((W as f32, H as f32)))
                        .unwrap();
                    document.update_all().unwrap();
                    document.render_to_bitmap().unwrap();
                });
                println!(
                    "| {n} | {scale} | {} | {} | {} | {} | {} | {} |",
                    target.map_or("-".to_string(), |t| t.to_string()),
                    ms(t_lttb),
                    ms(t_map),
                    ms(t_raster),
                    ms(t_handoff),
                    ms(t_lttb + t_map + t_raster + t_handoff)
                );
                if n == 100_000 && scale == 1.0 && target.is_some() {
                    std::fs::write(
                        out_dir().join("raster_100k_lttb.png"),
                        document.render_to_png_buffer().unwrap(),
                    )
                    .unwrap();
                }
            }
        }
    }
}

/// Streaming: replace the image 120 times (a 2 s window at 60 Hz), sliding a
/// 10k-point window over a 100k series, decimated to 1,600 points.
#[test]
fn streaming_frames() {
    let (document, img) = host();
    let data = series(100_000);
    let frames = 120;
    let mut retained = 0usize;
    let mut worst = std::time::Duration::ZERO;
    let ((), total) = timed(|| {
        for f in 0..frames {
            let ((), t) = timed(|| {
                let window = &data[f * 500..f * 500 + 10_000];
                let pts = to_pixels(&lttb(window, 1_600), W as f32, H as f32, 8.0);
                let bytes = skia_polyline_rgba8(W, H, 1.0, &pts, CHART_1, 2.0);
                retained += bytes.len();
                let id = register_rgba8(&document, bytes);
                img.set_image_resource(id, Some((W as f32, H as f32)))
                    .unwrap();
                document.update_all().unwrap();
                document.render_to_bitmap().unwrap();
            });
            worst = worst.max(t);
        }
    });
    println!(
        "streaming: {frames} frames, mean {} ms, worst {} ms; resource bytes never freed: {:.1} MB",
        ms(total / frames as u32),
        ms(worst),
        retained as f64 / 1e6
    );
}
