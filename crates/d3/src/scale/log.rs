//! `d3.scaleLog`: `src/log.js` of d3-scale 4.0.3 on `src/continuous.js`,
//! with base 10 (the default, which Recharts' `scale="log"` keeps).
//!
//! A domain starting below zero is reflected (`-log(-x)`), as d3 does on
//! every `domain(…)` call. `invert`, `nice`, `tickFormat` and other bases
//! are not ported (Recharts' axes call none of them; `invert` would need
//! `Math.exp`).

use js::{math, number_to_string, string_to_number, JsError};

use super::linear::piecewise;
use crate::array::ticks;
use crate::interpolate::Interpolator;

#[derive(Debug, Clone, PartialEq)]
pub struct Log {
    domain: Vec<f64>,
    range: Vec<f64>,
}

impl Default for Log {
    fn default() -> Self {
        Self::new()
    }
}

/// `pow10(x)`: `+("1e" + x)` for finite x (NaN unless x prints as an
/// integer), 0 for -Infinity, else x.
fn pow10(x: f64) -> f64 {
    if x.is_finite() {
        string_to_number(&format!("1e{}", number_to_string(x)))
    } else if x < 0.0 {
        0.0
    } else {
        x
    }
}

impl Log {
    /// `d3.scaleLog()`: domain [1, 10], range [0, 1].
    pub fn new() -> Self {
        Self {
            domain: vec![1.0, 10.0],
            range: vec![0.0, 1.0],
        }
    }

    pub fn domain(&self) -> &[f64] {
        &self.domain
    }

    /// `scale.domain(values)`, with `values` already converted by `+`.
    pub fn set_domain(&mut self, domain: Vec<f64>) -> &mut Self {
        self.domain = domain;
        self
    }

    pub fn range(&self) -> &[f64] {
        &self.range
    }

    pub fn set_range(&mut self, range: Vec<f64>) -> &mut Self {
        self.range = range;
        self
    }

    /// Whether `rescale` chose the reflected transforms: `domain()[0] < 0`.
    fn reflected(&self) -> bool {
        self.domain.first().is_some_and(|&d| d < 0.0)
    }

    /// The transform: `Math.log`, or `-Math.log(-x)` when reflected.
    fn transform(&self, x: f64) -> f64 {
        if self.reflected() {
            -math::log(-x)
        } else {
            math::log(x)
        }
    }

    /// `logs`: `Math.log10`, reflected with the domain.
    fn logs(&self, x: f64) -> f64 {
        if self.reflected() {
            -math::log10(-x)
        } else {
            math::log10(x)
        }
    }

    /// `pows`: `pow10`, reflected with the domain.
    fn pows(&self, x: f64) -> f64 {
        if self.reflected() {
            -pow10(-x)
        } else {
            pow10(x)
        }
    }

    /// `scale(x)` for a number `x`: `None` (`undefined`) for NaN.
    pub fn apply(&self, x: f64) -> Option<f64> {
        if x.is_nan() {
            return None;
        }
        let domain: Vec<f64> = self.domain.iter().map(|&d| self.transform(d)).collect();
        piecewise(
            &domain,
            &self.range,
            Interpolator::Number,
            self.transform(x),
        )
    }

    /// `scale.ticks(count)`.
    pub fn ticks(&self, count: f64) -> Result<Vec<f64>, JsError> {
        let first = self.domain.first().copied().unwrap_or(f64::NAN);
        let last = self.domain.last().copied().unwrap_or(f64::NAN);
        let (mut u, mut v) = (first, last);
        let reversed = v < u;
        if reversed {
            std::mem::swap(&mut u, &mut v);
        }
        let mut i = self.logs(u);
        let mut j = self.logs(v);
        let n = count;
        let mut z = Vec::new();
        // `!(base % 1)` holds for base 10.
        if j - i < n {
            i = i.floor();
            j = j.ceil();
            if u > 0.0 {
                while i <= j {
                    for k in 1..10 {
                        let k = f64::from(k);
                        let t = if i < 0.0 {
                            k / self.pows(-i)
                        } else {
                            k * self.pows(i)
                        };
                        if t < u {
                            continue;
                        }
                        if t > v {
                            break;
                        }
                        z.push(t);
                    }
                    i += 1.0;
                }
            } else {
                while i <= j {
                    for k in (1..10).rev() {
                        let k = f64::from(k);
                        let t = if i > 0.0 {
                            k / self.pows(-i)
                        } else {
                            k * self.pows(i)
                        };
                        if t < u {
                            continue;
                        }
                        if t > v {
                            break;
                        }
                        z.push(t);
                    }
                    i += 1.0;
                }
            }
            if (z.len() as f64) * 2.0 < n {
                z = ticks(u, v, n)?;
            }
        } else {
            z = ticks(i, j, math::min(j - i, n))?
                .into_iter()
                .map(|t| self.pows(t))
                .collect();
        }
        if reversed {
            z.reverse();
        }
        Ok(z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pow10_parses_like_javascript() {
        assert_eq!(pow10(3.0), 1000.0);
        assert_eq!(pow10(-3.0), 0.001);
        assert_eq!(pow10(-0.0), 1.0);
        assert!(pow10(0.5).is_nan());
        assert_eq!(pow10(f64::NEG_INFINITY), 0.0);
        assert_eq!(pow10(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn ticks_of_a_decade_span() {
        let mut scale = Log::new();
        scale.set_domain(vec![1.0, 100.0]);
        assert_eq!(
            scale.ticks(10.0).unwrap(),
            [
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0,
                70.0, 80.0, 90.0, 100.0
            ]
        );
        scale.set_domain(vec![-100.0, -1.0]);
        assert_eq!(scale.apply(-10.0), Some(0.5));
    }
}
