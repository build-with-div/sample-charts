//! The props a page passes to `<BarChart>`, `<XAxis>`, `<Bar>` and the
//! like, and the settings Recharts derives from them and keeps in its store.
//!
//! Ports the default props and settings dispatch of `chart/CartesianChart.js`,
//! `cartesian/XAxis.js` (`xAxisDefaultProps`, `SetXAxisSettings`),
//! `cartesian/YAxis.js`, `cartesian/Bar.js`, `cartesian/Line.js` and
//! `cartesian/Area.js`, and the implicit axes of
//! `state/selectors/axisSelectors.js`. A prop left `None` is a prop the page
//! did not pass (`undefined`), which `resolveDefaultProps` fills.
//!
//! Props whose value is a function (`domain` functions, `dataKey`
//! functions, custom scales, `tickFormatter`) are not modelled yet.

use js::{Key, Value};

/// `layout` of a cartesian chart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Horizontal,
    Vertical,
}

impl Layout {
    pub fn from_name(name: &str) -> Option<Layout> {
        match name {
            "horizontal" => Some(Layout::Horizontal),
            "vertical" => Some(Layout::Vertical),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisType {
    XAxis,
    YAxis,
}

/// `isCategoricalAxis(layout, axisType)` of `util/ChartUtils.js`.
pub fn is_categorical_axis(layout: Layout, axis_type: AxisType) -> bool {
    matches!(
        (layout, axis_type),
        (Layout::Horizontal, AxisType::XAxis) | (Layout::Vertical, AxisType::YAxis)
    )
}

/// A `dataKey`: a property name or path (`"a.b"`, `"a[0]"`), or a number.
#[derive(Debug, Clone, PartialEq)]
pub enum DataKey {
    Name(String),
    Index(f64),
}

impl DataKey {
    /// `getValueByDataKey(obj, dataKey)` of `util/ChartUtils.js`, which uses
    /// es-toolkit's `compat/get`.
    pub fn value_of(&self, obj: &Value) -> Value {
        if obj.is_nullish() {
            return Value::Undefined;
        }
        match self {
            DataKey::Index(n) => {
                let key = if *n == 0.0 && n.is_sign_negative() {
                    "-0".to_owned()
                } else {
                    js::number_to_string(*n)
                };
                obj.get(&key)
            }
            DataKey::Name(path) => {
                if path == "__proto__" {
                    return Value::Undefined;
                }
                let direct = obj.get(path);
                if direct != Value::Undefined {
                    return direct;
                }
                if !is_deep_key(path) {
                    return Value::Undefined;
                }
                let mut current = obj.clone();
                let segments = to_path(path);
                if segments.is_empty() {
                    return Value::Undefined;
                }
                for segment in segments {
                    if current.is_nullish() || segment == "__proto__" {
                        return Value::Undefined;
                    }
                    current = current.get(&segment);
                }
                current
            }
        }
    }
}

/// es-toolkit `compat/_internal/isDeepKey`: whether a string key is a path.
fn is_deep_key(key: &str) -> bool {
    if key.is_empty() || key.starts_with('.') || key.ends_with('.') {
        return false;
    }
    // /\.|(\[(?:[^[\]]*|(["'])(?:(?!\2)[^\\]|\\.)*?\2)\])/
    if key.contains('.') {
        return true;
    }
    let chars: Vec<char> = key.chars().collect();
    for start in 0..chars.len() {
        if chars[start] != '[' {
            continue;
        }
        // `[` then anything without brackets, then `]`.
        let mut i = start + 1;
        while i < chars.len() && chars[i] != '[' && chars[i] != ']' {
            i += 1;
        }
        if i < chars.len() && chars[i] == ']' {
            return true;
        }
        // `[` then a quoted string (with escapes), then `]`.
        if let Some(&quote) = chars.get(start + 1) {
            if quote == '"' || quote == '\'' {
                let mut j = start + 2;
                while j < chars.len() {
                    if chars[j] == '\\' {
                        j += 2;
                        continue;
                    }
                    if chars[j] == quote {
                        if chars.get(j + 1) == Some(&']') {
                            return true;
                        }
                        // The lazy match may extend past this quote only
                        // through another escape or non-quote character,
                        // which cannot be the same quote: stop here.
                        break;
                    }
                    j += 1;
                }
            }
        }
    }
    false
}

/// es-toolkit `compat/util/toPath` for a string path.
fn to_path(deep_key: &str) -> Vec<String> {
    let chars: Vec<char> = deep_key.chars().collect();
    let mut result = Vec::new();
    if chars.is_empty() {
        return result;
    }
    let mut index = 0;
    let mut key = String::new();
    let mut quote: Option<char> = None;
    let mut bracket = false;
    let mut bracket_had_quote = false;
    let is_bracket_number = |key: &str| {
        // /^-?\d+(?:\.\d+)?$/
        let body = key.strip_prefix('-').unwrap_or(key);
        let (int, frac) = match body.split_once('.') {
            Some((int, frac)) => (int, Some(frac)),
            None => (body, None),
        };
        !int.is_empty()
            && int.chars().all(|c| c.is_ascii_digit())
            && frac.is_none_or(|f| !f.is_empty() && f.chars().all(|c| c.is_ascii_digit()))
    };
    if chars[0] == '.' {
        result.push(String::new());
    }
    while index < chars.len() {
        let c = chars[index];
        if let Some(q) = quote {
            if c == '\\' && index + 1 < chars.len() {
                index += 1;
                key.push(chars[index]);
            } else if c == q {
                quote = None;
            } else {
                key.push(c);
            }
        } else if bracket {
            if c == '"' || c == '\'' {
                quote = Some(c);
                bracket_had_quote = true;
            } else if c == ']' {
                bracket = false;
                if !bracket_had_quote && key.contains('.') && !is_bracket_number(&key) {
                    result.extend(key.split('.').filter(|s| !s.is_empty()).map(String::from));
                } else {
                    result.push(std::mem::take(&mut key));
                }
                key.clear();
            } else {
                key.push(c);
            }
        } else if c == '[' {
            bracket = true;
            bracket_had_quote = false;
            if !key.is_empty() {
                result.push(std::mem::take(&mut key));
            }
        } else if c == '.' {
            if !key.is_empty() {
                result.push(std::mem::take(&mut key));
            }
            match chars.get(index + 1) {
                None | Some('.') => result.push(String::new()),
                _ => {}
            }
        } else {
            key.push(c);
        }
        index += 1;
    }
    if !key.is_empty() {
        result.push(key);
    }
    result
}

/// `margin` of a chart. A side the page leaves out is 0 (`setMargin`).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Margin {
    pub top: Option<f64>,
    pub right: Option<f64>,
    pub bottom: Option<f64>,
    pub left: Option<f64>,
}

/// `stackOffset` of a chart: the keys of `STACK_OFFSET_MAP` in
/// `util/ChartUtils.js`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StackOffset {
    /// d3's `stackOffsetNone`.
    None,
    /// d3's `stackOffsetExpand`.
    Expand,
    /// d3's `stackOffsetSilhouette`.
    Silhouette,
    /// d3's `stackOffsetWiggle`.
    Wiggle,
    /// Recharts' `offsetSign`.
    Sign,
    /// Recharts' `offsetPositive`.
    Positive,
    /// Any other name, which stacks as `none`.
    Other(String),
}

impl StackOffset {
    pub fn from_name(name: &str) -> StackOffset {
        match name {
            "none" => StackOffset::None,
            "expand" => StackOffset::Expand,
            "silhouette" => StackOffset::Silhouette,
            "wiggle" => StackOffset::Wiggle,
            "sign" => StackOffset::Sign,
            "positive" => StackOffset::Positive,
            other => StackOffset::Other(other.into()),
        }
    }
}

/// Props of `<BarChart>`, `<LineChart>`, `<AreaChart>` and
/// `<ComposedChart>`; `chart_name` is which of them.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartProps {
    pub chart_name: String,
    pub width: f64,
    pub height: f64,
    pub layout: Option<Layout>,
    pub margin: Option<Margin>,
    pub data: Option<Vec<Value>>,
    pub bar_category_gap: Option<Value>,
    pub stack_offset: Option<StackOffset>,
    pub reverse_stack_order: Option<bool>,
}

impl ChartProps {
    pub fn new(chart_name: &str, width: f64, height: f64) -> Self {
        Self {
            chart_name: chart_name.into(),
            width,
            height,
            layout: None,
            margin: None,
            data: None,
            bar_category_gap: None,
            stack_offset: None,
            reverse_stack_order: None,
        }
    }
}

/// `type` of an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisDomainType {
    Number,
    Category,
    Auto,
}

/// `padding` of an axis: an object of sides, or a keyword (`"gap"`,
/// `"no-gap"`; others pad by 0).
#[derive(Debug, Clone, PartialEq)]
pub enum Padding {
    Sides {
        left: Option<f64>,
        right: Option<f64>,
        top: Option<f64>,
        bottom: Option<f64>,
    },
    Keyword(String),
}

/// Props of `<XAxis>` or `<YAxis>`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AxisProps {
    /// `xAxisId` or `yAxisId`.
    pub id: Option<Key>,
    pub data_key: Option<DataKey>,
    pub r#type: Option<AxisDomainType>,
    pub domain: Option<Vec<Value>>,
    pub scale: Option<String>,
    pub allow_data_overflow: Option<bool>,
    pub allow_decimals: Option<bool>,
    pub allow_duplicated_category: Option<bool>,
    pub include_hidden: Option<bool>,
    pub reversed: Option<bool>,
    pub hide: Option<bool>,
    pub mirror: Option<bool>,
    pub tick_count: Option<f64>,
    pub ticks: Option<Vec<Value>>,
    pub nice_ticks: Option<String>,
    pub padding: Option<Padding>,
    pub orientation: Option<String>,
    /// `height` of an `<XAxis>`.
    pub height: Option<f64>,
    /// `width` of a `<YAxis>` (`"auto"` is not modelled yet).
    pub width: Option<f64>,
}

/// An axis's settings in the store, all defaults applied.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisSettings {
    pub id: Key,
    pub data_key: Option<DataKey>,
    pub r#type: AxisDomainType,
    pub domain: Option<Vec<Value>>,
    pub scale: String,
    pub allow_data_overflow: bool,
    pub allow_decimals: bool,
    pub allow_duplicated_category: bool,
    pub include_hidden: bool,
    pub reversed: bool,
    pub hide: bool,
    pub mirror: bool,
    pub tick_count: f64,
    pub ticks: Option<Vec<Value>>,
    pub nice_ticks: String,
    pub padding: Padding,
    pub orientation: String,
    /// `height` (X axis) or `width` (Y axis).
    pub size: f64,
}

impl AxisSettings {
    /// `implicitXAxis` / `implicitYAxis`: the settings of an axis id no
    /// `<XAxis>` or `<YAxis>` declares.
    pub fn implicit(axis_type: AxisType) -> AxisSettings {
        let x = axis_type == AxisType::XAxis;
        AxisSettings {
            id: Key::number(0.0),
            data_key: None,
            r#type: if x {
                AxisDomainType::Category
            } else {
                AxisDomainType::Number
            },
            domain: if x {
                None
            } else {
                Some(vec![0.0.into(), "auto".into()])
            },
            scale: "auto".into(),
            allow_data_overflow: false,
            allow_decimals: true,
            allow_duplicated_category: true,
            include_hidden: false,
            reversed: false,
            hide: true,
            mirror: false,
            tick_count: 5.0,
            ticks: None,
            nice_ticks: "auto".into(),
            padding: default_padding(axis_type),
            orientation: if x { "bottom" } else { "left" }.into(),
            size: if x { 30.0 } else { 60.0 },
        }
    }

    /// `xAxisDefaultProps`/`yAxisDefaultProps` applied to `props`, and the
    /// settings `SetXAxisSettings`/`SetYAxisSettings` dispatch for them, with
    /// `type="auto"` resolved for the layout (`getAxisTypeBasedOnLayout`).
    pub fn from_props(props: &AxisProps, axis_type: AxisType, layout: Layout) -> AxisSettings {
        let x = axis_type == AxisType::XAxis;
        let declared = props.r#type.unwrap_or(if x {
            AxisDomainType::Category
        } else {
            AxisDomainType::Number
        });
        // getAxisTypeBasedOnLayout
        let r#type = match declared {
            AxisDomainType::Auto => {
                if is_categorical_axis(layout, axis_type) {
                    AxisDomainType::Category
                } else {
                    AxisDomainType::Number
                }
            }
            other => other,
        };
        AxisSettings {
            id: props.id.clone().unwrap_or(Key::number(0.0)),
            data_key: props.data_key.clone(),
            r#type,
            domain: props.domain.clone(),
            scale: props.scale.clone().unwrap_or_else(|| "auto".into()),
            allow_data_overflow: props.allow_data_overflow.unwrap_or(false),
            allow_decimals: props.allow_decimals.unwrap_or(true),
            allow_duplicated_category: props.allow_duplicated_category.unwrap_or(true),
            include_hidden: props.include_hidden.unwrap_or(false),
            reversed: props.reversed.unwrap_or(false),
            hide: props.hide.unwrap_or(false),
            mirror: props.mirror.unwrap_or(false),
            tick_count: props.tick_count.unwrap_or(5.0),
            ticks: props.ticks.clone(),
            nice_ticks: props.nice_ticks.clone().unwrap_or_else(|| "auto".into()),
            padding: props
                .padding
                .clone()
                .unwrap_or_else(|| default_padding(axis_type)),
            orientation: props
                .orientation
                .clone()
                .unwrap_or_else(|| if x { "bottom" } else { "left" }.into()),
            size: if x {
                props.height.unwrap_or(30.0)
            } else {
                props.width.unwrap_or(60.0)
            },
        }
    }
}

fn default_padding(axis_type: AxisType) -> Padding {
    match axis_type {
        AxisType::XAxis => Padding::Sides {
            left: Some(0.0),
            right: Some(0.0),
            top: None,
            bottom: None,
        },
        AxisType::YAxis => Padding::Sides {
            left: None,
            right: None,
            top: Some(0.0),
            bottom: Some(0.0),
        },
    }
}

/// Which cartesian graphical item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemKind {
    Bar,
    Line,
    Area,
}

/// Props of `<Bar>`, `<Line>` or `<Area>` that reach the store. `id` is the
/// item's id (Recharts generates one when the page gives none).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemProps {
    pub kind: ItemKind,
    pub id: String,
    pub data_key: Option<DataKey>,
    pub x_axis_id: Option<Key>,
    pub y_axis_id: Option<Key>,
    pub hide: Option<bool>,
    pub stack_id: Option<Value>,
    /// `data` of a `<Line>` or `<Area>`; `<Bar>` ignores its own.
    pub data: Option<Vec<Value>>,
}

impl ItemProps {
    pub fn new(kind: ItemKind, id: &str, data_key: Option<DataKey>) -> Self {
        Self {
            kind,
            id: id.into(),
            data_key,
            x_axis_id: None,
            y_axis_id: None,
            hide: None,
            stack_id: None,
            data: None,
        }
    }
}

/// A graphical item's settings in the store (`SetCartesianGraphicalItem`).
#[derive(Debug, Clone, PartialEq)]
pub struct ItemSettings {
    pub kind: ItemKind,
    pub id: String,
    pub data_key: Option<DataKey>,
    pub x_axis_id: Key,
    pub y_axis_id: Key,
    pub hide: bool,
    /// `getNormalizedStackId(stackId)`: `String(stackId)`. Lines have none.
    pub stack_id: Option<String>,
    pub data: Option<Vec<Value>>,
}

impl ItemSettings {
    pub fn from_props(props: &ItemProps) -> ItemSettings {
        ItemSettings {
            kind: props.kind,
            id: props.id.clone(),
            data_key: props.data_key.clone(),
            x_axis_id: props.x_axis_id.clone().unwrap_or(Key::number(0.0)),
            y_axis_id: props.y_axis_id.clone().unwrap_or(Key::number(0.0)),
            hide: props.hide.unwrap_or(false),
            stack_id: match props.kind {
                ItemKind::Line => None,
                _ => props
                    .stack_id
                    .as_ref()
                    .filter(|id| !id.is_nullish())
                    .map(Value::to_js_string),
            },
            data: match props.kind {
                ItemKind::Bar => None,
                _ => props.data.clone(),
            },
        }
    }

    /// `isStacked` of `state/types/StackedGraphicalItem.js`.
    pub fn is_stacked(&self) -> bool {
        self.stack_id.is_some() && self.data_key.is_some()
    }
}
