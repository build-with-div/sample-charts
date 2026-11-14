//! Recharts' `<Rectangle>` and `<Symbols>` against the npm package:
//! `fixtures/shapes.json` holds their props and the `<path>` React's server
//! renderer writes for them (nothing, or its `d` and `transform`), written
//! by `tools/reference/shape_fixtures.mjs`. Every attribute must be the same
//! string. Star symbols need Chromium's `Math.sin`/`Math.cos`, not ported
//! yet; the test accepts that only for them and prints the count.

use d3::path::PathError;
use js::Value;
use recharts::shape::{rectangle, symbols, Radius};
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/shapes.json");

fn decode(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::String(s) if s == "u" => Value::Undefined,
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

fn number(value: &Value) -> f64 {
    value
        .as_number()
        .unwrap_or_else(|| panic!("not a number: {value:?}"))
}

fn text(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => Some(s.clone()),
        Value::Null | Value::Undefined => None,
        other => panic!("not a string: {other:?}"),
    }
}

fn radius(value: &Value) -> Radius {
    match value {
        // resolveDefaultProps fills radius: 0
        Value::Undefined => Radius::Number(0.0),
        Value::Number(n) => Radius::Number(*n),
        Value::Array(items) => Radius::Corners(
            items
                .iter()
                .map(|v| match v {
                    Value::Null | Value::Undefined => None,
                    v => Some(number(v)),
                })
                .collect(),
        ),
        _ => Radius::Other,
    }
}

#[test]
fn rectangles_match_recharts() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["rectangles"].as_array().unwrap();
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let props = decode(&case["props"]);
        let got = rectangle(
            number(&props.get("x")),
            number(&props.get("y")),
            number(&props.get("width")),
            number(&props.get("height")),
            &radius(&props.get("radius")),
        );
        let want = text(&decode(&case["d"]));
        if got != want {
            failures.push(format!(
                "rectangle {i} {props:?}:\n  port    {got:?}\n  package {want:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} rectangles differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(8)].join("\n")
    );
}

#[test]
fn symbols_match_recharts() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["symbols"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut stars = 0;
    for (i, case) in cases.iter().enumerate() {
        let props = decode(&case["props"]);
        let r#type = match props.get("type") {
            Value::String(s) => Some(s),
            _ => None,
        };
        let optional = |name: &str| match props.get(name) {
            Value::Undefined => None,
            v => Some(number(&v)),
        };
        let size_type = text(&props.get("sizeType"));
        let got = symbols(
            r#type.as_deref(),
            optional("size"),
            size_type.as_deref(),
            optional("cx"),
            optional("cy"),
        );
        let want = match &case["path"] {
            Json::Null => None,
            path => Some((
                text(&decode(&path["transform"])).expect("a transform"),
                text(&decode(&path["d"])),
            )),
        };
        match got {
            Err(PathError::Trigonometry) if r#type.as_deref() == Some("star") => stars += 1,
            Err(error) => failures.push(format!("symbol {i} {props:?}: {error:?}")),
            Ok(got) => {
                let got = got.map(|p| (p.transform, p.d));
                if got != want {
                    failures.push(format!(
                        "symbol {i} {props:?}:\n  port    {got:?}\n  package {want:?}"
                    ));
                }
            }
        }
    }
    println!(
        "{} symbols: {stars} stars not ported yet (Math.sin, Math.cos)",
        cases.len()
    );
    assert!(
        failures.is_empty(),
        "{} of {} symbols differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(8)].join("\n")
    );
}
