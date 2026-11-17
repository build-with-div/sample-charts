//! Line geometry: `selectLinePoints` of `state/selectors/lineSelectors.js`
//! with `computeLinePoints` of `cartesian/Line.js` and
//! `getCateCoordinateOfLine` of `util/ChartUtils.js`.

use js::Value;

use super::axis::{AxisResult, Tick};
use super::bar::band_size_of_axis;
use super::props::{is_categorical_axis, AxisDomainType, AxisType, DataKey, ItemKind, Layout};
use super::{CartesianChart, ChartError};

/// One point of a line: `null` coordinates are `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct LinePoint {
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub value: Value,
    pub payload: Value,
}

/// `isNumber` of `util/DataUtils.js`: a number that is not NaN.
fn is_number(n: f64) -> bool {
    !n.is_nan()
}

/// `getValueByDataKey(entry, dataKey)`.
pub(super) fn value_by_data_key(entry: &Value, key: Option<&DataKey>) -> Value {
    match key {
        Some(key) => key.value_of(entry),
        None => Value::Undefined,
    }
}

/// `axis.dataKey` as a property name for `entry[axis.dataKey]`, when it is
/// truthy.
fn direct_key(key: Option<&DataKey>) -> Option<String> {
    match key {
        Some(DataKey::Name(name)) if !name.is_empty() => Some(name.clone()),
        Some(DataKey::Index(n)) if *n != 0.0 && !n.is_nan() => Some(js::number_to_string(*n)),
        _ => None,
    }
}

/// `getCateCoordinateOfLine({ axis, ticks, bandSize, entry, index })`.
pub(super) fn cate_coordinate(
    axis: &AxisResult,
    ticks: &[Tick],
    band_size: f64,
    entry: &Value,
    index: usize,
) -> Result<Option<f64>, ChartError> {
    let settings = &axis.settings;
    if settings.r#type == AxisDomainType::Category {
        if !settings.allow_duplicated_category {
            if let Some(key) = direct_key(settings.data_key.as_ref()) {
                if entry.is_nullish() {
                    return Err(ChartError::Unsupported(
                        "a data entry that is not an object",
                    ));
                }
                let category = entry.get(&key);
                if !category.is_nullish() {
                    if let Some(tick) = ticks.iter().find(|t| t.value.strict_equals(&category)) {
                        return Ok(Some(tick.coordinate + band_size / 2.0));
                    }
                }
            }
        }
        return Ok(ticks.get(index).map(|t| t.coordinate + band_size / 2.0));
    }
    let value = value_by_data_key(entry, settings.data_key.as_ref());
    let scale = axis.scale.as_ref().expect("an axis with a scale");
    Ok(scale.map(&value, None).filter(|n| is_number(*n)))
}

impl CartesianChart {
    /// `selectLinePoints` for the `<Line>` with this `id`: `None` where the
    /// selector gives `undefined`.
    pub fn line_points(&self, id: &str) -> Result<Option<Vec<LinePoint>>, ChartError> {
        let x_id = self.item_axis_id(AxisType::XAxis, id);
        let y_id = self.item_axis_id(AxisType::YAxis, id);
        let x_axis = self.axis(AxisType::XAxis, &x_id)?;
        let y_axis = self.axis(AxisType::YAxis, &y_id)?;
        let x_ticks = self.graphical_item_ticks(AxisType::XAxis, &x_axis)?;
        let y_ticks = self.graphical_item_ticks(AxisType::YAxis, &y_axis)?;
        let settings = self
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Line && item.id == id);
        let with_scale = |axis: &AxisResult| axis.scale.is_some();
        // selectBandSize
        let band_size = if is_categorical_axis(self.layout, AxisType::XAxis) {
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

        // computeLinePoints
        let mut points = Vec::with_capacity(displayed.len());
        for (index, entry) in displayed.iter().enumerate() {
            let value = value_by_data_key(entry, settings.data_key.as_ref());
            match self.layout {
                Layout::Horizontal => {
                    let x = cate_coordinate(&x_axis, &x_ticks, band_size, entry, index)?;
                    let y = if value.is_nullish() {
                        None
                    } else {
                        y_axis.scale.as_ref().expect("scale").map(&value, None)
                    };
                    points.push(LinePoint {
                        x,
                        y,
                        value,
                        payload: entry.clone(),
                    });
                }
                Layout::Vertical => {
                    let x = if value.is_nullish() {
                        None
                    } else {
                        x_axis.scale.as_ref().expect("scale").map(&value, None)
                    };
                    let y = cate_coordinate(&y_axis, &y_ticks, band_size, entry, index)?;
                    if let (Some(x), Some(y)) = (x, y) {
                        points.push(LinePoint {
                            x: Some(x),
                            y: Some(y),
                            value,
                            payload: entry.clone(),
                        });
                    }
                }
            }
        }
        Ok(Some(points))
    }
}
