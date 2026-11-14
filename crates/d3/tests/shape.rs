//! The d3-shape port against the npm package: `fixtures/shape.json` holds
//! the path data of `d3.line()` and `d3.area()` with each curve Recharts
//! uses, over points with gaps, written by `tools/reference/shape_fixtures.mjs`.
//! Every path must be the same string.

use d3::shape::{area, line, AreaPoint, Curve};
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/shape.json");

fn number(json: &Json) -> f64 {
    let s = json.as_str().expect("a tagged number");
    match s.strip_prefix("n:").expect("a number") {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        n => n.parse().unwrap(),
    }
}

fn curve(name: &str) -> Curve {
    match name {
        "curveBasis" => Curve::Basis,
        "curveBasisClosed" => Curve::BasisClosed,
        "curveBasisOpen" => Curve::BasisOpen,
        "curveBumpX" => Curve::BumpX,
        "curveBumpY" => Curve::BumpY,
        "curveLinear" => Curve::Linear,
        "curveLinearClosed" => Curve::LinearClosed,
        "curveMonotoneX" => Curve::MonotoneX,
        "curveMonotoneY" => Curve::MonotoneY,
        "curveNatural" => Curve::Natural,
        "curveStep" => Curve::Step,
        "curveStepAfter" => Curve::StepAfter,
        "curveStepBefore" => Curve::StepBefore,
        other => panic!("curve {other}"),
    }
}

#[test]
fn paths_match_d3_shape() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let points: Vec<Option<[f64; 4]>> = case["points"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| {
                p.as_array()
                    .map(|v| [number(&v[0]), number(&v[1]), number(&v[2]), number(&v[3])])
            })
            .collect();
        let digits = match &case["digits"] {
            Json::Null => None,
            d => Some(number(d) as u32),
        };
        let curve = curve(case["curve"].as_str().unwrap());
        let got = if case["kind"] == "line" {
            let points: Vec<_> = points.iter().map(|p| p.map(|p| [p[0], p[1]])).collect();
            line(&points, curve, digits)
        } else {
            let (has_x1, has_y1) = (case["hasX1"] == true, case["hasY1"] == true);
            let points: Vec<_> = points
                .iter()
                .map(|p| {
                    p.map(|p| AreaPoint {
                        x0: p[0],
                        y0: p[2],
                        x1: has_x1.then_some(p[3]),
                        y1: has_y1.then_some(p[1]),
                    })
                })
                .collect();
            area(&points, curve, digits)
        };
        let want = case["path"]
            .as_str()
            .map(|s| s.strip_prefix("s:").unwrap().to_owned());
        if got != want {
            failures.push(format!(
                "case {i} {} {:?} digits {digits:?}:\n  port    {got:?}\n  package {want:?}",
                case["kind"], curve
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} paths differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(10)].join("\n")
    );
}
