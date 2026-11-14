//! Port of d3-interpolate 3.0.2's number interpolators
//! (`src/number.js`, `src/round.js`). `d3.interpolate` (`src/value.js`) picks
//! `interpolateNumber` when the end value is a number, which every range
//! Recharts builds is.

use js::math_round;

/// Which interpolator a continuous scale maps its range with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolator {
    /// `d3.interpolateNumber` (and `d3.interpolate` for numbers).
    Number,
    /// `d3.interpolateRound`, set by `scale.rangeRound`.
    Round,
}

impl Interpolator {
    /// `interpolator(a, b)(t)`.
    pub fn at(self, a: f64, b: f64, t: f64) -> f64 {
        let value = a * (1.0 - t) + b * t;
        match self {
            Interpolator::Number => value,
            Interpolator::Round => math_round(value),
        }
    }
}
