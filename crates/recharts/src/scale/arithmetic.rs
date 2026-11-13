//! Port of `recharts/lib/util/scale/util/arithmetic.js` (Recharts 3.8.0).

use crate::decimal::Decimal;
use crate::JsError;
use std::f64::consts::LN_10;

/// `Math.floor(new Decimal(value).abs().log(10).toNumber())` for a finite,
/// non-zero `value`.
///
/// decimal.js-light computes the logarithm to 20 significant digits and
/// `toNumber()` rounds that to the nearest double, so for values just below
/// a power of ten, 10^k, the result rounds up to `k` and the floor is `k`,
/// not `k - 1`: `getDigitCount(999.9999999999999)` is 4. The port finds the
/// leading decimal exponent exactly, then decides whether the logarithm,
/// `k - gap`, is within half a unit in the last place of `k`, where the
/// conversion lands on `k`.
fn floor_log10(value: f64) -> Result<i64, JsError> {
    let decimal = Decimal::from_f64(value)?;
    if decimal.is_zero() {
        return Err(JsError::new("[DecimalError] -Infinity"));
    }
    let exponent = decimal.leading_exponent();
    let k = exponent + 1;
    if k == 0 {
        // log10 is in [-1, 0); doubles near 0 are fine enough to keep it
        // negative.
        return Ok(exponent);
    }
    // |value| = 0.c × 10^k with c the coefficient's n digits, so
    // log10 |value| = k + log10(c / 10^n) = k - gap.
    let digits = decimal.digits().len() as i32;
    let coefficient = decimal
        .coefficient_u64()
        .expect("a double has at most 17 significant digits") as f64;
    let shortfall = 1.0 - coefficient / 10f64.powi(digits);
    let gap = -(-shortfall).ln_1p() / LN_10;
    let half_ulp = if k > 0 {
        let at = k as f64;
        (at - f64::from_bits(at.to_bits() - 1)) / 2.0
    } else {
        let at = (-k) as f64;
        (f64::from_bits(at.to_bits() + 1) - at) / 2.0
    };
    Ok(if gap <= half_ulp { k } else { exponent })
}

/// `getDigitCount(value)`: `floor(log10(|value|)) + 1`, and 1 for 0.
pub fn get_digit_count(value: f64) -> Result<i64, JsError> {
    if value == 0.0 {
        return Ok(1);
    }
    Ok(floor_log10(value)? + 1)
}

/// The exponent `getSnap125Step` computes inline, with the same expression.
pub(crate) fn log10_floor(value: f64) -> Result<i64, JsError> {
    floor_log10(value)
}

/// `rangeStep(start, end, step)`: values from `start` while below `end`,
/// at most 100,000 of them.
pub fn range_step(start: &Decimal, end: &Decimal, step: &Decimal) -> Vec<f64> {
    let mut num = start.clone();
    let mut result = Vec::new();
    let mut i = 0;
    while num.lt(end) && i < 100_000 {
        result.push(num.to_f64());
        num = num.add(step);
        i += 1;
    }
    result
}
