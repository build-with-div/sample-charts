//! Numbers: `String(n)`, `+string`, `parseFloat`, `Math.round`, `x | 0`.

/// The shortest decimal digits that round-trip `value` (finite, non-zero,
/// sign ignored) and the decimal exponent `n` of ECMA-262
/// Number::toString: value = 0.d1d2…dk × 10^n.
pub fn shortest_digits(value: f64) -> (String, i64) {
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

/// ToInt32 (ECMA-262 7.1.6), as in `x | 0`.
pub fn to_int32(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    let int = value.trunc();
    let modulo = int.rem_euclid(4_294_967_296.0);
    if modulo >= 2_147_483_648.0 {
        (modulo - 4_294_967_296.0) as i32
    } else {
        modulo as i32
    }
}

/// StrWhiteSpaceChar (ECMA-262 7.1.4.1): WhiteSpace and LineTerminator.
fn is_str_white_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{B}' | '\u{C}' | '\u{FEFF}' | '\u{20}' | '\u{A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{A}'
                | '\u{D}'
                | '\u{2028}'
                | '\u{2029}'
    )
}

/// The length in bytes of the longest prefix of `text` that is a
/// StrUnsignedDecimalLiteral without "Infinity": `digits [. digits] [exp]` or
/// `. digits [exp]`.
fn unsigned_decimal_prefix(text: &str) -> usize {
    let bytes = text.as_bytes();
    let digits = |from: usize| {
        bytes[from..]
            .iter()
            .take_while(|b| b.is_ascii_digit())
            .count()
    };
    let int = digits(0);
    let mut end = int;
    if bytes.get(end) == Some(&b'.') {
        let fraction = digits(end + 1);
        if int == 0 && fraction == 0 {
            return 0;
        }
        end += 1 + fraction;
    } else if int == 0 {
        return 0;
    }
    if matches!(bytes.get(end), Some(b'e' | b'E')) {
        let mut at = end + 1;
        if matches!(bytes.get(at), Some(b'+' | b'-')) {
            at += 1;
        }
        let exponent = digits(at);
        if exponent > 0 {
            end = at + exponent;
        }
    }
    end
}

/// Parses `sign? (Infinity | unsigned decimal)` at the start of `text`;
/// returns the value and the bytes consumed, or `None`.
fn decimal_prefix(text: &str) -> Option<(f64, usize)> {
    let (negative, body) = match text.as_bytes().first() {
        Some(b'+') => (false, &text[1..]),
        Some(b'-') => (true, &text[1..]),
        _ => (false, text),
    };
    let sign_len = text.len() - body.len();
    let magnitude = if body.starts_with("Infinity") {
        (f64::INFINITY, "Infinity".len())
    } else {
        let len = unsigned_decimal_prefix(body);
        if len == 0 {
            return None;
        }
        // The prefix is plain ASCII decimal syntax, which Rust parses with
        // correct rounding, as V8 does.
        let value: f64 = body[..len].parse().expect("validated decimal literal");
        (value, len)
    };
    let value = if negative { -magnitude.0 } else { magnitude.0 };
    Some((value, sign_len + magnitude.1))
}

/// The value of the digits of a NonDecimalIntegerLiteral, rounded to the
/// nearest double with ties to even.
fn radix_value(digits: &str, radix: u32) -> Option<f64> {
    if digits.is_empty() {
        return None;
    }
    let bits_per_digit = radix.trailing_zeros();
    let mut bits: Vec<bool> = Vec::new();
    for c in digits.chars() {
        let d = c.to_digit(radix)?;
        for shift in (0..bits_per_digit).rev() {
            bits.push((d >> shift) & 1 == 1);
        }
    }
    let first = match bits.iter().position(|b| *b) {
        Some(first) => first,
        None => return Some(0.0),
    };
    let bits = &bits[first..];
    if bits.len() <= 53 {
        return Some(
            bits.iter()
                .fold(0.0, |acc, b| acc * 2.0 + f64::from(*b as u8)),
        );
    }
    let mut mantissa: u64 = bits[..53].iter().fold(0, |acc, b| acc * 2 + *b as u64);
    let half = bits[53];
    let sticky = bits[54..].iter().any(|b| *b);
    if half && (sticky || mantissa & 1 == 1) {
        mantissa += 1;
    }
    let exponent = (bits.len() - 53) as i32;
    Some(mantissa as f64 * 2f64.powi(exponent))
}

/// StringToNumber (ECMA-262 7.1.4.1.1), as in `+string` or `Number(string)`:
/// surrounding white space is ignored, the empty string is 0, and anything
/// that is not entirely a numeric literal is NaN.
pub fn string_to_number(text: &str) -> f64 {
    let trimmed = text.trim_matches(is_str_white_space);
    if trimmed.is_empty() {
        return 0.0;
    }
    let lower = trimmed.get(..2).map(str::to_ascii_lowercase);
    let radix = match lower.as_deref() {
        Some("0x") => Some(16),
        Some("0o") => Some(8),
        Some("0b") => Some(2),
        _ => None,
    };
    if let Some(radix) = radix {
        return radix_value(&trimmed[2..], radix).unwrap_or(f64::NAN);
    }
    match decimal_prefix(trimmed) {
        Some((value, len)) if len == trimmed.len() => value,
        _ => f64::NAN,
    }
}

/// The global `parseFloat(string)` (ECMA-262 19.2.4): the longest prefix,
/// after leading white space, that is a decimal literal; NaN if none.
pub fn parse_float(text: &str) -> f64 {
    let trimmed = text.trim_start_matches(is_str_white_space);
    decimal_prefix(trimmed).map_or(f64::NAN, |(value, _)| value)
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

    #[test]
    fn string_to_number_matches_javascript() {
        let same = |text: &str, value: f64| {
            let got = string_to_number(text);
            assert!(
                got.to_bits() == value.to_bits() || (got.is_nan() && value.is_nan()),
                "+{text:?} = {got:?}, want {value:?}"
            );
        };
        same("", 0.0);
        same(" \n\t ", 0.0);
        same(" 12 ", 12.0);
        same("\u{A0}1.5\u{FEFF}", 1.5);
        same("-0", -0.0);
        same("+.5", 0.5);
        same("5.", 5.0);
        same("1e3", 1000.0);
        same("1E-3", 0.001);
        same("1e", f64::NAN);
        same(".", f64::NAN);
        same("-Infinity", f64::NEG_INFINITY);
        same("infinity", f64::NAN);
        same("inf", f64::NAN);
        same("NaN", f64::NAN);
        same("0x1F", 31.0);
        same("0b101", 5.0);
        same("0o17", 15.0);
        same("-0x1", f64::NAN);
        same("0x", f64::NAN);
        same("1_000", f64::NAN);
        same("12px", f64::NAN);
        same("\u{85}1", f64::NAN);
        same("0x20000000000001", 9007199254740992.0);
        same("0x20000000000003", 9007199254740996.0);
    }

    #[test]
    fn parse_float_matches_javascript() {
        assert_eq!(parse_float("10%"), 10.0);
        assert_eq!(parse_float("  2.5e1x"), 25.0);
        assert_eq!(parse_float("-Infinityx"), f64::NEG_INFINITY);
        assert_eq!(parse_float("1e"), 1.0);
        assert!(parse_float("x1").is_nan());
        assert!(parse_float("").is_nan());
        assert_eq!(parse_float("0x10"), 0.0);
    }

    #[test]
    fn to_int32_matches_javascript() {
        assert_eq!(to_int32(3.9), 3);
        assert_eq!(to_int32(-3.9), -3);
        assert_eq!(to_int32(4294967296.0 + 5.0), 5);
        assert_eq!(to_int32(2147483648.0), -2147483648);
        assert_eq!(to_int32(f64::NAN), 0);
    }
}
