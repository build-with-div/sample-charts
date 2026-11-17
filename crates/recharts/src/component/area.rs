//! `<Area>` (`cartesian/Area.js`): its layer, the clip paths when an axis
//! lets data overflow, the area and its outline through `shape/Curve.js`
//! (the base line's outline too for ranged areas), and the dots
//! (`component/Dots.js`) in the scatter z-index layer.
//!
//! The area renders at rest with `isAnimationActive={false}`: the first
//! frame `AreaWithAnimation` renders, with its animation clip path layer.
//! The entrance animation itself (`JavascriptAnimate` stepping `t` on
//! animation frames, not ported yet), `label`s and dots given as elements or functions are
//! reported as unsupported. `activeDot` only renders while the tooltip
//! points at a point, which is not modelled (no pointer).

use js::Value;

use super::line::{clip_defs, coordinate, layer, render_dots, DotsLayer};
use super::svg_props::svg_properties;
use super::{z_index_layer, Output};
use crate::chart::{AreaBaseLine, CartesianChart, ChartError, Layout, PlotArea};
use crate::dom::{clsx, Element, Node, Props};
use crate::shape::{get_path, BaseLine, CurvePoint};

/// `defaultAreaProps`.
pub(super) fn default_area_props() -> Vec<(&'static str, Value)> {
    vec![
        ("activeDot", Value::Bool(true)),
        ("animationBegin", Value::Number(0.0)),
        ("animationDuration", Value::Number(1500.0)),
        ("animationEasing", "ease".into()),
        ("connectNulls", Value::Bool(false)),
        ("dot", Value::Bool(false)),
        ("fill", "#3182bd".into()),
        ("fillOpacity", Value::Number(0.6)),
        ("hide", Value::Bool(false)),
        ("isAnimationActive", "auto".into()),
        ("legendType", "line".into()),
        ("stroke", "#3182bd".into()),
        ("strokeWidth", Value::Number(1.0)),
        ("type", "linear".into()),
        ("label", Value::Bool(false)),
        ("xAxisId", Value::Number(0.0)),
        ("yAxisId", Value::Number(0.0)),
        // DefaultZIndexes.area
        ("zIndex", Value::Number(100.0)),
    ]
}

/// One `<Curve>`: its props (already SVG props), points and base line.
fn curve(
    mut props: Props,
    r#type: Option<&str>,
    points: &[CurvePoint],
    base_line: &BaseLine,
    layout: &str,
    connect_nulls: bool,
) -> Option<Node> {
    if points.is_empty() {
        return None;
    }
    let d = get_path(r#type, points, base_line, Some(layout), connect_nulls);
    props.set(
        "className",
        clsx(&["recharts-curve".into(), props.get("className")]).into(),
    );
    props.set("d", d.map_or(Value::Undefined, Value::String));
    Some(Node::Element(Element::svg("path").props(&props)))
}

/// `<Area>` with its resolved props (defaults applied, `id` resolved), in
/// a chart called `chart_name`.
pub(super) fn render(
    chart: &CartesianChart,
    chart_name: &str,
    props: &Props,
    id: &Value,
    layout: Layout,
    plot_area: PlotArea,
    out: &mut Output,
) -> Result<(), ChartError> {
    if props.get("label").is_truthy() {
        return Err(ChartError::Unsupported("area labels"));
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
    let geometry = chart.area(&id.to_js_string())?;
    // AreaImpl
    if chart_name != "AreaChart" && chart_name != "ComposedChart" {
        return Ok(());
    }
    let Some(geometry) = geometry.filter(|g| !g.points.is_empty()) else {
        return Ok(());
    };
    // AreaWithState
    if props.get("hide").is_truthy() {
        return Ok(());
    }
    if props.get("isAnimationActive") != Value::Bool(false) {
        return Err(ChartError::Unsupported("the area entrance animation"));
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

    // AreaWithState's props: AreaImpl's `everythingElse` (the resolved
    // props with the id, without the ones it names), then the ones it
    // passes. `propsWithoutId` drops the id again.
    let named = [
        "activeDot",
        "animationBegin",
        "animationDuration",
        "animationEasing",
        "connectNulls",
        "dot",
        "fill",
        "fillOpacity",
        "hide",
        "isAnimationActive",
        "legendType",
        "stroke",
        "xAxisId",
        "yAxisId",
    ];
    let mut state = props.filter(|key| !named.contains(&key) && key != "id");
    for key in [
        "activeDot",
        "animationBegin",
        "animationDuration",
        "animationEasing",
    ] {
        state.set(key, props.get(key));
    }
    // baseLine and points are not SVG props; their values do not matter
    // here, only that the keys exist in this order.
    state.set("baseLine", Value::Undefined);
    for key in ["connectNulls", "dot", "fill", "fillOpacity"] {
        state.set(key, props.get(key));
    }
    state.set("height", Value::Number(height));
    state.set("hide", props.get("hide"));
    state.set("layout", layout_name.into());
    state.set("isAnimationActive", props.get("isAnimationActive"));
    state.set("isRange", Value::Bool(geometry.is_range));
    state.set("legendType", props.get("legendType"));
    state.set("needClip", Value::Bool(need_clip));
    state.set("points", Value::Undefined);
    state.set("stroke", props.get("stroke"));
    state.set("width", Value::Number(width));
    state.set("left", Value::Number(left));
    state.set("top", Value::Number(top));
    state.set("xAxisId", props.get("xAxisId"));
    state.set("yAxisId", props.get("yAxisId"));

    let mut area = layer(
        &["recharts-area".into(), state.get("className")],
        &Props::new(),
    );
    if let Some(defs) = clip_defs((clip_x, clip_y), plot_area, &id_string, &dot) {
        area.children.push(Node::Element(defs));
    }

    // StaticArea
    let r#type = match state.get("type") {
        Value::String(s) => Some(s),
        Value::Undefined => None,
        _ => return Err(ChartError::Unsupported("a curve type other than a name")),
    };
    let connect_nulls = state.get("connectNulls").is_truthy();
    let points: Vec<CurvePoint> = geometry
        .points
        .iter()
        .map(|p| CurvePoint {
            x: coordinate(p.x),
            y: coordinate(p.y),
        })
        .collect();
    let base_points = match &geometry.base_line {
        AreaBaseLine::Points(points) => Some(
            points
                .iter()
                .map(|(x, y)| CurvePoint {
                    x: coordinate(*x),
                    y: coordinate(*y),
                })
                .collect::<Vec<_>>(),
        ),
        AreaBaseLine::Coordinate(_) => None,
    };
    let base_line = match (&geometry.base_line, &base_points) {
        (_, Some(points)) => BaseLine::Points(points.iter().cloned().map(Some).collect()),
        (AreaBaseLine::Coordinate(n), None) if !n.is_nan() => BaseLine::Number(*n),
        _ => BaseLine::None,
    };
    let svg = svg_properties(&state);
    let stroke = state.get("stroke");
    let draws_stroke = !matches!(&stroke, Value::String(s) if s == "none");
    let mut static_area = Vec::new();
    if geometry.points.len() > 1 {
        let mut shapes = Vec::new();
        let mut fill = svg.clone();
        fill.set("id", id.clone());
        fill.set("stroke", "none".into());
        fill.set("className", "recharts-area-area".into());
        shapes.extend(curve(
            fill,
            r#type.as_deref(),
            &points,
            &base_line,
            layout_name,
            connect_nulls,
        ));
        let outline = |points: &[CurvePoint]| {
            let mut props = svg.clone();
            props.set("className", "recharts-area-curve".into());
            props.set("fill", "none".into());
            curve(
                props,
                r#type.as_deref(),
                points,
                &BaseLine::None,
                layout_name,
                connect_nulls,
            )
        };
        if draws_stroke {
            shapes.extend(outline(&points));
            if geometry.is_range {
                if let Some(base_points) = &base_points {
                    shapes.extend(outline(base_points));
                }
            }
        }
        let mut clipped = layer(&[], &{
            let mut p = Props::new();
            if need_clip {
                p.set("clipPath", format!("url(#clipPath-{id_string})").into());
            }
            p
        });
        clipped.children = shapes;
        static_area.push(Node::Element(clipped));
    }

    // AreaWithAnimation's first frame: a layer, and in it a layer clipped
    // by the animation's clip path, which only exists while it animates.
    let mut animated = layer(&[], &{
        let mut p = Props::new();
        p.set(
            "clipPath",
            format!("url(#animationClipPath-{id_string})").into(),
        );
        p
    });
    animated.children = static_area;
    let mut frame = layer(&[], &Props::new());
    frame.children.push(Node::Element(animated));
    area.children.push(Node::Element(frame));

    // AreaDotsWrapper, inside StaticArea: its portal mounts with the area.
    let xy: Vec<_> = geometry.points.iter().map(|p| (p.x, p.y)).collect();
    render_dots(
        &xy,
        &dot,
        &svg,
        DotsLayer {
            class_name: "recharts-area-dots",
            dot_class_name: "recharts-area-dot",
            need_clip,
            clip_path_id: &id_string,
            outer_z_index: &props.get("zIndex"),
        },
        out,
    )?;
    z_index_layer(&props.get("zIndex"), vec![Node::Element(area)], out)
}
