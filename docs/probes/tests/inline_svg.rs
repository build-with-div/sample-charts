//! Probe 7: what Div does today with the inline SVG elements Recharts emits
//! and with a `<canvas>` element.
//!
//! `cargo test --release --test inline_svg -- --nocapture`

use chart_probes::*;
use div::prelude::*;

/// Every element name Recharts 3.8.0 renders for shadcn's chart examples,
/// plus the ones visx and the canvas libraries need.
const SVG_TAGS: &[&str] = &[
    "svg",
    "g",
    "path",
    "line",
    "rect",
    "circle",
    "polygon",
    "text",
    "tspan",
    "defs",
    "clipPath",
    "linearGradient",
    "radialGradient",
    "stop",
    "pattern",
    "marker",
    "title",
    "desc",
    "foreignObject",
];

#[test]
fn inline_svg() {
    let document = Document::new(400, 200).unwrap();
    css(&document.body(), "margin", "0px");
    css(&document.body(), "background-color", "#ffffff");
    for tag in SVG_TAGS {
        match Element::create(&document, tag) {
            Ok(e) => println!("{tag}: created, kind {:?}", e.kind().unwrap()),
            Err(error) => println!("{tag}: {error:?}"),
        }
    }
    // The supported entry point for SVG children today.
    println!(
        "Element::create_svg_foreign_object: {:?}",
        Element::create_svg_foreign_object(&document).map(|e| e.kind().unwrap())
    );

    // An <svg> root with a child it can hold, and a <canvas>.
    let svg = Element::create(&document, "svg").unwrap();
    svg.set_attribute("width", "200").unwrap();
    svg.set_attribute("height", "100").unwrap();
    svg.set_attribute("viewBox", "0 0 200 100").unwrap();
    document.body().append_child(&svg).unwrap();
    let canvas = Element::create(&document, "canvas").unwrap();
    document.body().append_child(&canvas).unwrap();
    document.update_all().unwrap();
    println!(
        "<svg width=200 height=100>: box {:?}",
        svg.bounding_rect().unwrap()
    );
    println!("<canvas>: box {:?}", canvas.bounding_rect().unwrap());
    let bitmap = document.render_to_bitmap().unwrap();
    println!("ink in the page: {} px", ink(&bitmap, (0, 0, 400, 200), 0));
}
