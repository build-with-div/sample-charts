//! Area geometry: `selectArea` of `state/selectors/areaSelectors.js` with
//! `computeArea` and `getBaseValue` of `cartesian/Area.js`.

use js::{math, Value};

use super::axis::AxisResult;
use super::bar::band_size_of_axis;
use super::line::{cate_coordinate, value_by_data_key};
use super::props::{is_categorical_axis, AxisDomainType, AxisType, ItemKind, Layout};
use super::{CartesianChart, ChartError};

/// One point of an area: `null` coordinates are `None`. `value` is the
/// `[base, value]` array the point was computed from (`undefined` where a
/// stack has no entry for it).
#[derive(Debug, Clone, PartialEq)]
pub struct AreaPoint {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub value: Value,
    pub payload: Value,
}

/// The base line of an area: a coordinate on the numerical axis, or a point
/// per data point for stacked and ranged areas.
#[derive(Debug, Clone, PartialEq)]
pub enum AreaBaseLine {
    Coordinate(f64),
    Points(Vec<(Option<f64>, Option<f64>)>),
}

/// `selectArea`'s result.
#[derive(Debug, Clone, PartialEq)]
pub struct AreaGeometry {
    pub points: Vec<AreaPoint>,
    pub base_line: AreaBaseLine,
    pub is_range: bool,
}

/// `isNumber`: a number that is not NaN.
fn as_number(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) if !n.is_nan() => Some(*n),
        _ => None,
    }
}

/// `getBaseValue(layout, chartBaseValue, itemBaseValue, xAxis, yAxis)`.
fn base_value(item: &Value, chart: &Value, numeric_axis: &AxisResult) -> Value {
    let base = if item.is_nullish() { chart } else { item };
    if as_number(base).is_some() {
        return base.clone();
    }
    let domain = numeric_axis
        .scale
        .as_ref()
        .map(|s| s.domain())
        .unwrap_or_default();
    let at = |i: usize| domain.get(i).cloned().unwrap_or(Value::Undefined);
    let is = |name: &str| matches!(base, Value::String(s) if s == name);
    if numeric_axis.settings.r#type == AxisDomainType::Number {
        let (a, b) = (at(0).to_number(), at(1).to_number());
        let max = math::max(a, b);
        let min = math::min(a, b);
        if is("dataMin") {
            return Value::Number(min);
        }
        if is("dataMax") {
            return Value::Number(max);
        }
        return Value::Number(if max < 0.0 { max } else { math::max(min, 0.0) });
    }
    if is("dataMax") {
        return at(1);
    }
    at(0)
}

impl CartesianChart {
    /// `selectArea` for the `<Area>` with this `id`: `None` where the
    /// selector gives `undefined`.
    pub fn area(&self, id: &str) -> Result<Option<AreaGeometry>, ChartError> {
        let x_id = self.item_axis_id(AxisType::XAxis, id);
        let y_id = self.item_axis_id(AxisType::YAxis, id);
        let x_axis = self.axis(AxisType::XAxis, &x_id)?;
        let y_axis = self.axis(AxisType::YAxis, &y_id)?;
        let x_ticks = self.graphical_item_ticks(AxisType::XAxis, &x_axis)?;
        let y_ticks = self.graphical_item_ticks(AxisType::YAxis, &y_axis)?;
        let settings = self
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Area && item.id == id);
        let with_scale = |axis: &AxisResult| axis.scale.is_some();
        let x_categorical = is_categorical_axis(self.layout, AxisType::XAxis);
        // selectBandSize
        let band_size = if x_categorical {
            band_size_of_axis(
                with_scale(&x_axis).then_some(&x_axis),
                x_ticks.as_deref(),
                false,
            )
        } else {
            band_size_of_axis(
                with_scale(&y_axis).then_some(&y_axis),
                y_ticks.as_deref(),
                false,
            )
        };
        let (Some(settings), Some(x_ticks), Some(y_ticks), Some(band_size)) =
            (settings, x_ticks, y_ticks, band_size)
        else {
            return Ok(None);
        };
        if !with_scale(&x_axis) || !with_scale(&y_axis) || x_ticks.is_empty() || y_ticks.is_empty()
        {
            return Ok(None);
        }
        let displayed: Vec<Value> = match &settings.data {
            Some(data) if !data.is_empty() => data.clone(),
            _ => match &self.chart_data {
                Some(chart_data) => {
                    let start = self.data_start_index.min(chart_data.len());
                    let end = (self.data_end_index + 1).min(chart_data.len()).max(start);
                    chart_data[start..end].to_vec()
                }
                None => return Ok(None),
            },
        };

        // selectGraphicalItemStackedData, on the numerical axis
        let (numeric_type, numeric_id) = if x_categorical {
            (AxisType::YAxis, &y_id)
        } else {
            (AxisType::XAxis, &x_id)
        };
        let stacked_data = settings.stack_id.as_ref().and_then(|stack_id| {
            let group = self
                .stack_groups(numeric_type, numeric_id)
                .into_iter()
                .find(|g| g.stack_id == *stack_id)?;
            let i = group.items.iter().position(|item| *item == settings.id)?;
            group.stacked_data.into_iter().nth(i)
        });

        // computeArea
        let has_stack = stacked_data.as_ref().is_some_and(|s| !s.is_empty());
        let horizontal = self.layout == Layout::Horizontal;
        let numeric_axis = if horizontal { &y_axis } else { &x_axis };
        let numeric_scale = numeric_axis.scale.as_ref().expect("scale");
        let base = base_value(
            settings.base_value.as_ref().unwrap_or(&Value::Undefined),
            &self.base_value,
            numeric_axis,
        );
        let mut is_range = false;
        let mut points = Vec::with_capacity(displayed.len());
        for (index, entry) in displayed.iter().enumerate() {
            let value = match (&stacked_data, has_stack) {
                (Some(series), true) => match series.get(self.data_start_index + index) {
                    Some([a, b]) => Value::Array(vec![Value::Number(*a), Value::Number(*b)]),
                    None => Value::Undefined,
                },
                _ => match value_by_data_key(entry, settings.data_key.as_ref()) {
                    Value::Array(items) => {
                        is_range = true;
                        Value::Array(items)
                    }
                    raw => Value::Array(vec![base.clone(), raw]),
                },
            };
            let value1 = match &value {
                Value::Array(items) => items.get(1).cloned().unwrap_or(Value::Undefined),
                _ => Value::Undefined,
            };
            let is_break_point = value1.is_nullish()
                || (has_stack
                    && !settings.connect_nulls
                    && value_by_data_key(entry, settings.data_key.as_ref()).is_nullish());
            let numeric = if is_break_point {
                None
            } else {
                numeric_scale.map(&value1, None)
            };
            let (x, y) = if horizontal {
                (
                    cate_coordinate(&x_axis, &x_ticks, band_size, entry, index)?,
                    numeric,
                )
            } else {
                (
                    numeric,
                    cate_coordinate(&y_axis, &y_ticks, band_size, entry, index)?,
                )
            };
            points.push(AreaPoint {
                x,
                y,
                value,
                payload: entry.clone(),
            });
        }
        let base_line = if has_stack || is_range {
            AreaBaseLine::Points(
                points
                    .iter()
                    .map(|point| {
                        let start = match &point.value {
                            Value::Array(items) => {
                                items.first().cloned().unwrap_or(Value::Undefined)
                            }
                            _ => Value::Null,
                        };
                        if horizontal {
                            let y = if !start.is_nullish() && point.y.is_some() {
                                numeric_scale.map(&start, None)
                            } else {
                                None
                            };
                            (point.x, y)
                        } else {
                            let x = if start.is_nullish() {
                                None
                            } else {
                                numeric_scale.map(&start, None)
                            };
                            (x, point.y)
                        }
                    })
                    .collect(),
            )
        } else {
            AreaBaseLine::Coordinate(numeric_scale.map(&base, None).unwrap_or(0.0))
        };
        Ok(Some(AreaGeometry {
            points,
            base_line,
            is_range,
        }))
    }
}
