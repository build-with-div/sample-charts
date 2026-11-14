//! `shape/Rectangle.js`: the `d` of the `<path>` a `<Rectangle>` draws
//! (bars, the cursor and reference areas), `getRectanglePath`, with the
//! rounding of `util/round.js`.
//!
//! The component renders without animation here (`isUpdateAnimationActive`
//! false, the default); animated frames interpolate the same function.

use js::{math_round, number_to_string};

/// `round(num)` of `util/round.js`: four decimal places, and 0 for -0.
pub(crate) fn round(num: f64) -> f64 {
    let rounded = math_round(num * 10_000.0) / 10_000.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

/// `roundTemplateLiteral(strings, ...values)` for number values.
pub(crate) fn round_template(strings: &[&str], values: &[f64]) -> String {
    debug_assert_eq!(strings.len(), values.len() + 1);
    let mut result = String::from(strings[0]);
    for (value, string) in values.iter().zip(&strings[1..]) {
        result.push_str(&number_to_string(round(*value)));
        result.push_str(string);
    }
    result
}

/// `radius` of a rectangle.
#[derive(Debug, Clone, PartialEq)]
pub enum Radius {
    /// A number (any, NaN included).
    Number(f64),
    /// An array: one radius per corner from the top left, clockwise;
    /// `None` for `null` or `undefined` (and for entries past the end).
    Corners(Vec<Option<f64>>),
    /// Anything else (`undefined`, a string): square corners.
    Other,
}

const M: [&str; 3] = ["M", ",", ""];
const A_FIRST: [&str; 6] = ["A ", ",", ",0,0,", ",", ",", ""];
const L: [&str; 3] = ["L ", ",", ""];
const A_NEXT: [&str; 6] = ["A ", ",", ",0,0,", ",\n        ", ",", ""];
const ROUNDED: [&str; 29] = [
    "M ",
    ",",
    "\n            A ",
    ",",
    ",0,0,",
    ",",
    ",",
    "\n            L ",
    ",",
    "\n            A ",
    ",",
    ",0,0,",
    ",",
    ",",
    "\n            L ",
    ",",
    "\n            A ",
    ",",
    ",0,0,",
    ",",
    ",",
    "\n            L ",
    ",",
    "\n            A ",
    ",",
    ",0,0,",
    ",",
    ",",
    " Z",
];
const SQUARE: [&str; 6] = ["M ", ",", " h ", " v ", " h ", " Z"];

/// `getRectanglePath(x, y, width, height, radius)`.
pub fn get_rectangle_path(x: f64, y: f64, width: f64, height: f64, radius: &Radius) -> String {
    let rounded_width = round(width);
    let rounded_height = round(height);
    let max_radius = js::math::min(rounded_width.abs() / 2.0, rounded_height.abs() / 2.0);
    let y_sign = if rounded_height >= 0.0 { 1.0 } else { -1.0 };
    let x_sign = if rounded_width >= 0.0 { 1.0 } else { -1.0 };
    let clock_wise = if rounded_height >= 0.0 && rounded_width >= 0.0
        || rounded_height < 0.0 && rounded_width < 0.0
    {
        1.0
    } else {
        0.0
    };
    match radius {
        Radius::Corners(corners) if max_radius > 0.0 => {
            let mut r = [0.0; 4];
            for (i, slot) in r.iter_mut().enumerate() {
                let given = corners.get(i).copied().flatten().unwrap_or(0.0);
                *slot = if given > max_radius {
                    max_radius
                } else {
                    given
                };
            }
            let mut path = round_template(&M, &[x, y + y_sign * r[0]]);
            if r[0] > 0.0 {
                path += &round_template(&A_FIRST, &[r[0], r[0], clock_wise, x + x_sign * r[0], y]);
            }
            path += &round_template(&L, &[x + width - x_sign * r[1], y]);
            if r[1] > 0.0 {
                path += &round_template(
                    &A_NEXT,
                    &[r[1], r[1], clock_wise, x + width, y + y_sign * r[1]],
                );
            }
            path += &round_template(&L, &[x + width, y + height - y_sign * r[2]]);
            if r[2] > 0.0 {
                path += &round_template(
                    &A_NEXT,
                    &[
                        r[2],
                        r[2],
                        clock_wise,
                        x + width - x_sign * r[2],
                        y + height,
                    ],
                );
            }
            path += &round_template(&L, &[x + x_sign * r[3], y + height]);
            if r[3] > 0.0 {
                path += &round_template(
                    &A_NEXT,
                    &[r[3], r[3], clock_wise, x, y + height - y_sign * r[3]],
                );
            }
            path + "Z"
        }
        // `radius === +radius && radius > 0`
        Radius::Number(radius) if max_radius > 0.0 && *radius > 0.0 => {
            let r = js::math::min(max_radius, *radius);
            round_template(
                &ROUNDED,
                &[
                    x,
                    y + y_sign * r,
                    r,
                    r,
                    clock_wise,
                    x + x_sign * r,
                    y,
                    x + width - x_sign * r,
                    y,
                    r,
                    r,
                    clock_wise,
                    x + width,
                    y + y_sign * r,
                    x + width,
                    y + height - y_sign * r,
                    r,
                    r,
                    clock_wise,
                    x + width - x_sign * r,
                    y + height,
                    x + x_sign * r,
                    y + height,
                    r,
                    r,
                    clock_wise,
                    x,
                    y + height - y_sign * r,
                ],
            )
        }
        _ => round_template(&SQUARE, &[x, y, width, height, -width]),
    }
}

/// The `d` of the path `<Rectangle x y width height radius>` renders, or
/// `None` when it renders nothing (a NaN coordinate or a zero side).
pub fn rectangle(x: f64, y: f64, width: f64, height: f64, radius: &Radius) -> Option<String> {
    if x.is_nan() || y.is_nan() || width.is_nan() || height.is_nan() {
        return None;
    }
    if width == 0.0 || height == 0.0 {
        return None;
    }
    Some(get_rectangle_path(x, y, width, height, radius))
}
