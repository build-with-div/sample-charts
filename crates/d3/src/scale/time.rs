//! `d3.scaleTime`: `src/time.js` (`calendar` with d3-time's local
//! intervals) of d3-scale 4.0.3 on `src/continuous.js`.
//!
//! The domain is kept as numbers: `scale.domain(values)` stores each Date's
//! time value and `Math.round(+x)` of anything else, and `scale.domain()`
//! returns them as Dates (`new Date(t)`, so TimeClip'd), here as time
//! values. `invert`, `nice`, `tickFormat` and ticks at a given interval are
//! not ported (Recharts' axes call none of them).

use js::{time_clip, Value};

use super::linear::piecewise;
use crate::interpolate::Interpolator;
use crate::time::time_ticks;

#[derive(Debug, Clone, PartialEq)]
pub struct Time {
    domain: Vec<f64>,
    range: Vec<f64>,
}

impl Default for Time {
    fn default() -> Self {
        Self::new()
    }
}

/// `number(t)` of `time.js`: `t instanceof Date ? +t : Math.round(t)`.
pub fn domain_number(value: &Value) -> f64 {
    match value {
        Value::Date(date) => date.time(),
        other => js::math_round(other.to_number()),
    }
}

impl Time {
    /// `d3.scaleTime()`: domain [new Date(2000, 0, 1), new Date(2000, 0,
    /// 2)] in local time (UTC), range [0, 1].
    pub fn new() -> Self {
        Self {
            domain: vec![946_684_800_000.0, 946_771_200_000.0],
            range: vec![0.0, 1.0],
        }
    }

    /// `scale.domain()`: the Dates' time values.
    pub fn domain(&self) -> Vec<f64> {
        self.domain.iter().map(|&t| time_clip(t)).collect()
    }

    /// `scale.domain(values)`, with `values` already converted by
    /// [`domain_number`].
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

    /// `scale(x)` for `+x`: `None` (`undefined`) for NaN.
    pub fn apply(&self, x: f64) -> Option<f64> {
        if x.is_nan() {
            return None;
        }
        piecewise(&self.domain, &self.range, Interpolator::Number, x)
    }

    /// `scale.ticks(count)`: time values of the tick Dates. `calendar`
    /// reads the continuous scale's own domain here, the numbers before
    /// `new Date`, so a value beyond TimeClip's range still ticks.
    pub fn ticks(&self, count: f64) -> Vec<f64> {
        let d = &self.domain;
        let first = d.first().copied().unwrap_or(f64::NAN);
        let last = d.last().copied().unwrap_or(f64::NAN);
        time_ticks(first, last, count)
    }
}
