//! The `js` crate against V8: `fixtures/math.json` holds `Math.log10`,
//! `Math.log`, `Number(string)` and `parseFloat(string)` results written by
//! `tools/reference/axis_fixtures.mjs`. Every result must have the same bits.

use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/math.json");

fn number(json: &Json) -> f64 {
    let text = json
        .as_str()
        .and_then(|s| s.strip_prefix("n:"))
        .expect("a number");
    match text {
        "NaN" => f64::NAN,
        "Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ => text.parse().unwrap_or_else(|_| panic!("number {text}")),
    }
}

fn same(a: f64, b: f64) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn check(name: &str, cases: &Json, f: impl Fn(&Json) -> f64) {
    let mut failures = Vec::new();
    let cases = cases.as_array().unwrap();
    for case in cases {
        let got = f(&case[0]);
        let want = number(&case[1]);
        if !same(got, want) {
            failures.push(format!("{name}({}) = {got:?}, want {want:?}", case[0]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} differ:\n{}",
        failures.len(),
        cases.len(),
        failures[..failures.len().min(20)].join("\n")
    );
}

#[test]
fn log10_matches_v8() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    check("Math.log10", &fixtures["log10"], |x| {
        js::math::log10(number(x))
    });
}

#[test]
fn log_matches_v8() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    check("Math.log", &fixtures["log"], |x| js::math::log(number(x)));
}

#[test]
fn string_to_number_matches_v8() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    check("Number", &fixtures["toNumber"], |s| {
        js::string_to_number(s.as_str().unwrap())
    });
    check("parseFloat", &fixtures["parseFloat"], |s| {
        js::parse_float(s.as_str().unwrap())
    });
}
