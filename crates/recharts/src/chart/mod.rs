//! Recharts' cartesian chart state and the axis computations it derives
//! from it: the domain, the nice ticks, the range, the d3 scale and the
//! ticks of every axis.
//!
//! In Recharts the chart components report their props into a Redux store
//! (`state/*Slice.js`) and the axis selectors
//! (`state/selectors/axisSelectors.js`) compute everything else from it.
//! [`CartesianChart`] is that store after a chart has mounted, built from
//! the components' props; its methods are the selectors.
//!
//! Not yet ported, and reported as [`ChartError::Unsupported`] rather than
//! computed differently: scales other than linear, log, band and point, and
//! props given as functions.
//! Error bars, reference elements, brushes and legends are not modelled, so
//! a chart here has none.

mod axis;
mod props;
mod scale;
mod stack;

pub use axis::{AxisResult, Tick};
pub use props::{
    is_categorical_axis, AxisDomainType, AxisProps, AxisSettings, AxisType, ChartProps, DataKey,
    ItemKind, ItemProps, ItemSettings, Layout, Margin, Padding, StackOffset,
};
pub use scale::{BandPosition, RechartsScale};
pub use stack::StackGroup;

use js::{JsError, Key, Value};

/// Why an axis could not be computed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChartError {
    /// The exception Recharts throws.
    Js(JsError),
    /// Recharts behaviour this port does not cover yet.
    Unsupported(&'static str),
}

impl From<JsError> for ChartError {
    fn from(error: JsError) -> Self {
        ChartError::Js(error)
    }
}

impl std::fmt::Display for ChartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChartError::Js(error) => write!(f, "{error}"),
            ChartError::Unsupported(what) => write!(f, "not ported yet: {what}"),
        }
    }
}

impl std::error::Error for ChartError {}

/// The axes of one kind, keyed as the store's object keys them: by
/// `String(id)`. `values()` follows JavaScript's property order (array
/// indices ascending, then other keys in insertion order).
#[derive(Debug, Clone, PartialEq, Default)]
struct AxisMap {
    entries: Vec<(String, AxisSettings)>,
}

impl AxisMap {
    fn insert(&mut self, settings: AxisSettings) {
        let key = settings.id.to_value().to_js_string();
        match self.entries.iter_mut().find(|(k, _)| *k == key) {
            Some(entry) => entry.1 = settings,
            None => self.entries.push((key, settings)),
        }
    }

    fn get(&self, id: &Key) -> Option<&AxisSettings> {
        let key = id.to_value().to_js_string();
        self.entries.iter().find(|(k, _)| *k == key).map(|(_, s)| s)
    }

    fn values(&self) -> Vec<&AxisSettings> {
        let index = |key: &str| -> Option<u32> {
            let n: u32 = key.parse().ok()?;
            (n != u32::MAX && n.to_string() == key).then_some(n)
        };
        let mut indexed: Vec<(u32, &AxisSettings)> = self
            .entries
            .iter()
            .filter_map(|(k, s)| index(k).map(|i| (i, s)))
            .collect();
        indexed.sort_by_key(|(i, _)| *i);
        indexed
            .into_iter()
            .map(|(_, s)| s)
            .chain(
                self.entries
                    .iter()
                    .filter(|(k, _)| index(k).is_none())
                    .map(|(_, s)| s),
            )
            .collect()
    }
}

/// A cartesian chart's store after its components mounted.
#[derive(Debug, Clone, PartialEq)]
pub struct CartesianChart {
    chart_name: String,
    layout: Layout,
    width: f64,
    height: f64,
    /// top, right, bottom, left
    margin: [f64; 4],
    chart_data: Option<Vec<Value>>,
    data_start_index: usize,
    data_end_index: usize,
    bar_category_gap: Value,
    stack_offset: StackOffset,
    reverse_stack_order: bool,
    x_axes: AxisMap,
    y_axes: AxisMap,
    items: Vec<ItemSettings>,
}

impl CartesianChart {
    /// The store of `<chart>` with the given axes and graphical items, as
    /// `CartesianChart`, the axis components and the item components fill
    /// it (`defaultCartesianChartProps`, `setMargin`, `setChartData`,
    /// `addXAxis`, `addCartesianGraphicalItem`).
    pub fn new(
        chart: &ChartProps,
        x_axes: &[AxisProps],
        y_axes: &[AxisProps],
        items: &[ItemProps],
    ) -> CartesianChart {
        let layout = chart.layout.unwrap_or(Layout::Horizontal);
        let margin = chart.margin.unwrap_or(Margin {
            top: Some(5.0),
            right: Some(5.0),
            bottom: Some(5.0),
            left: Some(5.0),
        });
        let data_end_index = match &chart.data {
            Some(data) if !data.is_empty() => data.len() - 1,
            _ => 0,
        };
        let mut x_map = AxisMap::default();
        for props in x_axes {
            x_map.insert(AxisSettings::from_props(props, AxisType::XAxis, layout));
        }
        let mut y_map = AxisMap::default();
        for props in y_axes {
            y_map.insert(AxisSettings::from_props(props, AxisType::YAxis, layout));
        }
        CartesianChart {
            chart_name: chart.chart_name.clone(),
            layout,
            width: chart.width,
            height: chart.height,
            margin: [
                margin.top.unwrap_or(0.0),
                margin.right.unwrap_or(0.0),
                margin.bottom.unwrap_or(0.0),
                margin.left.unwrap_or(0.0),
            ],
            chart_data: chart.data.clone(),
            data_start_index: 0,
            data_end_index,
            bar_category_gap: chart
                .bar_category_gap
                .clone()
                .unwrap_or_else(|| "10%".into()),
            stack_offset: chart.stack_offset.clone().unwrap_or(StackOffset::None),
            reverse_stack_order: chart.reverse_stack_order.unwrap_or(false),
            x_axes: x_map,
            y_axes: y_map,
            items: items.iter().map(ItemSettings::from_props).collect(),
        }
    }

    pub fn layout(&self) -> Layout {
        self.layout
    }
}
