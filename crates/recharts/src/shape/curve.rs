//! `shape/Curve.js`: `getCurveFactory` and `getPath`, the `d` of the
//! `<path>` that `<Line>` and `<Area>` draw, through d3-shape's line and
//! area generators (three-digit paths, d3's default).
//!
//! `type` given as a function (a custom d3 curve) is not modelled.

use d3::shape::{area, line, AreaPoint, Curve};
use js::Value;

use super::upper_first;

/// One point of a curve: `{ x, y }`, where either may be `null` or not a
/// number (a missing value); only finite numbers are drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct CurvePoint {
    pub x: Value,
    pub y: Value,
}

/// `baseLine` of a curve.
#[derive(Debug, Clone, PartialEq)]
pub enum BaseLine {
    /// Anything that is neither a number nor an array (`undefined`, `null`,
    /// a string): the curve is a line.
    None,
    /// A number (`isNumber`; NaN counts as none): an area down to it.
    Number(f64),
    /// One base point per point (`null` entries leave a gap): an area.
    Points(Vec<Option<CurvePoint>>),
}

impl BaseLine {
    /// The base line a JavaScript value gives.
    pub fn from_value(value: &Value) -> BaseLine {
        match value {
            Value::Number(n) if !n.is_nan() => BaseLine::Number(*n),
            Value::Array(items) => BaseLine::Points(
                items
                    .iter()
                    .map(|v| {
                        (!v.is_nullish()).then(|| CurvePoint {
                            x: v.get("x"),
                            y: v.get("y"),
                        })
                    })
                    .collect(),
            ),
            _ => BaseLine::None,
        }
    }
}

/// `getCurveFactory(type, layout)`: `type` is the curve name (`undefined`
/// is `'linear'`, the default prop; `null` matches no curve), `layout` the
/// chart's layout (`""` or `None` is falsy).
pub fn curve_factory(r#type: Option<&str>, layout: Option<&str>) -> Curve {
    let name = format!("curve{}", upper_first(r#type.unwrap_or("linear")));
    let layout = layout.filter(|l| !l.is_empty());
    if let (Some(layout), "curveMonotone" | "curveBump") = (layout, name.as_str()) {
        let vertical = layout == "vertical";
        return match (name.as_str(), vertical) {
            ("curveMonotone", true) => Curve::MonotoneY,
            ("curveMonotone", false) => Curve::MonotoneX,
            (_, true) => Curve::BumpY,
            (_, false) => Curve::BumpX,
        };
    }
    match name.as_str() {
        "curveBasisClosed" => Curve::BasisClosed,
        "curveBasisOpen" => Curve::BasisOpen,
        "curveBasis" => Curve::Basis,
        "curveBumpX" => Curve::BumpX,
        "curveBumpY" => Curve::BumpY,
        "curveLinearClosed" => Curve::LinearClosed,
        "curveMonotoneX" => Curve::MonotoneX,
        "curveMonotoneY" => Curve::MonotoneY,
        "curveNatural" => Curve::Natural,
        "curveStep" => Curve::Step,
        "curveStepAfter" => Curve::StepAfter,
        "curveStepBefore" => Curve::StepBefore,
        _ => Curve::Linear,
    }
}

/// `isWellBehavedNumber`: `Number.isFinite`.
fn finite(value: &Value) -> Option<f64> {
    match value {
        Value::Number(n) if n.is_finite() => Some(*n),
        _ => None,
    }
}

/// `defined(p)`: both coordinates finite numbers.
fn defined(p: &CurvePoint) -> Option<[f64; 2]> {
    Some([finite(&p.x)?, finite(&p.y)?])
}

/// `getPath({ type, points, baseLine, layout, connectNulls })`: the path
/// data, or `None` (`null`) for an empty path.
pub fn get_path(
    r#type: Option<&str>,
    points: &[CurvePoint],
    base_line: &BaseLine,
    layout: Option<&str>,
    connect_nulls: bool,
) -> Option<String> {
    let curve = curve_factory(r#type, layout);
    let vertical = layout == Some("vertical");
    match base_line {
        BaseLine::Points(bases) => {
            // `areaDefined`: the base present and defined, and the point.
            let area_points = points.iter().enumerate().map(|(i, p)| {
                let base = bases.get(i).cloned().flatten();
                let base = defined(base.as_ref()?)?;
                let [x, y] = defined(p)?;
                Some(if vertical {
                    // .y(getY).x1(getX).x0(d => d.base.x)
                    AreaPoint {
                        x0: base[0],
                        y0: y,
                        x1: Some(x),
                        y1: None,
                    }
                } else {
                    // .x(getX).y1(getY).y0(d => d.base.y)
                    AreaPoint {
                        x0: x,
                        y0: base[1],
                        x1: None,
                        y1: Some(y),
                    }
                })
            });
            let area_points: Vec<_> = if connect_nulls {
                area_points.filter(Option::is_some).collect()
            } else {
                area_points.collect()
            };
            area(&area_points, curve, Some(3))
        }
        BaseLine::Number(base) => {
            let area_points: Vec<_> = points
                .iter()
                .map(|p| {
                    let [x, y] = defined(p)?;
                    Some(if vertical {
                        // .y(getY).x1(getX).x0(baseLine)
                        AreaPoint {
                            x0: *base,
                            y0: y,
                            x1: Some(x),
                            y1: None,
                        }
                    } else {
                        // .x(getX).y1(getY).y0(baseLine)
                        AreaPoint {
                            x0: x,
                            y0: *base,
                            x1: None,
                            y1: Some(y),
                        }
                    })
                })
                .filter(|p| !connect_nulls || p.is_some())
                .collect();
            area(&area_points, curve, Some(3))
        }
        BaseLine::None => {
            let line_points: Vec<_> = points
                .iter()
                .map(defined)
                .filter(|p| !connect_nulls || p.is_some())
                .collect();
            line(&line_points, curve, Some(3))
        }
    }
}
