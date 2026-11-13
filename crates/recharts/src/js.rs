//! JavaScript semantics the ported code depends on: `Number#toString`,
//! `Math.round`, and the exceptions the original throws.

use std::fmt;

/// An exception the original JavaScript throws, with its message. The port
/// returns it where the package would throw, so callers see the same
/// failures (Recharts catches none of these around its tick code).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsError {
    pub message: String,
}

impl JsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// decimal.js-light's `invalidArgument` error, thrown by
    /// `new Decimal(value)` for `NaN` and infinities.
    pub fn decimal_invalid_argument(value: f64) -> Self {
        Self::new(format!(
            "[DecimalError] Invalid argument: {}",
            number_to_string(value)
        ))
    }

    /// A `RangeError` from `new Array(length)` with a negative length.
    pub fn invalid_array_length() -> Self {
        Self::new("Invalid array length")
    }

    /// V8's `RangeError` when recursion exhausts the stack.
    pub fn stack_overflow() -> Self {
        Self::new("Maximum call stack size exceeded")
    }
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsError {}

/// The shortest decimal digits that round-trip `value` (finite, non-zero,
/// sign ignored) and the decimal exponent `n` of ECMA-262
/// Number::toString: value = 0.d1d2…dk × 10^n.
pub(crate) fn shortest_digits(value: f64) -> (String, i64) {
    let text = format!("{:e}", value.abs());
    let (mantissa, exponent) = text.split_once('e').expect("`{:e}` writes an exponent");
    let exponent: i64 = exponent.parse().expect("`{:e}` writes an integer exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    (digits, exponent + 1)
}

/// `String(value)` for a JavaScript number (ECMA-262 Number::toString with
/// radix 10). Rust's `{:e}` formatting yields the same shortest round-trip
/// digits JavaScript picks.
pub fn number_to_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value == 0.0 {
        return "0".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let sign = if value < 0.0 { "-" } else { "" };
    let (digits, n) = shortest_digits(value);
    let k = digits.len() as i64;
    let body = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let exponent = n - 1;
        let sign = if exponent < 0 { '-' } else { '+' };
        let mantissa = if k == 1 {
            digits.clone()
        } else {
            format!("{}.{}", &digits[..1], &digits[1..])
        };
        format!("{mantissa}e{sign}{}", exponent.abs())
    };
    format!("{sign}{body}")
}

/// `Math.round(value)`: rounds half toward positive infinity and keeps the
/// sign of zero for values in [-0.5, -0].
pub fn math_round(value: f64) -> f64 {
    if !value.is_finite() || value == 0.0 {
        return value;
    }
    let floor = value.floor();
    let rounded = if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    };
    if rounded == 0.0 && value < 0.0 {
        -0.0
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_to_string_matches_javascript() {
        let cases = [
            (0.0, "0"),
            (-0.0, "0"),
            (1.0, "1"),
            (-42.5, "-42.5"),
            (0.1 + 0.2, "0.30000000000000004"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (123456789012345680000.0, "123456789012345680000"),
            (1e-6, "0.000001"),
            (1e-7, "1e-7"),
            (1.5e-7, "1.5e-7"),
            (5e-324, "5e-324"),
            (1.7976931348623157e308, "1.7976931348623157e+308"),
            (f64::INFINITY, "Infinity"),
            (f64::NAN, "NaN"),
        ];
        for (value, text) in cases {
            assert_eq!(number_to_string(value), text, "{value:?}");
        }
    }

    #[test]
    fn math_round_matches_javascript() {
        assert_eq!(math_round(2.5), 3.0);
        assert_eq!(math_round(-2.5), -2.0);
        assert_eq!(math_round(0.49999999999999994), 0.0);
        assert!(math_round(-0.4).is_sign_negative());
        assert_eq!(math_round(-0.6), -1.0);
    }
}
