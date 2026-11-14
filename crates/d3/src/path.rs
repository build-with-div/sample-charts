//! d3-path 3.1.1's `Path` (`dist/d3-path.mjs`): SVG path data built from
//! canvas-style commands, with numbers written as JavaScript writes them,
//! rounded to `digits` decimal places when given (`pathRound`), which is how
//! d3-shape's generators use it (three digits unless set otherwise).
//!
//! The commands d3-shape's line, area and symbol generators issue are
//! ported: `moveTo`, `lineTo`, `quadraticCurveTo`, `bezierCurveTo`,
//! `closePath`, `rect` and `arc`, which covers complete circles, what the
//! circle symbol draws; other arcs and `arcTo` are not ported (Recharts
//! never draws them). Its `Math.cos` and `Math.sin` are
//! [`js::math::cos`] and [`js::math::sin`].

use js::{math_round, number_to_string};

/// `Math.PI` and the constants of d3-path.
const PI: f64 = std::f64::consts::PI;
const TAU: f64 = 2.0 * PI;
const EPSILON: f64 = 1e-6;
const TAU_EPSILON: f64 = TAU - EPSILON;

/// Why a path command could not be ported.
#[derive(Debug, Clone, PartialEq)]
pub enum PathError {
    /// `new Error(\`negative radius: ${r}\`)`, with the message.
    NegativeRadius(String),
    /// An arc that is neither empty nor a complete circle: not ported.
    PartialArc,
}

/// A path being built (`new Path(digits)`).
#[derive(Debug, Clone, PartialEq)]
pub struct Path {
    x0: Option<f64>,
    y0: Option<f64>,
    x1: Option<f64>,
    y1: Option<f64>,
    data: String,
    /// `10 ** digits` when numbers are rounded; `None` writes them as they
    /// are (no digits, or more than 15).
    scale: Option<f64>,
}

impl Path {
    /// `new Path(digits)`: `None` for `d3.path()`. As `appendRound`, digits
    /// above 15 write numbers unrounded.
    pub fn new(digits: Option<u32>) -> Path {
        Path {
            x0: None,
            y0: None,
            x1: None,
            y1: None,
            data: String::new(),
            // `10 ** d` is exact for d <= 15; parsing gives the same double
            // without depending on the C library's pow.
            scale: digits
                .filter(|d| *d <= 15)
                .map(|d| format!("1e{d}").parse().expect("a power of ten")),
        }
    }

    fn push(&mut self, value: f64) {
        let value = match self.scale {
            Some(k) => math_round(value * k) / k,
            None => value,
        };
        self.data.push_str(&number_to_string(value));
    }

    fn push_all(&mut self, command: char, values: &[f64]) {
        self.data.push(command);
        for (i, v) in values.iter().enumerate() {
            if i > 0 {
                self.data.push(',');
            }
            self.push(*v);
        }
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        (self.x0, self.x1, self.y0, self.y1) = (Some(x), Some(x), Some(y), Some(y));
        self.push_all('M', &[x, y]);
    }

    pub fn close_path(&mut self) {
        if self.x1.is_some() {
            (self.x1, self.y1) = (self.x0, self.y0);
            self.data.push('Z');
        }
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        (self.x1, self.y1) = (Some(x), Some(y));
        self.push_all('L', &[x, y]);
    }

    pub fn quadratic_curve_to(&mut self, x1: f64, y1: f64, x: f64, y: f64) {
        (self.x1, self.y1) = (Some(x), Some(y));
        self.push_all('Q', &[x1, y1, x, y]);
    }

    pub fn bezier_curve_to(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        (self.x1, self.y1) = (Some(x), Some(y));
        self.push_all('C', &[x1, y1, x2, y2, x, y]);
    }

    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        (self.x0, self.x1, self.y0, self.y1) = (Some(x), Some(x), Some(y), Some(y));
        self.push_all('M', &[x, y]);
        self.push_all('h', &[w]);
        self.push_all('v', &[h]);
        self.push_all('h', &[-w]);
        self.data.push('Z');
    }

    /// `arc(x, y, r, a0, a1, ccw)`.
    #[allow(clippy::too_many_arguments)]
    pub fn arc(
        &mut self,
        x: f64,
        y: f64,
        r: f64,
        a0: f64,
        a1: f64,
        ccw: bool,
    ) -> Result<(), PathError> {
        if r < 0.0 {
            return Err(PathError::NegativeRadius(format!(
                "negative radius: {}",
                number_to_string(r)
            )));
        }
        let (dx, dy) = (r * js::math::cos(a0), r * js::math::sin(a0));
        let (x0, y0) = (x + dx, y + dy);
        let cw = if ccw { 0.0 } else { 1.0 };
        let mut da = if ccw { a0 - a1 } else { a1 - a0 };
        match (self.x1, self.y1) {
            (None, _) | (_, None) => self.push_all('M', &[x0, y0]),
            (Some(x1), Some(y1)) => {
                if (x1 - x0).abs() > EPSILON || (y1 - y0).abs() > EPSILON {
                    self.push_all('L', &[x0, y0]);
                }
            }
        }
        // `if (!r) return;`
        if r == 0.0 || r.is_nan() {
            return Ok(());
        }
        if da < 0.0 {
            da = da % TAU + TAU;
        }
        if da > TAU_EPSILON {
            // A complete circle: two arcs.
            self.push_all('A', &[r, r]);
            self.data.push_str(",0,1,");
            self.push(cw);
            self.data.push(',');
            self.push(x - dx);
            self.data.push(',');
            self.push(y - dy);
            self.push_all('A', &[r, r]);
            self.data.push_str(",0,1,");
            self.push(cw);
            self.data.push(',');
            self.push(x0);
            self.data.push(',');
            self.push(y0);
            (self.x1, self.y1) = (Some(x0), Some(y0));
            Ok(())
        } else if da > EPSILON {
            Err(PathError::PartialArc)
        } else {
            Ok(())
        }
    }

    /// `path.toString()`.
    pub fn data(&self) -> &str {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_like_path_round() {
        let mut p = Path::new(Some(3));
        p.move_to(1.0 / 3.0, -0.0005);
        p.line_to(2.0, 12345.6789);
        p.close_path();
        assert_eq!(p.data(), "M0.333,0L2,12345.679Z");
        let mut p = Path::new(None);
        p.move_to(1.0 / 3.0, -0.0);
        assert_eq!(p.data(), "M0.3333333333333333,0");
    }
}
