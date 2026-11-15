//! What the chart components report to the store from their props
//! (`ReportChartProps`, `ReportMainChartProps`, `SetXAxisSettings`,
//! `SetYAxisSettings`, `SetCartesianGraphicalItem`), as the selectors'
//! typed props.
//!
//! Props the selectors read in a form this port does not model (a function
//! `dataKey`, a string `width`) are reported as unsupported; props that only
//! change what an axis draws are not read here.

use js::{Key, Value};

use crate::chart::{
    AxisDomainType, AxisProps, ChartError, ChartProps, DataKey, ItemKind, ItemProps, Layout,
    Margin, Padding, StackOffset,
};
use crate::dom::Props;

fn unsupported(what: &'static str) -> ChartError {
    ChartError::Unsupported(what)
}

fn number(value: Value, what: &'static str) -> Result<Option<f64>, ChartError> {
    match value {
        Value::Undefined => Ok(None),
        Value::Number(n) => Ok(Some(n)),
        _ => Err(unsupported(what)),
    }
}

fn boolean(value: Value, what: &'static str) -> Result<Option<bool>, ChartError> {
    match value {
        Value::Undefined => Ok(None),
        Value::Bool(b) => Ok(Some(b)),
        _ => Err(unsupported(what)),
    }
}

fn string(value: Value, what: &'static str) -> Result<Option<String>, ChartError> {
    match value {
        Value::Undefined => Ok(None),
        Value::String(s) => Ok(Some(s)),
        _ => Err(unsupported(what)),
    }
}

fn array(value: Value, what: &'static str) -> Result<Option<Vec<Value>>, ChartError> {
    match value {
        Value::Undefined => Ok(None),
        Value::Array(items) => Ok(Some(items)),
        _ => Err(unsupported(what)),
    }
}

fn data_key(value: Value) -> Result<Option<DataKey>, ChartError> {
    match value {
        Value::Undefined => Ok(None),
        Value::String(s) => Ok(Some(DataKey::Name(s))),
        Value::Number(n) => Ok(Some(DataKey::Index(n))),
        _ => Err(unsupported("a dataKey other than a string or number")),
    }
}

fn key(value: Value) -> Result<Option<Key>, ChartError> {
    match value {
        Value::Undefined => Ok(None),
        other => Key::of(&other)
            .map(Some)
            .ok_or(unsupported("an axis id other than a primitive")),
    }
}

/// The chart's props as the store holds them.
pub(crate) fn chart_props(chart_name: &str, props: &Props) -> Result<ChartProps, ChartError> {
    let width = number(props.get("width"), "a chart width other than a number")?;
    let height = number(props.get("height"), "a chart height other than a number")?;
    let (Some(width), Some(height)) = (width, height) else {
        return Err(unsupported("a chart without a width and height"));
    };
    for (name, what) in [
        ("responsive", "responsive charts"),
        ("compact", "compact charts"),
        ("baseValue", "a chart baseValue"),
    ] {
        if props.get(name).is_truthy() {
            return Err(unsupported(what));
        }
    }
    let mut chart = ChartProps::new(chart_name, width, height);
    chart.layout = match string(props.get("layout"), "a layout other than a string")? {
        None => None,
        Some(name) => Some(Layout::from_name(&name).ok_or(unsupported("a polar layout"))?),
    };
    chart.margin = match props.get("margin") {
        Value::Undefined => None,
        Value::Object(_) => {
            let margin = Props::of(&props.get("margin"));
            let side = |name| number(margin.get(name), "a margin other than numbers");
            Some(Margin {
                top: side("top")?,
                right: side("right")?,
                bottom: side("bottom")?,
                left: side("left")?,
            })
        }
        _ => return Err(unsupported("a margin other than an object")),
    };
    chart.data = array(props.get("data"), "chart data other than an array")?;
    let optional = |value: Value| (!matches!(value, Value::Undefined)).then_some(value);
    chart.bar_category_gap = optional(props.get("barCategoryGap"));
    chart.bar_gap = optional(props.get("barGap"));
    chart.bar_size = optional(props.get("barSize"));
    chart.max_bar_size = number(props.get("maxBarSize"), "a maxBarSize other than a number")?;
    chart.stack_offset = string(
        props.get("stackOffset"),
        "a stackOffset other than a string",
    )?
    .map(|name| StackOffset::from_name(&name));
    chart.reverse_stack_order = boolean(
        props.get("reverseStackOrder"),
        "a reverseStackOrder other than a boolean",
    )?;
    Ok(chart)
}

/// `<XAxis>` or `<YAxis>` props the axis selectors read.
pub(crate) fn axis_props(props: &Props, id_prop: &str) -> Result<AxisProps, ChartError> {
    let mut axis = AxisProps {
        id: key(props.get(id_prop))?,
        data_key: data_key(props.get("dataKey"))?,
        ..AxisProps::default()
    };
    axis.r#type = match string(props.get("type"), "an axis type other than a string")? {
        None => None,
        Some(name) => Some(match name.as_str() {
            "number" => AxisDomainType::Number,
            "category" => AxisDomainType::Category,
            "auto" => AxisDomainType::Auto,
            _ => {
                return Err(unsupported(
                    "an axis type other than number, category or auto",
                ))
            }
        }),
    };
    axis.domain = array(props.get("domain"), "an axis domain other than an array")?;
    axis.scale = string(props.get("scale"), "an axis scale other than a name")?;
    let flag = |name, what| boolean(props.get(name), what);
    axis.allow_data_overflow = flag("allowDataOverflow", "a non-boolean allowDataOverflow")?;
    axis.allow_decimals = flag("allowDecimals", "a non-boolean allowDecimals")?;
    axis.allow_duplicated_category = flag(
        "allowDuplicatedCategory",
        "a non-boolean allowDuplicatedCategory",
    )?;
    axis.include_hidden = flag("includeHidden", "a non-boolean includeHidden")?;
    axis.reversed = flag("reversed", "a non-boolean reversed")?;
    axis.hide = flag("hide", "a non-boolean hide")?;
    axis.mirror = flag("mirror", "a non-boolean mirror")?;
    axis.tick_count = number(props.get("tickCount"), "a tickCount other than a number")?;
    axis.ticks = array(props.get("ticks"), "axis ticks other than an array")?;
    axis.nice_ticks = string(props.get("niceTicks"), "a niceTicks other than a string")?;
    axis.padding = match props.get("padding") {
        Value::Undefined => None,
        Value::String(keyword) => Some(Padding::Keyword(keyword)),
        Value::Object(_) => {
            let sides = Props::of(&props.get("padding"));
            let side = |name| number(sides.get(name), "axis padding other than numbers");
            Some(Padding::Sides {
                left: side("left")?,
                right: side("right")?,
                top: side("top")?,
                bottom: side("bottom")?,
            })
        }
        _ => return Err(unsupported("axis padding other than an object or keyword")),
    };
    axis.orientation = string(
        props.get("orientation"),
        "an axis orientation other than a string",
    )?;
    axis.height = number(props.get("height"), "an axis height other than a number")?;
    axis.width = number(props.get("width"), "an axis width other than a number")?;
    Ok(axis)
}

/// What `<Bar>` reports with `SetCartesianGraphicalItem`, for the bar whose
/// resolved id is `id`.
pub(crate) fn bar_item_props(props: &Props, id: &str) -> Result<ItemProps, ChartError> {
    let mut item = ItemProps::new(ItemKind::Bar, id, data_key(props.get("dataKey"))?);
    item.x_axis_id = key(props.get("xAxisId"))?;
    item.y_axis_id = key(props.get("yAxisId"))?;
    item.hide = Some(props.get("hide").is_truthy());
    item.stack_id = match props.get("stackId") {
        Value::Undefined => None,
        other => Some(other),
    };
    item.bar_size = match props.get("barSize") {
        Value::Undefined => None,
        other => Some(other),
    };
    item.max_bar_size = number(props.get("maxBarSize"), "a maxBarSize other than a number")?;
    item.min_point_size = number(
        props.get("minPointSize"),
        "a minPointSize other than a number",
    )?;
    Ok(item)
}
