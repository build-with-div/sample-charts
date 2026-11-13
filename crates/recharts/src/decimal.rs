//! The part of `decimal.js-light` 2.5.1 (`decimal.js`) that Recharts' tick
//! code uses, with its results.
//!
//! Recharts uses the library's default configuration: `precision` 20. Every
//! arithmetic method (`plus`, `minus`, `times`, `div`, `mod`) returns the
//! exact result truncated toward zero to 20 significant digits; the library
//! applies its `rounding` setting only in rounding methods, which Recharts'
//! tick code does not call. `new Decimal(number)` reads the number's
//! `String()` digits, and `toNumber()` parses the decimal's digits back.
//!
//! The port computes exact results with big integers and then truncates,
//! which gives the library's digits: its long division produces at least
//! 22 correct leading digits before truncating, and its addition only
//! shortens an operand at digits below the 20 that are kept.

use crate::js::{shortest_digits, JsError};
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive, Zero};
use std::cmp::Ordering;

/// `Decimal.precision`: significant digits kept by arithmetic.
pub const PRECISION: usize = 20;

/// A decimal number: `(-1)^negative × coefficient × 10^exponent`. Zero is
/// unsigned with exponent 0; otherwise the coefficient has no trailing
/// zeros, so equal values have equal representations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal {
    negative: bool,
    coefficient: BigUint,
    exponent: i64,
}

fn pow10(n: u64) -> BigUint {
    BigUint::from(10u32).pow(u32::try_from(n).expect("decimal exponent difference fits u32"))
}

impl Decimal {
    pub fn zero() -> Self {
        Self {
            negative: false,
            coefficient: BigUint::zero(),
            exponent: 0,
        }
    }

    /// Normalize: strip trailing zeros and keep at most `limit` significant
    /// digits, truncating toward zero.
    fn from_parts(negative: bool, coefficient: BigUint, exponent: i64, limit: usize) -> Self {
        if coefficient.is_zero() {
            return Self::zero();
        }
        let digits = coefficient.to_string();
        let keep = digits.len().min(limit);
        let kept = digits[..keep].trim_end_matches('0');
        let exponent = exponent + (digits.len() - kept.len()) as i64;
        Self {
            negative,
            coefficient: kept.parse().expect("decimal digits"),
            exponent,
        }
    }

    /// An exact result, truncated to `PRECISION` digits.
    fn rounded(negative: bool, coefficient: BigUint, exponent: i64) -> Self {
        Self::from_parts(negative, coefficient, exponent, PRECISION)
    }

    /// `new Decimal(value)` for a JavaScript number.
    pub fn from_f64(value: f64) -> Result<Self, JsError> {
        if !value.is_finite() {
            return Err(JsError::decimal_invalid_argument(value));
        }
        if value == 0.0 {
            return Ok(Self::zero());
        }
        let (digits, n) = shortest_digits(value);
        let exponent = n - digits.len() as i64;
        Ok(Self::from_parts(
            value < 0.0,
            digits.parse().expect("decimal digits"),
            exponent,
            usize::MAX,
        ))
    }

    /// `new Decimal(10).pow(n)` for an integer `n`, which the library
    /// computes exactly.
    pub fn power_of_ten(n: i64) -> Self {
        Self {
            negative: false,
            coefficient: BigUint::one(),
            exponent: n,
        }
    }

    /// `toNumber()`: the nearest number to the decimal's value, as
    /// JavaScript's `Number(string)` gives (overflow to ±Infinity,
    /// underflow to ±0).
    pub fn to_f64(&self) -> f64 {
        if self.is_zero() {
            return 0.0;
        }
        let sign = if self.negative { "-" } else { "" };
        format!("{sign}{}e{}", self.coefficient, self.exponent)
            .parse()
            .expect("decimal text parses as f64")
    }

    pub fn is_zero(&self) -> bool {
        self.coefficient.is_zero()
    }

    /// `isint()`.
    pub fn is_integer(&self) -> bool {
        self.is_zero() || self.exponent >= 0
    }

    /// `abs()`.
    pub fn abs(&self) -> Self {
        Self {
            negative: false,
            ..self.clone()
        }
    }

    pub fn neg(&self) -> Self {
        Self {
            negative: !self.negative && !self.is_zero(),
            ..self.clone()
        }
    }

    /// The decimal exponent of the leading digit, `floor(log10(|x|))`, for
    /// a non-zero value.
    pub fn leading_exponent(&self) -> i64 {
        self.coefficient.to_string().len() as i64 - 1 + self.exponent
    }

    /// The coefficient's digits.
    pub fn digits(&self) -> String {
        self.coefficient.to_string()
    }

    /// Both magnitudes as integers over the smaller exponent.
    fn aligned(&self, other: &Self) -> (BigUint, BigUint, i64) {
        let exponent = self.exponent.min(other.exponent);
        let a = &self.coefficient * pow10((self.exponent - exponent) as u64);
        let b = &other.coefficient * pow10((other.exponent - exponent) as u64);
        (a, b, exponent)
    }

    /// `plus(y)`.
    pub fn add(&self, other: &Self) -> Self {
        if other.is_zero() {
            return self.clone();
        }
        if self.is_zero() {
            return other.clone();
        }
        let (a, b, exponent) = self.aligned(other);
        if self.negative == other.negative {
            return Self::rounded(self.negative, a + b, exponent);
        }
        match a.cmp(&b) {
            Ordering::Equal => Self::zero(),
            Ordering::Greater => Self::rounded(self.negative, a - b, exponent),
            Ordering::Less => Self::rounded(other.negative, b - a, exponent),
        }
    }

    /// `minus(y)`.
    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    /// `times(y)`.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::zero();
        }
        Self::rounded(
            self.negative != other.negative,
            &self.coefficient * &other.coefficient,
            self.exponent + other.exponent,
        )
    }

    /// `div(y)`.
    pub fn div(&self, other: &Self) -> Result<Self, JsError> {
        if other.is_zero() {
            return Err(JsError::new("[DecimalError] Division by zero"));
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        // Scale the dividend so the integer quotient has more than
        // PRECISION digits; its leading digits are then the true quotient's.
        let a_digits = self.coefficient.to_string().len() as i64;
        let b_digits = other.coefficient.to_string().len() as i64;
        let scale = (PRECISION as i64 + 2 + b_digits - a_digits).max(0);
        let quotient = (&self.coefficient * pow10(scale as u64)) / &other.coefficient;
        Ok(Self::rounded(
            self.negative != other.negative,
            quotient,
            self.exponent - other.exponent - scale,
        ))
    }

    /// `mod(y)`: `x - trunc(x / y) × y`, with the sign of `x`.
    pub fn rem(&self, other: &Self) -> Result<Self, JsError> {
        if other.is_zero() {
            return Err(JsError::new("[DecimalError] NaN"));
        }
        if self.is_zero() {
            return Ok(Self::zero());
        }
        let (a, b, exponent) = self.aligned(other);
        let remainder = &a - (&a / &b) * &b;
        Ok(Self::rounded(self.negative, remainder, exponent))
    }

    /// `cmp(y)`.
    pub fn compare(&self, other: &Self) -> Ordering {
        let sign = |d: &Self| {
            if d.is_zero() {
                0
            } else if d.negative {
                -1
            } else {
                1
            }
        };
        let (s, o) = (sign(self), sign(other));
        if s != o || s == 0 {
            return s.cmp(&o);
        }
        let (a, b, _) = self.aligned(other);
        let magnitude = a.cmp(&b);
        if s < 0 {
            magnitude.reverse()
        } else {
            magnitude
        }
    }

    /// `lt(y)`.
    pub fn lt(&self, other: &Self) -> bool {
        self.compare(other) == Ordering::Less
    }

    /// `lte(y)`.
    pub fn lte(&self, other: &Self) -> bool {
        self.compare(other) != Ordering::Greater
    }

    /// The coefficient as a `u64`, if it fits.
    pub(crate) fn coefficient_u64(&self) -> Option<u64> {
        self.coefficient.to_u64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(value: f64) -> Decimal {
        Decimal::from_f64(value).unwrap()
    }

    #[test]
    fn reads_number_digits() {
        assert_eq!(d(0.1).digits(), "1");
        assert_eq!(d(0.1).leading_exponent(), -1);
        assert_eq!(d(0.1 + 0.2).digits(), "30000000000000004");
        assert_eq!(d(1200.0).digits(), "12");
        assert_eq!(d(-0.0), Decimal::zero());
        assert!(Decimal::from_f64(f64::NAN).is_err());
    }

    #[test]
    fn arithmetic_truncates_to_twenty_digits() {
        // 1/3 and 2/3 truncate, they do not round.
        assert_eq!(d(1.0).div(&d(3.0)).unwrap().digits(), "3".repeat(20));
        assert_eq!(d(2.0).div(&d(3.0)).unwrap().digits(), "6".repeat(20));
        assert_eq!(d(-2.0).div(&d(3.0)).unwrap().to_f64(), -2.0 / 3.0);
        assert_eq!(d(0.1).add(&d(0.2)).to_f64(), 0.3);
        assert_eq!(d(1e30).add(&d(1.0)).digits(), "1");
        assert_eq!(d(1.0).sub(&d(1e-30)).digits(), "9".repeat(20));
        assert_eq!(d(7.5).rem(&d(2.0)).unwrap().to_f64(), 1.5);
        assert_eq!(d(-7.5).rem(&d(2.0)).unwrap().to_f64(), -1.5);
        assert!(d(1.0).div(&Decimal::zero()).is_err());
    }

    #[test]
    fn compares() {
        assert!(d(-2.0).lt(&d(-1.0)));
        assert!(d(-1.0).lt(&Decimal::zero()));
        assert!(d(1e-300).lt(&d(1e300)));
        assert!(d(5.0).lte(&d(5.0)));
    }
}
