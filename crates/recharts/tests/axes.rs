//! Recharts' axis selectors against the npm package: `fixtures/axes.json`
//! holds the props of random cartesian charts and of shadcn's chart
//! examples, and for every axis what Recharts' browser build
//! (`recharts/es6`) selects from its store, written by
//! `tools/reference/axis_fixtures.mjs`. Every number must have the same bits.
//!
//! Behaviour the port does not cover yet is reported as
//! [`ChartError::Unsupported`]; the test accepts that only where the
//! package's result shows the case really needs it, and prints the counts.

use std::collections::BTreeMap;

use js::{Key, Value};
use recharts::chart::{
    AxisDomainType, AxisProps, AxisResult, AxisType, CartesianChart, ChartError, ChartProps,
    DataKey, ItemKind, ItemProps, Layout, Margin, Padding, StackOffset,
};
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/axes.json");

/// A tagged fixture value ("u", null, bools, "n:…", "s:…", arrays,
/// {"o": …}) as a JavaScript value.
fn decode(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::String(s) if s == "u" => Value::Undefined,
        Json::String(s) if s.starts_with("t:") => panic!("Date values are not modelled yet: {s}"),
        Json::String(s) => match s.strip_prefix("n:") {
            Some(n) => Value::Number(match n {
                "NaN" => f64::NAN,
                "Infinity" => f64::INFINITY,
                "-Infinity" => f64::NEG_INFINITY,
                _ => n.parse().unwrap_or_else(|_| panic!("number {n}")),
            }),
            None => Value::String(s.strip_prefix("s:").expect("tagged string").to_owned()),
        },
        Json::Array(items) => Value::Array(items.iter().map(decode).collect()),
        Json::Object(map) => Value::Object(
            map["o"]
                .as_object()
                .expect("tagged object")
                .iter()
                .map(|(k, v)| (k.clone(), decode(v)))
                .collect(),
        ),
        Json::Number(_) => panic!("untagged number"),
    }
}

/// Equal as JavaScript values: numbers by bits (NaN equal to NaN, -0
/// different from 0), objects by their own properties in any order.
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        (Value::Object(x), Value::Object(y)) => {
            let x: BTreeMap<_, _> = x.iter().map(|(k, v)| (k, v)).collect();
            let y: BTreeMap<_, _> = y.iter().map(|(k, v)| (k, v)).collect();
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

fn props_of(value: &Value) -> &[(String, Value)] {
    match value {
        Value::Object(props) => props,
        other => panic!("not an object: {other:?}"),
    }
}

fn list(value: &Value) -> Vec<Value> {
    match value {
        Value::Array(items) => items.clone(),
        other => panic!("not an array: {other:?}"),
    }
}

fn number(value: &Value) -> f64 {
    value
        .as_number()
        .unwrap_or_else(|| panic!("not a number: {value:?}"))
}

fn string(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => panic!("not a string: {other:?}"),
    }
}

fn boolean(value: &Value) -> bool {
    match value {
        Value::Bool(b) => *b,
        other => panic!("not a boolean: {other:?}"),
    }
}

fn data_key(value: &Value) -> DataKey {
    match value {
        Value::String(s) => DataKey::Name(s.clone()),
        Value::Number(n) => DataKey::Index(*n),
        other => panic!("data key {other:?}"),
    }
}

fn key(value: &Value) -> Key {
    Key::of(value).expect("a primitive id")
}

fn chart_props(value: &Value) -> ChartProps {
    let mut chart = ChartProps::new("", 0.0, 0.0);
    for (name, v) in props_of(value) {
        match name.as_str() {
            "chartName" => chart.chart_name = string(v),
            "width" => chart.width = number(v),
            "height" => chart.height = number(v),
            "layout" => chart.layout = Some(Layout::from_name(&string(v)).expect("layout")),
            "margin" => {
                let mut margin = Margin::default();
                for (side, n) in props_of(v) {
                    let n = Some(number(n));
                    match side.as_str() {
                        "top" => margin.top = n,
                        "right" => margin.right = n,
                        "bottom" => margin.bottom = n,
                        "left" => margin.left = n,
                        other => panic!("margin {other}"),
                    }
                }
                chart.margin = Some(margin);
            }
            "data" => chart.data = Some(list(v)),
            "barCategoryGap" => chart.bar_category_gap = Some(v.clone()),
            "stackOffset" => {
                chart.stack_offset = Some(match string(v).as_str() {
                    "none" => StackOffset::None,
                    "expand" => StackOffset::Expand,
                    other => StackOffset::Other(other.into()),
                })
            }
            other => panic!("chart prop {other}"),
        }
    }
    chart
}

fn axis_props(value: &Value) -> AxisProps {
    let mut axis = AxisProps::default();
    for (name, v) in props_of(value) {
        match name.as_str() {
            "xAxisId" | "yAxisId" => axis.id = Some(key(v)),
            "dataKey" => axis.data_key = Some(data_key(v)),
            "type" => {
                axis.r#type = Some(match string(v).as_str() {
                    "number" => AxisDomainType::Number,
                    "category" => AxisDomainType::Category,
                    "auto" => AxisDomainType::Auto,
                    other => panic!("type {other}"),
                })
            }
            "domain" => axis.domain = Some(list(v)),
            "scale" => axis.scale = Some(string(v)),
            "allowDataOverflow" => axis.allow_data_overflow = Some(boolean(v)),
            "allowDecimals" => axis.allow_decimals = Some(boolean(v)),
            "allowDuplicatedCategory" => axis.allow_duplicated_category = Some(boolean(v)),
            "includeHidden" => axis.include_hidden = Some(boolean(v)),
            "reversed" => axis.reversed = Some(boolean(v)),
            "hide" => axis.hide = Some(boolean(v)),
            "mirror" => axis.mirror = Some(boolean(v)),
            "tickCount" => axis.tick_count = Some(number(v)),
            "ticks" => axis.ticks = Some(list(v)),
            "niceTicks" => axis.nice_ticks = Some(string(v)),
            "padding" => {
                axis.padding = Some(match v {
                    Value::String(keyword) => Padding::Keyword(keyword.clone()),
                    sides => {
                        let side = |name: &str| sides.get(name).as_number();
                        Padding::Sides {
                            left: side("left"),
                            right: side("right"),
                            top: side("top"),
                            bottom: side("bottom"),
                        }
                    }
                })
            }
            "orientation" => axis.orientation = Some(string(v)),
            "height" => axis.height = Some(number(v)),
            "width" => axis.width = Some(number(v)),
            other => panic!("axis prop {other}"),
        }
    }
    axis
}

fn item_props(value: &Value) -> ItemProps {
    let kind = match string(&value.get("kind")).as_str() {
        "bar" => ItemKind::Bar,
        "line" => ItemKind::Line,
        "area" => ItemKind::Area,
        other => panic!("item kind {other}"),
    };
    let mut item = ItemProps::new(kind, &string(&value.get("id")), None);
    for (name, v) in props_of(value) {
        match name.as_str() {
            "kind" | "id" => {}
            "dataKey" => item.data_key = Some(data_key(v)),
            "hide" => item.hide = Some(boolean(v)),
            "xAxisId" => item.x_axis_id = Some(key(v)),
            "yAxisId" => item.y_axis_id = Some(key(v)),
            "stackId" => item.stack_id = Some(v.clone()),
            "data" => item.data = Some(list(v)),
            other => panic!("item prop {other}"),
        }
    }
    item
}

fn numbers(values: &[f64]) -> Value {
    Value::Array(values.iter().map(|n| Value::Number(*n)).collect())
}

fn option<T>(value: &Option<T>, f: impl Fn(&T) -> Value) -> Value {
    value.as_ref().map_or(Value::Undefined, f)
}

/// The port's result in the fixture's shape, field by field.
fn result_fields(result: &AxisResult) -> Vec<(&'static str, Value)> {
    let scale = match &result.scale {
        None => Value::Null,
        Some(scale) => {
            let map = result
                .applied_values
                .iter()
                .map(|v| {
                    Value::Array(vec![
                        option(&scale.map(v, None), |n| Value::Number(*n)),
                        option(
                            &scale.map(v, Some(recharts::chart::BandPosition::Middle)),
                            |n| Value::Number(*n),
                        ),
                    ])
                })
                .collect();
            Value::Object(vec![
                ("domain".into(), Value::Array(scale.domain())),
                // RechartsScale's `range()` is `[rangeMin, rangeMax]`.
                ("range".into(), numbers(&scale.range_extent())),
                (
                    "bandwidth".into(),
                    option(&scale.bandwidth(), |n| Value::Number(*n)),
                ),
                ("map".into(), Value::Array(map)),
            ])
        }
    };
    let ticks = option(&result.ticks, |ticks| {
        Value::Array(
            ticks
                .iter()
                .map(|t| {
                    Value::Array(vec![
                        Value::Number(t.index as f64),
                        Value::Number(t.coordinate),
                        t.value.clone(),
                        Value::Number(t.offset),
                    ])
                })
                .collect(),
        )
    });
    let values = |v: &Option<Vec<Value>>| option(v, |v| Value::Array(v.clone()));
    vec![
        ("appliedValues", Value::Array(result.applied_values.clone())),
        (
            "realScaleType",
            option(&result.real_scale_type, |s| Value::String(s.clone())),
        ),
        ("domain", values(&result.domain)),
        ("niceTicks", option(&result.nice_ticks, |t| numbers(t))),
        (
            "domainIncludingNiceTicks",
            values(&result.domain_including_nice_ticks),
        ),
        ("checkedDomain", values(&result.checked_domain)),
        ("range", numbers(&result.range)),
        ("rangeWithReverse", numbers(&result.range_with_reverse)),
        ("scale", scale),
        ("ticks", ticks),
    ]
}

/// Why the package's result shows a case needs what the port reports as
/// not ported yet; `None` when it does not.
fn unsupported_is_expected(
    reason: &str,
    want: &Json,
    items: &[ItemProps],
    applied: Option<&Value>,
) -> bool {
    match reason {
        "stacked series (milestone M3)" => items.iter().any(|i| {
            i.kind != ItemKind::Line && i.stack_id.as_ref().is_some_and(|s| !s.is_nullish())
        }),
        "d3 scales other than linear, band and point" => match want.get("realScaleType") {
            Some(t) => !matches!(
                decode(t),
                Value::String(ref s) if s == "linear" || s == "band" || s == "point"
            ),
            None => false,
        },
        "object domain values" => applied.is_some_and(|v| {
            list(v)
                .iter()
                .any(|v| matches!(v, Value::Array(_) | Value::Object(_)))
        }),
        _ => false,
    }
}

#[test]
fn axes_match_recharts() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut unsupported: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut thrown = 0;
    let mut axes = 0;
    for case in cases {
        let chart = chart_props(&decode(&case["chart"]));
        let x_axes: Vec<_> = list(&decode(&case["xAxes"]))
            .iter()
            .map(axis_props)
            .collect();
        let y_axes: Vec<_> = list(&decode(&case["yAxes"]))
            .iter()
            .map(axis_props)
            .collect();
        let items: Vec<_> = list(&decode(&case["items"]))
            .iter()
            .map(item_props)
            .collect();
        let store = CartesianChart::new(&chart, &x_axes, &y_axes, &items);
        for want in case["axes"].as_array().unwrap() {
            axes += 1;
            let axis_type = match want["axisType"].as_str().unwrap() {
                "xAxis" => AxisType::XAxis,
                _ => AxisType::YAxis,
            };
            let id = key(&decode(&want["id"]));
            let label = format!("case {} {:?} {:?}", case["index"], axis_type, id);
            match (store.axis(axis_type, &id), want.get("error")) {
                (Err(ChartError::Unsupported(reason)), _) => {
                    let applied = want.get("appliedValues").map(decode);
                    if unsupported_is_expected(reason, want, &items, applied.as_ref()) {
                        *unsupported.entry(reason).or_default() += 1;
                    } else {
                        failures.push(format!(
                            "{label}: not ported ({reason}), package gives {want}"
                        ));
                    }
                }
                (Err(ChartError::Js(error)), Some(message)) => {
                    thrown += 1;
                    if error.message != message.as_str().unwrap() {
                        failures.push(format!(
                            "{label}: throws {:?}, package throws {message}",
                            error.message
                        ));
                    }
                }
                (Err(ChartError::Js(error)), None) => {
                    failures.push(format!(
                        "{label}: throws {:?}, package does not",
                        error.message
                    ));
                }
                (Ok(_), Some(message)) => {
                    failures.push(format!("{label}: does not throw, package throws {message}"));
                }
                (Ok(result), None) => {
                    for (field, got) in result_fields(&result) {
                        // `scale` is written as a plain object of tagged fields.
                        let expected = match (field, &want[field]) {
                            ("scale", Json::Object(fields)) => Value::Object(
                                fields.iter().map(|(k, v)| (k.clone(), decode(v))).collect(),
                            ),
                            (_, json) => decode(json),
                        };
                        if !same(&got, &expected) {
                            failures
                                .push(format!("{label} {field}: {got:?}, package {expected:?}"));
                        }
                    }
                }
            }
        }
    }
    println!(
        "{axes} axes of {} charts: {} exceptions matched, not ported yet: {unsupported:?}",
        cases.len(),
        thrown
    );
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures[..failures.len().min(30)].join("\n")
    );
}

/// The shadcn bar chart's value axis, as the design document quotes it.
#[test]
fn shadcn_bar_default_value_axis() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let case = fixtures["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["index"] == "shadcn-bar-default")
        .expect("the shadcn bar chart case");
    let chart = chart_props(&decode(&case["chart"]));
    let x_axes: Vec<_> = list(&decode(&case["xAxes"]))
        .iter()
        .map(axis_props)
        .collect();
    let items: Vec<_> = list(&decode(&case["items"]))
        .iter()
        .map(item_props)
        .collect();
    let y = CartesianChart::new(&chart, &x_axes, &[], &items)
        .axis(AxisType::YAxis, &Key::number(0.0))
        .unwrap();
    assert_eq!(y.nice_ticks, Some(vec![0.0, 80.0, 160.0, 240.0, 320.0]));
    assert_eq!(y.range_with_reverse, [265.0, 5.0]);
}

/// The fixtures are only Chromium's results if Chromium's `Math.pow` gives
/// what the generator's did for every call it made; the port's `Math.pow`
/// follows Chromium's (see `js::math::pow`).
#[test]
fn recorded_pow_calls_agree_with_the_port() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let mut failures = Vec::new();
    for call in fixtures["powCalls"].as_array().unwrap() {
        let [x, y, want] = [0, 1, 2].map(|i| number(&decode(&call[i])));
        let got = js::math::pow(x, y);
        if got.to_bits() != want.to_bits() {
            failures.push(format!(
                "Math.pow({x:?}, {y:?}): port {got:?}, generator {want:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
