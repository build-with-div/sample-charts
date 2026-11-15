//! `<CartesianGrid>` (`cartesian/CartesianGrid.js`): the background, the
//! stripes and the grid lines, placed at the ticks the axes label
//! (`getTicks` without a font) plus the plot area's edges
//! (`util/ChartUtils.js`'s `getCoordinatesOfGrid`).
//!
//! Custom line elements or functions (`horizontal`, `vertical`), custom
//! coordinate generators and `horizontalValues` / `verticalValues` are not
//! ported and are reported as unsupported.

use js::{math_round, Key, Value};

use super::axis::default_cartesian_axis_props;
use super::svg_props::svg_properties;
use super::text::{is_number, StringSizes};
use super::ticks::{get_ticks, TicksConfig};
use super::{is_positive_number, z_index_layer, Measure, Output};
use crate::chart::{AxisType, CartesianChart, ChartError};
use crate::dom::{Element, Node, Props};

/// `defaultCartesianGridProps`.
pub(super) fn default_grid_props() -> Vec<(&'static str, Value)> {
    vec![
        ("horizontal", Value::Bool(true)),
        ("vertical", Value::Bool(true)),
        ("horizontalPoints", Value::Array(Vec::new())),
        ("verticalPoints", Value::Array(Vec::new())),
        ("stroke", "#ccc".into()),
        ("fill", "none".into()),
        ("verticalFill", Value::Array(Vec::new())),
        ("horizontalFill", Value::Array(Vec::new())),
        ("xAxisId", Value::Number(0.0)),
        ("yAxisId", Value::Number(0.0)),
        ("syncWithTicks", Value::Bool(false)),
        ("zIndex", Value::Number(-100.0)),
    ]
}

fn axis_key(props: &Props, name: &str) -> Result<Key, ChartError> {
    Key::of(&props.get(name)).ok_or(ChartError::Unsupported("an axis id other than a primitive"))
}

/// A non-empty array prop.
fn non_empty(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Array(items) if !items.is_empty() => Some(items),
        _ => None,
    }
}

/// `<CartesianGrid>` with its props; `settings` gives the store's settings
/// object of an axis id (`selectXAxisSettings`, implicit when undeclared).
pub(super) fn render(
    chart: &CartesianChart,
    props: &Props,
    settings: &dyn Fn(AxisType, &Key) -> Props,
    sizes: &mut StringSizes,
    measure: &mut dyn Measure,
    out: &mut Output,
) -> Result<(), ChartError> {
    for name in [
        "horizontalCoordinatesGenerator",
        "verticalCoordinatesGenerator",
        "horizontalValues",
        "verticalValues",
    ] {
        if props.get(name).is_truthy() {
            return Err(ChartError::Unsupported(
                "grid coordinate generators and values",
            ));
        }
    }
    let offset = chart.plot_area();
    let (chart_width, chart_height) = chart.size();
    let mut grid = props.clone().with_defaults(&default_grid_props());
    for (key, fallback) in [
        ("x", offset.x),
        ("y", offset.y),
        ("width", offset.width),
        ("height", offset.height),
    ] {
        let value = match props.get(key) {
            v if is_number(&v) => v,
            _ => Value::Number(fallback),
        };
        grid.set(key, value);
    }
    let x_axis_id = axis_key(&grid, "xAxisId")?;
    let y_axis_id = axis_key(&grid, "yAxisId")?;
    // The grid's axis selectors run before it decides whether to render.
    let x_ticks = chart.axis(AxisType::XAxis, &x_axis_id)?.ticks;
    let y_ticks = chart.axis(AxisType::YAxis, &y_axis_id)?.ticks;
    let (width, height, x, y) = (
        grid.get("width"),
        grid.get("height"),
        grid.get("x"),
        grid.get("y"),
    );
    if !is_positive_number(&width)
        || !is_positive_number(&height)
        || !is_number(&x)
        || !is_number(&y)
    {
        return Ok(());
    }
    let (x, y, width, height) = (
        x.to_number(),
        y.to_number(),
        width.to_number(),
        height.to_number(),
    );
    let sync_with_ticks = grid.get("syncWithTicks").is_truthy();

    // The default coordinate generators: the axis's labelled ticks, plus
    // the plot area's edges unless synced with the ticks.
    let mut coordinates = |axis_type: AxisType,
                           id: &Key,
                           ticks: Option<Vec<crate::chart::Tick>>,
                           min: f64,
                           max: f64|
     -> Result<Vec<Value>, ChartError> {
        let mut axis = Props::new();
        for (key, value) in default_cartesian_axis_props() {
            axis.set(key, value);
        }
        let settings = settings(axis_type, id);
        for key in [
            "angle",
            "interval",
            "minTickGap",
            "orientation",
            "tick",
            "tickFormatter",
            "unit",
        ] {
            axis.set(key, settings.get(key));
        }
        let config = TicksConfig {
            tick: axis.get("tick"),
            ticks: ticks.as_deref(),
            view_box: [0.0, 0.0, chart_width, chart_height],
            min_tick_gap: axis.get("minTickGap"),
            orientation: axis.get("orientation"),
            interval: axis.get("interval"),
            tick_formatter: axis.get("tickFormatter"),
            unit: axis.get("unit"),
            angle: axis.get("angle"),
        };
        let shown = get_ticks(
            &config,
            sizes,
            measure,
            &Value::Undefined,
            &Value::Undefined,
        )?;
        let mut values: Vec<Value> = shown
            .iter()
            .map(|e| Value::Number(e.tick.coordinate))
            .collect();
        if !sync_with_ticks {
            if !shown.iter().any(|e| e.tick.coordinate == min) {
                values.push(Value::Number(min));
            }
            if !shown.iter().any(|e| e.tick.coordinate == max) {
                values.push(Value::Number(max));
            }
        }
        Ok(values)
    };
    let horizontal_points = match non_empty(&grid.get("horizontalPoints")) {
        Some(points) => points.clone(),
        None => coordinates(
            AxisType::YAxis,
            &y_axis_id,
            y_ticks,
            offset.y,
            offset.y + offset.height,
        )?,
    };
    let vertical_points = match non_empty(&grid.get("verticalPoints")) {
        Some(points) => points.clone(),
        None => coordinates(
            AxisType::XAxis,
            &x_axis_id,
            x_ticks,
            offset.x,
            offset.x + offset.width,
        )?,
    };

    let mut root = Element::svg("g");
    root.prop("className", &"recharts-cartesian-grid".into());
    let rect = |pairs: Vec<(&str, Value)>| {
        let mut p = Props::new();
        for (k, v) in pairs {
            p.set(k, v);
        }
        Node::Element(Element::svg("rect").props(&p))
    };

    // Background
    let fill = grid.get("fill");
    if fill.is_truthy() && fill.to_js_string() != "none" {
        root.children.push(rect(vec![
            ("x", Value::Number(x)),
            ("y", Value::Number(y)),
            ("ry", grid.get("ry")),
            ("width", Value::Number(width)),
            ("height", Value::Number(height)),
            ("stroke", "none".into()),
            ("fill", fill),
            ("fillOpacity", grid.get("fillOpacity")),
            ("className", "recharts-cartesian-grid-bg".into()),
        ]));
    }

    // HorizontalStripes and VerticalStripes
    for horizontal in [true, false] {
        let (flag, fills, points, start, size) = if horizontal {
            (
                "horizontal",
                "horizontalFill",
                &horizontal_points,
                y,
                height,
            )
        } else {
            ("vertical", "verticalFill", &vertical_points, x, width)
        };
        let Some(fills) = non_empty(&grid.get(fills)).cloned() else {
            continue;
        };
        if !grid.get(flag).is_truthy() {
            continue;
        }
        let mut rounded: Vec<f64> = points
            .iter()
            .map(|e| math_round(e.to_number() + start - start))
            .collect();
        rounded.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        if rounded.first() != Some(&start) {
            rounded.insert(0, 0.0);
        }
        let mut stripes = Element::svg("g");
        stripes.prop(
            "className",
            &format!("recharts-cartesian-gridstripes-{flag}").into(),
        );
        for (i, entry) in rounded.iter().enumerate() {
            let extent = match rounded.get(i + 1) {
                Some(next) => next - entry,
                None => start + size - entry,
            };
            if extent <= 0.0 {
                continue;
            }
            let color = fills[i % fills.len()].clone();
            let mut pairs = if horizontal {
                vec![
                    ("y", Value::Number(*entry)),
                    ("x", Value::Number(x)),
                    ("height", Value::Number(extent)),
                    ("width", Value::Number(width)),
                ]
            } else {
                vec![
                    ("x", Value::Number(*entry)),
                    ("y", Value::Number(y)),
                    ("width", Value::Number(extent)),
                    ("height", Value::Number(height)),
                ]
            };
            pairs.extend([
                ("stroke", "none".into()),
                ("fill", color),
                ("fillOpacity", grid.get("fillOpacity")),
                ("className", "recharts-cartesian-grid-bg".into()),
            ]);
            stripes.children.push(rect(pairs));
        }
        root.children.push(Node::Element(stripes));
    }

    // HorizontalGridLines and VerticalGridLines: LineItem with a plain
    // option draws the grid's SVG props.
    let line_props = {
        let mut p = svg_properties(&grid.filter(|key| !matches!(key, "xAxisId" | "yAxisId")));
        p.remove("offset");
        p
    };
    for horizontal in [true, false] {
        let (flag, points) = if horizontal {
            ("horizontal", &horizontal_points)
        } else {
            ("vertical", &vertical_points)
        };
        if !grid.get(flag).is_truthy() || points.is_empty() {
            continue;
        }
        let mut lines = Element::svg("g");
        lines.prop(
            "className",
            &format!("recharts-cartesian-grid-{flag}").into(),
        );
        for entry in points {
            let mut line = line_props.clone();
            let (x1, y1, x2, y2) = if horizontal {
                (
                    Value::Number(x),
                    entry.clone(),
                    Value::Number(x + width),
                    entry.clone(),
                )
            } else {
                (
                    entry.clone(),
                    Value::Number(y),
                    entry.clone(),
                    Value::Number(y + height),
                )
            };
            line.set("x1", x1);
            line.set("y1", y1);
            line.set("x2", x2);
            line.set("y2", y2);
            line.set("fill", "none".into());
            lines
                .children
                .push(Node::Element(Element::svg("line").props(&line)));
        }
        root.children.push(Node::Element(lines));
    }
    z_index_layer(&grid.get("zIndex"), vec![Node::Element(root)], out)
}
