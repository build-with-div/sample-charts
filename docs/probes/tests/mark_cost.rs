//! Probe 2: cost of one element per mark (the Recharts/SVG model mapped to
//! Div elements), for N scatter dots of 4x4 px in an 800x400 plot.
//!
//! For each N: build the elements, first style+layout (`update_all`),
//! first raster (`render_to_bitmap`), then move every dot and lay out and
//! raster again. "reactive" binds each dot to its own signal through an
//! effect, the way a `For` + `view!` chart would, and updates them all in
//! one `batch`.
//!
//! `cargo test --release --test mark_cost -- --nocapture --test-threads 1`

use chart_probes::*;
use div::prelude::*;
use std::time::Duration;

const W: f32 = 800.0;
const H: f32 = 400.0;
/// Stop growing N once one step costs more than this.
const BUDGET: Duration = Duration::from_secs(60);

fn dot(document: &Document, parent: &Element, x: f32, y: f32) -> Element {
    let e = Element::create(document, "div").unwrap();
    css(&e, "position", "absolute");
    css(&e, "left", &format!("{x}px"));
    css(&e, "top", &format!("{y}px"));
    css(&e, "width", "4px");
    css(&e, "height", "4px");
    css(&e, "border-radius", "2px");
    css(&e, "background-color", "#f54a00");
    parent.append_child(&e).unwrap();
    e
}

fn plot(document: &Document) -> Element {
    css(&document.body(), "margin", "0px");
    let plot = Element::create(document, "div").unwrap();
    css(&plot, "position", "relative");
    css(&plot, "width", &format!("{W}px"));
    css(&plot, "height", &format!("{H}px"));
    document.body().append_child(&plot).unwrap();
    plot
}

fn positions(n: usize, phase: f32) -> Vec<(f32, f32)> {
    (0..n)
        .map(|i| {
            let t = i as f32 / n as f32;
            (
                t * (W - 4.0),
                (H - 4.0) * (0.5 + 0.45 * ((t * 40.0 + phase).sin())),
            )
        })
        .collect()
}

fn raw(n: usize) -> Duration {
    let document = Document::new(W as i32, H as i32).unwrap();
    let plot = plot(&document);
    let pts = positions(n, 0.0);
    let (dots, t_build) = timed(|| {
        pts.iter()
            .map(|&(x, y)| dot(&document, &plot, x, y))
            .collect::<Vec<_>>()
    });
    let ((), t_layout) = timed(|| document.update_all().unwrap());
    let (_, t_raster) = timed(|| document.render_to_bitmap().unwrap());
    let moved = positions(n, 1.0);
    let ((), t_move) = timed(|| {
        for (e, &(x, y)) in dots.iter().zip(&moved) {
            css(e, "left", &format!("{x}px"));
            css(e, "top", &format!("{y}px"));
        }
    });
    let ((), t_relayout) = timed(|| document.update_all().unwrap());
    let (_, t_reraster) = timed(|| document.render_to_bitmap().unwrap());
    println!(
        "| raw | {n} | {} | {} | {} | {} | {} | {} |",
        ms(t_build),
        ms(t_layout),
        ms(t_raster),
        ms(t_move),
        ms(t_relayout),
        ms(t_reraster)
    );
    t_build + t_layout + t_raster + t_move + t_relayout + t_reraster
}

fn reactive(n: usize) -> Duration {
    let document = Document::new(W as i32, H as i32).unwrap();
    let plot = plot(&document);
    let pts = positions(n, 0.0);
    let (signals, t_build) = timed(|| {
        pts.iter()
            .map(|&(x, y)| {
                let s = create_signal((x, y));
                let e = dot(&document, &plot, x, y);
                create_effect(move || {
                    let (x, y) = s.get();
                    css(&e, "left", &format!("{x}px"));
                    css(&e, "top", &format!("{y}px"));
                });
                s
            })
            .collect::<Vec<_>>()
    });
    let ((), t_layout) = timed(|| document.update_all().unwrap());
    let (_, t_raster) = timed(|| document.render_to_bitmap().unwrap());
    let moved = positions(n, 1.0);
    let ((), t_move) = timed(|| {
        batch(|| {
            for (s, &p) in signals.iter().zip(&moved) {
                s.set(p);
            }
        })
    });
    let ((), t_relayout) = timed(|| document.update_all().unwrap());
    let (_, t_reraster) = timed(|| document.render_to_bitmap().unwrap());
    println!(
        "| reactive | {n} | {} | {} | {} | {} | {} | {} |",
        ms(t_build),
        ms(t_layout),
        ms(t_raster),
        ms(t_move),
        ms(t_relayout),
        ms(t_reraster)
    );
    t_build + t_layout + t_raster + t_move + t_relayout + t_reraster
}

#[test]
fn mark_cost() {
    println!(
        "| model | N | build ms | layout ms | raster ms | move ms | relayout ms | reraster ms |"
    );
    println!("| --- | --- | --- | --- | --- | --- | --- | --- |");
    for variant in [raw as fn(usize) -> Duration, reactive] {
        for n in [100, 1_000, 10_000, 100_000] {
            if variant(n) > BUDGET {
                println!("(stopped: step over {} s)", BUDGET.as_secs());
                break;
            }
        }
    }
}
