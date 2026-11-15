//! Recharts' cartesian chart components rendered to the DOM tree that
//! `react-dom` builds for them (`dom`): the chart's wrapper `<div>` and
//! `<svg>` surface (`chart/CartesianChart.js`, `CategoricalChart.js`,
//! `RechartsWrapper.js`, `container/RootSurface.js`, `Surface.js`), its
//! z-index layers (`zIndex/*`), the plot area's clip path
//! (`container/ClipPathProvider.js`) and `<Bar>` (`bar`).
//!
//! A chart is given as React would get it: the chart component's props and
//! its children, each a component and its props ([`ChartElement`]). The
//! selectors run on the [`CartesianChart`] those props make.
//!
//! Not ported yet: what `<CartesianGrid>`, `<XAxis>` and `<YAxis>` draw
//! (the grid lines and the axes measure text through the tick label filter
//! and `<Text>`). They take part in the chart's state and their selectors
//! run, so an exception they would throw unmounts the chart as in React,
//! but they add no elements here; [`Rendered::not_rendered`] lists them.
//! Tooltips, legends, other graphical items and interaction are not ported.

mod bar;
mod props;
mod svg_props;

use js::{number_to_string, Value};

use crate::chart::{AxisType, CartesianChart, ChartError, PlotArea};
use crate::dom::{clsx, Element, Namespace, Node, Props};
use svg_props::svg_properties;

/// A chart child component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    CartesianGrid,
    XAxis,
    YAxis,
    Bar,
}

/// One child element of a chart: its component and props.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentElement {
    pub component: Component,
    pub props: Props,
}

/// `<BarChart {...props}>{children}</BarChart>` (or `ComposedChart`,
/// `LineChart`, `AreaChart`: `chart_name`).
#[derive(Debug, Clone, PartialEq)]
pub struct ChartElement {
    pub chart_name: String,
    pub props: Props,
    pub children: Vec<ComponentElement>,
}

/// The id counters of the JavaScript realm a chart renders in: React's
/// `useId` (`_r_0_`, `_r_1_`, …) and Recharts' `uniqueId` (`util/DataUtils.js`).
///
/// A new realm, such as a page that renders one chart, starts both at 0.
/// Recharts also takes a `uniqueId` on every render of an animated
/// component, so on a page with several charts the clip path ids of the
/// later charts depend on how often the earlier ones rendered; only the
/// first chart in a realm is modelled exactly.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ids {
    use_id: u32,
    unique_id: u32,
}

impl Ids {
    pub fn new() -> Ids {
        Ids::default()
    }

    /// React's client `useId()` of a newly mounted component:
    /// `"_" + identifierPrefix + "r_" + counter.toString(32) + "_"`.
    fn use_id(&mut self) -> String {
        let id = format!("_r_{}_", radix32(self.use_id));
        self.use_id += 1;
        id
    }

    /// `uniqueId(prefix)`: the prefix and the incremented counter.
    fn unique_id(&mut self, prefix: &str) -> String {
        self.unique_id += 1;
        format!("{prefix}{}", self.unique_id)
    }
}

fn radix32(mut n: u32) -> String {
    let digits = b"0123456789abcdefghijklmnopqrstuv";
    let mut out = Vec::new();
    loop {
        out.push(digits[(n % 32) as usize]);
        n /= 32;
        if n == 0 {
            break;
        }
    }
    out.reverse();
    String::from_utf8(out).expect("ascii")
}

/// What a chart renders into its container.
#[derive(Debug, Clone, PartialEq)]
pub struct Rendered {
    pub nodes: Vec<Node>,
    /// Child components whose own elements are not rendered yet (see the
    /// module documentation), in tree order.
    pub not_rendered: Vec<Component>,
}

/// What a component renders: in place, and through `ZIndexLayer` portals.
#[derive(Debug, Default)]
struct Output {
    inline: Vec<Node>,
    /// Portal contents in mount order, by `String(zIndex)`.
    portals: Vec<(String, Vec<Node>)>,
    /// The z-indexes registered with `registerZIndexPortal`, as keys.
    registered: Vec<String>,
}

/// `<ZIndexLayer zIndex>`: children render in place for no z-index or 0,
/// and into that layer's portal otherwise.
fn z_index_layer(z_index: &Value, nodes: Vec<Node>, out: &mut Output) -> Result<(), ChartError> {
    match z_index {
        Value::Undefined => out.inline.extend(nodes),
        Value::Number(z) if *z == 0.0 => out.inline.extend(nodes),
        Value::Number(z) if z.is_finite() => {
            let key = number_to_string(*z);
            out.registered.push(key.clone());
            out.portals.push((key, nodes));
        }
        _ => {
            return Err(ChartError::Unsupported(
                "a z-index other than a finite number",
            ))
        }
    }
    Ok(())
}

/// `DefaultZIndexes`, always registered.
const DEFAULT_Z_INDEXES: [f64; 12] = [
    -100.0, -50.0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 1000.0, 1100.0, 1200.0, 2000.0,
];

/// `parseInt(key, 10)` of a z-index key (`String` of a finite number).
fn parse_int(key: &str) -> f64 {
    let (sign, digits) = match key.strip_prefix('-') {
        Some(rest) => (-1.0, rest),
        None => (1.0, key),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<f64>().unwrap_or(f64::NAN)
}

/// `isPositiveNumber`: a finite number above 0.
fn is_positive_number(value: &Value) -> bool {
    matches!(value, Value::Number(n) if n.is_finite() && *n > 0.0)
}

/// `defaultCartesianChartProps`, without `initialEventSettingsState`'s
/// event throttling settings, which render nothing.
fn default_chart_props() -> Vec<(&'static str, Value)> {
    let margin = Value::Object(
        ["top", "right", "bottom", "left"]
            .iter()
            .map(|side| ((*side).to_owned(), Value::Number(5.0)))
            .collect(),
    );
    vec![
        ("accessibilityLayer", Value::Bool(true)),
        ("barCategoryGap", "10%".into()),
        ("barGap", Value::Number(4.0)),
        ("layout", "horizontal".into()),
        ("margin", margin),
        ("responsive", Value::Bool(false)),
        ("reverseStackOrder", Value::Bool(false)),
        ("stackOffset", "none".into()),
        ("syncMethod", "index".into()),
    ]
}

/// Renders a chart into an empty container, as `root.render(<chart/>)`
/// leaves it once settled.
///
/// An exception a selector throws during render (`ChartError::Js`) makes
/// React unmount the whole root, leaving the container empty; it is
/// returned as the error. `ChartError::Unsupported` names what this port
/// does not cover yet.
pub fn render(element: &ChartElement, ids: &mut Ids) -> Result<Rendered, ChartError> {
    if !matches!(
        element.chart_name.as_str(),
        "BarChart" | "ComposedChart" | "LineChart" | "AreaChart"
    ) {
        return Err(ChartError::Unsupported("charts other than cartesian ones"));
    }
    let root = element.props.clone().with_defaults(&default_chart_props());
    let chart_props = props::chart_props(&element.chart_name, &root)?;

    // The components report their settings, then the chart renders.
    let mut x_axes = Vec::new();
    let mut y_axes = Vec::new();
    let mut items = Vec::new();
    let mut bars = Vec::new();
    let mut not_rendered = Vec::new();
    for child in &element.children {
        match child.component {
            Component::XAxis => x_axes.push(props::axis_props(&child.props, "xAxisId")?),
            Component::YAxis => y_axes.push(props::axis_props(&child.props, "yAxisId")?),
            Component::CartesianGrid => {}
            Component::Bar => {
                // useUniqueId('recharts-bar', props.id): a truthy id as it is.
                let mut bar = child.props.clone().with_defaults(&bar::default_bar_props());
                // `useUniqueId` calls `useId()` before it looks at the bar's
                // own id (hooks are never conditional), so every bar takes a
                // value from the counter.
                let generated = ids.use_id();
                let id = match bar.get("id") {
                    id if id.is_truthy() => id,
                    _ => Value::String(format!("recharts-bar-{generated}")),
                };
                bar.set("id", id.clone());
                items.push(props::bar_item_props(&bar, &id.to_js_string())?);
                bars.push((bar, id));
            }
        }
        if child.component != Component::Bar {
            if !matches!(child.props.get("zIndex"), Value::Undefined) {
                return Err(ChartError::Unsupported("z-indexes of grids and axes"));
            }
            not_rendered.push(child.component);
        }
    }
    let chart = CartesianChart::new(&chart_props, &x_axes, &y_axes, &items);

    // RechartsWrapper's <div>.
    let mut wrapper_props = Props::new();
    let style = root.get("style");
    // `width ?? style?.width`
    for side in ["width", "height"] {
        let value = match root.get(side) {
            v if v.is_nullish() => style.get(side),
            v => v,
        };
        wrapper_props.set(side, value);
    }
    wrapper_props.set(
        "className",
        clsx(&["recharts-wrapper".into(), root.get("className")]).into(),
    );
    let mut wrapper_style = Props::new();
    wrapper_style.set("position", "relative".into());
    wrapper_style.set("cursor", "default".into());
    wrapper_style.set("width", root.get("width"));
    wrapper_style.set("height", root.get("height"));
    wrapper_style.spread(&style);
    wrapper_props.set("style", wrapper_style.to_value());
    let mut wrapper = Element::new(Namespace::Html, "div").props(&wrapper_props);

    // MainChartSurface renders once the chart has a positive size.
    if is_positive_number(&root.get("width")) && is_positive_number(&root.get("height")) {
        let plot_area = chart.plot_area();
        let mut out = Output::default();
        let clip_path_id = format!("{}-clip", ids.unique_id("recharts"));
        let mut bars = bars.into_iter();
        for child in &element.children {
            match child.component {
                Component::Bar => {
                    let (props, id) = bars.next().expect("one per bar");
                    bar::render(&chart, &props, &id, plot_area, &mut out)?;
                }
                Component::XAxis | Component::YAxis => {
                    let (axis_type, id_prop) = match child.component {
                        Component::XAxis => (AxisType::XAxis, "xAxisId"),
                        _ => (AxisType::YAxis, "yAxisId"),
                    };
                    chart.axis(axis_type, &axis_id(&child.props, id_prop)?)?;
                }
                Component::CartesianGrid => {
                    chart.axis(AxisType::XAxis, &axis_id(&child.props, "xAxisId")?)?;
                    chart.axis(AxisType::YAxis, &axis_id(&child.props, "yAxisId")?)?;
                }
            }
        }
        wrapper
            .children
            .push(Node::Element(surface(&root, out, plot_area, &clip_path_id)));
    }
    Ok(Rendered {
        nodes: vec![Node::Element(wrapper)],
        not_rendered,
    })
}

fn axis_id(props: &Props, name: &str) -> Result<js::Key, ChartError> {
    match props.get(name) {
        Value::Undefined => Ok(js::Key::number(0.0)),
        id => js::Key::of(&id).ok_or(ChartError::Unsupported("an axis id other than a primitive")),
    }
}

/// RootSurface's `<svg>` with its title, description and z-index layers
/// around the clip path and the components that render in place.
fn surface(root: &Props, out: Output, plot_area: PlotArea, clip_path_id: &str) -> Element {
    // CategoricalChart passes the SVG props of the other chart props on.
    let others = root.filter(|key| {
        !matches!(
            key,
            "width"
                | "height"
                | "responsive"
                | "children"
                | "className"
                | "style"
                | "compact"
                | "title"
                | "desc"
        )
    });
    let attributes = svg_properties(&others);
    let accessibility = root.get("accessibilityLayer").is_truthy();
    let tab_index = match attributes.get("tabIndex") {
        Value::Number(n) => Value::Number(n),
        _ if accessibility => Value::Number(0.0),
        _ => Value::Undefined,
    };
    let role = match attributes.get("role") {
        Value::String(s) => Value::String(s),
        _ if accessibility => "application".into(),
        _ => Value::Undefined,
    };
    let mut props = attributes;
    props.set("role", role);
    props.set("tabIndex", tab_index);
    let (width, height) = (root.get("width"), root.get("height"));
    let mut svg = Element::svg("svg").props(&svg_properties(&props));
    let mut rest = Props::new();
    rest.set("className", "recharts-surface".into());
    rest.set("width", width.clone());
    rest.set("height", height.clone());
    let mut full = Props::new();
    full.set("width", "100%".into());
    full.set("height", "100%".into());
    full.set("display", "block".into());
    rest.set("style", full.to_value());
    rest.set(
        "viewBox",
        format!("0 0 {} {}", width.to_js_string(), height.to_js_string()).into(),
    );
    for (key, value) in rest.entries() {
        svg.prop(key, value);
    }
    for tag in ["title", "desc"] {
        let mut text = Element::svg(tag);
        text.prop("children", &root.get(tag));
        svg.children.push(Node::Element(text));
    }

    // AllZIndexPortals: every registered z-index and the defaults, sorted;
    // negative layers before the plot area's clip path and in-place
    // content, positive ones after.
    let mut layers: Vec<f64> = out.registered.iter().map(|k| parse_int(k)).collect();
    layers.extend(DEFAULT_Z_INDEXES);
    let mut unique: Vec<f64> = Vec::new();
    for z in layers {
        if !unique.contains(&z) {
            unique.push(z);
        }
    }
    unique.sort_by(|a, b| a.partial_cmp(b).expect("finite z-indexes"));
    let layer = |z: f64| {
        // A portal finds its layer by the key its z-index registered.
        let key = number_to_string(z);
        let mut g = Element::svg("g");
        g.prop("tabIndex", &Value::Number(-1.0));
        g.prop("className", &format!("recharts-zIndex-layer_{key}").into());
        for (portal, nodes) in &out.portals {
            if *portal == key {
                g.children.extend(nodes.iter().cloned());
            }
        }
        Node::Element(g)
    };
    for &z in unique.iter().filter(|z| **z < 0.0) {
        svg.children.push(layer(z));
    }
    // ClipPathProvider.
    let PlotArea {
        x,
        y,
        width,
        height,
    } = plot_area;
    let mut rect = Element::svg("rect");
    rect.prop("x", &Value::Number(x));
    rect.prop("y", &Value::Number(y));
    rect.prop("height", &Value::Number(height));
    rect.prop("width", &Value::Number(width));
    let mut clip = Element::svg("clipPath");
    clip.prop("id", &clip_path_id.into());
    clip.children.push(Node::Element(rect));
    let mut defs = Element::svg("defs");
    defs.children.push(Node::Element(clip));
    svg.children.push(Node::Element(defs));
    svg.children.extend(out.inline);
    for &z in unique.iter().filter(|z| **z > 0.0) {
        svg.children.push(layer(z));
    }
    svg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn use_id_counts_in_base_32() {
        let mut ids = Ids::new();
        let first: Vec<String> = (0..33).map(|_| ids.use_id()).collect();
        assert_eq!(first[0], "_r_0_");
        assert_eq!(first[10], "_r_a_");
        assert_eq!(first[32], "_r_10_");
        assert_eq!(ids.unique_id("recharts"), "recharts1");
    }

    #[test]
    fn z_index_keys_parse_as_parse_int() {
        assert_eq!(parse_int("-50"), -50.0);
        assert_eq!(parse_int("2.5"), 2.0);
        assert_eq!(parse_int("1e+21"), 1.0);
    }
}
