//! `d3.scaleLinear`: `src/continuous.js` (with the identity transform) and
//! `src/linear.js` of d3-scale 4.0.3.
//!
//! Domain and range values are numbers; a JavaScript caller's `+x` and the
//! `x == null` test of `scale(x)` are the caller's (Recharts' port does them
//! on its `Value`s). Missing domain entries read as `undefined`, which the
//! scale turns into NaN, as JavaScript does.

use js::{math, JsError};

use crate::array::{bisect_right, tick_increment, ticks};
use crate::interpolate::Interpolator;

#[derive(Debug, Clone, PartialEq)]
pub struct Linear {
    domain: Vec<f64>,
    range: Vec<f64>,
    interpolator: Interpolator,
    clamp: bool,
}

impl Default for Linear {
    fn default() -> Self {
        Self::new()
    }
}

/// `normalize(a, b)(x)`: x's position between a and b, or a constant when
/// a and b coincide (0.5) or are not numbers (NaN).
fn normalize(a: f64, b: f64, x: f64) -> f64 {
    let span = b - a;
    if span != 0.0 && !span.is_nan() {
        (x - a) / span
    } else if span.is_nan() {
        f64::NAN
    } else {
        0.5
    }
}

/// `interpolate(a, b)(t)` where a missing `b` is `undefined`, for which
/// `d3.interpolate` returns `undefined` and `interpolateRound` NaN.
fn interpolate(interpolator: Interpolator, a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    match (interpolator, b) {
        (Interpolator::Number, None) => None,
        _ => Some(interpolator.at(a.unwrap_or(f64::NAN), b.unwrap_or(f64::NAN), t)),
    }
}

/// `bimap` or `polymap` of `continuous.js`, applied to `x`.
fn piecewise(domain: &[f64], range: &[f64], interpolator: Interpolator, x: f64) -> Option<f64> {
    let n = domain.len().min(range.len());
    if n > 2 {
        let j = n - 1;
        let (domain, range): (Vec<f64>, Vec<f64>) = if domain[j] < domain[0] {
            (
                domain.iter().rev().copied().collect(),
                range.iter().rev().copied().collect(),
            )
        } else {
            (domain.to_vec(), range.to_vec())
        };
        let i = bisect_right(&domain, x, 1, j) - 1;
        let t = normalize(domain[i], domain[i + 1], x);
        return interpolate(interpolator, Some(range[i]), Some(range[i + 1]), t);
    }
    let d = |i: usize| domain.get(i).copied().unwrap_or(f64::NAN);
    let r = |i: usize| range.get(i).copied();
    let (d0, d1) = (d(0), d(1));
    if d1 < d0 {
        interpolate(interpolator, r(1), r(0), normalize(d1, d0, x))
    } else {
        interpolate(interpolator, r(0), r(1), normalize(d0, d1, x))
    }
}

impl Linear {
    /// `d3.scaleLinear()`: domain and range [0, 1].
    pub fn new() -> Self {
        Self {
            domain: vec![0.0, 1.0],
            range: vec![0.0, 1.0],
            interpolator: Interpolator::Number,
            clamp: false,
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

    /// `scale.rangeRound(range)`: the range, interpolated with
    /// `interpolateRound`.
    pub fn set_range_round(&mut self, range: Vec<f64>) -> &mut Self {
        self.range = range;
        self.interpolator = Interpolator::Round;
        self
    }

    pub fn set_clamp(&mut self, clamp: bool) -> &mut Self {
        self.clamp = clamp;
        self
    }

    /// `clamper(domain[0], domain[n - 1])(x)` when clamping is on.
    fn clamped(&self, x: f64) -> f64 {
        if !self.clamp {
            return x;
        }
        let n = self.domain.len().min(self.range.len());
        let a = self.domain.first().copied().unwrap_or(f64::NAN);
        let b = n
            .checked_sub(1)
            .and_then(|i| self.domain.get(i))
            .copied()
            .unwrap_or(f64::NAN);
        let (a, b) = if a > b { (b, a) } else { (a, b) };
        math::max(a, math::min(b, x))
    }

    /// `scale(x)` for a number `x`: `None` where d3 returns the scale's
    /// unknown value (`undefined`), that is for NaN.
    pub fn apply(&self, x: f64) -> Option<f64> {
        if x.is_nan() {
            return None;
        }
        piecewise(
            &self.domain,
            &self.range,
            self.interpolator,
            self.clamped(x),
        )
    }

    /// `scale.invert(y)`.
    pub fn invert(&self, y: f64) -> f64 {
        let x = piecewise(&self.range, &self.domain, Interpolator::Number, y)
            .expect("interpolateNumber always returns a number");
        self.clamped(x)
    }

    /// `scale.ticks(count)`.
    pub fn ticks(&self, count: f64) -> Result<Vec<f64>, JsError> {
        let first = self.domain.first().copied().unwrap_or(f64::NAN);
        let last = self.domain.last().copied().unwrap_or(f64::NAN);
        ticks(first, last, count)
    }

    /// `scale.nice(count)`: extends the domain to round values.
    pub fn nice(&mut self, count: f64) -> &mut Self {
        if self.domain.is_empty() {
            return self;
        }
        let mut d = self.domain.clone();
        let (mut i0, mut i1) = (0, d.len() - 1);
        let (mut start, mut stop) = (d[i0], d[i1]);
        if stop < start {
            std::mem::swap(&mut start, &mut stop);
            std::mem::swap(&mut i0, &mut i1);
        }
        let mut prestep: Option<f64> = None;
        for _ in 0..10 {
            let step = tick_increment(start, stop, count);
            if Some(step) == prestep {
                d[i0] = start;
                d[i1] = stop;
                self.domain = d;
                return self;
            } else if step > 0.0 {
                start = (start / step).floor() * step;
                stop = (stop / step).ceil() * step;
            } else if step < 0.0 {
                start = (start * step).ceil() / step;
                stop = (stop * step).floor() / step;
            } else {
                break;
            }
            prestep = Some(step);
        }
        self
    }
}
