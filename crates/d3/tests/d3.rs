//! The d3 port against the npm packages: `fixtures/d3.json` holds results of
//! d3-array's `ticks`, `tickIncrement` and `tickStep`, and of d3-scale's
//! linear, band and point scales, written by
//! `tools/reference/axis_fixtures.mjs`. Every number must have the same bits.

use d3::array::{tick_increment, tick_step, ticks};
use d3::scale::{Band, Linear};
use js::Value;
use serde_json::Value as Json;

const FIXTURES: &str = include_str!("fixtures/d3.json");

/// A tagged fixture value ("u", null, bools, "n:…", "s:…", arrays,
/// {"o": …}) as a JavaScript value.
fn decode(json: &Json) -> Value {
    match json {
        Json::Null => Value::Null,
        Json::Bool(b) => Value::Bool(*b),
        Json::String(s) if s == "u" => Value::Undefined,
        Json::String(s) => {
            if let Some(n) = s.strip_prefix("n:") {
                Value::Number(match n {
                    "NaN" => f64::NAN,
                    "Infinity" => f64::INFINITY,
                    "-Infinity" => f64::NEG_INFINITY,
                    _ => n.parse().unwrap_or_else(|_| panic!("number {n}")),
                })
            } else {
                Value::String(s.strip_prefix("s:").expect("tagged string").to_owned())
            }
        }
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

fn number(json: &Json) -> f64 {
    decode(json).as_number().expect("a number")
}

fn numbers(json: &Json) -> Vec<f64> {
    match decode(json) {
        Value::Array(items) => items.iter().map(|v| v.as_number().unwrap()).collect(),
        other => panic!("not an array: {other:?}"),
    }
}

/// Equal as JavaScript values with numbers compared by bits (NaN equal to
/// NaN, -0 different from 0).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => {
            x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan())
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| same(a, b))
        }
        _ => a == b,
    }
}

fn opt(value: Option<f64>) -> Value {
    value.map_or(Value::Undefined, Value::Number)
}

fn list(values: &[f64]) -> Value {
    Value::Array(values.iter().map(|n| Value::Number(*n)).collect())
}

fn result(json: &Json) -> Result<Value, String> {
    match json.get("error") {
        Some(error) => Err(error.as_str().unwrap().to_owned()),
        None => Ok(decode(json)),
    }
}

fn same_result(got: &Result<Value, String>, want: &Result<Value, String>) -> bool {
    match (got, want) {
        (Ok(a), Ok(b)) => same(a, b),
        (Err(a), Err(b)) => a == b,
        _ => false,
    }
}

fn finish(name: &str, total: usize, failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{name}: {} of {total} results differ:\n{}",
        failures.len(),
        failures[..failures.len().min(25)].join("\n")
    );
}

#[test]
fn ticks_match_d3_array() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["ticks"].as_array().unwrap();
    let mut failures = Vec::new();
    for case in cases {
        let (start, stop, count) = (
            number(&case["start"]),
            number(&case["stop"]),
            number(&case["count"]),
        );
        let got = ticks(start, stop, count)
            .map(|t| list(&t))
            .map_err(|e| e.message);
        if !same_result(&got, &result(&case["ticks"])) {
            failures.push(format!(
                "ticks({start:?}, {stop:?}, {count:?}) = {got:?}, want {:?}",
                result(&case["ticks"])
            ));
        }
        let (lo, hi) = (js::math::min(start, stop), js::math::max(start, stop));
        let got = Ok(Value::Number(tick_increment(lo, hi, count)));
        if !same_result(&got, &result(&case["tickIncrement"])) {
            failures.push(format!(
                "tickIncrement({lo:?}, {hi:?}, {count:?}) = {got:?}, want {:?}",
                result(&case["tickIncrement"])
            ));
        }
        let got = Ok(Value::Number(tick_step(start, stop, count)));
        if !same_result(&got, &result(&case["tickStep"])) {
            failures.push(format!(
                "tickStep({start:?}, {stop:?}, {count:?}) = {got:?}, want {:?}",
                result(&case["tickStep"])
            ));
        }
    }
    finish("d3-array", cases.len() * 3, failures);
}

#[test]
fn linear_scales_match_d3_scale() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["linear"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut total = 0;
    for case in cases {
        let mut scale = Linear::new();
        scale.set_domain(numbers(&case["domain"]));
        let range = numbers(&case["range"]);
        if case["round"].as_bool().unwrap() {
            scale.set_range_round(range);
        } else {
            scale.set_range(range);
        }
        scale.set_clamp(case["clamp"].as_bool().unwrap());
        if !case["nice"].is_null() {
            let count = match decode(&case["nice"]) {
                Value::Undefined => 10.0,
                v => v.as_number().unwrap(),
            };
            scale.nice(count);
        }
        let label = format!("linear({}, {})", case["domain"], case["range"]);
        total += 1;
        if !same(&list(scale.domain()), &decode(&case["niceDomain"])) {
            failures.push(format!(
                "{label}.nice domain = {:?}, want {:?}",
                scale.domain(),
                decode(&case["niceDomain"])
            ));
        }
        for pair in case["apply"].as_array().unwrap() {
            total += 1;
            let x = number(&pair[0]);
            let got = opt(scale.apply(x));
            if !same(&got, &decode(&pair[1])) {
                failures.push(format!(
                    "{label}({x:?}) = {got:?}, want {:?}",
                    decode(&pair[1])
                ));
            }
        }
        for pair in case["invert"].as_array().unwrap() {
            total += 1;
            let y = number(&pair[0]);
            let got = Value::Number(scale.invert(y));
            if !same(&got, &decode(&pair[1])) {
                failures.push(format!(
                    "{label}.invert({y:?}) = {got:?}, want {:?}",
                    decode(&pair[1])
                ));
            }
        }
        total += 1;
        let count = match decode(&case["count"]) {
            Value::Undefined => 10.0,
            v => v.as_number().unwrap(),
        };
        let got = scale.ticks(count).map(|t| list(&t)).map_err(|e| e.message);
        if !same_result(&got, &Ok(decode(&case["ticks"]))) {
            failures.push(format!(
                "{label}.ticks({count:?}) = {got:?}, want {:?}",
                decode(&case["ticks"])
            ));
        }
    }
    finish("d3-scale linear", total, failures);
}

#[test]
fn band_and_point_scales_match_d3_scale() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["band"].as_array().unwrap();
    let mut failures = Vec::new();
    let mut total = 0;
    for case in cases {
        let point = case["point"].as_bool().unwrap();
        let mut scale = if point { Band::point() } else { Band::new() };
        let domain = match decode(&case["domain"]) {
            Value::Array(items) => items,
            _ => unreachable!(),
        };
        scale.set_domain(domain).expect("primitive domain values");
        let range = numbers(&case["range"]);
        let range = [range[0], range[1]];
        if case["roundRange"].as_bool().unwrap() {
            scale.set_range_round(range);
        } else {
            scale.set_range(range);
        }
        if let Value::Object(settings) = decode(&case["settings"]) {
            for (name, value) in settings {
                match (name.as_str(), value) {
                    ("padding", Value::Number(p)) if point => scale.set_padding_outer(p),
                    ("padding", Value::Number(p)) => scale.set_padding(p),
                    ("paddingInner", Value::Number(p)) => scale.set_padding_inner(p),
                    ("paddingOuter", Value::Number(p)) => scale.set_padding_outer(p),
                    ("align", Value::Number(a)) => scale.set_align(a),
                    ("round", Value::Bool(r)) => scale.set_round(r),
                    other => panic!("setting {other:?}"),
                };
            }
        }
        let label = format!(
            "{}({}, {})",
            if point { "point" } else { "band" },
            case["domain"],
            case["range"]
        );
        total += 3;
        let scale_domain = Value::Array(scale.domain().to_vec());
        if !same(&scale_domain, &decode(&case["scaleDomain"])) {
            failures.push(format!("{label}.domain() = {scale_domain:?}"));
        }
        if !same(
            &Value::Number(scale.bandwidth()),
            &decode(&case["bandwidth"]),
        ) {
            failures.push(format!(
                "{label}.bandwidth() = {:?}, want {:?}",
                scale.bandwidth(),
                decode(&case["bandwidth"])
            ));
        }
        if !same(&Value::Number(scale.step()), &decode(&case["step"])) {
            failures.push(format!(
                "{label}.step() = {:?}, want {:?}",
                scale.step(),
                decode(&case["step"])
            ));
        }
        for pair in case["apply"].as_array().unwrap() {
            total += 1;
            let x = decode(&pair[0]);
            let got = opt(scale.apply(&x));
            if !same(&got, &decode(&pair[1])) {
                failures.push(format!(
                    "{label}({x:?}) = {got:?}, want {:?}",
                    decode(&pair[1])
                ));
            }
        }
    }
    finish("d3-scale band/point", total, failures);
}

/// The fixtures were written by Node, whose `Math.pow` may differ from
/// Chromium's (the platform C library's), which the port follows. Every
/// `Math.pow` call made while generating them must give the same result
/// here, or the fixtures above do not show Chromium's behaviour.
#[test]
fn recorded_math_calls_agree_with_the_port() {
    let fixtures: Json = serde_json::from_str(FIXTURES).unwrap();
    let mut failures = Vec::new();
    for call in fixtures["powCalls"].as_array().unwrap() {
        let (x, y, want) = (number(&call[0]), number(&call[1]), number(&call[2]));
        let got = js::math::pow(x, y);
        if !same(&Value::Number(got), &Value::Number(want)) {
            failures.push(format!(
                "Math.pow({x:?}, {y:?}): port {got:?}, fixture generator {want:?}"
            ));
        }
    }
    for call in fixtures["log10Calls"].as_array().unwrap() {
        let (x, want) = (number(&call[0]), number(&call[1]));
        let got = js::math::log10(x);
        if !same(&Value::Number(got), &Value::Number(want)) {
            failures.push(format!(
                "Math.log10({x:?}): port {got:?}, fixture generator {want:?}"
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
