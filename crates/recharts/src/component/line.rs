//! `<Line>` (`cartesian/Line.js`): its layer, the clip paths when an axis
//! lets data overflow, the curve through `util/ActiveShapeUtils.js`'s
//! `Shape` and `shape/Curve.js`, and the dots (`component/Dots.js`,
//! `shape/Dot.js`) in the scatter z-index layer.
//!
//! The line renders at rest with `isAnimationActive={false}`. With the
//! entrance animation on, its last frame sets `strokeDasharray` from the
//! path's `getTotalLength()`, which Div does not provide yet (V4); that, a
//! custom `shape`, `label`s and dots given as elements or functions are
//! reported as unsupported. `activeDot` only renders while the tooltip
//! points at a point, which is not modelled (no pointer).

use js::Value;

use super::svg_props::svg_properties;
use super::{nested_z_index_layer, z_index_layer, Output};
use crate::chart::{CartesianChart, ChartError, Layout, PlotArea};
use crate::dom::{clsx, Element, Node, Props};
use crate::shape::{get_path, BaseLine, CurvePoint};

/// `defaultLineProps`.
pub(super) fn default_line_props() -> Vec<(&'static str, Value)> {
    vec![
        ("activeDot", Value::Bool(true)),
        ("animateNewValues", Value::Bool(true)),
        ("animationBegin", Value::Number(0.0)),
        ("animationDuration", Value::Number(1500.0)),
        ("animationEasing", "ease".into()),
        ("connectNulls", Value::Bool(false)),
        ("dot", Value::Bool(true)),
        ("fill", "#fff".into()),
        ("hide", Value::Bool(false)),
        ("isAnimationActive", "auto".into()),
        ("label", Value::Bool(false)),
        ("legendType", "line".into()),
        ("stroke", "#3182bd".into()),
        ("strokeWidth", Value::Number(1.0)),
        ("xAxisId", Value::Number(0.0)),
        ("yAxisId", Value::Number(0.0)),
        ("zIndex", Value::Number(400.0)),
        ("type", "linear".into()),
    ]
}

/// `DefaultZIndexes.scatter`, where `<Dots>` render.
const DOTS_Z_INDEX: f64 = 600.0;

/// `<Layer className>` with the other props' SVG attributes.
pub(super) fn layer(class_name: &[Value], rest: &Props) -> Element {
    let mut props = Props::new();
    let mut classes = vec![Value::from("recharts-layer")];
    classes.extend_from_slice(class_name);
    props.set("className", clsx(&classes).into());
    props.spread_props(&svg_properties(rest));
    Element::svg("g").props(&props)
}

pub(super) fn clip_path(id: String, x: f64, y: f64, width: f64, height: f64) -> Element {
    let mut rect = Props::new();
    rect.set("x", Value::Number(x));
    rect.set("y", Value::Number(y));
    rect.set("width", Value::Number(width));
    rect.set("height", Value::Number(height));
    let mut clip = Props::new();
    clip.set("id", id.into());
    let mut element = Element::svg("clipPath").props(&clip);
    element
        .children
        .push(Node::Element(Element::svg("rect").props(&rect)));
    element
}

/// A dot option given as a plain object (`isPlainObject`), whose SVG props
/// apply to every dot.
fn dot_object(dot: &Value) -> Option<Props> {
    match dot {
        Value::Object(_) => Some(Props::of(dot)),
        _ => None,
    }
}

/// `isClipDot(dot)`.
pub(super) fn is_clip_dot(dot: &Value) -> bool {
    match dot_object(dot) {
        Some(props) if props.has("clipDot") => props.get("clipDot").is_truthy(),
        _ => true,
    }
}

/// `getRadiusAndStrokeWidthFromDot(dot)`.
pub(super) fn radius_and_stroke_width(dot: &Value) -> (f64, f64) {
    let Some(props) = dot_object(dot).map(|p| svg_properties(&p)) else {
        return (3.0, 2.0);
    };
    let real = |key: &str, default: f64| {
        let n = props.get(key).to_number();
        if n.is_nan() || n < 0.0 {
            default
        } else {
            n
        }
    };
    (real("r", 3.0), real("strokeWidth", 2.0))
}

/// `isNumber`: a number that is not NaN.
fn is_number(value: &Value) -> bool {
    matches!(value, Value::Number(n) if !n.is_nan())
}

pub(super) fn coordinate(c: Option<f64>) -> Value {
    c.map_or(Value::Null, Value::Number)
}

/// The `<defs>` of a graphical item whose axes let data overflow:
/// `GraphicalItemClipPath`, and the dots' clip path when `dot.clipDot` is
/// false. `None` when neither axis needs a clip.
pub(super) fn clip_defs(
    (clip_x, clip_y): (bool, bool),
    plot_area: PlotArea,
    id: &str,
    dot: &Value,
) -> Option<Element> {
    if !(clip_x || clip_y) {
        return None;
    }
    let PlotArea {
        x: left,
        y: top,
        width,
        height,
    } = plot_area;
    let mut defs = Element::svg("defs");
    defs.children.push(Node::Element(clip_path(
        format!("clipPath-{id}"),
        if clip_x { left } else { left - width / 2.0 },
        if clip_y { top } else { top - height / 2.0 },
        if clip_x { width } else { width * 2.0 },
        if clip_y { height } else { height * 2.0 },
    )));
    if !is_clip_dot(dot) {
        let (r, stroke_width) = radius_and_stroke_width(dot);
        let dot_size = r * 2.0 + stroke_width;
        defs.children.push(Node::Element(clip_path(
            format!("clipPath-dots-{id}"),
            left - dot_size / 2.0,
            top - dot_size / 2.0,
            width + dot_size,
            height + dot_size,
        )));
    }
    Some(defs)
}

/// `<Line>` with its resolved props (defaults applied, `id` resolved).
pub(super) fn render(
    chart: &CartesianChart,
    props: &Props,
    id: &Value,
    layout: Layout,
    plot_area: PlotArea,
    out: &mut Output,
) -> Result<(), ChartError> {
    for (name, what) in [("shape", "a custom line shape"), ("label", "line labels")] {
        if props.get(name).is_truthy() {
            return Err(ChartError::Unsupported(what));
        }
    }
    let dot = props.get("dot");
    if !matches!(
        dot,
        Value::Bool(_) | Value::Object(_) | Value::Undefined | Value::Null
    ) {
        return Err(ChartError::Unsupported(
            "a dot other than a boolean or an object",
        ));
    }
    let axis_key = |name| {
        js::Key::of(&props.get(name))
            .ok_or(ChartError::Unsupported("an axis id other than a primitive"))
    };
    let (x_axis_id, y_axis_id) = (axis_key("xAxisId")?, axis_key("yAxisId")?);
    let (clip_x, clip_y) = chart.needs_clip(&x_axis_id, &y_axis_id);
    let need_clip = clip_x || clip_y;
    let Some(points) = chart.line_points(&id.to_js_string())? else {
        return Ok(());
    };
    if props.get("hide").is_truthy() {
        return Ok(());
    }
    if props.get("isAnimationActive") != Value::Bool(false) {
        return Err(ChartError::Unsupported(
            "the line entrance animation (getTotalLength)",
        ));
    }
    let id_string = id.to_js_string();
    let PlotArea {
        x: left,
        y: top,
        width,
        height,
    } = plot_area;
    let layout_name = match layout {
        Layout::Horizontal => "horizontal",
        Layout::Vertical => "vertical",
    };

    // LineWithState's props: LineImpl's `everythingElse` (the resolved
    // props without the ones it names), then the ones it passes.
    let mut state = props.filter(|key| {
        !matches!(
            key,
            "activeDot"
                | "animateNewValues"
                | "animationBegin"
                | "animationDuration"
                | "animationEasing"
                | "connectNulls"
                | "dot"
                | "hide"
                | "isAnimationActive"
                | "label"
                | "legendType"
                | "xAxisId"
                | "yAxisId"
                | "id"
        )
    });
    state.set("id", id.clone());
    for key in [
        "connectNulls",
        "dot",
        "activeDot",
        "animateNewValues",
        "animationBegin",
        "animationDuration",
        "animationEasing",
        "isAnimationActive",
        "hide",
        "label",
        "legendType",
        "xAxisId",
        "yAxisId",
    ] {
        state.set(key, props.get(key));
    }
    state.set("layout", layout_name.into());
    state.set("height", Value::Number(height));
    state.set("width", Value::Number(width));
    state.set("left", Value::Number(left));
    state.set("top", Value::Number(top));
    state.set("needClip", Value::Bool(need_clip));

    let mut line = layer(
        &["recharts-line".into(), state.get("className")],
        &Props::new(),
    );
    if let Some(defs) = clip_defs((clip_x, clip_y), plot_area, &id_string, &dot) {
        line.children.push(Node::Element(defs));
    }

    // StaticCurve: the curve, when there are at least two points.
    if points.len() > 1 {
        let mut curve = svg_properties(&state.filter(|key| {
            !matches!(
                key,
                "type" | "layout" | "connectNulls" | "needClip" | "shape"
            )
        }));
        curve.set("fill", "none".into());
        curve.set("className", "recharts-line-curve".into());
        curve.set(
            "clipPath",
            if need_clip {
                format!("url(#clipPath-{id_string})").into()
            } else {
                Value::Undefined
            },
        );
        let dasharray = match state.get("strokeDasharray") {
            v if v.is_nullish() => v,
            v => Value::String(v.to_js_string()),
        };
        curve.set("strokeDasharray", dasharray);
        // Curve
        let r#type = match state.get("type") {
            Value::String(s) => Some(s),
            Value::Undefined => None,
            _ => return Err(ChartError::Unsupported("a curve type other than a name")),
        };
        let curve_points: Vec<CurvePoint> = points
            .iter()
            .map(|p| CurvePoint {
                x: coordinate(p.x),
                y: coordinate(p.y),
            })
            .collect();
        let d = get_path(
            r#type.as_deref(),
            &curve_points,
            &BaseLine::None,
            Some(layout_name),
            state.get("connectNulls").is_truthy(),
        );
        curve.set(
            "className",
            clsx(&["recharts-curve".into(), curve.get("className")]).into(),
        );
        curve.set("d", d.map_or(Value::Undefined, Value::String));
        let mut shape = layer(&["recharts-shape".into()], &Props::new());
        shape
            .children
            .push(Node::Element(Element::svg("path").props(&curve)));
        line.children.push(Node::Element(shape));
    }

    // LineDotsWrapper
    let base = svg_properties(&state.filter(|key| key != "id"));
    let xy: Vec<_> = points.iter().map(|p| (p.x, p.y)).collect();
    render_dots(
        &xy,
        &dot,
        &base,
        DotsLayer {
            class_name: "recharts-line-dots",
            dot_class_name: "recharts-line-dot",
            need_clip,
            clip_path_id: &id_string,
            outer_z_index: &props.get("zIndex"),
        },
        out,
    )?;
    z_index_layer(&props.get("zIndex"), vec![Node::Element(line)], out)
}

/// Where `<Dots>` renders.
pub(super) struct DotsLayer<'a> {
    pub class_name: &'static str,
    pub dot_class_name: &'static str,
    pub need_clip: bool,
    pub clip_path_id: &'a str,
    /// The z-index of the item's own `ZIndexLayer`, which `<Dots>`' layer
    /// sits in.
    pub outer_z_index: &'a Value,
}

/// `<Dots>` (`component/Dots.js`) in the scatter z-index layer, when the
/// dots render: `dot` is truthy or there is exactly one point.
pub(super) fn render_dots(
    points: &[(Option<f64>, Option<f64>)],
    dot: &Value,
    base: &Props,
    layer_of: DotsLayer,
    out: &mut Output,
) -> Result<(), ChartError> {
    if !(dot.is_truthy() || points.len() == 1) {
        return Ok(());
    }
    let dots = dots(points, dot, base, layer_of.dot_class_name);
    let mut dots_layer = layer(&[layer_of.class_name.into()], &{
        let mut p = Props::new();
        if layer_of.need_clip {
            let infix = if is_clip_dot(dot) { "" } else { "dots-" };
            p.set(
                "clipPath",
                format!("url(#clipPath-{infix}{})", layer_of.clip_path_id).into(),
            );
        }
        p
    });
    dots_layer.children.extend(dots);
    nested_z_index_layer(
        layer_of.outer_z_index,
        &Value::Number(DOTS_Z_INDEX),
        vec![Node::Element(dots_layer)],
        out,
    )
}

/// `<Dots>`' dot items: a `<Dot>` per point with a position.
fn dots(
    points: &[(Option<f64>, Option<f64>)],
    dot: &Value,
    base: &Props,
    dot_class_name: &str,
) -> Vec<Node> {
    let custom = dot_object(dot).map(|p| svg_properties(&p));
    let class_name = clsx(&[
        dot_class_name.into(),
        match dot {
            Value::Bool(_) => Value::from(""),
            other => other.get("className"),
        },
    ]);
    let mut nodes = Vec::new();
    for (x, y) in points {
        let mut p = Props::new();
        p.set("r", Value::Number(3.0));
        p.spread_props(base);
        if let Some(custom) = &custom {
            p.spread_props(custom);
        }
        let cx = x.map_or(Value::Undefined, Value::Number);
        let cy = y.map_or(Value::Undefined, Value::Number);
        // Dot
        let r = p.get("r");
        if !(is_number(&cx) && is_number(&cy) && is_number(&r)) {
            continue;
        }
        let mut circle = svg_properties(&p);
        circle.set(
            "className",
            clsx(&["recharts-dot".into(), class_name.clone().into()]).into(),
        );
        circle.set("cx", cx);
        circle.set("cy", cy);
        circle.set("r", r);
        nodes.push(Node::Element(Element::svg("circle").props(&circle)));
    }
    nodes
}
