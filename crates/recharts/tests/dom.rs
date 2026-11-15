//! The element tree the port renders for a chart against the DOM Recharts
//! renders in Chromium: `fixtures/dom.json` holds chart cases (the chart's
//! props and its children) and what each left in its container in headless
//! Chrome, written by `tools/reference/dom_fixtures.mjs`.
//!
//! Every element must have the same name, namespace and attributes (as a
//! set, with the same values), the same inline style declarations in the
//! same order, and the same children; text must be equal. Attribute order is
//! not compared: it does not change what is drawn.
//!
//! What `<CartesianGrid>`, `<XAxis>` and `<YAxis>` draw is not ported yet
//! (`recharts::component`): their subtrees are taken out of Chrome's tree
//! before comparing, and counted. Everything else in the chart must match.

use std::collections::BTreeMap;

use js::Value;
use recharts::chart::ChartError;
use recharts::component::{render, ChartElement, Component, ComponentElement, Ids};
use recharts::dom::{Element, Namespace, Node, Props};
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/dom.json");

/// A tagged fixture value ("u", null, bools, "n:…", "s:…", "t:…" Dates,
/// arrays, {"o": …}) as a JavaScript value.
fn decode(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::String(s) if s == "u" => Value::Undefined,
        Json::String(s) if s.starts_with("t:") => Value::Date(js::Date::new(match &s[2..] {
            "NaN" => f64::NAN,
            t => t.parse().unwrap_or_else(|_| panic!("time {t}")),
        })),
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

fn chart_element(case: &Json) -> ChartElement {
    let mut props = Props::of(&decode(&case["chart"]));
    let chart_name = match props.get("chartName") {
        Value::String(name) => name,
        other => panic!("chartName {other:?}"),
    };
    props.remove("chartName");
    let children = case["children"]
        .as_array()
        .expect("children")
        .iter()
        .map(|child| ComponentElement {
            component: match child["type"].as_str().expect("type") {
                "CartesianGrid" => Component::CartesianGrid,
                "XAxis" => Component::XAxis,
                "YAxis" => Component::YAxis,
                "Bar" => Component::Bar,
                other => panic!("component {other}"),
            },
            props: Props::of(&decode(&child["props"])),
        })
        .collect();
    ChartElement {
        chart_name,
        props,
        children,
    }
}

/// A node of Chrome's tree.
#[derive(Debug, Clone)]
enum Want {
    Element {
        tag: String,
        svg: bool,
        attributes: Vec<(String, String)>,
        /// [property, value, priority]
        style: Vec<[String; 3]>,
        children: Vec<Want>,
    },
    Text(String),
}

fn want(json: &Json) -> Want {
    if let Some(text) = json.as_str() {
        return Want::Text(text.to_owned());
    }
    let strings = |v: &Json| -> Vec<String> {
        v.as_array()
            .expect("strings")
            .iter()
            .map(|s| s.as_str().expect("string").to_owned())
            .collect()
    };
    let svg = match json.get("ns").and_then(Json::as_str) {
        None => false,
        Some("http://www.w3.org/2000/svg") => true,
        Some(other) => panic!("namespace {other}"),
    };
    Want::Element {
        tag: json["tag"].as_str().expect("tag").to_owned(),
        svg,
        attributes: json["attributes"]
            .as_array()
            .expect("attributes")
            .iter()
            .map(|pair| {
                let pair = strings(pair);
                (pair[0].clone(), pair[1].clone())
            })
            .collect(),
        style: json
            .get("style")
            .map(|style| {
                style
                    .as_array()
                    .expect("style")
                    .iter()
                    .map(|d| {
                        let d = strings(d);
                        [d[0].clone(), d[1].clone(), d[2].clone()]
                    })
                    .collect()
            })
            .unwrap_or_default(),
        children: json["children"]
            .as_array()
            .expect("children")
            .iter()
            .map(want)
            .collect(),
    }
}

/// The subtrees of components the port does not draw yet, by the class
/// Recharts gives their root element.
fn not_ported(want: &Want, components: &[Component]) -> Option<Component> {
    let Want::Element { attributes, .. } = want else {
        return None;
    };
    let class = attributes
        .iter()
        .find(|(n, _)| n == "class")
        .map_or("", |(_, v)| v.as_str());
    let tokens: Vec<&str> = class.split_ascii_whitespace().collect();
    let has = |t: &str| tokens.contains(&t);
    let component = if has("recharts-cartesian-grid") {
        Component::CartesianGrid
    } else if has("recharts-xAxis") || has("recharts-xAxis-tick-labels") {
        Component::XAxis
    } else if has("recharts-yAxis") || has("recharts-yAxis-tick-labels") {
        Component::YAxis
    } else {
        return None;
    };
    components.contains(&component).then_some(component)
}

fn strip(
    nodes: &[Want],
    components: &[Component],
    stripped: &mut BTreeMap<String, usize>,
) -> Vec<Want> {
    nodes
        .iter()
        .filter_map(|node| {
            if let Some(component) = not_ported(node, components) {
                *stripped.entry(format!("{component:?}")).or_default() += 1;
                return None;
            }
            Some(match node {
                Want::Element {
                    tag,
                    svg,
                    attributes,
                    style,
                    children,
                } => Want::Element {
                    tag: tag.clone(),
                    svg: *svg,
                    attributes: attributes.clone(),
                    style: style.clone(),
                    children: strip(children, components, stripped),
                },
                text => text.clone(),
            })
        })
        .collect()
}

/// The CSS property an IDL attribute of `CSSStyleDeclaration` names
/// (`backgroundColor` → `background-color`, `WebkitX` → `-webkit-x`).
fn css_property(idl: &str) -> String {
    if idl.starts_with("--") {
        return idl.to_owned();
    }
    let mut out = String::new();
    for c in idl.chars() {
        if c.is_ascii_uppercase() {
            out.push('-');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[derive(Default)]
struct Counts {
    elements: usize,
    attributes: usize,
    declarations: usize,
}

fn compare(got: &[Node], want: &[Want], path: &str, counts: &mut Counts) -> Result<(), String> {
    if got.len() != want.len() {
        return Err(format!(
            "{path}: {} children, Chrome has {}",
            got.len(),
            want.len()
        ));
    }
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        match (g, w) {
            (Node::Text(a), Want::Text(b)) if a == b => {}
            (
                Node::Element(element),
                Want::Element {
                    tag,
                    svg,
                    attributes,
                    style,
                    children,
                },
            ) => {
                let here = format!("{path}/{}[{i}]", element.tag);
                compare_element(element, tag, *svg, attributes, style, &here, counts)?;
                compare(&element.children, children, &here, counts)?;
            }
            _ => return Err(format!("{path}[{i}]: port {g:?}, Chrome {w:?}")),
        }
    }
    Ok(())
}

fn compare_element(
    element: &Element,
    tag: &str,
    svg: bool,
    attributes: &[(String, String)],
    style: &[[String; 3]],
    path: &str,
    counts: &mut Counts,
) -> Result<(), String> {
    counts.elements += 1;
    if element.tag != tag || (element.namespace == Namespace::Svg) != svg {
        return Err(format!(
            "{path}: port <{} {:?}>, Chrome <{tag}> (svg: {svg})",
            element.tag, element.namespace
        ));
    }
    let mut got: Vec<(String, String)> = element.attributes.clone();
    let mut want: Vec<(String, String)> = attributes
        .iter()
        .filter(|(n, _)| n != "style")
        .cloned()
        .collect();
    got.sort();
    want.sort();
    if got != want {
        return Err(format!(
            "{path}: attributes\n    port   {got:?}\n    Chrome {want:?}"
        ));
    }
    counts.attributes += got.len();
    let got_style: Vec<[String; 3]> = element
        .style
        .iter()
        .map(|(name, value)| [css_property(name), value.clone(), String::new()])
        .collect();
    if got_style != style {
        return Err(format!(
            "{path}: style\n    port   {got_style:?}\n    Chrome {style:?}"
        ));
    }
    let has_style_attribute = attributes.iter().any(|(n, _)| n == "style");
    if has_style_attribute == got_style.is_empty() {
        return Err(format!(
            "{path}: style attribute present: {has_style_attribute}"
        ));
    }
    counts.declarations += got_style.len();
    Ok(())
}

#[test]
fn bar_charts_match_chromium() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();
    let mut stripped: BTreeMap<String, usize> = BTreeMap::new();
    let mut counts = Counts::default();
    let (mut equal, mut thrown) = (0, 0);
    for case in cases {
        let index = case["index"].as_str().unwrap();
        let element = chart_element(case);
        let errors = case["errors"].as_array().unwrap();
        let dom: Vec<Want> = case["dom"].as_array().unwrap().iter().map(want).collect();
        match render(&element, &mut Ids::new()) {
            Err(ChartError::Unsupported(what)) => {
                *unsupported.entry(what.to_owned()).or_default() += 1;
            }
            Err(ChartError::Js(error)) => {
                if errors.is_empty() || !dom.is_empty() {
                    failures.push(format!("{index}: the port throws {error}, Chrome does not"));
                } else {
                    thrown += 1;
                }
            }
            Ok(rendered) => {
                if !errors.is_empty() {
                    failures.push(format!(
                        "{index}: Chrome throws {errors:?}, the port does not"
                    ));
                    continue;
                }
                let want = strip(&dom, &rendered.not_rendered, &mut stripped);
                match compare(&rendered.nodes, &want, "", &mut counts) {
                    Ok(()) => equal += 1,
                    Err(difference) => failures.push(format!("{index}: {difference}")),
                }
            }
        }
    }
    println!(
        "{equal} charts equal ({} elements, {} attributes, {} style declarations), \
         {thrown} exceptions matched, of {} cases",
        counts.elements,
        counts.attributes,
        counts.declarations,
        cases.len()
    );
    println!("not ported, taken out of Chrome's tree: {stripped:?}");
    println!("unsupported: {unsupported:?}");
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(10)].join("\n")
    );
}
