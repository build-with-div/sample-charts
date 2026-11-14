//! `shape/Sector.js`: the `d` of the `<path>` a `<Sector>` draws (pie
//! slices, radial bars, the active shape), `getSectorPath` and
//! `getSectorWithCorner`, with `polarToCartesian` of `util/PolarUtils.js`
//! and `getPercentValue` and `mathSign` of `util/DataUtils.js`.
//!
//! `Math.sin` and `Math.cos` are [`js::math::sin`] and [`js::math::cos`]
//! (correctly rounded, so Chromium's except on the arguments it does not
//! round correctly), `Math.asin` is [`js::math::asin`] (V8's fdlibm). The
//! SVG attributes `<Sector>` passes through to its path (`cx`, `cy`, …)
//! are not modelled here; the class is always `recharts-sector` plus
//! `className`.

use js::math::{asin, cos, min, sin};
use js::{number_to_string, parse_float, string_to_number};

use super::rectangle::round_template;

/// `RADIAN` of `util/PolarUtils.js`: `Math.PI / 180`.
const RADIAN: f64 = std::f64::consts::PI / 180.0;

/// `cornerRadius` of a sector.
#[derive(Debug, Clone, PartialEq)]
pub enum CornerRadius {
    /// A number (NaN counts as none).
    Number(f64),
    /// A string: a percentage of the ring's width when it ends with `%`,
    /// else the number it parses as.
    Text(String),
    /// Anything else: none.
    Other,
}

/// The props of `<Sector>` that shape its path, with Recharts' defaults
/// (`defaultSectorProps`: zeros and `false`).
#[derive(Debug, Clone, PartialEq)]
pub struct SectorProps {
    pub cx: f64,
    pub cy: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub corner_radius: CornerRadius,
    pub force_corner_radius: bool,
    pub corner_is_external: bool,
}

impl Default for SectorProps {
    fn default() -> SectorProps {
        SectorProps {
            cx: 0.0,
            cy: 0.0,
            inner_radius: 0.0,
            outer_radius: 0.0,
            start_angle: 0.0,
            end_angle: 0.0,
            corner_radius: CornerRadius::Number(0.0),
            force_corner_radius: false,
            corner_is_external: false,
        }
    }
}

/// `mathSign(value)`: 0 for ±0, 1 for positive, -1 otherwise (NaN too).
fn math_sign(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else if value > 0.0 {
        1.0
    } else {
        -1.0
    }
}

/// `getPercentValue(percent, totalValue, 0, true)`.
fn percent_value(percent: &CornerRadius, total: f64) -> f64 {
    let value = match percent {
        CornerRadius::Number(n) if !n.is_nan() => *n,
        CornerRadius::Text(text) => {
            // isPercent: the first '%' is the last character (or the
            // string is empty, where indexOf gives -1 = length - 1).
            match text.find('%') {
                Some(index) if index + 1 == text.len() => {
                    total * parse_float(&text[..index]) / 100.0
                }
                None if text.is_empty() => total * parse_float("") / 100.0,
                _ => string_to_number(text),
            }
        }
        _ => return 0.0,
    };
    let value = if value.is_nan() { 0.0 } else { value };
    if value > total {
        total
    } else {
        value
    }
}

/// `polarToCartesian(cx, cy, radius, angle)`, angle in degrees.
pub fn polar_to_cartesian(cx: f64, cy: f64, radius: f64, angle: f64) -> (f64, f64) {
    (
        cx + cos(-RADIAN * angle) * radius,
        cy + sin(-RADIAN * angle) * radius,
    )
}

/// `getDeltaAngle(startAngle, endAngle)`.
fn delta_angle(start_angle: f64, end_angle: f64) -> f64 {
    let sign = math_sign(end_angle - start_angle);
    let delta = min((end_angle - start_angle).abs(), 359.999);
    sign * delta
}

const SECTOR: [&str; 9] = [
    "M ",
    ",",
    "\n    A ",
    ",",
    ",0,\n    ",
    ",",
    ",\n    ",
    ",",
    "\n  ",
];
const SECTOR_INNER: [&str; 9] = [
    "L ",
    ",",
    "\n            A ",
    ",",
    ",0,\n            ",
    ",",
    ",\n            ",
    ",",
    " Z",
];
const SECTOR_CENTER: [&str; 3] = ["L ", ",", " Z"];
const FORCED_CORNER: [&str; 9] = [
    "M ",
    ",",
    "\n        a",
    ",",
    ",0,0,1,",
    ",0\n        a",
    ",",
    ",0,0,1,",
    ",0\n      ",
];
const CORNER_OUTER: [&str; 19] = [
    "M ", ",", "\n    A", ",", ",0,0,", ",", ",", "\n    A", ",", ",0,", ",", ",", ",", "\n    A",
    ",", ",0,0,", ",", ",", "\n  ",
];
const CORNER_INNER: [&str; 19] = [
    "L",
    ",",
    "\n      A",
    ",",
    ",0,0,",
    ",",
    ",",
    "\n      A",
    ",",
    ",0,",
    ",",
    ",",
    ",",
    "\n      A",
    ",",
    ",0,0,",
    ",",
    ",",
    "Z",
];
const CORNER_CENTER: [&str; 3] = ["L", ",", "Z"];

/// `+bool`.
fn flag(value: bool) -> f64 {
    if value {
        1.0
    } else {
        0.0
    }
}

/// `getSectorPath({ cx, cy, innerRadius, outerRadius, startAngle, endAngle })`.
pub fn get_sector_path(
    cx: f64,
    cy: f64,
    inner_radius: f64,
    outer_radius: f64,
    start_angle: f64,
    end_angle: f64,
) -> String {
    let angle = delta_angle(start_angle, end_angle);
    // When the angle is 360 the start and end points coincide.
    let temp_end_angle = start_angle + angle;
    let outer_start = polar_to_cartesian(cx, cy, outer_radius, start_angle);
    let outer_end = polar_to_cartesian(cx, cy, outer_radius, temp_end_angle);
    let mut path = round_template(
        &SECTOR,
        &[
            outer_start.0,
            outer_start.1,
            outer_radius,
            outer_radius,
            flag(angle.abs() > 180.0),
            flag(start_angle > temp_end_angle),
            outer_end.0,
            outer_end.1,
        ],
    );
    if inner_radius > 0.0 {
        let inner_start = polar_to_cartesian(cx, cy, inner_radius, start_angle);
        let inner_end = polar_to_cartesian(cx, cy, inner_radius, temp_end_angle);
        path += &round_template(
            &SECTOR_INNER,
            &[
                inner_end.0,
                inner_end.1,
                inner_radius,
                inner_radius,
                flag(angle.abs() > 180.0),
                flag(start_angle <= temp_end_angle),
                inner_start.0,
                inner_start.1,
            ],
        );
    } else {
        path += &round_template(&SECTOR_CENTER, &[cx, cy]);
    }
    path
}

/// What `getTangentCircle` returns (its `center` is unused).
struct Tangent {
    circle: (f64, f64),
    line: (f64, f64),
    theta: f64,
}

/// `getTangentCircle({ cx, cy, radius, angle, sign, isExternal,
/// cornerRadius, cornerIsExternal })`.
#[allow(clippy::too_many_arguments)]
fn tangent_circle(
    cx: f64,
    cy: f64,
    radius: f64,
    angle: f64,
    sign: f64,
    is_external: bool,
    corner_radius: f64,
    corner_is_external: bool,
) -> Tangent {
    let center_radius = corner_radius * if is_external { 1.0 } else { -1.0 } + radius;
    let theta = asin(corner_radius / center_radius) / RADIAN;
    let center_angle = if corner_is_external {
        angle
    } else {
        angle + sign * theta
    };
    // The point tangent to the circle, and the one tangent to the radius.
    let circle = polar_to_cartesian(cx, cy, radius, center_angle);
    let line_angle = if corner_is_external {
        angle - sign * theta
    } else {
        angle
    };
    let line = polar_to_cartesian(cx, cy, center_radius * cos(theta * RADIAN), line_angle);
    Tangent {
        circle,
        line,
        theta,
    }
}

/// `getSectorWithCorner({ cx, cy, innerRadius, outerRadius, cornerRadius,
/// forceCornerRadius, cornerIsExternal, startAngle, endAngle })`.
#[allow(clippy::too_many_arguments)]
pub fn get_sector_with_corner(
    cx: f64,
    cy: f64,
    inner_radius: f64,
    outer_radius: f64,
    corner_radius: f64,
    force_corner_radius: bool,
    corner_is_external: bool,
    start_angle: f64,
    end_angle: f64,
) -> String {
    let sign = math_sign(end_angle - start_angle);
    let tangent = |radius, angle, sign, is_external| {
        tangent_circle(
            cx,
            cy,
            radius,
            angle,
            sign,
            is_external,
            corner_radius,
            corner_is_external,
        )
    };
    let so = tangent(outer_radius, start_angle, sign, false);
    let eo = tangent(outer_radius, end_angle, -sign, false);
    let outer_arc_angle = if corner_is_external {
        (start_angle - end_angle).abs()
    } else {
        (start_angle - end_angle).abs() - so.theta - eo.theta
    };
    if outer_arc_angle < 0.0 {
        if force_corner_radius {
            return round_template(
                &FORCED_CORNER,
                &[
                    so.line.0,
                    so.line.1,
                    corner_radius,
                    corner_radius,
                    corner_radius * 2.0,
                    corner_radius,
                    corner_radius,
                    -corner_radius * 2.0,
                ],
            );
        }
        return get_sector_path(cx, cy, inner_radius, outer_radius, start_angle, end_angle);
    }
    let mut path = round_template(
        &CORNER_OUTER,
        &[
            so.line.0,
            so.line.1,
            corner_radius,
            corner_radius,
            flag(sign < 0.0),
            so.circle.0,
            so.circle.1,
            outer_radius,
            outer_radius,
            flag(outer_arc_angle > 180.0),
            flag(sign < 0.0),
            eo.circle.0,
            eo.circle.1,
            corner_radius,
            corner_radius,
            flag(sign < 0.0),
            eo.line.0,
            eo.line.1,
        ],
    );
    if inner_radius > 0.0 {
        let si = tangent(inner_radius, start_angle, sign, true);
        let ei = tangent(inner_radius, end_angle, -sign, true);
        let inner_arc_angle = if corner_is_external {
            (start_angle - end_angle).abs()
        } else {
            (start_angle - end_angle).abs() - si.theta - ei.theta
        };
        if inner_arc_angle < 0.0 && corner_radius == 0.0 {
            // "".concat(path, "L").concat(cx, ",").concat(cy, "Z"): unrounded.
            return format!("{path}L{},{}Z", number_to_string(cx), number_to_string(cy));
        }
        path += &round_template(
            &CORNER_INNER,
            &[
                ei.line.0,
                ei.line.1,
                corner_radius,
                corner_radius,
                flag(sign < 0.0),
                ei.circle.0,
                ei.circle.1,
                inner_radius,
                inner_radius,
                flag(inner_arc_angle > 180.0),
                flag(sign > 0.0),
                si.circle.0,
                si.circle.1,
                corner_radius,
                corner_radius,
                flag(sign < 0.0),
                si.line.0,
                si.line.1,
            ],
        );
    } else {
        path += &round_template(&CORNER_CENTER, &[cx, cy]);
    }
    path
}

/// The `d` of the path `<Sector>` renders, or `None` when it renders
/// nothing (`outerRadius < innerRadius` or `startAngle === endAngle`).
pub fn sector(props: &SectorProps) -> Option<String> {
    let p = props;
    if p.outer_radius < p.inner_radius || p.start_angle == p.end_angle {
        return None;
    }
    let delta_radius = p.outer_radius - p.inner_radius;
    let cr = percent_value(&p.corner_radius, delta_radius);
    Some(if cr > 0.0 && (p.start_angle - p.end_angle).abs() < 360.0 {
        get_sector_with_corner(
            p.cx,
            p.cy,
            p.inner_radius,
            p.outer_radius,
            min(cr, delta_radius / 2.0),
            p.force_corner_radius,
            p.corner_is_external,
            p.start_angle,
            p.end_angle,
        )
    } else {
        get_sector_path(
            p.cx,
            p.cy,
            p.inner_radius,
            p.outer_radius,
            p.start_angle,
            p.end_angle,
        )
    })
}
