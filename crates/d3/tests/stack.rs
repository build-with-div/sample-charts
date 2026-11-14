//! The d3-shape stack port against the npm package: `fixtures/stack.json`
//! holds `d3.stack()` with `stackOrderNone` and each offset Recharts maps,
//! over generated values, written by `tools/reference/shape_fixtures.mjs`.
//! Every `[y0, y1]` must have the same bits (NaN equal to NaN).

use d3::shape::stack::{self, Series};
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/stack.json");

fn number(json: &Json) -> f64 {
    let s = json.as_str().expect("a tagged number");
    match s.strip_prefix("n:").expect("a number") {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        n => n.parse().unwrap(),
    }
}

fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

#[test]
fn stacks_match_d3_shape() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    let mut failures = Vec::new();
    for (i, case) in cases.iter().enumerate() {
        let values: Vec<Vec<f64>> = case["values"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_array().unwrap().iter().map(number).collect())
            .collect();
        let offset = case["offset"].as_str().unwrap();
        let got: Vec<Series> = match offset {
            "stackOffsetNone" => stack::stack(&values, stack::offset_none),
            "stackOffsetExpand" => stack::stack(&values, stack::offset_expand),
            "stackOffsetSilhouette" => stack::stack(&values, stack::offset_silhouette),
            "stackOffsetWiggle" => stack::stack(&values, stack::offset_wiggle),
            other => panic!("offset {other}"),
        };
        let want: Vec<Series> = case["series"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| {
                s.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| [number(&p[0]), number(&p[1])])
                    .collect()
            })
            .collect();
        let equal = got.len() == want.len()
            && got.iter().zip(&want).all(|(g, w)| {
                g.len() == w.len()
                    && g.iter()
                        .zip(w)
                        .all(|(g, w)| same(g[0], w[0]) && same(g[1], w[1]))
            });
        if !equal {
            failures.push(format!(
                "case {i} {offset} {values:?}:\n  port    {got:?}\n  package {want:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} stacks differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(8)].join("\n")
    );
}
