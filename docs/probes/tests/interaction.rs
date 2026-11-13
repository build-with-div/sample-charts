//! Probe 4: the input a chart plot area needs (crosshair, brush, pan, wheel
//! zoom, keyboard focus on points), on an `<img>` plot surface.
//!
//! `cargo test --release --test interaction -- --nocapture`

use chart_probes::*;
use div::prelude::*;
use div::PointerEventKind;
use std::cell::RefCell;
use std::rc::Rc;

fn setup() -> (Document, Element, Rc<RefCell<Vec<String>>>) {
    let document = Document::new(800, 400).unwrap();
    css(&document.body(), "margin", "0px");
    let plot = Element::create(&document, "img").unwrap();
    let id = register_rgba8(
        &document,
        skia_polyline_rgba8(400, 200, 1.0, &[(0.0, 100.0), (400.0, 100.0)], CHART_1, 2.0),
    );
    plot.set_image_resource(id, Some((400.0, 200.0))).unwrap();
    css(&plot, "position", "absolute");
    css(&plot, "left", "50px");
    css(&plot, "top", "50px");
    css(&plot, "width", "400px");
    css(&plot, "height", "200px");
    plot.set_attribute("tabindex", "0").unwrap();
    document.body().append_child(&plot).unwrap();

    let log = Rc::new(RefCell::new(Vec::new()));
    for name in [
        "pointerdown",
        "pointermove",
        "pointerup",
        "mousedown",
        "mousemove",
        "mouseup",
        "click",
        "dblclick",
        "contextmenu",
        "wheel",
        "keydown",
        "focus",
        "mouseleave",
        "pointerleave",
    ] {
        let log = log.clone();
        let target = plot.clone();
        plot.on(name, move |event| {
            if event.event_type == "pointerdown" {
                let r = target.set_pointer_capture(event.pointer_id);
                log.borrow_mut()
                    .push(format!("  set_pointer_capture -> {r:?}"));
            }
            let rect = target.bounding_rect().unwrap().unwrap();
            log.borrow_mut().push(format!(
                "{} client=({}, {}) local=({}, {}) delta=({}, {}) key={:?} button={}",
                event.event_type,
                event.mouse_x,
                event.mouse_y,
                event.mouse_x - rect.x,
                event.mouse_y - rect.y,
                event.delta_x,
                event.delta_y,
                event.key_text,
                event.mouse_button,
            ));
        })
        .unwrap();
    }
    document.update_all().unwrap();
    (document, plot, log)
}

fn dump(title: &str, log: &Rc<RefCell<Vec<String>>>) {
    println!("--- {title}");
    for line in log.borrow_mut().drain(..) {
        println!("{line}");
    }
}

#[test]
fn interaction() {
    let (document, plot, log) = setup();
    let none = Modifiers::NONE;

    // Brush/pan drag with pointer names: press inside, move and release
    // outside the plot. With capture, the move and up should reach it.
    document
        .dispatch_pointer_event(
            1,
            PointerEventKind::Down,
            100.0,
            100.0,
            MouseButton::Left,
            none,
            true,
        )
        .unwrap();
    document
        .dispatch_pointer_event(
            1,
            PointerEventKind::Move,
            600.0,
            300.0,
            MouseButton::Left,
            none,
            true,
        )
        .unwrap();
    document
        .dispatch_pointer_event(
            1,
            PointerEventKind::Up,
            600.0,
            300.0,
            MouseButton::Left,
            none,
            true,
        )
        .unwrap();
    dump("pointer drag (pointer names) inside -> outside", &log);

    // The same with mouse names (what dispatch_mouse_event sends).
    document
        .dispatch_mouse_event(MouseEventType::Down, 100.0, 100.0, MouseButton::Left, none)
        .unwrap();
    document
        .dispatch_mouse_event(MouseEventType::Move, 600.0, 300.0, MouseButton::Left, none)
        .unwrap();
    document
        .dispatch_mouse_event(MouseEventType::Up, 600.0, 300.0, MouseButton::Left, none)
        .unwrap();
    dump("mouse drag inside -> outside", &log);

    // Hover crosshair: moves inside only.
    for x in [60.0, 200.0, 440.0] {
        document
            .dispatch_pointer_event(
                2,
                PointerEventKind::Move,
                x,
                150.0,
                MouseButton::Left,
                none,
                true,
            )
            .unwrap();
    }
    dump("hover moves", &log);

    // Right click (no contextmenu expected) and wheel zoom with Ctrl.
    document
        .dispatch_mouse_event(MouseEventType::Down, 200.0, 150.0, MouseButton::Right, none)
        .unwrap();
    document
        .dispatch_mouse_event(MouseEventType::Up, 200.0, 150.0, MouseButton::Right, none)
        .unwrap();
    document
        .dispatch_wheel_event(200.0, 150.0, 0.0, -120.0, Modifiers::CTRL)
        .unwrap();
    dump("right click + ctrl wheel", &log);

    // Keyboard: focus the plot and step through points with arrows.
    plot.focus().unwrap();
    document
        .dispatch_key(KeyEventType::Down, Key::ArrowRight, none, false)
        .unwrap();
    document
        .dispatch_key(KeyEventType::Down, Key::Home, none, false)
        .unwrap();
    dump("focus + ArrowRight + Home", &log);
    println!(
        "focused is plot: {:?}",
        document
            .focused_element()
            .unwrap()
            .map(|e| e.is_same_node(&plot))
    );

    let tree = document.accessibility_update().unwrap();
    println!("accessibility nodes in the update: {}", tree.nodes.len());
}
