//! `<XAxis>` and `<YAxis>` (`cartesian/XAxis.js`, `YAxis.js`): the settings
//! they report to the store, and `CartesianAxis` (`cartesian/CartesianAxis.js`)
//! drawing the axis line, the tick lines and the tick labels.
//!
//! The tick labels are `<Text>`s in the label z-index layer. Which ticks get
//! one depends on measured text (`getTicks`); `CartesianAxis` measures with
//! the font size and letter spacing it reads from its first label with
//! `getComputedStyle`, so it renders until that settles.
//!
//! Not ported, and reported as unsupported: axis labels (`label`), custom
//! tick elements or functions, `tickFormatter`, `width="auto"` and rotated
//! labels.

use js::{Key, Value};

use super::svg_props::svg_properties;
use super::text::{text, StringSizes};
use super::ticks::{get_ticks, label_coordinate, TickEntry, TicksConfig};
use super::{z_index_layer, Measure, Output};
use crate::chart::{is_categorical_axis, AxisType, CartesianChart, ChartError, Layout};
use crate::dom::{clsx, Element, Node, Props};

fn padding(sides: [&str; 2]) -> Value {
    Value::Object(
        sides
            .iter()
            .map(|side| ((*side).to_owned(), Value::Number(0.0)))
            .collect(),
    )
}

/// `xAxisDefaultProps` / `yAxisDefaultProps`.
pub(super) fn default_axis_props(axis_type: AxisType) -> Vec<(&'static str, Value)> {
    let x = axis_type == AxisType::XAxis;
    let mut props = vec![
        ("allowDataOverflow", Value::Bool(false)),
        ("allowDecimals", Value::Bool(true)),
        ("allowDuplicatedCategory", Value::Bool(true)),
        ("angle", Value::Number(0.0)),
        ("axisLine", Value::Bool(true)),
    ];
    if x {
        props.push(("height", Value::Number(30.0)));
    }
    props.extend([
        ("hide", Value::Bool(false)),
        ("includeHidden", Value::Bool(false)),
        ("interval", "preserveEnd".into()),
        ("label", Value::Bool(false)),
        ("minTickGap", Value::Number(5.0)),
        ("mirror", Value::Bool(false)),
        ("orientation", if x { "bottom" } else { "left" }.into()),
        (
            "padding",
            padding(if x {
                ["left", "right"]
            } else {
                ["top", "bottom"]
            }),
        ),
        ("reversed", Value::Bool(false)),
        ("scale", "auto".into()),
        ("tick", Value::Bool(true)),
        ("tickCount", Value::Number(5.0)),
        ("tickLine", Value::Bool(true)),
        ("tickSize", Value::Number(6.0)),
        ("type", if x { "category" } else { "number" }.into()),
        ("niceTicks", "auto".into()),
    ]);
    if x {
        props.push(("xAxisId", Value::Number(0.0)));
    } else {
        props.push(("width", Value::Number(60.0)));
        props.push(("yAxisId", Value::Number(0.0)));
    }
    props
}

/// The settings `SetXAxisSettings` / `SetYAxisSettings` report, from the
/// axis's props with defaults: the props the store keeps, with `type`
/// evaluated for the layout (`getAxisTypeBasedOnLayout`).
pub(super) fn settings_props(props: &Props, axis_type: AxisType, layout: Layout) -> Props {
    let x = axis_type == AxisType::XAxis;
    let keys: &[&str] = if x {
        &[
            "allowDataOverflow",
            "allowDecimals",
            "allowDuplicatedCategory",
            "angle",
            "dataKey",
            "domain",
            "height",
            "hide",
            "id",
            "includeHidden",
            "interval",
            "minTickGap",
            "mirror",
            "name",
            "orientation",
            "padding",
            "reversed",
            "scale",
            "tick",
            "tickCount",
            "tickFormatter",
            "ticks",
            "unit",
            "niceTicks",
        ]
    } else {
        &[
            "interval",
            "id",
            "scale",
            "domain",
            "allowDataOverflow",
            "dataKey",
            "allowDuplicatedCategory",
            "allowDecimals",
            "tickCount",
            "padding",
            "includeHidden",
            "reversed",
            "ticks",
            "width",
            "orientation",
            "mirror",
            "hide",
            "unit",
            "name",
            "angle",
            "minTickGap",
            "tick",
            "tickFormatter",
            "niceTicks",
        ]
    };
    let mut settings = Props::new();
    for key in keys {
        let value = match *key {
            "id" => props.get(if x { "xAxisId" } else { "yAxisId" }),
            key => props.get(key),
        };
        settings.set(key, value);
    }
    let declared = props.get("type");
    settings.set(
        "type",
        if matches!(&declared, Value::String(s) if s == "auto") {
            if is_categorical_axis(layout, axis_type) {
                "category"
            } else {
                "number"
            }
            .into()
        } else {
            declared
        },
    );
    settings
}

/// `implicitXAxis` / `implicitYAxis` as the store's settings object: what
/// an axis id with no `<XAxis>` or `<YAxis>` has.
pub(super) fn implicit_settings(axis_type: AxisType) -> Props {
    let x = axis_type == AxisType::XAxis;
    let mut settings = Props::new();
    settings.set("allowDataOverflow", Value::Bool(false));
    settings.set("allowDecimals", Value::Bool(true));
    settings.set("allowDuplicatedCategory", Value::Bool(true));
    settings.set("angle", Value::Number(0.0));
    settings.set("dataKey", Value::Undefined);
    settings.set(
        "domain",
        if x {
            Value::Undefined
        } else {
            Value::Array(vec![Value::Number(0.0), "auto".into()])
        },
    );
    if x {
        settings.set("height", Value::Number(30.0));
    }
    settings.set("hide", Value::Bool(true));
    settings.set("id", Value::Number(0.0));
    settings.set("includeHidden", Value::Bool(false));
    settings.set("interval", "preserveEnd".into());
    settings.set("minTickGap", Value::Number(5.0));
    settings.set("mirror", Value::Bool(false));
    settings.set("name", Value::Undefined);
    settings.set("orientation", if x { "bottom" } else { "left" }.into());
    settings.set(
        "padding",
        padding(if x {
            ["left", "right"]
        } else {
            ["top", "bottom"]
        }),
    );
    settings.set("reversed", Value::Bool(false));
    settings.set("scale", "auto".into());
    settings.set("tick", Value::Bool(true));
    settings.set("tickCount", Value::Number(5.0));
    settings.set("tickFormatter", Value::Undefined);
    settings.set("ticks", Value::Undefined);
    settings.set("type", if x { "category" } else { "number" }.into());
    settings.set("unit", Value::Undefined);
    settings.set("niceTicks", "auto".into());
    if !x {
        settings.set("width", Value::Number(60.0));
    }
    settings
}

/// `defaultCartesianAxisProps`, without `viewBox` and `ticks`, which the
/// axis and the grid always pass.
pub(super) fn default_cartesian_axis_props() -> Vec<(&'static str, Value)> {
    vec![
        ("x", Value::Number(0.0)),
        ("y", Value::Number(0.0)),
        ("width", Value::Number(0.0)),
        ("height", Value::Number(0.0)),
        ("orientation", "bottom".into()),
        ("stroke", "#666".into()),
        ("tickLine", Value::Bool(true)),
        ("axisLine", Value::Bool(true)),
        ("tick", Value::Bool(true)),
        ("mirror", Value::Bool(false)),
        ("minTickGap", Value::Number(5.0)),
        ("tickSize", Value::Number(6.0)),
        ("tickMargin", Value::Number(2.0)),
        ("interval", "preserveEnd".into()),
        ("zIndex", Value::Number(500.0)),
    ]
}

/// `DefaultZIndexes.label`.
const LABEL_Z_INDEX: f64 = 2000.0;

/// `get(value, 'className')` of es-toolkit: an object's `className`.
fn class_name_of(value: &Value) -> Value {
    match value {
        Value::Object(_) => value.get("className"),
        _ => Value::Undefined,
    }
}

/// `<Layer className>` with no other props.
fn layer(class_name: &[Value]) -> Element {
    let mut classes = vec![Value::from("recharts-layer")];
    classes.extend_from_slice(class_name);
    let mut g = Element::svg("g");
    g.prop("className", &clsx(&classes).into());
    g
}

/// `<XAxis>` / `<YAxis>` with its props (defaults applied) and the
/// settings the store holds for its id, with the tick font its state holds.
/// Gives the first tick label, whose computed font CartesianAxis reads once
/// the chart is in the document.
#[allow(clippy::too_many_arguments)]
pub(super) fn render(
    chart: &CartesianChart,
    axis_type: AxisType,
    props: &Props,
    settings: &Props,
    id: &Key,
    font: &(Value, Value),
    sizes: &mut StringSizes,
    measure: &mut dyn Measure,
    out: &mut Output,
) -> Result<Option<Element>, ChartError> {
    let x_axis = axis_type == AxisType::XAxis;
    let name = if x_axis { "xAxis" } else { "yAxis" };
    if props.get("label").is_truthy() {
        return Err(ChartError::Unsupported("axis labels"));
    }
    let axis_ticks = chart.axis(axis_type, id)?.ticks;
    let (width, height) = chart.axis_size(axis_type, id);
    let (x, y) = chart.axis_position(axis_type, id);
    let (chart_width, chart_height) = chart.size();

    // XAxisImpl / YAxisImpl: the props, then the store's settings, then
    // the computed geometry.
    let mut axis =
        props.filter(|key| !matches!(key, "dangerouslySetInnerHTML" | "ticks" | "scale"));
    axis.spread_props(&settings.filter(|key| !matches!(key, "id" | "scale")));
    axis.set("x", Value::Number(x));
    axis.set("y", Value::Number(y));
    let tick_text_props = if x_axis {
        Value::Undefined
    } else {
        let width = props.get("width");
        if width.to_js_string() == "auto" {
            return Err(ChartError::Unsupported("width=\"auto\" on a YAxis"));
        }
        Value::Object(vec![("width".to_owned(), width)])
    };
    axis.set("tickTextProps", tick_text_props.clone());
    axis.set("width", Value::Number(width));
    axis.set("height", Value::Number(height));
    axis.set(
        "className",
        clsx(&[
            format!("recharts-{name} {name}").into(),
            props.get("className"),
        ])
        .into(),
    );
    let axis = axis.with_defaults(&default_cartesian_axis_props());

    // CartesianAxisComponent
    if axis.get("hide").is_truthy() {
        return Ok(None);
    }
    if width <= 0.0 || height <= 0.0 {
        return Ok(None);
    }
    let mut root = layer(&["recharts-cartesian-axis".into(), axis.get("className")]);
    let svg_props = svg_properties(&axis);
    let orientation = axis.get("orientation").to_js_string();
    let mirror = axis.get("mirror").is_truthy();

    // AxisLine
    let axis_line = axis.get("axisLine");
    if axis_line.is_truthy() {
        let mut line = svg_props.clone();
        if let Value::Object(_) = axis_line {
            line.spread_props(&svg_properties(&Props::of(&axis_line)));
        }
        line.set("fill", "none".into());
        let (x1, y1, x2, y2) = if orientation == "top" || orientation == "bottom" {
            let need = f64::from(u8::from(
                (orientation == "top" && !mirror) || (orientation == "bottom" && mirror),
            ));
            (x, y + need * height, x + width, y + need * height)
        } else {
            let need = f64::from(u8::from(
                (orientation == "left" && !mirror) || (orientation == "right" && mirror),
            ));
            (x + need * width, y, x + need * width, y + height)
        };
        for (key, value) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
            line.set(key, Value::Number(value));
        }
        line.set(
            "className",
            clsx(&[
                "recharts-cartesian-axis-line".into(),
                class_name_of(&axis_line),
            ])
            .into(),
        );
        root.children
            .push(Node::Element(Element::svg("line").props(&line)));
    }

    // Ticks, measured with the font read back from the first label once the
    // chart committed (`font`: `''` and `''` at first).
    let config = TicksConfig {
        tick: axis.get("tick"),
        ticks: axis_ticks.as_deref(),
        view_box: [0.0, 0.0, chart_width, chart_height],
        min_tick_gap: axis.get("minTickGap"),
        orientation: axis.get("orientation"),
        interval: axis.get("interval"),
        tick_formatter: axis.get("tickFormatter"),
        unit: axis.get("unit"),
        angle: axis.get("angle"),
    };
    let final_ticks = get_ticks(&config, sizes, measure, &font.0, &font.1)?;
    let (ticks_group, labels, first_label) = ticks(
        &axis,
        &svg_props,
        &tick_text_props,
        &final_ticks,
        name,
        sizes,
        measure,
    )?;
    if let Some(labels) = labels {
        z_index_layer(
            &Value::Number(LABEL_Z_INDEX),
            vec![Node::Element(labels)],
            out,
        )?;
    }
    root.children.push(Node::Element(ticks_group));
    z_index_layer(&axis.get("zIndex"), vec![Node::Element(root)], out)?;
    Ok(first_label)
}

/// `getTickTextAnchor`.
fn text_anchor(orientation: &str, mirror: bool) -> &'static str {
    match (orientation, mirror) {
        ("left", false) | ("right", true) => "end",
        ("left", true) | ("right", false) => "start",
        _ => "middle",
    }
}

/// `getTickVerticalAnchor`.
fn vertical_anchor(orientation: &str, mirror: bool) -> &'static str {
    match (orientation, mirror) {
        ("left" | "right", _) => "middle",
        ("top", true) => "start",
        ("top", false) => "end",
        (_, true) => "end",
        (_, false) => "start",
    }
}

/// `Ticks`: the group of tick lines, the group of tick labels (which goes
/// to the label layer) and the first label's `<text>`.
fn ticks(
    axis: &Props,
    svg_props: &Props,
    tick_text_props: &Value,
    final_ticks: &[TickEntry],
    name: &str,
    sizes: &mut StringSizes,
    measure: &mut dyn Measure,
) -> Result<(Element, Option<Element>, Option<Element>), ChartError> {
    let orientation = axis.get("orientation").to_js_string();
    let mirror = axis.get("mirror").is_truthy();
    let number = |key| axis.get(key).to_number();
    let (x, y, width, height) = (number("x"), number("y"), number("width"), number("height"));
    let tick_size = axis.get("tickSize");
    let tick_margin = number("tickMargin");
    let tick = axis.get("tick");
    let custom_tick_props = match &tick {
        Value::Object(_) => svg_properties(&Props::of(&tick)),
        _ => Props::new(),
    };
    let text_anchor = match svg_props.get("textAnchor") {
        Value::String(a) if matches!(a.as_str(), "start" | "middle" | "end" | "inherit") => a,
        _ => text_anchor(&orientation, mirror).to_owned(),
    };
    let vertical_anchor = vertical_anchor(&orientation, mirror);
    let tick_line = axis.get("tickLine");
    let mut tick_line_props = svg_props.clone();
    tick_line_props.set("fill", "none".into());
    if let Value::Object(_) = tick_line {
        tick_line_props.spread(&tick_line);
    }

    let sign = if mirror { -1.0 } else { 1.0 };
    let mut lines = Vec::new();
    let mut labels = Vec::new();
    let mut first_label = None;
    for (index, entry) in final_ticks.iter().enumerate() {
        // getTickLineCoord; a tick never has its own tickSize here.
        let size = tick_size.to_number();
        let coordinate = entry.tick.coordinate;
        let tick_coord = label_coordinate(entry);
        let not_mirror = f64::from(u8::from(!mirror));
        let is_mirror = f64::from(u8::from(mirror));
        let (x1, y1, x2, y2, tx, ty);
        match orientation.as_str() {
            "top" => {
                x1 = coordinate;
                x2 = coordinate;
                y2 = y + not_mirror * height;
                y1 = y2 - sign * size;
                ty = y1 - sign * tick_margin;
                tx = tick_coord;
            }
            "left" => {
                y1 = coordinate;
                y2 = coordinate;
                x2 = x + not_mirror * width;
                x1 = x2 - sign * size;
                tx = x1 - sign * tick_margin;
                ty = tick_coord;
            }
            "right" => {
                y1 = coordinate;
                y2 = coordinate;
                x2 = x + is_mirror * width;
                x1 = x2 + sign * size;
                tx = x1 + sign * tick_margin;
                ty = tick_coord;
            }
            _ => {
                x1 = coordinate;
                x2 = coordinate;
                y2 = y + is_mirror * height;
                y1 = y2 + sign * size;
                ty = y1 + sign * tick_margin;
                tx = tick_coord;
            }
        }

        let mut tick_layer = layer(&["recharts-cartesian-axis-tick".into()]);
        if tick_line.is_truthy() {
            let mut line = tick_line_props.clone();
            for (key, value) in [("x1", x1), ("y1", y1), ("x2", x2), ("y2", y2)] {
                line.set(key, Value::Number(value));
            }
            line.set(
                "className",
                clsx(&[
                    "recharts-cartesian-axis-tick-line".into(),
                    class_name_of(&tick_line),
                ])
                .into(),
            );
            tick_layer
                .children
                .push(Node::Element(Element::svg("line").props(&line)));
        }
        lines.push(Node::Element(tick_layer));

        let mut label_layer = layer(&["recharts-cartesian-axis-tick-label".into()]);
        if tick.is_truthy() {
            let mut tick_props = Props::new();
            tick_props.set("verticalAnchor", vertical_anchor.into());
            tick_props.spread_props(svg_props);
            tick_props.set("textAnchor", text_anchor.clone().into());
            tick_props.set("stroke", "none".into());
            tick_props.set("fill", axis.get("stroke"));
            tick_props.set("x", Value::Number(tx));
            tick_props.set("y", Value::Number(ty));
            tick_props.set("index", Value::Number(index as f64));
            tick_props.set("visibleTicksCount", Value::Number(final_ticks.len() as f64));
            tick_props.set("tickFormatter", axis.get("tickFormatter"));
            tick_props.set("padding", axis.get("padding"));
            tick_props.spread(tick_text_props);
            let angle = match tick_text_props.get("angle") {
                a if !a.is_nullish() => a,
                _ => match svg_props.get("angle") {
                    a if !a.is_nullish() => a,
                    _ => Value::Number(0.0),
                },
            };
            tick_props.set("angle", angle);
            tick_props.spread_props(&custom_tick_props);
            // TickItem with a plain option: a <Text>.
            let class_name = match &tick {
                Value::Bool(_) => "recharts-cartesian-axis-tick-value".to_owned(),
                option => clsx(&[
                    "recharts-cartesian-axis-tick-value".into(),
                    match option {
                        Value::Object(_) => match option.get("className") {
                            Value::String(c) => Value::String(c),
                            _ => "".into(),
                        },
                        _ => "".into(),
                    },
                ]),
            };
            tick_props.set("className", class_name.into());
            let unit = axis.get("unit");
            let value = format!(
                "{}{}",
                entry.tick.value.to_js_string(),
                if unit.is_truthy() {
                    unit.to_js_string()
                } else {
                    String::new()
                }
            );
            tick_props.set("children", value.into());
            if let Some(element) = text(sizes, measure, &tick_props)? {
                if first_label.is_none() {
                    first_label = Some(element.clone());
                }
                label_layer.children.push(Node::Element(element));
            }
        }
        labels.push(Node::Element(label_layer));
    }

    let mut group = Element::svg("g");
    group.prop(
        "className",
        &format!("recharts-cartesian-axis-ticks recharts-{name}-ticks").into(),
    );
    let labels = (!labels.is_empty()).then(|| {
        let mut g = Element::svg("g");
        g.prop(
            "className",
            &format!("recharts-cartesian-axis-tick-labels recharts-{name}-tick-labels").into(),
        );
        g.children = labels;
        g
    });
    if !lines.is_empty() {
        let mut g = Element::svg("g");
        g.prop(
            "className",
            &format!("recharts-cartesian-axis-tick-lines recharts-{name}-tick-lines").into(),
        );
        g.children = lines;
        group.children.push(Node::Element(g));
    }
    Ok((group, labels, first_label))
}
