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
//! Recharts measures text in the page and reads the computed font of tick
//! labels; the port gets both from what the fixture recorded in Chrome for
//! that case. A measurement the port asks for that Chrome did not make is a
//! failure, and so is a font Chrome read that the port does not. Chrome
//! measures more: Recharts renders the chart several times while its store
//! fills in, and the port only renders the settled chart.
//!
//! When Recharts throws, React unmounts the chart: the port must throw the
//! same exception. Messages are compared as V8 gives them for Recharts'
//! source; the fixture's come from an esbuild bundle, which renames the
//! `d3Scales` import to `d3_scale_exports`.

use std::collections::{BTreeMap, HashMap, HashSet};

use js::Value;
use recharts::chart::ChartError;
use recharts::component::{
    render, string_cache_key, ChartElement, Component, ComponentElement, Ids, Measure, Size,
};
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

/// What Chrome measured for one case.
struct Recorded {
    sizes: HashMap<String, Size>,
    tick_fonts: Vec<(String, String)>,
    /// Measurements and fonts the port asked for that were not recorded.
    misses: Vec<String>,
    /// The cache keys of the texts the port measured.
    asked: HashSet<String>,
    /// Whether the port read a tick label's font.
    read_font: bool,
}

impl Recorded {
    fn new(case: &Json) -> Recorded {
        let mut sizes = HashMap::new();
        for call in case["measurements"].as_array().expect("measurements") {
            let call = call.as_array().expect("call");
            // getStringSize gives 0 × 0 for these without measuring.
            if matches!(decode(&call[0]), Value::Undefined | Value::Null) {
                continue;
            }
            let style = Props::of(&decode(&call[1]));
            let key = string_cache_key(&decode(&call[0]).to_js_string(), &style);
            let size = Size {
                width: decode(&call[2]).to_number(),
                height: decode(&call[3]).to_number(),
            };
            // getStringSize caches by this key, so a key has one size.
            if let Some(previous) = sizes.insert(key.clone(), size) {
                assert_eq!(previous, size, "{key}");
            }
        }
        let tick_fonts = case["tickFonts"]
            .as_array()
            .expect("tickFonts")
            .iter()
            .map(|f| {
                (
                    f[0].as_str().expect("fontSize").to_owned(),
                    f[1].as_str().expect("letterSpacing").to_owned(),
                )
            })
            .collect();
        Recorded {
            sizes,
            tick_fonts,
            misses: Vec::new(),
            asked: HashSet::new(),
            read_font: false,
        }
    }

    /// How many texts Chrome measured that the port did not ask for.
    /// Recharts renders a chart several times before its store holds every
    /// component's settings and data, and measures in each render; the port
    /// renders the settled chart only.
    fn unasked(&self) -> usize {
        self.sizes
            .keys()
            .filter(|key| !self.asked.contains(*key))
            .count()
    }
}

impl Measure for Recorded {
    fn string_size(&mut self, text: &str, style: &Props) -> Size {
        let key = string_cache_key(text, style);
        self.asked.insert(key.clone());
        match self.sizes.get(&key) {
            Some(size) => *size,
            None => {
                self.misses.push(format!("getStringSize {key:?}"));
                Size {
                    width: 0.0,
                    height: 0.0,
                }
            }
        }
    }

    fn tick_font(&mut self, _label: &Element) -> (String, String) {
        self.read_font = true;
        // Chrome gave every tick label of a case the same font, or the case
        // has none.
        match self.tick_fonts.as_slice() {
            [font] => font.clone(),
            fonts => {
                self.misses
                    .push(format!("getComputedStyle: recorded {fonts:?}"));
                (String::new(), String::new())
            }
        }
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

/// What a case's tree holds, counted.
#[derive(Default)]
struct Counts {
    elements: usize,
    attributes: usize,
    declarations: usize,
    texts: usize,
    /// Elements by the Recharts class that names them (first token).
    by_class: BTreeMap<String, usize>,
}

fn compare(got: &[Node], want: &[Want], path: &str, counts: &mut Counts) -> Result<(), String> {
    if got.len() != want.len() {
        return Err(format!(
            "{path}: {} children, Chrome has {}\n    port   {:?}\n    Chrome {:?}",
            got.len(),
            want.len(),
            got.iter().map(summary).collect::<Vec<_>>(),
            want.iter().map(want_summary).collect::<Vec<_>>(),
        ));
    }
    for (i, (g, w)) in got.iter().zip(want).enumerate() {
        match (g, w) {
            (Node::Text(a), Want::Text(b)) if a == b => counts.texts += 1,
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

fn summary(node: &Node) -> String {
    match node {
        Node::Text(t) => format!("{t:?}"),
        Node::Element(e) => format!("<{} {}>", e.tag, e.attribute("class").unwrap_or("")),
    }
}

fn want_summary(node: &Want) -> String {
    match node {
        Want::Text(t) => format!("{t:?}"),
        Want::Element {
            tag, attributes, ..
        } => format!(
            "<{tag} {}>",
            attributes
                .iter()
                .find(|(n, _)| n == "class")
                .map_or("", |(_, v)| v.as_str())
        ),
    }
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
    if let Some(class) = element.attribute("class") {
        let name = class
            .split_ascii_whitespace()
            .find(|c| *c != "recharts-layer")
            .unwrap_or("recharts-layer");
        *counts.by_class.entry(name.to_owned()).or_default() += 1;
    }
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
fn charts_match_chromium() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut unsupported: BTreeMap<String, usize> = BTreeMap::new();
    let mut thrown: BTreeMap<String, usize> = BTreeMap::new();
    let mut counts = Counts::default();
    let mut equal = 0;
    let mut measured = 0;
    let mut unasked = 0;
    for case in cases {
        let index = case["index"].as_str().unwrap();
        let element = chart_element(case);
        let errors: Vec<&str> = case["errors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e.as_str().unwrap())
            .collect();
        let dom: Vec<Want> = case["dom"].as_array().unwrap().iter().map(want).collect();
        let mut recorded = Recorded::new(case);
        let result = render(&element, &mut Ids::new(), &mut recorded);
        if !recorded.misses.is_empty() {
            failures.push(format!("{index}: not recorded: {:?}", recorded.misses));
            continue;
        }
        if !recorded.tick_fonts.is_empty() && !recorded.read_font {
            failures.push(format!(
                "{index}: Chrome read a tick label's font, the port did not"
            ));
            continue;
        }
        unasked += recorded.unasked();
        match result {
            Err(ChartError::Unsupported(what)) => {
                *unsupported.entry(what.to_owned()).or_default() += 1;
            }
            Err(ChartError::Js(error)) => {
                let message = error.to_string();
                let bundled = message.replace("d3Scales[", "d3_scale_exports[");
                if errors != [bundled.as_str()] || !dom.is_empty() {
                    failures.push(format!(
                        "{index}: the port throws {message:?}, Chrome {errors:?}"
                    ));
                } else {
                    *thrown.entry(message).or_default() += 1;
                }
            }
            Ok(rendered) => {
                if !errors.is_empty() {
                    failures.push(format!(
                        "{index}: Chrome throws {errors:?}, the port does not"
                    ));
                    continue;
                }
                match compare(&rendered.nodes, &dom, "", &mut counts) {
                    Ok(()) => {
                        equal += 1;
                        measured += recorded.asked.len();
                    }
                    Err(difference) => failures.push(format!("{index}: {difference}")),
                }
            }
        }
    }
    println!(
        "{equal} charts equal ({} elements, {} attributes, {} style declarations, {} texts; \
         {measured} text sizes, as measured in Chrome), of {} cases",
        counts.elements,
        counts.attributes,
        counts.declarations,
        counts.texts,
        cases.len()
    );
    println!("texts Chrome also measured in earlier renders: {unasked}");
    println!("exceptions matched: {thrown:?}");
    println!("unsupported: {unsupported:?}");
    println!("elements by class: {:?}", counts.by_class);
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(8)].join("\n")
    );
}
