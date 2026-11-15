//! The tick label filter: which of an axis's ticks get a label without
//! labels overlapping (`cartesian/getTicks.js`, `getEquidistantTicks.js`,
//! `util/TickUtils.js`, `util/getEveryNth.js`, `util/CartesianUtils.js`'s
//! `getAngledRectangleWidth`).
//!
//! Labels are measured with `getStringSize`, lazily and in Recharts' order,
//! so the port asks for exactly the measurements Recharts makes.

use js::Value;

use super::text::{Size, StringSizes};
use super::Measure;
use crate::chart::{ChartError, Tick};
use crate::dom::Props;

/// A tick as `getTicks` returns it: the axis tick, and where its label is
/// centred when the filter moved it (`tickCoord`).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TickEntry {
    pub(super) tick: Tick,
    pub(super) tick_coord: Option<f64>,
    is_show: bool,
}

impl TickEntry {
    fn new(tick: &Tick) -> TickEntry {
        TickEntry {
            tick: tick.clone(),
            tick_coord: None,
            is_show: false,
        }
    }
}

/// The props `getTicks` reads.
pub(super) struct TicksConfig<'a> {
    pub(super) tick: Value,
    pub(super) ticks: Option<&'a [Tick]>,
    /// x, y, width, height
    pub(super) view_box: [f64; 4],
    pub(super) min_tick_gap: Value,
    pub(super) orientation: Value,
    pub(super) interval: Value,
    pub(super) tick_formatter: Value,
    pub(super) unit: Value,
    pub(super) angle: Value,
}

/// `mathSign`.
fn math_sign(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else if value > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `getEveryNth(array, n)`: every `n`th item from the first; a fractional
/// step skips the indices that are not whole.
fn every_nth<T: Clone>(array: &[T], n: f64) -> Vec<T> {
    if n < 1.0 {
        return Vec::new();
    }
    if n == 1.0 {
        return array.to_vec();
    }
    let mut result = Vec::new();
    let mut i = 0.0;
    while i < array.len() as f64 {
        if i.fract() == 0.0 {
            result.push(array[i as usize].clone());
        }
        i += n;
    }
    result
}

/// `isVisible`: whether a label of `size` centred at `position` fits
/// between `start` and `end`. The size is only measured when the position
/// is inside them.
fn is_visible(
    sign: f64,
    position: f64,
    size: &mut dyn FnMut() -> Result<f64, ChartError>,
    start: f64,
    end: f64,
) -> Result<bool, ChartError> {
    if sign * position < sign * start || sign * position > sign * end {
        return Ok(false);
    }
    let size = size()?;
    Ok(sign * (position - sign * size / 2.0 - start) >= 0.0
        && sign * (position + sign * size / 2.0 - end) <= 0.0)
}

/// `getAngledRectangleWidth` of an unrotated label (angle 0 after
/// `normalizeAngle`): its width. Rotated labels are not ported.
fn angled_width(size: Size, angle: &Value) -> Result<f64, ChartError> {
    let angle = match angle {
        Value::Undefined => 0.0,
        other => other.to_number(),
    };
    let normalized = (angle % 180.0 + 180.0) % 180.0;
    if normalized.is_nan() {
        // `width / Math.cos(NaN)`
        return Ok(f64::NAN);
    }
    if normalized != 0.0 {
        return Err(ChartError::Unsupported("rotated tick labels"));
    }
    // `angleRadians > Math.atan(height / width)` is false at 0, so the
    // width over `Math.cos(0)`, which is 1.
    Ok(size.width.abs())
}

/// `getTicks(props, fontSize, letterSpacing)`.
pub(super) fn get_ticks(
    config: &TicksConfig,
    sizes: &mut StringSizes,
    measure: &mut dyn Measure,
    font_size: &Value,
    letter_spacing: &Value,
) -> Result<Vec<TickEntry>, ChartError> {
    let ticks = match config.ticks {
        Some(ticks) if !ticks.is_empty() && config.tick.is_truthy() => ticks,
        _ => return Ok(Vec::new()),
    };
    if !config.tick_formatter.is_nullish() {
        return Err(ChartError::Unsupported("a tickFormatter"));
    }
    let entries: Vec<TickEntry> = ticks.iter().map(TickEntry::new).collect();
    if let Value::Number(interval) = config.interval {
        if !interval.is_nan() {
            return Ok(every_nth(&entries, interval + 1.0));
        }
    }
    let min_tick_gap = match config.min_tick_gap {
        Value::Number(gap) => gap,
        _ => return Err(ChartError::Unsupported("a minTickGap other than a number")),
    };
    let width_key = matches!(config.orientation.to_js_string().as_str(), "top" | "bottom");
    let mut style = Props::new();
    style.set("fontSize", font_size.clone());
    style.set("letterSpacing", letter_spacing.clone());
    let unit_size = if config.unit.is_truthy() && width_key {
        sizes.get(measure, &config.unit, &style)
    } else {
        Size {
            width: 0.0,
            height: 0.0,
        }
    };
    let angle = config.angle.clone();
    let mut tick_size = |entry: &TickEntry| -> Result<f64, ChartError> {
        let size = sizes.get(measure, &entry.tick.value, &style);
        if width_key {
            angled_width(
                Size {
                    width: size.width + unit_size.width,
                    height: size.height + unit_size.height,
                },
                &angle,
            )
        } else {
            Ok(size.height)
        }
    };
    let sign = if ticks.len() >= 2 {
        math_sign(ticks[1].coordinate - ticks[0].coordinate)
    } else {
        1.0
    };
    // getTickBoundaries
    let [x, y, width, height] = config.view_box;
    let (start, end) = match (sign == 1.0, width_key) {
        (true, true) => (x, x + width),
        (true, false) => (y, y + height),
        (false, true) => (x + width, x),
        (false, false) => (y + height, y),
    };
    let interval = config.interval.to_js_string();
    let candidates = match interval.as_str() {
        "equidistantPreserveStart" => {
            return equidistant_ticks(sign, start, end, &mut tick_size, &entries, min_tick_gap)
        }
        "equidistantPreserveEnd" => {
            return equidistant_preserve_end_ticks(
                sign,
                start,
                end,
                &mut tick_size,
                &entries,
                min_tick_gap,
            )
        }
        "preserveStart" | "preserveStartEnd" => ticks_start(
            sign,
            start,
            end,
            &mut tick_size,
            &entries,
            min_tick_gap,
            interval == "preserveStartEnd",
        )?,
        _ => ticks_end(sign, start, end, &mut tick_size, &entries, min_tick_gap)?,
    };
    Ok(candidates.into_iter().filter(|e| e.is_show).collect())
}

type SizeOf<'a> = dyn FnMut(&TickEntry) -> Result<f64, ChartError> + 'a;

/// `getTicksEnd`: from the last tick back, each label that fits before the
/// one after it.
fn ticks_end(
    sign: f64,
    start: f64,
    mut end: f64,
    tick_size: &mut SizeOf,
    ticks: &[TickEntry],
    min_tick_gap: f64,
) -> Result<Vec<TickEntry>, ChartError> {
    let mut result = ticks.to_vec();
    let len = result.len();
    for i in (0..len).rev() {
        let initial = result[i].clone();
        let mut size: Option<f64> = None;
        let mut get_size = || -> Result<f64, ChartError> {
            if size.is_none() {
                size = Some(tick_size(&initial)?);
            }
            Ok(size.expect("measured"))
        };
        let coordinate = initial.tick.coordinate;
        let tick_coord = if i == len - 1 {
            let gap = sign * (coordinate + sign * get_size()? / 2.0 - end);
            if gap > 0.0 {
                coordinate - gap * sign
            } else {
                coordinate
            }
        } else {
            coordinate
        };
        result[i].tick_coord = Some(tick_coord);
        if is_visible(sign, tick_coord, &mut get_size, start, end)? {
            end = tick_coord - sign * (get_size()? / 2.0 + min_tick_gap);
            result[i].is_show = true;
        }
    }
    Ok(result)
}

/// `getTicksStart`: from the first tick on, each label that fits after the
/// one before it; with `preserveEnd`, the last label first.
fn ticks_start(
    sign: f64,
    mut start: f64,
    mut end: f64,
    tick_size: &mut SizeOf,
    ticks: &[TickEntry],
    min_tick_gap: f64,
    preserve_end: bool,
) -> Result<Vec<TickEntry>, ChartError> {
    let mut result = ticks.to_vec();
    let len = result.len();
    if preserve_end {
        let tail = &ticks[len - 1];
        let tail_size = tick_size(tail)?;
        let gap = sign * (tail.tick.coordinate + sign * tail_size / 2.0 - end);
        let tick_coord = if gap > 0.0 {
            tail.tick.coordinate - gap * sign
        } else {
            tail.tick.coordinate
        };
        result[len - 1].tick_coord = Some(tick_coord);
        if is_visible(sign, tick_coord, &mut || Ok(tail_size), start, end)? {
            end = tick_coord - sign * (tail_size / 2.0 + min_tick_gap);
            result[len - 1].is_show = true;
        }
    }
    let count = if preserve_end { len - 1 } else { len };
    for (i, entry) in result.iter_mut().take(count).enumerate() {
        let initial = entry.clone();
        let mut size: Option<f64> = None;
        let mut get_size = || -> Result<f64, ChartError> {
            if size.is_none() {
                size = Some(tick_size(&initial)?);
            }
            Ok(size.expect("measured"))
        };
        let coordinate = initial.tick.coordinate;
        let tick_coord = if i == 0 {
            let gap = sign * (coordinate - sign * get_size()? / 2.0 - start);
            if gap < 0.0 {
                coordinate - gap * sign
            } else {
                coordinate
            }
        } else {
            coordinate
        };
        entry.tick_coord = Some(tick_coord);
        if is_visible(sign, tick_coord, &mut get_size, start, end)? {
            start = tick_coord + sign * (get_size()? / 2.0 + min_tick_gap);
            entry.is_show = true;
        }
    }
    Ok(result)
}

/// `getEquidistantTicks`: every `n`th tick from the first, for the
/// smallest `n` whose labels do not overlap.
fn equidistant_ticks(
    sign: f64,
    initial_start: f64,
    end: f64,
    tick_size: &mut SizeOf,
    ticks: &[TickEntry],
    min_tick_gap: f64,
) -> Result<Vec<TickEntry>, ChartError> {
    let mut index = 0;
    let mut step = 1;
    let mut start = initial_start;
    while step <= ticks.len() {
        let Some(entry) = ticks.get(index) else {
            return Ok(every_nth(ticks, step as f64));
        };
        let mut size: Option<f64> = None;
        let mut get_size = || -> Result<f64, ChartError> {
            if size.is_none() {
                size = Some(tick_size(entry)?);
            }
            Ok(size.expect("measured"))
        };
        let coordinate = entry.tick.coordinate;
        let show = index == 0 || is_visible(sign, coordinate, &mut get_size, start, end)?;
        if show {
            start = coordinate + sign * (get_size()? / 2.0 + min_tick_gap);
            index += step;
        } else {
            index = 0;
            start = initial_start;
            step += 1;
        }
    }
    Ok(Vec::new())
}

/// `getEquidistantPreserveEndTicks`: the same, counted back from the last
/// tick.
fn equidistant_preserve_end_ticks(
    sign: f64,
    initial_start: f64,
    end: f64,
    tick_size: &mut SizeOf,
    ticks: &[TickEntry],
    min_tick_gap: f64,
) -> Result<Vec<TickEntry>, ChartError> {
    let len = ticks.len();
    for step in 1..=len {
        let offset = (len - 1) % step;
        let mut start = initial_start;
        let mut ok = true;
        let mut index = offset;
        while index < len {
            let entry = &ticks[index];
            let mut size: Option<f64> = None;
            let mut get_size = || -> Result<f64, ChartError> {
                if size.is_none() {
                    size = Some(tick_size(entry)?);
                }
                Ok(size.expect("measured"))
            };
            let coordinate = entry.tick.coordinate;
            let show = index == offset || is_visible(sign, coordinate, &mut get_size, start, end)?;
            if !show {
                ok = false;
                break;
            }
            start = coordinate + sign * (get_size()? / 2.0 + min_tick_gap);
            index += step;
        }
        if ok {
            return Ok((offset..len)
                .step_by(step)
                .map(|i| ticks[i].clone())
                .collect());
        }
    }
    Ok(Vec::new())
}

/// `isNumber(entry.tickCoord) ? entry.tickCoord : entry.coordinate`.
pub(super) fn label_coordinate(entry: &TickEntry) -> f64 {
    match entry.tick_coord {
        Some(c) if !c.is_nan() => c,
        _ => entry.tick.coordinate,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_nth_skips_fractional_indices() {
        let items = [0, 1, 2, 3, 4];
        assert_eq!(every_nth(&items, 2.0), [0, 2, 4]);
        assert_eq!(every_nth(&items, 1.5), [0, 3]);
        assert_eq!(every_nth(&items, 0.5), Vec::<i32>::new());
    }
}
