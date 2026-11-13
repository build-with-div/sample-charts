//! The tick port against the original: `fixtures/nice_ticks.json` holds the
//! results of recharts@3.8.0's `getNiceTickValues`,
//! `getTickValuesFixedDomain` and `getDigitCount` under Node, written by
//! `tools/reference/gen_fixtures.mjs`. Every result must be equal: the same
//! numbers bit for bit (compared as JavaScript's `String(n)`, with `-0`
//! kept), or the same exception message.

use recharts::js::number_to_string;
use recharts::scale::{
    get_digit_count, get_nice_tick_values, get_tick_values_fixed_domain, NiceTicks,
};
use recharts::JsError;
use serde_json::Value;

const FIXTURES: &str = include_str!("fixtures/nice_ticks.json");

fn number(text: &str) -> f64 {
    text.parse()
        .unwrap_or_else(|_| panic!("fixture number {text}"))
}

/// `String(n)`, with `-0` for negative zero, as the generator writes it.
fn encode(value: f64) -> String {
    if value == 0.0 && value.is_sign_negative() {
        "-0".into()
    } else {
        number_to_string(value)
    }
}

/// The expected result: tick strings, or an error message.
fn expected(result: &Value) -> Result<Vec<String>, String> {
    if let Some(error) = result.get("error") {
        return Err(error.as_str().unwrap().to_owned());
    }
    if let Some(ticks) = result.get("ticks") {
        return Ok(ticks
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_str().unwrap().to_owned())
            .collect());
    }
    let mut ticks = Vec::new();
    for run in result["runs"].as_array().unwrap() {
        let value = run[0].as_str().unwrap();
        let count = run[1].as_u64().unwrap() as usize;
        ticks.extend(std::iter::repeat_n(value.to_owned(), count));
    }
    Ok(ticks)
}

fn actual(result: Result<Vec<f64>, JsError>) -> Result<Vec<String>, String> {
    result
        .map(|ticks| ticks.into_iter().map(encode).collect())
        .map_err(|error| error.message)
}

#[test]
fn ticks_match_recharts() {
    let fixtures: Value = serde_json::from_str(FIXTURES).unwrap();
    let cases = fixtures["cases"].as_array().unwrap();
    assert!(
        cases.len() > 3000,
        "the fixture lists {} cases",
        cases.len()
    );
    let mut failures = Vec::new();
    for case in cases {
        let domain: Vec<f64> = case["domain"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| number(v.as_str().unwrap()))
            .collect();
        let domain = [domain[0], domain[1]];
        let tick_count = case["tickCount"].as_u64().unwrap() as u32;
        let allow_decimals = case["allowDecimals"].as_bool().unwrap();
        let mode = NiceTicks::from_name(case["mode"].as_str().unwrap()).unwrap();
        let nice = actual(get_nice_tick_values(
            domain,
            tick_count,
            allow_decimals,
            mode,
        ));
        if nice != expected(&case["nice"]) {
            failures.push(format!(
                "getNiceTickValues({domain:?}, {tick_count}, {allow_decimals}, {mode:?}): \
                 got {}, want {}",
                short(&nice),
                short(&expected(&case["nice"]))
            ));
        }
        let fixed = actual(get_tick_values_fixed_domain(
            domain,
            tick_count,
            allow_decimals,
            mode,
        ));
        if fixed != expected(&case["fixed"]) {
            failures.push(format!(
                "getTickValuesFixedDomain({domain:?}, {tick_count}, {allow_decimals}, {mode:?}): \
                 got {}, want {}",
                short(&fixed),
                short(&expected(&case["fixed"]))
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} results differ:\n{}",
        failures.len(),
        cases.len() * 2,
        failures[..failures.len().min(30)].join("\n")
    );
}

fn short(result: &Result<Vec<String>, String>) -> String {
    match result {
        Ok(ticks) if ticks.len() > 12 => {
            format!("[{} … ({} ticks)]", ticks[..6].join(", "), ticks.len())
        }
        Ok(ticks) => format!("[{}]", ticks.join(", ")),
        Err(error) => format!("error {error:?}"),
    }
}

#[test]
fn digit_counts_match_recharts() {
    let fixtures: Value = serde_json::from_str(FIXTURES).unwrap();
    let mut failures = Vec::new();
    for entry in fixtures["digitCount"].as_array().unwrap() {
        let value = number(entry["value"].as_str().unwrap());
        let want = entry["digitCount"].as_i64().unwrap();
        let got = get_digit_count(value).unwrap();
        if got != want {
            failures.push(format!("getDigitCount({value:?}): got {got}, want {want}"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
