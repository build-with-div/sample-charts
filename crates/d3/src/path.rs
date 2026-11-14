//! d3-path 3.1.1's `Path` (`dist/d3-path.mjs`): SVG path data built from
//! canvas-style commands, with numbers written as JavaScript writes them,
//! rounded to `digits` decimal places when given (`pathRound`), which is how
//! d3-shape's generators use it (three digits unless set otherwise).
//!
//! The commands d3-shape's line and area generators issue are ported:
//! `moveTo`, `lineTo`, `quadraticCurveTo`, `bezierCurveTo`, `closePath`.
//! `arc`, `arcTo` and `rect` come with the arc generator.

use js::{math_round, number_to_string};

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
