//! Stacked series: `combineDisplayedStackedData`
//! (`state/selectors/combiners/combineDisplayedStackedData.js`),
//! `combineStackGroups` and `combineDomainOfStackGroups`
//! (`state/selectors/axisSelectors.js`), and `getStackedData`,
//! `offsetSign`, `offsetPositive` and `getDomainOfStackGroups`
//! (`util/ChartUtils.js`), over d3-shape's stack.

use std::collections::HashMap;

use d3::shape::stack::{self, Series};
use js::{math, Key, Value};

use super::props::{AxisType, ItemKind, ItemSettings, Layout, StackOffset};
use super::CartesianChart;

/// One stack (`StackGroup`): the items with the same `stackId` on an axis,
/// in stacking order, and their stacked `[y0, y1]` per displayed datum.
#[derive(Debug, Clone, PartialEq)]
pub struct StackGroup {
    /// `getNormalizedStackId(stackId)`.
    pub stack_id: String,
    /// The `id` of each item (`graphicalItems`), in stacking order.
    pub items: Vec<String>,
    /// `stackedData`: one series per item, in the same order.
    pub stacked_data: Vec<Series>,
}

/// One datum of `combineDisplayedStackedData`: an object from series
/// identifier (the item's `id`) to the item's value, in insertion order.
type StackedDatum = Vec<(String, Value)>;

/// `getValueByDataKey(d, key, 0)` for a series identifier.
fn series_value(datum: &StackedDatum, id: &str) -> Value {
    datum
        .iter()
        .find(|(k, _)| k == id)
        .map_or(Value::Number(0.0), |(_, v)| v.clone())
}

/// `isNumber`: a number that is not NaN.
fn is_number(value: &Value) -> Option<f64> {
    value.as_number().filter(|n| !n.is_nan())
}

/// `offsetSign`: positive values stack up from zero, negative ones down.
fn offset_sign(series: &mut [Series], _order: &[usize]) {
    let m = match series.first() {
        Some(first) if !first.is_empty() => first.len(),
        _ => return,
    };
    for j in 0..m {
        let (mut positive, mut negative) = (0.0, 0.0);
        for row in series.iter_mut() {
            let Some(col) = row.get_mut(j) else { continue };
            let value = if col[1].is_nan() { col[0] } else { col[1] };
            if value >= 0.0 {
                col[0] = positive;
                positive += value;
                col[1] = positive;
            } else {
                col[0] = negative;
                negative += value;
                col[1] = negative;
            }
        }
    }
}

/// `offsetPositive`: positive values stack up from zero, negative ones
/// become `[0, 0]`.
fn offset_positive(series: &mut [Series], _order: &[usize]) {
    let m = match series.first() {
        Some(first) if !first.is_empty() => first.len(),
        _ => return,
    };
    for j in 0..m {
        let mut positive = 0.0;
        for row in series.iter_mut() {
            let Some(col) = row.get_mut(j) else { continue };
            let value = if col[1].is_nan() { col[0] } else { col[1] };
            if value >= 0.0 {
                col[0] = positive;
                positive += value;
                col[1] = positive;
            } else {
                col[0] = 0.0;
                col[1] = 0.0;
            }
        }
    }
}

/// `getStackedData(data, dataKeys, offsetType)`.
fn stacked_data(data: &[StackedDatum], ids: &[String], offset: &StackOffset) -> Vec<Series> {
    // .value((d, key) => Number(getValueByDataKey(d, key, 0)))
    let values: Vec<Vec<f64>> = ids
        .iter()
        .map(|id| {
            data.iter()
                .map(|d| series_value(d, id).to_number())
                .collect()
        })
        .collect();
    let mut result = match offset {
        StackOffset::Expand => stack::stack(&values, stack::offset_expand),
        StackOffset::Silhouette => stack::stack(&values, stack::offset_silhouette),
        StackOffset::Wiggle => stack::stack(&values, stack::offset_wiggle),
        StackOffset::Sign => stack::stack(&values, offset_sign),
        StackOffset::Positive => stack::stack(&values, offset_positive),
        StackOffset::None | StackOffset::Other(_) => stack::stack(&values, stack::offset_none),
    };
    // A value that is an array of two numbers is a range: drawn as given.
    for (series, id) in result.iter_mut().zip(ids) {
        for (point, datum) in series.iter_mut().zip(data) {
            if let Value::Array(range) = series_value(datum, id) {
                if let [Some(low), Some(high)] = [
                    range.first().and_then(is_number),
                    range.get(1).and_then(is_number),
                ] {
                    if range.len() == 2 {
                        *point = [low, high];
                    }
                }
            }
        }
    }
    result
}

/// Whether `key` is an array index, which a JavaScript object lists first.
fn array_index(key: &str) -> Option<u32> {
    let n: u32 = key.parse().ok()?;
    (n != u32::MAX && n.to_string() == key).then_some(n)
}

impl CartesianChart {
    /// `selectStackedCartesianItemsSettings`.
    fn stacked_items<'a>(items: &[&'a ItemSettings]) -> Vec<&'a ItemSettings> {
        items
            .iter()
            .filter(|item| matches!(item.kind, ItemKind::Area | ItemKind::Bar) && item.is_stacked())
            .copied()
            .collect()
    }

    /// `combineDisplayedStackedData`: the chart's data merged by tooltip
    /// category (or by index), one value per stacked item.
    fn displayed_stacked_data(&self, stacked: &[&ItemSettings]) -> Vec<StackedDatum> {
        // selectTooltipAxis: the categorical axis with the tooltip's axisId
        // (0; there is no <Tooltip> here).
        let tooltip_axis_type = match self.layout {
            Layout::Horizontal => AxisType::XAxis,
            Layout::Vertical => AxisType::YAxis,
        };
        let tooltip = self.base_axis(tooltip_axis_type, &Key::number(0.0));
        let chart_data = self.chart_data.as_deref().unwrap_or(&[]);
        let mut index_of: HashMap<String, usize> = HashMap::new();
        let mut data: Vec<StackedDatum> = Vec::new();
        for item in stacked {
            let resolved = item.data.as_deref().unwrap_or(chart_data);
            if resolved.is_empty() {
                continue;
            }
            let data_key = item
                .data_key
                .as_ref()
                .expect("stacked items have a dataKey");
            for (index, entry) in resolved.iter().enumerate() {
                // Map keys: the index (a number) or String(value); the two
                // never mix within one chart.
                let key = match &tooltip.data_key {
                    Some(key) if !tooltip.allow_duplicated_category => {
                        let value = key.value_of(entry);
                        let value = if value == Value::Undefined {
                            Value::Null
                        } else {
                            value
                        };
                        value.to_js_string()
                    }
                    _ => index.to_string(),
                };
                let numeric = match data_key.value_of(entry) {
                    Value::Undefined => Value::Number(0.0),
                    value => value,
                };
                let position = *index_of.entry(key).or_insert_with(|| {
                    data.push(Vec::new());
                    data.len() - 1
                });
                let datum = &mut data[position];
                match datum.iter_mut().find(|(k, _)| *k == item.id) {
                    Some(slot) => slot.1 = numeric,
                    None => datum.push((item.id.clone(), numeric)),
                }
            }
        }
        data
    }

    /// `combineStackGroups` over the items of an axis, in the order of the
    /// object's keys (array indices ascending, then insertion order).
    pub(super) fn stack_groups_of(&self, items: &[&ItemSettings]) -> Vec<StackGroup> {
        let stacked = Self::stacked_items(items);
        let data = self.displayed_stacked_data(&stacked);
        let mut groups: Vec<(String, Vec<&ItemSettings>)> = Vec::new();
        for item in &stacked {
            let id = item.stack_id.clone().expect("stacked items have a stackId");
            match groups.iter_mut().find(|(k, _)| *k == id) {
                Some(group) => group.1.push(item),
                None => groups.push((id, vec![item])),
            }
        }
        let mut indexed: Vec<(u32, usize)> = groups
            .iter()
            .enumerate()
            .filter_map(|(i, (k, _))| array_index(k).map(|n| (n, i)))
            .collect();
        indexed.sort();
        let order: Vec<usize> = indexed
            .into_iter()
            .map(|(_, i)| i)
            .chain(
                groups
                    .iter()
                    .enumerate()
                    .filter(|(_, (k, _))| array_index(k).is_none())
                    .map(|(i, _)| i),
            )
            .collect();
        order
            .into_iter()
            .map(|i| {
                let (stack_id, items) = &groups[i];
                let mut items: Vec<&ItemSettings> = items.clone();
                if self.reverse_stack_order {
                    items.reverse();
                }
                let ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
                StackGroup {
                    stack_id: stack_id.clone(),
                    stacked_data: stacked_data(&data, &ids, &self.stack_offset),
                    items: ids,
                }
            })
            .collect()
    }

    /// `selectStackGroups` for the X or Y axis `id`.
    pub fn stack_groups(&self, axis_type: AxisType, id: &Key) -> Vec<StackGroup> {
        let settings = self.base_axis(axis_type, id);
        let items = self.axis_items(axis_type, id, &settings);
        self.stack_groups_of(&items)
    }

    /// `combineDomainOfStackGroups` (the user gave no domain) with
    /// `getDomainOfStackGroups`.
    pub(super) fn domain_of_stack_groups(&self, groups: &[StackGroup]) -> Option<[f64; 2]> {
        let (start, end) = (self.data_start_index, self.data_end_index);
        let mut result = [f64::INFINITY, f64::NEG_INFINITY];
        for group in groups {
            let mut domain = [f64::INFINITY, f64::NEG_INFINITY];
            for series in &group.stacked_data {
                // getSliced
                let sliced: &[[f64; 2]] = if start + end != 0 {
                    let from = start.min(series.len());
                    let to = (end + 1).min(series.len()).max(from);
                    &series[from..to]
                } else {
                    series
                };
                // getDomainOfSingle: the numbers of data.flat(2)
                let flat = sliced.iter().flatten().copied().filter(|n| !n.is_nan());
                let s = [math::min_of(flat.clone()), math::max_of(flat)];
                if !s[0].is_finite() || !s[1].is_finite() {
                    continue;
                }
                domain = [math::min(domain[0], s[0]), math::max(domain[1], s[1])];
            }
            result = [
                math::min(domain[0], result[0]),
                math::max(domain[1], result[1]),
            ];
        }
        // makeDomainFinite
        let finite = [
            if result[0] == f64::INFINITY {
                0.0
            } else {
                result[0]
            },
            if result[1] == f64::NEG_INFINITY {
                0.0
            } else {
                result[1]
            },
        ];
        (finite != [0.0, 0.0]).then_some(finite)
    }
}
