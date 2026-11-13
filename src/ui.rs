//! The app's window contents.

// div-gap: V1 inline SVG (docs/framework-gaps.md). Recharts renders charts
// as inline SVG, which Div does not have yet, so the window shows the parts of the
// port that run today: the numeric axis ticks Recharts computes for the
// data of shadcn's chart examples. Charts appear here once Div renders
// inline SVG; nothing draws them by other means meanwhile.

use div::prelude::*;
use recharts::js::number_to_string;
use recharts::scale::{get_nice_tick_values, NiceTicks};

/// A data series from a shadcn chart example: its largest value and the
/// `tickCount` of a numeric axis over it.
pub struct AxisExample {
    pub id: &'static str,
    pub example: &'static str,
    pub data_max: f64,
    pub tick_count: u32,
}

/// Series of shadcn's chart examples (`shadcn@4.23.0`,
/// `apps/v4/registry/new-york-v4/charts`), with Recharts' defaults for a
/// numeric axis: domain `[0, 'auto']`, `tickCount` 5.
pub const AXES: &[AxisExample] = &[
    AxisExample {
        id: "bar-default",
        example: "chart-bar-default: desktop",
        data_max: 305.0,
        tick_count: 5,
    },
    AxisExample {
        id: "bar-multiple-mobile",
        example: "chart-bar-multiple: mobile",
        data_max: 200.0,
        tick_count: 5,
    },
    AxisExample {
        id: "area-interactive-desktop",
        example: "chart-area-interactive: desktop",
        data_max: 499.0,
        tick_count: 5,
    },
    AxisExample {
        id: "area-interactive-mobile",
        example: "chart-area-interactive: mobile",
        data_max: 530.0,
        tick_count: 5,
    },
];

/// The ticks Recharts' `getNiceTickValues` gives for `[0, data_max]`, as
/// Recharts prints them.
pub fn axis_ticks(axis: &AxisExample) -> String {
    match get_nice_tick_values([0.0, axis.data_max], axis.tick_count, true, NiceTicks::Auto) {
        Ok(ticks) => ticks
            .into_iter()
            .map(number_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        Err(error) => format!("error: {error}"),
    }
}

/// The whole window.
pub fn root() -> ViewNode {
    let rows: Vec<ViewNode> = AXES
        .iter()
        .map(|axis| {
            view! {
                <div style:display="flex" style:gap="16px" style:margin-top="8px">
                    <div style:width="280px" style:flex-shrink="0">{axis.example}</div>
                    <div id={format!("ticks-{}", axis.id)}>{axis_ticks(axis)}</div>
                </div>
            }
        })
        .collect();
    view! {
        <div
            id="app"
            style:padding="24px"
            style:font-size="14px"
            style:line-height="20px"
            style:color="#0a0a0a"
            style:background-color="#ffffff"
            style:height="100vh"
        >
            <div style:font-size="20px" style:line-height="28px" style:font-weight="600">
                {"Sample Charts"}
            </div>
            <div style:margin-top="4px" style:color="#737373">
                {"Recharts 3.8.0 ported onto Div. Charts render once Div has inline SVG. Until then: getNiceTickValues([0, max], 5) for series of shadcn's chart examples."}
            </div>
            <div style:margin-top="16px" style:font-weight="600">{"Numeric axis ticks"}</div>
            {rows}
        </div>
    }
}
