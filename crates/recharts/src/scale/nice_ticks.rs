//! Port of `recharts/lib/util/scale/getNiceTickValues.js` (Recharts 3.8.0):
//! the tick values of a numeric axis.
//!
//! The functions keep the original's names (in snake case), arguments,
//! defaults and results, including the exceptions it throws, returned as
//! [`JsError`]. Numbers are JavaScript numbers (`f64`); the arithmetic is
//! decimal.js-light's ([`Decimal`]).

use super::arithmetic::{get_digit_count, log10_floor, range_step};
use crate::decimal::Decimal;
use crate::js::math_round;
use crate::JsError;

/// `NiceTicksAlgorithm`, the axis `niceTicks` prop.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NiceTicks {
    /// `'auto'`: `getAdaptiveStep`.
    #[default]
    Auto,
    /// `'snap125'`: `getSnap125Step`.
    Snap125,
}

impl NiceTicks {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "snap125" => Some(Self::Snap125),
            _ => None,
        }
    }

    fn step_fn(self) -> StepFn {
        match self {
            Self::Auto => get_adaptive_step,
            Self::Snap125 => get_snap125_step,
        }
    }
}

type StepFn = fn(&Decimal, bool, i64) -> Result<Decimal, JsError>;

fn decimal(value: f64) -> Result<Decimal, JsError> {
    Decimal::from_f64(value)
}

/// `getValidInterval([min, max])`.
pub fn get_valid_interval(min: f64, max: f64) -> (f64, f64) {
    if min > max {
        (max, min)
    } else {
        (min, max)
    }
}

/// `getAdaptiveStep(roughStep, allowDecimals, correctionFactor)`.
pub fn get_adaptive_step(
    rough_step: &Decimal,
    allow_decimals: bool,
    correction_factor: i64,
) -> Result<Decimal, JsError> {
    if rough_step.lte(&Decimal::zero()) {
        return Ok(Decimal::zero());
    }
    let digit_count = get_digit_count(rough_step.to_f64())?;
    let digit_count_value = Decimal::power_of_ten(digit_count);
    let step_ratio = rough_step.div(&digit_count_value)?;
    let step_ratio_scale = decimal(if digit_count != 1 { 0.05 } else { 0.1 })?;
    let amend_step_ratio = decimal(step_ratio.div(&step_ratio_scale)?.to_f64().ceil())?
        .add(&decimal(correction_factor as f64)?)
        .mul(&step_ratio_scale);
    let format_step = amend_step_ratio.mul(&digit_count_value);
    if allow_decimals {
        decimal(format_step.to_f64())
    } else {
        decimal(format_step.to_f64().ceil())
    }
}

const NICE_STEPS: [f64; 4] = [1.0, 2.0, 2.5, 5.0];

/// `getSnap125Step(roughStep, allowDecimals, correctionFactor)`.
pub fn get_snap125_step(
    rough_step: &Decimal,
    allow_decimals: bool,
    correction_factor: i64,
) -> Result<Decimal, JsError> {
    if rough_step.lte(&Decimal::zero()) {
        return Ok(Decimal::zero());
    }
    let rough_num = rough_step.to_f64();
    let exponent = log10_floor(rough_num)?;
    let mut magnitude = Decimal::power_of_ten(exponent);
    let normalized = rough_step.div(&magnitude)?.to_f64();
    let mut nice_idx = match NICE_STEPS.iter().position(|&s| s >= normalized - 1e-10) {
        Some(index) => index as i64,
        None => {
            magnitude = magnitude.mul(&decimal(10.0)?);
            0
        }
    };
    nice_idx += correction_factor;
    let len = NICE_STEPS.len() as i64;
    if nice_idx >= len {
        let extra_mag = nice_idx.div_euclid(len);
        nice_idx %= len;
        magnitude = magnitude.mul(&Decimal::power_of_ten(extra_mag));
    }
    let nice_step = NICE_STEPS.get(nice_idx as usize).copied().unwrap_or(1.0);
    let format_step = decimal(nice_step)?.mul(&magnitude);
    if allow_decimals {
        Ok(format_step)
    } else {
        decimal(format_step.to_f64().ceil())
    }
}

/// `getTickOfSingleValue(value, tickCount, allowDecimals)`.
pub fn get_tick_of_single_value(
    value: f64,
    tick_count: u32,
    allow_decimals: bool,
) -> Result<Vec<f64>, JsError> {
    let tick_count = f64::from(tick_count);
    let mut step = decimal(1.0)?;
    let mut middle = decimal(value)?;
    if !middle.is_integer() && allow_decimals {
        let abs_val = value.abs();
        if abs_val < 1.0 {
            step = Decimal::power_of_ten(get_digit_count(value)? - 1);
            middle = decimal(middle.div(&step)?.to_f64().floor())?.mul(&step);
        } else if abs_val > 1.0 {
            middle = decimal(value.floor())?;
        }
    } else if value == 0.0 {
        middle = decimal(((tick_count - 1.0) / 2.0).floor())?;
    } else if !allow_decimals {
        middle = decimal(value.floor())?;
    }
    let middle_index = ((tick_count - 1.0) / 2.0).floor();
    let mut ticks = Vec::new();
    let mut i = 0.0;
    while i < tick_count {
        ticks.push(middle.add(&decimal(i - middle_index)?.mul(&step)).to_f64());
        i += 1.0;
    }
    Ok(ticks)
}

/// The result of `calculateStep`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub step: Decimal,
    pub tick_min: Decimal,
    pub tick_max: Decimal,
}

/// How many times `calculateStep` may call itself before the port reports
/// the stack overflow the original runs into. The original recurses with a
/// growing correction factor until the ticks fit; when they cannot (two
/// ticks for an interval that strictly contains 0) it recurses until V8
/// throws `RangeError: Maximum call stack size exceeded`, or, with
/// `snap125` and integer ticks, until the step overflows to `Infinity`,
/// which takes fewer than 2,600 calls. Intervals that fit converge within
/// a few dozen calls. The exact depth of V8's overflow is not modelled.
///
/// After such an overflow inside decimal.js-light the browser's copy of the
/// library can be left with a changed `precision` and internal state, which
/// changes later results on the page; the port does not reproduce that.
pub const MAX_CALCULATE_STEP_DEPTH: i64 = 3_000;

/// `calculateStep(min, max, tickCount, allowDecimals, correctionFactor,
/// stepFn)`.
pub fn calculate_step(
    min: f64,
    max: f64,
    tick_count: u32,
    allow_decimals: bool,
    correction_factor: i64,
    mode: NiceTicks,
) -> Result<Step, JsError> {
    let step_fn = mode.step_fn();
    let tick_count = f64::from(tick_count);
    // "dirty hack (for recharts' test)"
    if !((max - min) / (tick_count - 1.0)).is_finite() {
        return Ok(Step {
            step: Decimal::zero(),
            tick_min: Decimal::zero(),
            tick_max: Decimal::zero(),
        });
    }
    let mut correction_factor = correction_factor;
    loop {
        if correction_factor >= MAX_CALCULATE_STEP_DEPTH {
            return Err(JsError::stack_overflow());
        }
        let step = step_fn(
            &decimal(max)?
                .sub(&decimal(min)?)
                .div(&decimal(tick_count - 1.0)?)?,
            allow_decimals,
            correction_factor,
        )?;
        let middle = if min <= 0.0 && max >= 0.0 {
            Decimal::zero()
        } else {
            let middle = decimal(min)?.add(&decimal(max)?).div(&decimal(2.0)?)?;
            middle.sub(&middle.rem(&step)?)
        };
        let mut below_count = middle.sub(&decimal(min)?).div(&step)?.to_f64().ceil();
        let mut up_count = decimal(max)?.sub(&middle).div(&step)?.to_f64().ceil();
        let scale_count = below_count + up_count + 1.0;
        if scale_count > tick_count {
            correction_factor += 1;
            continue;
        }
        if scale_count < tick_count {
            if max > 0.0 {
                up_count += tick_count - scale_count;
            } else {
                below_count += tick_count - scale_count;
            }
        }
        return Ok(Step {
            tick_min: middle.sub(&decimal(below_count)?.mul(&step)),
            tick_max: middle.add(&decimal(up_count)?.mul(&step)),
            step,
        });
    }
}

/// `getNiceTickValues([min, max], tickCount = 6, allowDecimals = true,
/// niceTicksMode = 'auto')`: ticks for the interval, which may extend
/// beyond it to land on round values.
pub fn get_nice_tick_values(
    [min, max]: [f64; 2],
    tick_count: u32,
    allow_decimals: bool,
    mode: NiceTicks,
) -> Result<Vec<f64>, JsError> {
    let count = tick_count.max(2);
    let (cormin, cormax) = get_valid_interval(min, max);
    if cormin == f64::NEG_INFINITY || cormax == f64::INFINITY {
        let fill = tick_count
            .checked_sub(1)
            .ok_or_else(JsError::invalid_array_length)? as usize;
        let mut values = if cormax == f64::INFINITY {
            let mut values = vec![cormin];
            values.extend(std::iter::repeat_n(f64::INFINITY, fill));
            values
        } else {
            let mut values = vec![f64::NEG_INFINITY; fill];
            values.push(cormax);
            values
        };
        if min > max {
            values.reverse();
        }
        return Ok(values);
    }
    if cormin == cormax {
        return get_tick_of_single_value(cormin, tick_count, allow_decimals);
    }
    let Step {
        step,
        tick_min,
        tick_max,
    } = calculate_step(cormin, cormax, count, allow_decimals, 0, mode)?;
    let end = tick_max.add(&decimal(0.1)?.mul(&step));
    let mut values = range_step(&tick_min, &end, &step);
    if min > max {
        values.reverse();
    }
    Ok(values)
}

/// `getTickValuesFixedDomain([min, max], tickCount, allowDecimals = true,
/// niceTicksMode = 'auto')`: ticks constrained to the interval.
pub fn get_tick_values_fixed_domain(
    [min, max]: [f64; 2],
    tick_count: u32,
    allow_decimals: bool,
    mode: NiceTicks,
) -> Result<Vec<f64>, JsError> {
    let (cormin, cormax) = get_valid_interval(min, max);
    if cormin == f64::NEG_INFINITY || cormax == f64::INFINITY {
        return Ok(vec![min, max]);
    }
    if cormin == cormax {
        return Ok(vec![cormin]);
    }
    let count = f64::from(tick_count.max(2));
    let step = mode.step_fn()(
        &decimal(cormax)?
            .sub(&decimal(cormin)?)
            .div(&decimal(count - 1.0)?)?,
        allow_decimals,
        0,
    )?;
    let mut values = range_step(&decimal(cormin)?, &decimal(cormax)?, &step);
    values.push(cormax);
    if !allow_decimals {
        values = values.into_iter().map(math_round).collect();
    }
    if min > max {
        values.reverse();
    }
    Ok(values)
}
