//! `<Bar>` (`cartesian/Bar.js`): its layer, the clip path when an axis lets
//! data overflow, the background rectangles and one `<Rectangle>` per bar,
//! each through `util/BarUtils.js`'s `BarRectangle` and
//! `util/ActiveShapeUtils.js`'s `Shape`.
//!
//! The bar renders at rest: its entrance animation's last frame, which is
//! what `isAnimationActive={false}` renders at once. A custom `shape`,
//! `label`s, `<Cell>`s, `<LabelList>`s and `<ErrorBar>`s are not ported;
//! the first two are reported as unsupported. `activeBar` only changes a
//! bar while the tooltip points at it, which is not modelled (no pointer),
//! so it renders as inactive.

use js::Value;

use super::svg_props::svg_properties;
use super::{z_index_layer, Output};
use crate::chart::{BarRectangle, CartesianChart, ChartError, PlotArea};
use crate::dom::{clsx, Element, Node, Props};
use crate::shape::{get_rectangle_path, Radius};

/// `defaultBarProps`.
pub(super) fn default_bar_props() -> Vec<(&'static str, Value)> {
    vec![
        ("activeBar", Value::Bool(false)),
        ("animationBegin", Value::Number(0.0)),
        ("animationDuration", Value::Number(400.0)),
        ("animationEasing", "ease".into()),
        ("background", Value::Bool(false)),
        ("hide", Value::Bool(false)),
        ("isAnimationActive", "auto".into()),
        ("label", Value::Bool(false)),
        ("legendType", "rect".into()),
        ("minPointSize", Value::Number(0.0)),
        ("xAxisId", Value::Number(0.0)),
        ("yAxisId", Value::Number(0.0)),
        ("zIndex", Value::Number(300.0)),
    ]
}

/// `DefaultZIndexes.barBackground`.
const BAR_BACKGROUND_Z_INDEX: f64 = -50.0;

fn svg(tag: &str, props: &Props) -> Element {
    Element::svg(tag).props(props)
}

fn layer(class_name: &[Value], rest: &Props) -> Element {
    // `<Layer className>`: `clsx('recharts-layer', className)` then the
    // other props' SVG attributes.
    let mut props = Props::new();
    let mut classes = vec![Value::from("recharts-layer")];
    classes.extend_from_slice(class_name);
    props.set("className", clsx(&classes).into());
    props.spread_props(&svg_properties(rest));
    svg("g", &props)
}

/// `<Bar>` with its resolved props (defaults applied, `id` resolved).
pub(super) fn render(
    chart: &CartesianChart,
    props: &Props,
    id: &Value,
    plot_area: PlotArea,
    out: &mut Output,
) -> Result<(), ChartError> {
    for (name, what) in [("shape", "a custom bar shape"), ("label", "bar labels")] {
        if props.get(name).is_truthy() {
            return Err(ChartError::Unsupported(what));
        }
    }
    let axis_key = |name| {
        js::Key::of(&props.get(name))
            .ok_or(ChartError::Unsupported("an axis id other than a primitive"))
    };
    let (x_axis_id, y_axis_id) = (axis_key("xAxisId")?, axis_key("yAxisId")?);
    let (clip_x, clip_y) = chart.needs_clip(&x_axis_id, &y_axis_id);
    let need_clip = clip_x || clip_y;
    let rectangles = chart.bar(&id.to_js_string())?.rectangles;

    let bar_z_index = props.get("zIndex");
    // BarWithState renders nothing for a hidden bar or one with no data;
    // its z-index layer is registered all the same.
    let Some(rectangles) = rectangles.filter(|_| !props.get("hide").is_truthy()) else {
        return z_index_layer(&bar_z_index, Vec::new(), out);
    };

    let mut bar = layer(&["recharts-bar".into(), props.get("className")], &{
        let mut p = Props::new();
        p.set("id", id.clone());
        p
    });
    // An id that is not a string still renders as one (`clsx`, `concat`).
    let clip_path_id = format!("clipPath-{}", id.to_js_string());
    if need_clip {
        let PlotArea {
            x,
            y,
            width,
            height,
        } = plot_area;
        let mut rect = Props::new();
        rect.set("x", Value::Number(if clip_x { x } else { x - width / 2.0 }));
        rect.set(
            "y",
            Value::Number(if clip_y { y } else { y - height / 2.0 }),
        );
        rect.set(
            "width",
            Value::Number(if clip_x { width } else { width * 2.0 }),
        );
        rect.set(
            "height",
            Value::Number(if clip_y { height } else { height * 2.0 }),
        );
        let mut clip = Props::new();
        clip.set("id", clip_path_id.clone().into());
        let mut clip_path = svg("clipPath", &clip);
        clip_path.children.push(Node::Element(svg("rect", &rect)));
        let mut defs = Element::svg("defs");
        defs.children.push(Node::Element(clip_path));
        bar.children.push(Node::Element(defs));
    }
    let mut rectangles_layer = layer(&["recharts-bar-rectangles".into()], &{
        let mut p = Props::new();
        if need_clip {
            p.set("clipPath", format!("url(#{clip_path_id})").into());
        }
        p
    });

    // BarBackground, in its own z-index layer.
    let background = props.get("background");
    if background.is_truthy() {
        let background_z_index = Value::Number(match &background {
            Value::Object(_) | Value::Array(_) => match background.get("zIndex") {
                Value::Number(z) if z.is_finite() => z,
                _ => BAR_BACKGROUND_Z_INDEX,
            },
            _ => BAR_BACKGROUND_Z_INDEX,
        });
        let in_portal = |z: &Value| !matches!(z, Value::Number(n) if *n == 0.0);
        if in_portal(&background_z_index)
            && in_portal(&bar_z_index)
            && background_z_index.to_js_string() == bar_z_index.to_js_string()
        {
            return Err(ChartError::Unsupported(
                "a bar background in its bar's own z-index layer",
            ));
        }
        let nodes = background_rectangles(&rectangles, &background)?;
        let mut inline = Output::default();
        z_index_layer(&background_z_index, nodes, &mut inline)?;
        rectangles_layer.children.extend(inline.inline);
        out.portals.extend(inline.portals);
        out.registered.extend(inline.registered);
    }

    // RenderRectangles at rest: a <Layer> of BarStackClipLayers.
    let base_props = {
        let mut p = svg_properties(props);
        p.remove("id");
        p
    };
    let mut all = layer(&[], &Props::new());
    for (index, entry) in rectangles.iter().enumerate() {
        let mut p = base_props.clone();
        p.set("name", p.get("name").to_js_string().into());
        p.spread_props(&rectangle_item(entry));
        p.set("isActive", Value::Bool(false));
        p.set("index", Value::Number(index as f64));
        p.set("dataKey", props.get("dataKey"));
        let mut stack_layer = layer(&["recharts-bar-rectangle".into()], &Props::new());
        stack_layer
            .children
            .push(Node::Element(bar_rectangle(&p, &Value::Undefined)?));
        all.children.push(Node::Element(stack_layer));
    }
    rectangles_layer.children.push(Node::Element(all));
    bar.children.push(Node::Element(rectangles_layer));
    z_index_layer(&bar_z_index, vec![Node::Element(bar)], out)
}

/// A rectangle as `computeBarRectangles` returns it: the data entry's own
/// properties, then the geometry.
fn rectangle_item(entry: &BarRectangle) -> Props {
    let mut item = Props::of(&entry.payload);
    let [bx, by, bw, bh] = entry.background;
    let point = |pairs: &[(&str, f64)]| {
        Value::Object(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), Value::Number(*v)))
                .collect(),
        )
    };
    item.set(
        "stackedBarStart",
        entry
            .stacked_bar_start
            .map_or(Value::Undefined, Value::Number),
    );
    item.set("x", Value::Number(entry.x));
    item.set("y", Value::Number(entry.y));
    item.set("width", Value::Number(entry.width));
    item.set("height", Value::Number(entry.height));
    item.set("value", entry.value.clone());
    item.set("payload", entry.payload.clone());
    item.set(
        "background",
        point(&[("x", bx), ("y", by), ("width", bw), ("height", bh)]),
    );
    item.set(
        "tooltipPosition",
        point(&[
            ("x", entry.x + entry.width / 2.0),
            ("y", entry.y + entry.height / 2.0),
        ]),
    );
    item.set(
        "originalDataIndex",
        Value::Number(entry.original_data_index as f64),
    );
    item
}

/// BarBackground's rectangles: the `background` prop drawn at each entry's
/// background box.
fn background_rectangles(
    rectangles: &[BarRectangle],
    background: &Value,
) -> Result<Vec<Node>, ChartError> {
    let background_props = match background {
        Value::Object(_) => svg_properties(&Props::of(background)),
        _ => Props::new(),
    };
    let mut nodes = Vec::new();
    for (index, entry) in rectangles.iter().enumerate() {
        let item = rectangle_item(entry);
        let mut p = Props::new();
        p.set("isActive", Value::Bool(false));
        for (key, value) in item.entries() {
            if !matches!(key, "value" | "background" | "tooltipPosition") {
                p.set(key, value.clone());
            }
        }
        p.set("fill", "#eee".into());
        p.spread(&item.get("background"));
        p.spread_props(&background_props);
        p.set("index", Value::Number(index as f64));
        p.set("className", "recharts-bar-background-rectangle".into());
        nodes.push(Node::Element(bar_rectangle(&p, background)?));
    }
    Ok(nodes)
}

/// `<BarRectangle {...props} option={option}>`: `Shape` with the
/// rectangle shape type, in a layer named by whether it is active.
fn bar_rectangle(props: &Props, option: &Value) -> Result<Element, ChartError> {
    // A plain object option is merged over the props; `true` and others
    // draw the props as they are.
    let mut element_props = props.clone();
    if let Value::Object(_) = option {
        element_props.spread(option);
    }
    let class = if element_props.get("isActive").is_truthy() {
        "recharts-active-bar"
    } else {
        "recharts-inactive-bar"
    };
    let mut shape_layer = layer(&[class.into()], &Props::new());
    if let Some(path) = rectangle(&element_props)? {
        shape_layer.children.push(Node::Element(path));
    }
    Ok(shape_layer)
}

/// `defaultRectangleProps`.
fn default_rectangle_props() -> Vec<(&'static str, Value)> {
    vec![
        ("x", Value::Number(0.0)),
        ("y", Value::Number(0.0)),
        ("width", Value::Number(0.0)),
        ("height", Value::Number(0.0)),
        ("radius", Value::Number(0.0)),
        ("isAnimationActive", Value::Bool(false)),
        ("isUpdateAnimationActive", Value::Bool(false)),
        ("animationBegin", Value::Number(0.0)),
        ("animationDuration", Value::Number(1500.0)),
        ("animationEasing", "ease".into()),
    ]
}

/// `<Rectangle>` (`shape/Rectangle.js`) without its update animation: the
/// `<path>`, or nothing when a coordinate is not a number or a side is 0.
fn rectangle(props: &Props) -> Result<Option<Element>, ChartError> {
    let props = props.clone().with_defaults(&default_rectangle_props());
    if props.get("isUpdateAnimationActive").is_truthy() {
        return Err(ChartError::Unsupported("rectangle update animations"));
    }
    // `x !== +x`: only a number other than NaN passes.
    let number = |name| match props.get(name) {
        Value::Number(n) if !n.is_nan() => Some(n),
        _ => None,
    };
    let (Some(x), Some(y), Some(width), Some(height)) =
        (number("x"), number("y"), number("width"), number("height"))
    else {
        return Ok(None);
    };
    if width == 0.0 || height == 0.0 {
        return Ok(None);
    }
    let radius_value = props.get("radius");
    let radius = match &radius_value {
        Value::Number(n) => Radius::Number(*n),
        Value::Array(items) => Radius::Corners(
            items
                .iter()
                .map(|v| match v {
                    Value::Null | Value::Undefined => None,
                    v => Some(v.to_number()),
                })
                .collect(),
        ),
        _ => Radius::Other,
    };
    let round = |n: f64| Value::Number(crate::shape::round(n));
    let mut path = svg_properties(&props);
    path.remove("radius");
    path.set("x", round(x));
    path.set("y", round(y));
    path.set("width", round(width));
    path.set("height", round(height));
    path.set(
        "radius",
        match radius_value {
            Value::Number(_) => radius_value.clone(),
            _ => Value::Undefined,
        },
    );
    path.set(
        "className",
        clsx(&["recharts-rectangle".into(), props.get("className")]).into(),
    );
    path.set("d", get_rectangle_path(x, y, width, height, &radius).into());
    Ok(Some(svg("path", &path)))
}
