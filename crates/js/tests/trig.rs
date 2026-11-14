//! `js::math::sin`, `cos`, `tan` and `asin` against Chromium 147:
//! `fixtures/chrome_trig.json` holds Chrome's `Math.sin`, `Math.cos` and
//! `Math.tan` on 15,484 arguments and `Math.asin` on 8,011
//! (`tools/chrome/trig_probe.mjs`), and `fixtures/chrome_trig_gaps.json` the
//! `sin` and `cos` results Chrome does not round correctly
//! (`tools/chrome/trig_gaps.py`).
//!
//! `sin` and `cos` are correctly rounded, so they must equal Chrome on
//! every argument but the recorded gaps, and equal the correctly rounded
//! value on those; the gaps stay a Chromium-fidelity difference
//! (`docs/framework-gaps.md`). `tan` and `asin` are V8's fdlibm and must
//! equal Chrome everywhere.

use std::collections::BTreeSet;

use serde_json::Value as Json;

const CHROME: &str = include_str!("fixtures/chrome_trig.json");
const GAPS: &str = include_str!("fixtures/chrome_trig_gaps.json");

fn number(json: &Json) -> f64 {
    let text = json.as_str().expect("a number string");
    text.parse()
        .unwrap_or_else(|_| panic!("not a number: {text}"))
}

#[test]
fn sin_and_cos_match_chrome_except_the_recorded_gaps() {
    let chrome: Json = serde_json::from_str(CHROME).unwrap();
    let gaps: Json = serde_json::from_str(GAPS).unwrap();
    let mut recorded = BTreeSet::new();
    for gap in gaps.as_array().unwrap() {
        let name = gap["fn"].as_str().unwrap();
        let x = number(&gap["x"]);
        let got = if name == "sin" {
            js::math::sin(x)
        } else {
            js::math::cos(x)
        };
        assert_eq!(
            got.to_bits(),
            number(&gap["correctlyRounded"]).to_bits(),
            "Math.{name}({x:?}) is not the correctly rounded value"
        );
        recorded.insert((name.to_owned(), gap["x"].as_str().unwrap().to_owned()));
    }

    let cases = chrome["cases"].as_array().unwrap();
    let mut differing = BTreeSet::new();
    let mut unexpected = Vec::new();
    for case in cases {
        let x_text = case[0].as_str().unwrap();
        let x = number(&case[0]);
        for (name, got, want) in [
            ("sin", js::math::sin(x), number(&case[1])),
            ("cos", js::math::cos(x), number(&case[2])),
        ] {
            if got.to_bits() != want.to_bits() {
                let key = (name.to_owned(), x_text.to_owned());
                if !recorded.contains(&key) {
                    unexpected.push(format!("Math.{name}({x_text}) = {got:?}, Chrome {want:?}"));
                }
                differing.insert(key);
            }
        }
    }
    println!(
        "{} arguments: {} sin and {} cos results differ from Chrome (recorded gaps)",
        cases.len(),
        differing.iter().filter(|(name, _)| name == "sin").count(),
        differing.iter().filter(|(name, _)| name == "cos").count(),
    );
    assert!(
        unexpected.is_empty(),
        "{} unrecorded differences:\n{}",
        unexpected.len(),
        unexpected[..unexpected.len().min(20)].join("\n")
    );
    let missing: Vec<_> = recorded.difference(&differing).collect();
    assert!(
        missing.is_empty(),
        "recorded gaps that now match: {missing:?}"
    );
}

#[test]
fn star_symbol_arguments_match_chrome() {
    // d3's symbolStar: sin(π/10), sin(7π/10), sin and cos of τ/10 and of
    // τ·i/5. cos(τ·3/5) is where Node 24's fdlibm differs from Chrome.
    let pi = std::f64::consts::PI;
    assert_eq!(js::math::sin(pi / 10.0), 0.3090169943749474);
    assert_eq!(js::math::sin(7.0 * pi / 10.0), 0.8090169943749475);
    assert_eq!(js::math::cos(2.0 * pi * 3.0 / 5.0), -0.8090169943749476);
    assert_eq!(js::math::sin(2.0 * pi * 3.0 / 5.0), -0.587785252292473);
}

/// Every result must have Chrome's bits (NaN any NaN).
fn check_all(name: &str, cases: &[Json], column: usize, f: fn(f64) -> f64) {
    let mut failures = Vec::new();
    for case in cases {
        let x = number(&case[0]);
        let (got, want) = (f(x), number(&case[column]));
        if got.to_bits() != want.to_bits() && !(got.is_nan() && want.is_nan()) {
            failures.push(format!(
                "Math.{name}({}) = {got:?}, Chrome {want:?}",
                case[0]
            ));
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
fn tan_matches_chrome() {
    let chrome: Json = serde_json::from_str(CHROME).unwrap();
    check_all("tan", chrome["cases"].as_array().unwrap(), 3, js::math::tan);
}

#[test]
fn asin_matches_chrome() {
    let chrome: Json = serde_json::from_str(CHROME).unwrap();
    check_all(
        "asin",
        chrome["asin"].as_array().unwrap(),
        1,
        js::math::asin,
    );
}
