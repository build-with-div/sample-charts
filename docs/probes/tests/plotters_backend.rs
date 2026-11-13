//! Probe 5: does `plotters` fit as the drawing layer? A `DrawingBackend`
//! implemented on a Skia canvas (what a Div canvas backend would be), a
//! plotters line chart drawn through it, and the result shown in Div as an
//! RGBA image.
//!
//! `cargo test --release --test plotters_backend -- --nocapture`

use chart_probes::*;
use div::prelude::*;
use plotters::prelude::*;
use plotters_backend::{
    BackendColor, BackendCoord, BackendStyle, DrawingBackend, DrawingErrorKind,
};

/// plotters backend over a Skia canvas. Note the coordinate type: plotters
/// hands the backend integer pixels (`BackendCoord = (i32, i32)`).
struct SkiaBackend<'a> {
    canvas: &'a skia_safe::Canvas,
    size: (u32, u32),
    calls: std::rc::Rc<std::cell::Cell<usize>>,
}

#[derive(Debug)]
struct NoError;
impl std::fmt::Display for NoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("no error")
    }
}
impl std::error::Error for NoError {}

fn paint(color: BackendColor, width: u32, fill: bool) -> skia_safe::Paint {
    let mut p = skia_safe::Paint::default();
    p.set_anti_alias(true);
    let (r, g, b) = color.rgb;
    p.set_color(skia_safe::Color::from_argb(
        (color.alpha * 255.0).round() as u8,
        r,
        g,
        b,
    ));
    if fill {
        p.set_style(skia_safe::paint::Style::Fill);
    } else {
        p.set_style(skia_safe::paint::Style::Stroke);
        p.set_stroke_width(width as f32);
    }
    p
}

impl DrawingBackend for SkiaBackend<'_> {
    type ErrorType = NoError;

    fn get_size(&self) -> (u32, u32) {
        self.size
    }
    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<NoError>> {
        Ok(())
    }
    fn present(&mut self) -> Result<(), DrawingErrorKind<NoError>> {
        Ok(())
    }
    fn draw_pixel(
        &mut self,
        (x, y): BackendCoord,
        color: BackendColor,
    ) -> Result<(), DrawingErrorKind<NoError>> {
        self.calls.set(self.calls.get() + 1);
        self.canvas.draw_rect(
            skia_safe::Rect::from_xywh(x as f32, y as f32, 1.0, 1.0),
            &paint(color, 0, true),
        );
        Ok(())
    }
    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<NoError>> {
        self.calls.set(self.calls.get() + 1);
        // An SVG-style 1px line sits on the half pixel; plotters gives none.
        self.canvas.draw_line(
            (from.0 as f32 + 0.5, from.1 as f32 + 0.5),
            (to.0 as f32 + 0.5, to.1 as f32 + 0.5),
            &paint(style.color(), style.stroke_width(), false),
        );
        Ok(())
    }
    fn draw_rect<S: BackendStyle>(
        &mut self,
        ul: BackendCoord,
        br: BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<NoError>> {
        self.calls.set(self.calls.get() + 1);
        self.canvas.draw_rect(
            skia_safe::Rect::from_ltrb(ul.0 as f32, ul.1 as f32, br.0 as f32, br.1 as f32),
            &paint(style.color(), style.stroke_width(), fill),
        );
        Ok(())
    }
    fn draw_path<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<NoError>> {
        self.calls.set(self.calls.get() + 1);
        let mut b = skia_safe::PathBuilder::new();
        for (i, (x, y)) in path.into_iter().enumerate() {
            if i == 0 {
                b.move_to((x as f32, y as f32));
            } else {
                b.line_to((x as f32, y as f32));
            }
        }
        self.canvas.draw_path(
            &b.detach(),
            &paint(style.color(), style.stroke_width(), false),
        );
        Ok(())
    }
    fn draw_circle<S: BackendStyle>(
        &mut self,
        center: BackendCoord,
        radius: u32,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<NoError>> {
        self.calls.set(self.calls.get() + 1);
        self.canvas.draw_circle(
            (center.0 as f32, center.1 as f32),
            radius as f32,
            &paint(style.color(), style.stroke_width(), fill),
        );
        Ok(())
    }
    fn fill_polygon<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        vert: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<NoError>> {
        self.calls.set(self.calls.get() + 1);
        let pts: Vec<skia_safe::Point> = vert
            .into_iter()
            .map(|(x, y)| skia_safe::Point::new(x as f32, y as f32))
            .collect();
        let path = skia_safe::Path::polygon(&pts, true, None, None);
        self.canvas.draw_path(&path, &paint(style.color(), 0, true));
        Ok(())
    }
}

fn draw(canvas: &skia_safe::Canvas, w: u32, h: u32, data: &[(f64, f64)]) -> usize {
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    {
        let root = SkiaBackend {
            canvas,
            size: (w, h),
            calls: calls.clone(),
        }
        .into_drawing_area();
        let (x1, y_max) = data
            .iter()
            .fold((0.0f64, 0.0f64), |a, p| (a.0.max(p.0), a.1.max(p.1)));
        let mut chart = ChartBuilder::on(&root)
            .margin(10)
            .build_cartesian_2d(0.0..x1, 0.0..y_max * 1.1)
            .unwrap();
        // No labels: text in plotters goes through its own font stack
        // (`ttf`/`ab_glyph` features), not the backend's text shaping.
        chart
            .configure_mesh()
            .disable_x_mesh()
            .light_line_style(RGBColor(0xe4, 0xe4, 0xe7))
            .x_labels(0)
            .y_labels(0)
            .draw()
            .unwrap();
        chart
            .draw_series(LineSeries::new(
                data.iter().copied(),
                ShapeStyle::from(RGBColor(CHART_1.0, CHART_1.1, CHART_1.2)).stroke_width(2),
            ))
            .unwrap();
    }
    calls.get()
}

#[test]
fn plotters_backend() {
    const W: u32 = 800;
    const H: u32 = 400;
    for n in [1_000usize, 100_000] {
        let data = series(n);
        let mut surface = skia_safe::surfaces::raster_n32_premul((W as i32, H as i32)).unwrap();
        surface.canvas().clear(skia_safe::Color::WHITE);
        let (calls, t) = timed(|| draw(surface.canvas(), W, H, &data));
        println!("plotters {n} points: {} ms, {calls} backend calls", ms(t));
        if n == 1_000 {
            let image = surface.image_snapshot();
            let png = image
                .encode(None, skia_safe::EncodedImageFormat::PNG, None)
                .unwrap();
            let document = Document::new(W as i32, H as i32).unwrap();
            css(&document.body(), "margin", "0px");
            let id = document
                .register_image_resource(
                    "plotters.png",
                    "image/png",
                    "probe-plotters-png",
                    png.as_bytes().to_vec(),
                )
                .unwrap();
            let img = Element::create(&document, "img").unwrap();
            img.set_image_resource(id, None).unwrap();
            document.body().append_child(&img).unwrap();
            std::fs::write(
                out_dir().join("plotters_1k.png"),
                document.render_to_png_buffer().unwrap(),
            )
            .unwrap();
        }
    }
}
