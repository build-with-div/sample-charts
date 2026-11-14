//! `d3.line()` and `d3.area()` rendering to a path string (no `context`).

use super::curve::{Curve, CurveOutput};
use crate::path::Path;

/// `line.curve(curve).digits(digits)(data)` for data whose accessors give
/// `points` (`None` where `defined` is false): the path data, or `None`
/// (`null`) when it is empty.
pub fn line(points: &[Option<[f64; 2]>], curve: Curve, digits: Option<u32>) -> Option<String> {
    let mut path = Path::new(digits);
    let mut output = CurveOutput::new(curve);
    let mut defined0 = false;
    for i in 0..=points.len() {
        let point = points.get(i).copied().flatten();
        if point.is_some() != defined0 {
            defined0 = !defined0;
            if defined0 {
                output.line_start();
            } else {
                output.line_end(&mut path);
            }
        }
        if let Some([x, y]) = point {
            output.point(&mut path, x, y);
        }
    }
    let data = path.data();
    (!data.is_empty()).then(|| data.to_owned())
}

/// One datum of an area: its `x0`, `y0`, and `x1`, `y1` where the area
/// has those accessors (`None` uses `x0` or `y0`, as d3 does when `x1` or
/// `y1` is null).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AreaPoint {
    pub x0: f64,
    pub y0: f64,
    pub x1: Option<f64>,
    pub y1: Option<f64>,
}

/// `area.curve(curve).digits(digits)(data)`, as [`line`].
pub fn area(points: &[Option<AreaPoint>], curve: Curve, digits: Option<u32>) -> Option<String> {
    let mut path = Path::new(digits);
    let mut output = CurveOutput::new(curve);
    let n = points.len();
    let (mut x0z, mut y0z) = (vec![0.0; n], vec![0.0; n]);
    let mut defined0 = false;
    let mut j = 0;
    for i in 0..=n {
        let point = points.get(i).copied().flatten();
        if point.is_some() != defined0 {
            defined0 = !defined0;
            if defined0 {
                j = i;
                output.area_start();
                output.line_start();
            } else {
                output.line_end(&mut path);
                output.line_start();
                for k in (j..i).rev() {
                    output.point(&mut path, x0z[k], y0z[k]);
                }
                output.line_end(&mut path);
                output.area_end();
            }
        }
        if let Some(p) = point {
            x0z[i] = p.x0;
            y0z[i] = p.y0;
            output.point(&mut path, p.x1.unwrap_or(p.x0), p.y1.unwrap_or(p.y0));
        }
    }
    let data = path.data();
    (!data.is_empty()).then(|| data.to_owned())
}
