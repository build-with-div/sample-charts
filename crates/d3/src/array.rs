//! Port of d3-array 3.2.5: `ticks`, `tickIncrement`, `tickStep`
//! (`src/ticks.js`), `range` (`src/range.js`) and `bisectRight`
//! (`src/bisect.js` with the `ascending` comparator).

use js::{math, math_round, to_int32, JsError};

/// The largest length `new Array(n)` accepts.
const MAX_ARRAY_LENGTH: f64 = 4_294_967_295.0;

fn e10() -> f64 {
    50f64.sqrt()
}

fn e5() -> f64 {
    10f64.sqrt()
}

fn e2() -> f64 {
    2f64.sqrt()
}

/// `tickSpec(start, stop, count)`: the first and last tick index and the
/// increment, negative when it is the reciprocal of a power of ten.
fn tick_spec(start: f64, stop: f64, count: f64) -> (f64, f64, f64) {
    let step = (stop - start) / math::max(0.0, count);
    let power = math::log10(step).floor();
    let error = step / math::pow(10.0, power);
    let factor = if error >= e10() {
        10.0
    } else if error >= e5() {
        5.0
    } else if error >= e2() {
        2.0
    } else {
        1.0
    };
    let (mut i1, mut i2, mut inc);
    if power < 0.0 {
        inc = math::pow(10.0, -power) / factor;
        i1 = math_round(start * inc);
        i2 = math_round(stop * inc);
        if i1 / inc < start {
            i1 += 1.0;
        }
        if i2 / inc > stop {
            i2 -= 1.0;
        }
        inc = -inc;
    } else {
        inc = math::pow(10.0, power) * factor;
        i1 = math_round(start / inc);
        i2 = math_round(stop / inc);
        if i1 * inc < start {
            i1 += 1.0;
        }
        if i2 * inc > stop {
            i2 -= 1.0;
        }
    }
    if i2 < i1 && (0.5..2.0).contains(&count) {
        return tick_spec(start, stop, count * 2.0);
    }
    (i1, i2, inc)
}

/// `d3.ticks(start, stop, count)`: about `count` round values between
/// `start` and `stop`, inclusive, in the order of the arguments. Fails as
/// `new Array(n)` does when there would be more than 2^32 - 1.
pub fn ticks(start: f64, stop: f64, count: f64) -> Result<Vec<f64>, JsError> {
    if !(count > 0.0) {
        return Ok(Vec::new());
    }
    if start == stop {
        return Ok(vec![start]);
    }
    let reverse = stop < start;
    let (i1, i2, inc) = if reverse {
        tick_spec(stop, start, count)
    } else {
        tick_spec(start, stop, count)
    };
    if !(i2 >= i1) {
        return Ok(Vec::new());
    }
    let n = i2 - i1 + 1.0;
    // `new Array(n)` throws unless ToUint32(n) == n; n is NaN when both
    // indices are infinite.
    if !(n <= MAX_ARRAY_LENGTH && n.fract() == 0.0) {
        return Err(JsError::invalid_array_length());
    }
    Ok((0..n as usize)
        .map(|i| {
            let i = i as f64;
            match (reverse, inc < 0.0) {
                (true, true) => (i2 - i) / -inc,
                (true, false) => (i2 - i) * inc,
                (false, true) => (i1 + i) / -inc,
                (false, false) => (i1 + i) * inc,
            }
        })
        .collect())
}

/// `d3.tickIncrement(start, stop, count)`; `start` must not exceed `stop`.
pub fn tick_increment(start: f64, stop: f64, count: f64) -> f64 {
    tick_spec(start, stop, count).2
}

/// `d3.tickStep(start, stop, count)`.
pub fn tick_step(start: f64, stop: f64, count: f64) -> f64 {
    let reverse = stop < start;
    let inc = if reverse {
        tick_increment(stop, start, count)
    } else {
        tick_increment(start, stop, count)
    };
    (if reverse { -1.0 } else { 1.0 }) * (if inc < 0.0 { 1.0 / -inc } else { inc })
}

/// `d3.range(start, stop, step)`. Fails as `new Array(n)` does when the
/// count wraps negative in `| 0`.
pub fn range(start: f64, stop: f64, step: f64) -> Result<Vec<f64>, JsError> {
    let n = to_int32(math::max(0.0, ((stop - start) / step).ceil()));
    if n < 0 {
        return Err(JsError::invalid_array_length());
    }
    Ok((0..n as usize).map(|i| start + i as f64 * step).collect())
}

/// `d3.ascending(a, b)` for numbers.
fn ascending(a: f64, b: f64) -> f64 {
    if a < b {
        -1.0
    } else if a > b {
        1.0
    } else if a >= b {
        0.0
    } else {
        f64::NAN
    }
}

/// `d3.bisectRight(values, x, lo, hi)` for ascending numbers.
pub fn bisect_right(values: &[f64], x: f64, lo: usize, hi: usize) -> usize {
    let (mut lo, mut hi) = (lo, hi);
    if lo < hi {
        if ascending(x, x) != 0.0 {
            return hi;
        }
        while lo < hi {
            let mid = (lo + hi) / 2;
            if ascending(values[mid], x) <= 0.0 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
    }
    lo
}
