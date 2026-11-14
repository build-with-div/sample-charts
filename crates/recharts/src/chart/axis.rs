//! The axis selectors of `state/selectors/axisSelectors.js` (Recharts
//! 3.8.0) for X and Y axes, with `util/isDomainSpecifiedByUser.js`,
//! `combineRealScaleType.js`, `combineCheckedDomain.js`,
//! `combineAxisRangeWithReverse.js` and the chart offset of
//! `selectChartOffsetInternal.js`. Each function names the selector or
//! combiner it ports and keeps its order of operations.

use js::{math, Key, Value};

use super::props::{
    is_categorical_axis, AxisDomainType, AxisSettings, AxisType, ItemKind, ItemSettings, Layout,
    Padding, StackOffset,
};
use super::scale::{is_supported_scale_name, RechartsScale};
use super::{CartesianChart, ChartError};
use crate::scale::{get_nice_tick_values, get_tick_values_fixed_domain, NiceTicks};

/// One tick of an axis (`CartesianTickItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct Tick {
    pub index: usize,
    pub coordinate: f64,
    pub value: Value,
    pub offset: f64,
}

/// Everything the selectors compute for one axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisResult {
    /// `selectBaseAxis`.
    pub settings: AxisSettings,
    /// `selectAllAppliedValues`, the `value` of each entry.
    pub applied_values: Vec<Value>,
    /// `selectRealScaleType`.
    pub real_scale_type: Option<String>,
    /// `selectAxisDomain`.
    pub domain: Option<Vec<Value>>,
    /// `selectNiceTicks`.
    pub nice_ticks: Option<Vec<f64>>,
    /// `selectAxisDomainIncludingNiceTicks`.
    pub domain_including_nice_ticks: Option<Vec<Value>>,
    /// `selectCheckedAxisDomain`.
    pub checked_domain: Option<Vec<Value>>,
    /// `selectAxisRange`.
    pub range: [f64; 2],
    /// `selectAxisRangeWithReverse`.
    pub range_with_reverse: [f64; 2],
    /// `selectAxisScale`.
    pub scale: Option<RechartsScale>,
    /// `selectTicksOfAxis`.
    pub ticks: Option<Vec<Tick>>,
}

/// `isWellBehavedNumber`: a finite number.
fn finite(value: &Value) -> Option<f64> {
    value.as_number().filter(|n| n.is_finite())
}

/// `isNumber` of `util/DataUtils.js`: a number that is not NaN.
fn is_number(value: &Value) -> bool {
    matches!(value, Value::Number(n) if !n.is_nan())
}

/// `isNumOrStr`.
fn is_num_or_str(value: &Value) -> bool {
    is_number(value) || matches!(value, Value::String(_))
}

/// `isWellFormedNumberDomain`: two finite numbers.
fn well_formed(domain: &[Value]) -> Option<[f64; 2]> {
    match domain {
        [a, b] => Some([finite(a)?, finite(b)?]),
        _ => None,
    }
}

fn numbers(values: [f64; 2]) -> Vec<Value> {
    values.iter().map(|n| Value::Number(*n)).collect()
}

/// `makeNumber`.
fn make_number(value: &Value) -> Option<f64> {
    if !is_num_or_str(value) {
        return None;
    }
    Some(value.to_number()).filter(|n| n.is_finite())
}

/// `makeDomain`.
fn make_domain(value: &Value) -> Option<[f64; 2]> {
    if let Value::Array(items) = value {
        let get = |i: usize| items.get(i).and_then(make_number);
        return Some([get(0)?, get(1)?]);
    }
    let n = make_number(value)?;
    Some([n, n])
}

/// `onlyAllowNumbers`.
fn only_allow_numbers(values: &[Value]) -> Vec<f64> {
    values.iter().filter_map(make_number).collect()
}

/// `mergeDomains(...domains)`.
fn merge_domains(domains: &[Option<[f64; 2]>]) -> Option<[f64; 2]> {
    let all: Vec<f64> = domains.iter().flatten().flatten().copied().collect();
    if all.is_empty() {
        return None;
    }
    Some([
        math::min_of(all.iter().copied()),
        math::max_of(all.iter().copied()),
    ])
}

/// `getDomainDefinition(axisSettings)`.
fn domain_definition(settings: &AxisSettings) -> Vec<Value> {
    if let Some(domain) = &settings.domain {
        return domain.clone();
    }
    if let Some(ticks) = &settings.ticks {
        match settings.r#type {
            AxisDomainType::Number => {
                let all = only_allow_numbers(ticks);
                return vec![
                    Value::Number(math::min_of(all.iter().copied())),
                    Value::Number(math::max_of(all.iter().copied())),
                ];
            }
            AxisDomainType::Category => {
                return ticks
                    .iter()
                    .map(|t| Value::String(t.to_js_string()))
                    .collect();
            }
            AxisDomainType::Auto => {}
        }
    }
    vec![Value::Number(0.0), Value::from("auto")]
}

/// `numericalDomainSpecifiedWithoutRequiringData(userDomain, allowDataOverflow)`.
fn domain_without_data(user_domain: &[Value], allow_data_overflow: bool) -> Option<[f64; 2]> {
    if !allow_data_overflow {
        return None;
    }
    match user_domain {
        [min, max] => Some([finite(min)?, finite(max)?]),
        _ => None,
    }
}

/// `MIN_VALUE_REG` / `MAX_VALUE_REG` of `util/ChartUtils.js`:
/// `^dataMin\s*-\s*(\d+(\.\d+)?)$` and `^dataMax\s*\+\s*(...)$`; returns the
/// number.
fn data_offset(text: &str, keyword: &str, operator: char) -> Option<f64> {
    let is_space = |c: char| {
        // JavaScript's \s: WhiteSpace and LineTerminator.
        matches!(
            c,
            '\u{9}'..='\u{D}'
                | '\u{20}'
                | '\u{A0}'
                | '\u{1680}'
                | '\u{2000}'..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
        )
    };
    let rest = text.strip_prefix(keyword)?;
    let rest = rest.trim_start_matches(is_space).strip_prefix(operator)?;
    let number = rest.trim_start_matches(is_space);
    let (int, frac) = match number.split_once('.') {
        Some((int, frac)) => (int, Some(frac)),
        None => (number, None),
    };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if !digits(int) || frac.is_some_and(|f| !digits(f)) {
        return None;
    }
    Some(js::string_to_number(number))
}

/// `extendDomain(providedDomain, boundaryDomain, allowDataOverflow)`.
fn extend_domain(provided: [f64; 2], boundary: [f64; 2], allow_data_overflow: bool) -> [f64; 2] {
    if allow_data_overflow {
        return provided;
    }
    [
        math::min(provided[0], boundary[0]),
        math::max(provided[1], boundary[1]),
    ]
}

/// `parseNumericalUserDomain(userDomain, dataDomain, allowDataOverflow)`.
fn parse_numerical_user_domain(
    user_domain: &[Value],
    data_domain: Option<[f64; 2]>,
    allow_data_overflow: bool,
) -> Option<[f64; 2]> {
    if !allow_data_overflow && data_domain.is_none() {
        return None;
    }
    let [provided_min, provided_max] = user_domain else {
        return None;
    };
    let bound = |provided: &Value, is_min: bool| -> Option<f64> {
        if matches!(provided, Value::String(s) if s == "auto") {
            return data_domain.map(|[a, b]| {
                if is_min {
                    math::min(a, b)
                } else {
                    math::max(a, b)
                }
            });
        }
        if is_number(provided) {
            return provided.as_number();
        }
        if let Value::String(text) = provided {
            let offset = if is_min {
                data_offset(text, "dataMin", '-')
            } else {
                data_offset(text, "dataMax", '+')
            };
            if let Some(offset) = offset {
                return data_domain.map(|[a, b]| if is_min { a - offset } else { b + offset });
            }
        }
        data_domain.map(|[a, b]| if is_min { a } else { b })
    };
    let min = bound(provided_min, true);
    let max = bound(provided_max, false);
    let candidate = match (min, max) {
        (Some(min), Some(max)) if min.is_finite() && max.is_finite() => [min, max],
        _ => return None,
    };
    Some(match data_domain {
        None => candidate,
        Some(data) => extend_domain(candidate, data, allow_data_overflow),
    })
}

/// `hasDuplicate`: whether two values have the same `String(value)`.
fn has_duplicate(values: &[Value]) -> bool {
    let mut seen = std::collections::HashSet::new();
    values.iter().any(|v| !seen.insert(v.to_js_string()))
}

/// es-toolkit `range(0, n)`.
fn index_range(n: usize) -> Vec<Value> {
    (0..n).map(|i| Value::Number(i as f64)).collect()
}

/// `getPercentValue(percent, totalValue)` with the default 0.
fn get_percent_value(percent: &Value, total: f64) -> f64 {
    let value = match percent {
        Value::Number(n) if !n.is_nan() => *n,
        Value::String(s) => {
            // isPercent: the first '%' is the last character.
            if !s.is_empty() && s.find('%') == Some(s.len() - 1) {
                total * js::parse_float(&s[..s.len() - 1]) / 100.0
            } else {
                js::string_to_number(s)
            }
        }
        _ => return 0.0,
    };
    if value.is_nan() {
        0.0
    } else {
        value
    }
}

/// The chart offset of `selectChartOffsetInternal`: left, right, top,
/// bottom, width, height (no brush, no legend).
struct Offset {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

impl CartesianChart {
    /// `selectBaseAxis`.
    fn base_axis(&self, axis_type: AxisType, id: &Key) -> AxisSettings {
        let axes = match axis_type {
            AxisType::XAxis => &self.x_axes,
            AxisType::YAxis => &self.y_axes,
        };
        axes.get(id)
            .cloned()
            .unwrap_or_else(|| AxisSettings::implicit(axis_type))
    }

    /// `selectHasBar`.
    fn has_bar(&self) -> bool {
        self.items.iter().any(|item| item.kind == ItemKind::Bar)
    }

    /// `selectCartesianItemsSettings`.
    fn axis_items(
        &self,
        axis_type: AxisType,
        id: &Key,
        settings: &AxisSettings,
    ) -> Vec<&ItemSettings> {
        self.items
            .iter()
            .filter(|item| {
                let item_id = match axis_type {
                    AxisType::XAxis => &item.x_axis_id,
                    AxisType::YAxis => &item.y_axis_id,
                };
                item_id.to_value().strict_equals(&id.to_value())
            })
            .filter(|item| settings.include_hidden || !item.hide)
            .collect()
    }

    /// `selectDisplayedData`.
    fn displayed_data(&self, items: &[&ItemSettings]) -> Vec<Value> {
        let items_data: Vec<Value> = items
            .iter()
            .filter_map(|item| item.data.as_ref())
            .flatten()
            .cloned()
            .collect();
        if !items_data.is_empty() {
            return items_data;
        }
        let chart_data = self.chart_data.as_deref().unwrap_or(&[]);
        let start = self.data_start_index.min(chart_data.len());
        let end = (self.data_end_index + 1).min(chart_data.len()).max(start);
        chart_data[start..end].to_vec()
    }

    /// `combineAppliedValues`.
    fn applied_values(
        data: &[Value],
        settings: &AxisSettings,
        items: &[&ItemSettings],
    ) -> Vec<Value> {
        if let Some(key) = &settings.data_key {
            return data.iter().map(|entry| key.value_of(entry)).collect();
        }
        if !items.is_empty() {
            return items
                .iter()
                .flat_map(|item| {
                    data.iter().map(move |entry| match &item.data_key {
                        Some(key) => key.value_of(entry),
                        None => Value::Undefined,
                    })
                })
                .collect();
        }
        data.to_vec()
    }

    /// `selectChartOffsetInternal`.
    fn offset(&self) -> Offset {
        let visible = |s: &&AxisSettings, side: &str| s.orientation == side && !s.mirror && !s.hide;
        let sum = |axes: Vec<&AxisSettings>, side: &str| {
            axes.into_iter()
                .filter(|s| visible(s, side))
                .fold(0.0, |total, s| total + s.size)
        };
        let [top, right, bottom, left] = self.margin;
        let left = left + sum(self.y_axes.values(), "left");
        let right = right + sum(self.y_axes.values(), "right");
        let top = top + sum(self.x_axes.values(), "top");
        let bottom = bottom + sum(self.x_axes.values(), "bottom");
        Offset {
            left,
            top,
            width: math::max(self.width - left - right, 0.0),
            height: math::max(self.height - top - bottom, 0.0),
        }
    }

    /// `selectSmallestDistanceBetweenValues`.
    fn smallest_distance(applied: &[Value], settings: &AxisSettings) -> Option<f64> {
        if settings.r#type != AxisDomainType::Number {
            return None;
        }
        let mut sorted = only_allow_numbers(applied);
        sorted.sort_by(|a, b| a.partial_cmp(b).expect("finite numbers"));
        let (Some(first), Some(last)) = (sorted.first(), sorted.last()) else {
            return Some(f64::INFINITY);
        };
        let diff = last - first;
        if diff == 0.0 {
            return Some(f64::INFINITY);
        }
        let smallest = sorted.windows(2).fold(f64::INFINITY, |smallest, pair| {
            math::min(smallest, pair[1] - pair[0])
        });
        Some(smallest / diff)
    }

    /// `selectCalculatedPadding` for a keyword padding.
    fn calculated_padding(&self, keyword: &str, applied: &[Value], settings: &AxisSettings) -> f64 {
        let Some(smallest) = Self::smallest_distance(applied, settings).filter(|n| n.is_finite())
        else {
            return 0.0;
        };
        let offset = self.offset();
        let range_width = if self.layout == Layout::Vertical {
            offset.height
        } else {
            offset.width
        };
        match keyword {
            "gap" => smallest * range_width / 2.0,
            "no-gap" => {
                let gap = get_percent_value(&self.bar_category_gap, smallest * range_width);
                let half_band = smallest * range_width / 2.0;
                half_band - gap - (half_band - gap) / range_width * gap
            }
            _ => 0.0,
        }
    }

    /// `selectXAxisRange` / `selectYAxisRange` (not in a brush panorama).
    fn axis_range(
        &self,
        axis_type: AxisType,
        applied: &[Value],
        settings: &AxisSettings,
    ) -> [f64; 2] {
        let offset = self.offset();
        let (start, end) = match &settings.padding {
            Padding::Keyword(keyword) => {
                let calculated = self.calculated_padding(keyword, applied, settings);
                (calculated, calculated)
            }
            Padding::Sides {
                left,
                right,
                top,
                bottom,
            } => match axis_type {
                AxisType::XAxis => (left.unwrap_or(0.0), right.unwrap_or(0.0)),
                AxisType::YAxis => (top.unwrap_or(0.0), bottom.unwrap_or(0.0)),
            },
        };
        match axis_type {
            AxisType::XAxis => [offset.left + start, offset.left + offset.width - end],
            AxisType::YAxis => {
                if self.layout == Layout::Horizontal {
                    [offset.top + offset.height - end, offset.top + start]
                } else {
                    [offset.top + start, offset.top + offset.height - end]
                }
            }
        }
    }

    /// `selectNumericalDomain` (via `combineNumericalDomain`).
    fn numerical_domain(
        &self,
        axis_type: AxisType,
        settings: &AxisSettings,
        items: &[&ItemSettings],
        data: &[Value],
    ) -> Result<Option<[f64; 2]>, ChartError> {
        let definition = domain_definition(settings);
        let from_user = domain_without_data(&definition, settings.allow_data_overflow);
        if from_user.is_some() {
            return Ok(from_user);
        }
        let include_stacks = matches!(
            (self.layout, axis_type),
            (Layout::Vertical, AxisType::XAxis) | (Layout::Horizontal, AxisType::YAxis)
        );
        let stacked = items
            .iter()
            .any(|item| matches!(item.kind, ItemKind::Area | ItemKind::Bar) && item.is_stacked());
        if include_stacks && stacked {
            return Err(ChartError::Unsupported("stacked series (milestone M3)"));
        }
        // selectDomainOfAllAppliedNumericalValuesIncludingErrorValues, with
        // the items that are not stacked and no error bars.
        let not_stacked: Vec<&&ItemSettings> = items
            .iter()
            .filter(|item| item.stack_id.is_none())
            .collect();
        let mut lower: Option<f64> = None;
        let mut upper: Option<f64> = None;
        let mut include = |value: &Value| {
            if let Some([a, b]) = make_domain(value) {
                lower = Some(lower.map_or(a, |l| math::min(l, a)));
                upper = Some(upper.map_or(b, |u| math::max(u, b)));
            }
        };
        if !not_stacked.is_empty() {
            for entry in data {
                for item in &not_stacked {
                    let key = settings.data_key.as_ref().or(item.data_key.as_ref());
                    let value = key.map_or(Value::Undefined, |key| key.value_of(entry));
                    include(&value);
                }
            }
        }
        if let Some(key) = &settings.data_key {
            for entry in data {
                include(&key.value_of(entry));
            }
        }
        let data_domain = match (lower, upper) {
            (Some(l), Some(u)) if l.is_finite() && u.is_finite() => Some([l, u]),
            _ => None,
        };
        let merged = merge_domains(&[data_domain]);
        Ok(parse_numerical_user_domain(
            &definition,
            merged,
            settings.allow_data_overflow,
        ))
    }

    /// `combineAxisDomain`.
    fn axis_domain(
        &self,
        axis_type: AxisType,
        settings: &AxisSettings,
        data: &[Value],
        applied: &[Value],
        numerical: Option<[f64; 2]>,
    ) -> Option<Vec<Value>> {
        if data.is_empty() && numerical.is_none() {
            return None;
        }
        let categorical = is_categorical_axis(self.layout, axis_type);
        if categorical && settings.data_key.is_none() {
            return Some(index_range(data.len()));
        }
        if settings.r#type == AxisDomainType::Category {
            // computeDomainOfTypeCategory
            let domain: Vec<Value> = applied
                .iter()
                .filter(|v| is_num_or_str(v))
                .cloned()
                .collect();
            if categorical
                && (settings.data_key.is_none()
                    || settings.allow_duplicated_category && has_duplicate(&domain))
            {
                return Some(index_range(applied.len()));
            }
            if settings.allow_duplicated_category {
                return Some(domain);
            }
            let mut seen = std::collections::HashSet::new();
            return Some(
                domain
                    .into_iter()
                    .filter(|v| seen.insert(Key::of(v).expect("numbers and strings")))
                    .collect(),
            );
        }
        if self.stack_offset == StackOffset::Expand {
            return Some(vec![Value::Number(0.0), Value::Number(1.0)]);
        }
        numerical.map(numbers)
    }

    /// `combineRealScaleType`.
    fn real_scale_type(&self, settings: &AxisSettings) -> Option<String> {
        if settings.scale == "auto" {
            let name = &self.chart_name;
            if settings.r#type == AxisDomainType::Category
                && (name.contains("LineChart")
                    || name.contains("AreaChart")
                    || name.contains("ComposedChart") && !self.has_bar())
            {
                return Some("point".into());
            }
            if settings.r#type == AxisDomainType::Category {
                return Some("band".into());
            }
            return Some("linear".into());
        }
        Some(if is_supported_scale_name(&settings.scale) {
            settings.scale.clone()
        } else {
            "point".into()
        })
    }

    /// `combineNiceTicks`.
    fn nice_ticks(
        settings: &AxisSettings,
        domain: Option<&[Value]>,
        real_scale_type: Option<&str>,
    ) -> Result<Option<Vec<f64>>, ChartError> {
        if settings.nice_ticks == "none" {
            return Ok(None);
        }
        let definition = domain_definition(settings);
        let is_auto = |v: Option<&Value>| matches!(v, Some(Value::String(s)) if s == "auto");
        let has_auto = is_auto(definition.first()) || is_auto(definition.get(1));
        let well = domain.and_then(well_formed);
        let count_truthy = settings.tick_count != 0.0 && !settings.tick_count.is_nan();
        let tick_count = || -> Result<u32, ChartError> {
            let n = settings.tick_count;
            if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) {
                Ok(n as u32)
            } else {
                Err(ChartError::Unsupported(
                    "tickCount that is not a whole number",
                ))
            }
        };
        let mode = match settings.nice_ticks.as_str() {
            "snap125" => Some(NiceTicks::Snap125),
            "adaptive" => Some(NiceTicks::Auto),
            _ => None,
        };
        if let (Some(mode), true, Some(domain)) = (mode, count_truthy, well) {
            if has_auto {
                return Ok(Some(get_nice_tick_values(
                    domain,
                    tick_count()?,
                    settings.allow_decimals,
                    mode,
                )?));
            }
            if settings.r#type == AxisDomainType::Number {
                return Ok(Some(get_tick_values_fixed_domain(
                    domain,
                    tick_count()?,
                    settings.allow_decimals,
                    mode,
                )?));
            }
        }
        if settings.nice_ticks == "auto" && real_scale_type == Some("linear") && count_truthy {
            if let (true, Some(domain)) = (has_auto, well) {
                return Ok(Some(get_nice_tick_values(
                    domain,
                    tick_count()?,
                    settings.allow_decimals,
                    NiceTicks::Auto,
                )?));
            }
            if let (AxisDomainType::Number, Some(domain)) = (settings.r#type, well) {
                return Ok(Some(get_tick_values_fixed_domain(
                    domain,
                    tick_count()?,
                    settings.allow_decimals,
                    NiceTicks::Auto,
                )?));
            }
        }
        Ok(None)
    }

    /// Every selector for the X or Y axis `id`.
    pub fn axis(&self, axis_type: AxisType, id: &Key) -> Result<AxisResult, ChartError> {
        let settings = self.base_axis(axis_type, id);
        let items = self.axis_items(axis_type, id, &settings);
        let data = self.displayed_data(&items);
        let applied = Self::applied_values(&data, &settings, &items);
        let numerical = self.numerical_domain(axis_type, &settings, &items, &data)?;
        let domain = self.axis_domain(axis_type, &settings, &data, &applied, numerical);
        let real_scale_type = self.real_scale_type(&settings);
        let nice_ticks =
            Self::nice_ticks(&settings, domain.as_deref(), real_scale_type.as_deref())?;

        // combineAxisDomainWithNiceTicks
        let domain_including_nice_ticks = match (&domain, &nice_ticks) {
            (Some(d), Some(ticks))
                if settings.r#type == AxisDomainType::Number && !ticks.is_empty() =>
            {
                match well_formed(d) {
                    Some([min, max]) => Some(numbers([
                        math::min(min, ticks[0]),
                        math::max(max, ticks[ticks.len() - 1]),
                    ])),
                    None => domain.clone(),
                }
            }
            _ => domain.clone(),
        };

        // combineCheckedDomain
        let checked_domain = match (&domain_including_nice_ticks, real_scale_type.as_deref()) {
            (Some(d), Some("linear")) if well_formed(d).is_none() => {
                let values: Vec<f64> = d.iter().filter_map(finite).collect();
                if values.is_empty() {
                    None
                } else {
                    Some(numbers([
                        values.iter().copied().fold(f64::NAN, |m, v| {
                            if m.is_nan() || v < m {
                                v
                            } else {
                                m
                            }
                        }),
                        values.iter().copied().fold(f64::NAN, |m, v| {
                            if m.is_nan() || v > m {
                                v
                            } else {
                                m
                            }
                        }),
                    ]))
                }
            }
            (d, _) => d.clone(),
        };

        let range = self.axis_range(axis_type, &applied, &settings);
        let range_with_reverse = if settings.reversed {
            [range[1], range[0]]
        } else {
            range
        };

        let scale = match (&real_scale_type, &checked_domain) {
            (Some(name), Some(d)) => RechartsScale::configure(name, d, range_with_reverse)?,
            _ => None,
        };

        let ticks = match &scale {
            Some(scale) => Some(self.axis_ticks(
                axis_type,
                &settings,
                scale,
                nice_ticks.as_deref(),
                &applied,
            )?),
            None => None,
        };

        Ok(AxisResult {
            settings,
            applied_values: applied,
            real_scale_type,
            domain,
            nice_ticks,
            domain_including_nice_ticks,
            checked_domain,
            range,
            range_with_reverse,
            scale,
            ticks,
        })
    }

    /// `combineAxisTicks` with `combineDuplicateDomain` and
    /// `combineCategoricalDomain`.
    fn axis_ticks(
        &self,
        axis_type: AxisType,
        settings: &AxisSettings,
        scale: &RechartsScale,
        nice_ticks: Option<&[f64]>,
        applied: &[Value],
    ) -> Result<Vec<Tick>, ChartError> {
        let categorical = is_categorical_axis(self.layout, axis_type);
        // `dataKey && …`: a truthy data key.
        let data_key_truthy = match &settings.data_key {
            Some(super::DataKey::Name(name)) => !name.is_empty(),
            Some(super::DataKey::Index(n)) => *n != 0.0 && !n.is_nan(),
            None => false,
        };
        let duplicate_domain = (data_key_truthy
            && categorical
            && settings.r#type == AxisDomainType::Category
            && settings.allow_duplicated_category
            && has_duplicate(applied))
        .then(|| applied.to_vec());
        let categorical_domain = (settings.data_key.is_some()
            && categorical
            && (settings.r#type == AxisDomainType::Number || settings.scale != "auto"))
            .then(|| applied.to_vec());

        let offset = match scale.bandwidth() {
            Some(bandwidth) if settings.r#type == AxisDomainType::Category => bandwidth / 2.0,
            _ => 0.0,
        };
        let tick = |index: usize, input: &Value, value: Value| -> Option<Tick> {
            let scaled = scale.map(input, None).filter(|n| n.is_finite())?;
            Some(Tick {
                index,
                coordinate: scaled + offset,
                value,
                offset,
            })
        };

        let user_or_nice: Option<Vec<Value>> = settings
            .ticks
            .clone()
            .or_else(|| nice_ticks.map(|t| t.iter().map(|n| Value::Number(*n)).collect()));
        if let Some(entries) = user_or_nice {
            return Ok(entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    let content = match &duplicate_domain {
                        Some(d) => Value::Number(
                            d.iter()
                                .position(|v| v.strict_equals(entry))
                                .map_or(-1.0, |i| i as f64),
                        ),
                        None => entry.clone(),
                    };
                    tick(index, &content, entry.clone())
                })
                .collect());
        }
        if let (true, Some(domain)) = (categorical, &categorical_domain) {
            return Ok(domain
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| tick(index, entry, entry.clone()))
                .collect());
        }
        if let Some(ticks) = scale.ticks(settings.tick_count) {
            return Ok(ticks?
                .iter()
                .enumerate()
                .filter_map(|(index, n)| tick(index, &Value::Number(*n), Value::Number(*n)))
                .collect());
        }
        Ok(scale
            .domain()
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                let value = match &duplicate_domain {
                    Some(d) => Value::Array(d.clone()).get(&entry.to_js_string()),
                    None => entry.clone(),
                };
                tick(index, entry, value)
            })
            .collect())
    }
}
