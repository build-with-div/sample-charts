//! Bar geometry: `state/selectors/barSelectors.js` with the combiners
//! `combineBarSizeList`, `combineAllBarPositions`, `combineBarPosition` and
//! `combineStackedData`, `combineGraphicalItemTicks` of
//! `axisSelectors.js`, and `computeBarRectangles` of `cartesian/Bar.js` with
//! `getBandSizeOfAxis`, `getCateCoordinateOfBar`, `getBaseValueOfBar` and
//! `truncateByDomain` of `util/ChartUtils.js`.
//!
//! `<Cell>` children, custom shapes and `minPointSize` functions are not
//! modelled: a bar here has none.

use js::{math, parse_float, string_to_number, to_int32, Key, Value};

use super::axis::{AxisResult, Tick};
use super::props::{
    is_categorical_axis, AxisDomainType, AxisType, DataKey, ItemKind, ItemSettings, Layout,
};
use super::{CartesianChart, ChartError};

/// One entry of `selectBarSizeList`: a stack (or an unstacked bar), its
/// data keys and its `barSize`.
#[derive(Debug, Clone, PartialEq)]
pub struct BarSize {
    pub stack_id: Option<String>,
    pub data_keys: Vec<DataKey>,
    pub bar_size: Option<f64>,
}

/// `selectBarPosition`: where in its band a bar starts, and its size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BarPosition {
    pub offset: f64,
    pub size: f64,
}

/// One rectangle of `selectBarRectangles` (the fields it adds to the
/// data entry).
#[derive(Debug, Clone, PartialEq)]
pub struct BarRectangle {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// The bar's value: `[start, end]` for a stacked bar.
    pub value: Value,
    /// `numericAxis.scale.map(baseValue)`.
    pub stacked_bar_start: Option<f64>,
    /// `background`: x, y, width, height.
    pub background: [f64; 4],
    pub original_data_index: usize,
    /// `payload`: the data entry the rectangle is drawn for.
    pub payload: Value,
}

/// Everything the bar selectors compute for one `<Bar>`.
#[derive(Debug, Clone, PartialEq)]
pub struct BarResult {
    /// `selectBarSizeList`.
    pub size_list: Vec<BarSize>,
    /// `selectBarBandSize`.
    pub bar_band_size: f64,
    /// `selectAxisBandSize`.
    pub axis_band_size: Option<f64>,
    /// `selectBarPosition`.
    pub position: Option<BarPosition>,
    /// `selectBarRectangles`.
    pub rectangles: Option<Vec<BarRectangle>>,
}

/// `getPercentValue(percent, totalValue, defaultValue, validate)` of
/// `util/DataUtils.js`.
pub(crate) fn get_percent_value(
    percent: &Value,
    total: Option<f64>,
    default: f64,
    validate: bool,
) -> f64 {
    let value = match percent {
        Value::Number(n) if !n.is_nan() => *n,
        Value::String(s) => {
            // isPercent: `indexOf('%') === length - 1`, in UTF-16 units, so
            // also the empty string.
            let units: Vec<u16> = s.encode_utf16().collect();
            let first = units.iter().position(|&u| u == u16::from(b'%'));
            let is_percent = match first {
                Some(i) => i + 1 == units.len(),
                None => units.is_empty(),
            };
            if is_percent {
                let Some(total) = total else {
                    return default;
                };
                let prefix = String::from_utf16_lossy(&units[..first.unwrap_or(0)]);
                total * parse_float(&prefix) / 100.0
            } else {
                string_to_number(s)
            }
        }
        _ => return default,
    };
    let value = if value.is_nan() { default } else { value };
    match total {
        Some(total) if validate && value > total => total,
        _ => value,
    }
}

/// `mathSign`: 0 for 0, 1 above, -1 otherwise (NaN included).
fn math_sign(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else if value > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `isWellBehavedNumber`.
fn well_behaved(value: Option<f64>) -> Option<f64> {
    value.filter(|n| n.is_finite())
}

/// `isNumber`: a number that is not NaN.
fn is_number(value: &Value) -> Option<f64> {
    value.as_number().filter(|n| !n.is_nan())
}

/// SameValueZero of two data keys (`Array.prototype.includes`).
fn same_key(a: &DataKey, b: &DataKey) -> bool {
    match (a, b) {
        (DataKey::Name(a), DataKey::Name(b)) => a == b,
        (DataKey::Index(a), DataKey::Index(b)) => a == b || (a.is_nan() && b.is_nan()),
        _ => false,
    }
}

/// A key of a JavaScript object that is an array index, which orders it
/// before the other keys.
fn array_index(key: &str) -> Option<u32> {
    let n: u32 = key.parse().ok()?;
    (n != u32::MAX && n.to_string() == key).then_some(n)
}

/// `getBarSize(globalSize, totalSize, selfSize)`.
fn bar_size(global: Option<&Value>, total: f64, own: Option<&Value>) -> Option<f64> {
    let size = own
        .filter(|v| !v.is_nullish())
        .or(global.filter(|v| !v.is_nullish()))?;
    Some(get_percent_value(size, Some(total), 0.0, false))
}

/// `getBarPositions(barGap, barCategoryGap, bandSize, sizeList,
/// maxBarSize)` of `combineAllBarPositions.js`.
fn bar_positions(
    bar_gap: &Value,
    bar_category_gap: &Value,
    band_size: f64,
    size_list: &[BarSize],
    max_bar_size: Option<f64>,
) -> Option<Vec<BarPosition>> {
    let len = size_list.len();
    if len < 1 {
        return None;
    }
    let n = len as f64;
    let mut real_bar_gap = get_percent_value(bar_gap, Some(band_size), 0.0, true);
    if well_behaved(size_list[0].bar_size).is_some() {
        let mut use_full = false;
        let mut full_bar_size = band_size / n;
        // `entry.barSize || 0`
        let mut sum = size_list.iter().fold(0.0, |res, entry| {
            res + entry.bar_size.filter(|s| !s.is_nan()).unwrap_or(0.0)
        });
        sum += (n - 1.0) * real_bar_gap;
        if sum >= band_size {
            sum -= (n - 1.0) * real_bar_gap;
            real_bar_gap = 0.0;
        }
        if sum >= band_size && full_bar_size > 0.0 {
            use_full = true;
            full_bar_size *= 0.9;
            sum = n * full_bar_size;
        }
        let offset = f64::from(to_int32((band_size - sum) / 2.0));
        let mut prev = BarPosition {
            offset: offset - real_bar_gap,
            size: 0.0,
        };
        Some(
            size_list
                .iter()
                .map(|entry| {
                    let position = BarPosition {
                        offset: prev.offset + prev.size + real_bar_gap,
                        size: if use_full {
                            full_bar_size
                        } else {
                            entry.bar_size.unwrap_or(0.0)
                        },
                    };
                    prev = position;
                    position
                })
                .collect(),
        )
    } else {
        let offset = get_percent_value(bar_category_gap, Some(band_size), 0.0, true);
        if band_size - 2.0 * offset - (n - 1.0) * real_bar_gap <= 0.0 {
            real_bar_gap = 0.0;
        }
        let mut original_size = (band_size - 2.0 * offset - (n - 1.0) * real_bar_gap) / n;
        if original_size > 1.0 {
            original_size = f64::from(to_int32(original_size));
        }
        let size = match well_behaved(max_bar_size) {
            Some(max) => math::min(original_size, max),
            None => original_size,
        };
        Some(
            (0..len)
                .map(|i| BarPosition {
                    offset: offset
                        + (original_size + real_bar_gap) * i as f64
                        + (original_size - size) / 2.0,
                    size,
                })
                .collect(),
        )
    }
}

/// `getBandSizeOfAxis(axis, ticks, isBar)`.
pub(super) fn band_size_of_axis(
    axis: Option<&AxisResult>,
    ticks: Option<&[Tick]>,
    is_bar: bool,
) -> Option<f64> {
    let axis = axis.filter(|a| a.scale.is_some());
    if let Some(bandwidth) = axis
        .and_then(|a| a.scale.as_ref())
        .and_then(|s| s.bandwidth())
    {
        if !is_bar || bandwidth > 0.0 {
            return Some(bandwidth);
        }
    }
    match ticks {
        Some(ticks) if axis.is_some() && ticks.len() >= 2 => {
            let mut ordered: Vec<f64> = ticks.iter().map(|t| t.coordinate).collect();
            // A stable sort by coordinate (all finite).
            ordered.sort_by(|a, b| a.partial_cmp(b).expect("finite coordinates"));
            let band = ordered.windows(2).fold(f64::INFINITY, |band, pair| {
                math::min(pair[1] - pair[0], band)
            });
            Some(if band == f64::INFINITY { 0.0 } else { band })
        }
        _ => {
            if is_bar {
                None
            } else {
                Some(0.0)
            }
        }
    }
}

/// `truncateByDomain(value, domain)`.
fn truncate_by_domain(value: [f64; 2], domain: &[Value]) -> [f64; 2] {
    let (Some(d0), Some(d1)) = (
        domain.first().and_then(is_number),
        domain.get(1).and_then(is_number),
    ) else {
        return value;
    };
    if domain.len() != 2 {
        return value;
    }
    let min = math::min(d0, d1);
    let max = math::max(d0, d1);
    let mut result = value;
    if value[0].is_nan() || value[0] < min {
        result[0] = min;
    }
    if value[1].is_nan() || value[1] > max {
        result[1] = max;
    }
    if result[0] > max {
        result[0] = max;
    }
    if result[1] < min {
        result[1] = min;
    }
    result
}

/// `getValueByDataKey(obj, dataKey, defaultValue)`.
fn value_by_data_key(obj: &Value, key: Option<&DataKey>, default: Value) -> Value {
    match key {
        Some(key) if !obj.is_nullish() => match key.value_of(obj) {
            Value::Undefined => default,
            value => value,
        },
        _ => default,
    }
}

/// `getCateCoordinateOfBar`.
fn cate_coordinate(
    axis: &AxisResult,
    ticks: &[Tick],
    offset: f64,
    band_size: f64,
    entry: &Value,
    index: usize,
) -> Option<f64> {
    let scale = axis.scale.as_ref()?;
    if axis.settings.r#type == AxisDomainType::Category {
        return ticks.get(index).map(|t| t.coordinate + offset);
    }
    let default = scale
        .domain()
        .get(index)
        .cloned()
        .unwrap_or(Value::Undefined);
    let value = value_by_data_key(entry, axis.settings.data_key.as_ref(), default);
    if value.is_nullish() {
        return None;
    }
    let scaled = scale.map(&value, None).filter(|n| !n.is_nan())?;
    Some(scaled - band_size / 2.0 + offset)
}

/// `getBaseValueOfBar({ numericAxis })`.
fn base_value(axis: &AxisResult) -> Value {
    let domain = axis.scale.as_ref().map(|s| s.domain()).unwrap_or_default();
    let at = |i: usize| domain.get(i).cloned().unwrap_or(Value::Undefined);
    if axis.settings.r#type == AxisDomainType::Number {
        let (a, b) = (at(0).to_number(), at(1).to_number());
        let min = math::min(a, b);
        let max = math::max(a, b);
        if min <= 0.0 && max >= 0.0 {
            return Value::Number(0.0);
        }
        if max < 0.0 {
            return Value::Number(max);
        }
        return Value::Number(min);
    }
    at(0)
}

impl CartesianChart {
    /// `selectXAxisIdFromGraphicalItemId` / `selectYAxisIdFromGraphicalItemId`.
    pub(super) fn item_axis_id(&self, axis_type: AxisType, id: &str) -> Key {
        match self.items.iter().find(|item| item.id == id) {
            Some(item) => match axis_type {
                AxisType::XAxis => item.x_axis_id.clone(),
                AxisType::YAxis => item.y_axis_id.clone(),
            },
            None => Key::number(0.0),
        }
    }

    /// `selectTicksOfGraphicalItem` (`combineGraphicalItemTicks`).
    pub(super) fn graphical_item_ticks(
        &self,
        axis_type: AxisType,
        axis: &AxisResult,
    ) -> Result<Option<Vec<Tick>>, ChartError> {
        let settings = &axis.settings;
        let Some(scale) = axis.scale.as_ref() else {
            return Ok(None);
        };
        if axis.range[0] == axis.range[1] {
            return Ok(None);
        }
        let categorical = is_categorical_axis(self.layout, axis_type);
        let categorical_domain = settings.data_key.is_some()
            && categorical
            && (settings.r#type == AxisDomainType::Number || settings.scale != "auto");
        let tick = |index: usize, entry: &Value| {
            scale
                .map(entry, None)
                .filter(|n| n.is_finite())
                .map(|coordinate| Tick {
                    index,
                    coordinate,
                    value: entry.clone(),
                    offset: 0.0,
                })
        };
        if categorical && categorical_domain {
            return Ok(Some(
                axis.applied_values
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| tick(i, v))
                    .collect(),
            ));
        }
        if let Some(ticks) = scale.ticks(settings.tick_count) {
            return Ok(Some(
                ticks?
                    .iter()
                    .enumerate()
                    .filter_map(|(i, v)| tick(i, v))
                    .collect(),
            ));
        }
        Ok(Some(
            scale
                .domain()
                .iter()
                .enumerate()
                .filter_map(|(i, v)| tick(i, v))
                .collect(),
        ))
    }

    /// `selectAllVisibleBars`.
    fn visible_bars(&self, id: &str) -> Vec<&ItemSettings> {
        let x_axis_id = self.item_axis_id(AxisType::XAxis, id);
        let y_axis_id = self.item_axis_id(AxisType::YAxis, id);
        self.items
            .iter()
            .filter(|item| match self.layout {
                Layout::Horizontal => item
                    .x_axis_id
                    .to_value()
                    .strict_equals(&x_axis_id.to_value()),
                Layout::Vertical => item
                    .y_axis_id
                    .to_value()
                    .strict_equals(&y_axis_id.to_value()),
            })
            .filter(|item| !item.hide && item.kind == ItemKind::Bar)
            .collect()
    }

    /// `selectBarSizeList` (`combineBarSizeList`).
    fn bar_size_list(&self, id: &str) -> Vec<BarSize> {
        let bars = self.visible_bars(id);
        let total = self.offset();
        let total = match self.layout {
            Layout::Horizontal => total.width,
            Layout::Vertical => total.height,
        };
        let global = self.bar_size.as_ref();
        let mut groups: Vec<(String, Vec<&ItemSettings>)> = Vec::new();
        for bar in bars.iter().filter(|b| b.is_stacked()) {
            let stack_id = bar.stack_id.clone().expect("stacked");
            match groups.iter_mut().find(|(k, _)| *k == stack_id) {
                Some(group) => group.1.push(bar),
                None => groups.push((stack_id, vec![bar])),
            }
        }
        // Object.entries: array-index keys ascending, then insertion order.
        let mut indexed: Vec<(u32, usize)> = groups
            .iter()
            .enumerate()
            .filter_map(|(i, (k, _))| array_index(k).map(|n| (n, i)))
            .collect();
        indexed.sort();
        let order = indexed.into_iter().map(|(_, i)| i).chain(
            groups
                .iter()
                .enumerate()
                .filter(|(_, (k, _))| array_index(k).is_none())
                .map(|(i, _)| i),
        );
        let stacked = order.map(|i| {
            let (stack_id, bars) = &groups[i];
            BarSize {
                stack_id: Some(stack_id.clone()),
                data_keys: bars.iter().filter_map(|b| b.data_key.clone()).collect(),
                bar_size: bar_size(global, total, bars[0].bar_size.as_ref()),
            }
        });
        let unstacked = bars
            .iter()
            .filter(|b| b.stack_id.is_none())
            .map(|b| BarSize {
                stack_id: None,
                data_keys: b.data_key.iter().cloned().collect(),
                bar_size: bar_size(global, total, b.bar_size.as_ref()),
            });
        stacked.chain(unstacked).collect()
    }

    /// Every bar selector for the `<Bar>` with this `id`.
    pub fn bar(&self, id: &str) -> Result<BarResult, ChartError> {
        let size_list = self.bar_size_list(id);
        let settings = self
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Bar && item.id == id);
        let (category_type, numeric_type) = match self.layout {
            Layout::Horizontal => (AxisType::XAxis, AxisType::YAxis),
            Layout::Vertical => (AxisType::YAxis, AxisType::XAxis),
        };
        let category_axis = self.axis(category_type, &self.item_axis_id(category_type, id))?;
        let category_ticks = self.graphical_item_ticks(category_type, &category_axis)?;
        let with_scale = category_axis.scale.is_some().then_some(&category_axis);

        // selectBarBandSize
        let max_bar_size = settings.and_then(|s| s.max_bar_size).or(self.max_bar_size);
        let bar_band_size = match settings {
            None => 0.0,
            Some(_) => band_size_of_axis(with_scale, category_ticks.as_deref(), true)
                .or(max_bar_size)
                .unwrap_or(0.0),
        };
        // selectAxisBandSize
        let axis_band_size = band_size_of_axis(with_scale, category_ticks.as_deref(), false);

        // selectBarPosition (combineAllBarPositions, combineBarPosition)
        let different = match axis_band_size {
            Some(band) => !(band == bar_band_size),
            None => true,
        };
        let positions = bar_positions(
            &self.bar_gap,
            &self.bar_category_gap,
            bar_band_size,
            &size_list,
            max_bar_size,
        )
        .map(|positions| {
            if different {
                positions
                    .into_iter()
                    .map(|p| BarPosition {
                        offset: p.offset - bar_band_size / 2.0,
                        size: p.size,
                    })
                    .collect()
            } else {
                positions
            }
        });
        let position = match (&positions, settings) {
            (Some(positions), Some(s)) => size_list
                .iter()
                .position(|entry| {
                    entry.stack_id == s.stack_id
                        && s.data_key
                            .as_ref()
                            .is_some_and(|k| entry.data_keys.iter().any(|d| same_key(d, k)))
                })
                .map(|i| positions[i]),
            _ => None,
        };

        // selectBarRectangles reads both axes and their ticks first.
        let numeric_id = self.item_axis_id(numeric_type, id);
        let numeric_axis = self.axis(numeric_type, &numeric_id)?;
        let numeric_ticks = self.graphical_item_ticks(numeric_type, &numeric_axis)?;
        let rectangles = match (settings, position, &category_ticks, &numeric_ticks) {
            (Some(settings), Some(position), Some(category_ticks), Some(_)) => self.bar_rectangles(
                settings,
                position,
                &category_axis,
                category_ticks,
                axis_band_size,
                (numeric_type, &numeric_id, &numeric_axis),
            ),
            _ => None,
        };
        Ok(BarResult {
            size_list,
            bar_band_size,
            axis_band_size,
            position,
            rectangles,
        })
    }

    /// `selectBarRectangles` with `computeBarRectangles`.
    fn bar_rectangles(
        &self,
        settings: &ItemSettings,
        position: BarPosition,
        category_axis: &AxisResult,
        category_ticks: &[Tick],
        band_size: Option<f64>,
        (numeric_type, numeric_id, numeric_axis): (AxisType, &Key, &AxisResult),
    ) -> Option<Vec<BarRectangle>> {
        let band_size = band_size?;
        let numeric_scale = numeric_axis.scale.as_ref()?;
        category_axis.scale.as_ref()?;
        let chart_data = self.chart_data.as_ref()?;
        let start = self.data_start_index.min(chart_data.len());
        let end = (self.data_end_index + 1).min(chart_data.len()).max(start);
        let displayed = &chart_data[start..end];

        // selectStackedDataOfItem
        let stacked_data = settings.stack_id.as_ref().and_then(|stack_id| {
            let groups = self.stack_groups(numeric_type, numeric_id);
            let group = groups.into_iter().find(|g| g.stack_id == *stack_id)?;
            let i = group.items.iter().position(|id| *id == settings.id)?;
            group.stacked_data.into_iter().nth(i)
        });
        let offset = self.offset();
        let stacked_domain = stacked_data.as_ref().map(|_| numeric_scale.domain());
        let base = base_value(numeric_axis);
        let stacked_bar_start = numeric_scale.map(&base, None);
        let horizontal = self.layout == Layout::Horizontal;
        let mps = settings.min_point_size;

        let mut rectangles = Vec::new();
        for (index, entry) in displayed.iter().enumerate() {
            let value: Vec<Value> = match (&stacked_data, &stacked_domain) {
                (Some(series), Some(domain)) => {
                    let Some(point) = series.get(index + self.data_start_index) else {
                        continue;
                    };
                    truncate_by_domain(*point, domain)
                        .iter()
                        .map(|n| Value::Number(*n))
                        .collect()
                }
                _ => match value_by_data_key(entry, settings.data_key.as_ref(), Value::Undefined) {
                    Value::Array(items) => items,
                    other => vec![base.clone(), other],
                },
            };
            let at = |i: usize| value.get(i).cloned().unwrap_or(Value::Undefined);
            let (Some(base_scaled), Some(current_scaled)) = (
                numeric_scale.map(&at(0), None),
                numeric_scale.map(&at(1), None),
            ) else {
                continue;
            };
            let cate = cate_coordinate(
                category_axis,
                category_ticks,
                position.offset,
                band_size,
                entry,
                index,
            );
            let (x, y, width, height, background);
            if horizontal {
                let mut top = current_scaled;
                let computed = base_scaled - current_scaled;
                let mut h = if computed.is_nan() { 0.0 } else { computed };
                background = [
                    cate.unwrap_or(f64::NAN),
                    offset.top,
                    position.size,
                    offset.height,
                ];
                if mps.abs() > 0.0 && h.abs() < mps.abs() {
                    let delta = math_sign(if h == 0.0 || h.is_nan() { mps } else { h })
                        * (mps.abs() - h.abs());
                    top -= delta;
                    h += delta;
                }
                (x, y, width, height) = (cate, Some(top), position.size, h);
            } else {
                let mut w = current_scaled - base_scaled;
                background = [
                    offset.left,
                    cate.unwrap_or(f64::NAN),
                    offset.width,
                    position.size,
                ];
                if mps.abs() > 0.0 && w.abs() < mps.abs() {
                    let delta = math_sign(if w == 0.0 || w.is_nan() { mps } else { w })
                        * (mps.abs() - w.abs());
                    w += delta;
                }
                (x, y, width, height) = (Some(base_scaled), cate, w, position.size);
            }
            let (Some(x), Some(y)) = (x, y) else {
                continue;
            };
            if width == 0.0 || height == 0.0 {
                continue;
            }
            rectangles.push(BarRectangle {
                x,
                y,
                width,
                height,
                value: if stacked_data.is_some() {
                    Value::Array(value)
                } else {
                    at(1)
                },
                stacked_bar_start,
                background,
                original_data_index: index,
                payload: entry.clone(),
            });
        }
        Some(rectangles)
    }
}
