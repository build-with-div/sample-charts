//! Recharts' `Curve` path against the npm package: `fixtures/curve.json`
//! holds inputs to `getPath` (`shape/Curve.js`) and the `d` it returns,
//! written by `tools/reference/shape_fixtures.mjs`. Every path must be the
//! same string.

use js::Value;
use recharts::shape::{get_path, BaseLine, CurvePoint};
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/curve.json");

/// A tagged fixture value as a JavaScript value.
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

fn string(value: &Value) -> Option<String> {
    match value {
        Value::Undefined => None,
        Value::String(s) => Some(s.clone()),
        other => panic!("not a string: {other:?}"),
    }
}

#[test]
fn paths_match_recharts() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let input = decode(&case["input"]);
        let points: Vec<CurvePoint> = match input.get("points") {
            Value::Array(points) => points
                .iter()
                .map(|p| CurvePoint {
                    x: p.get("x"),
                    y: p.get("y"),
                })
                .collect(),
            other => panic!("points {other:?}"),
        };
        let r#type = string(&input.get("type"));
        let layout = string(&input.get("layout"));
        let connect_nulls = input.get("connectNulls") == Value::Bool(true);
        let got = get_path(
            r#type.as_deref(),
            &points,
            &BaseLine::from_value(&input.get("baseLine")),
            layout.as_deref(),
            connect_nulls,
        );
        let want = match decode(&case["path"]) {
            Value::Null => None,
            Value::String(s) => Some(s),
            other => panic!("path {other:?}"),
        };
        if got != want {
            failures.push(format!(
                "case {i} {}:\n  port    {got:?}\n  package {want:?}",
                case["input"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} paths differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(8)].join("\n")
    );
}
