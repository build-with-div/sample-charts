//! The curves of d3-shape 3.2.1 that Recharts' `Curve` offers: linear,
//! linearClosed, basis, basisClosed, basisOpen, bumpX, bumpY, monotoneX,
//! monotoneY, natural, step, stepBefore and stepAfter.
//!
//! A d3 curve is an object receiving `areaStart`, `areaEnd`, `lineStart`,
//! `lineEnd` and `point` calls; [`CurveOutput`] is that object for any of
//! these curves. Its `_line` field starts `undefined`, which behaves as NaN
//! in every use (falsy, `!== 0`, `1 - _line` is NaN, `>= 0` is false), so it
//! is NaN here.

use js::math;

use crate::path::Path;

/// A d3 curve factory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Curve {
    Basis,
    BasisClosed,
    BasisOpen,
    BumpX,
    BumpY,
    Linear,
    LinearClosed,
    MonotoneX,
    MonotoneY,
    Natural,
    Step,
    StepAfter,
    StepBefore,
}

/// JavaScript truthiness of a number.
fn truthy(n: f64) -> bool {
    n != 0.0 && !n.is_nan()
}

/// `sign(x)` of the monotone curves: -1 below zero, else 1.
fn sign(x: f64) -> f64 {
    if x < 0.0 {
        -1.0
    } else {
        1.0
    }
}

pub(crate) struct CurveOutput {
    curve: Curve,
    line: f64,
    point: u8,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    x3: f64,
    y3: f64,
    x4: f64,
    y4: f64,
    t0: f64,
    /// `_t` of the step curves.
    t: f64,
    /// `_x`, `_y` of the natural curve.
    xs: Vec<f64>,
    ys: Vec<f64>,
}

impl CurveOutput {
    pub(crate) fn new(curve: Curve) -> CurveOutput {
        CurveOutput {
            curve,
            line: f64::NAN,
            point: 0,
            x0: f64::NAN,
            y0: f64::NAN,
            x1: f64::NAN,
            y1: f64::NAN,
            x2: f64::NAN,
            y2: f64::NAN,
            x3: f64::NAN,
            y3: f64::NAN,
            x4: f64::NAN,
            y4: f64::NAN,
            t0: f64::NAN,
            t: match curve {
                Curve::Step => 0.5,
                Curve::StepBefore => 0.0,
                _ => 1.0,
            },
            xs: Vec::new(),
            ys: Vec::new(),
        }
    }

    // The context, through `ReflectContext` for monotoneY.
    fn move_to(&self, path: &mut Path, x: f64, y: f64) {
        match self.curve {
            Curve::MonotoneY => path.move_to(y, x),
            _ => path.move_to(x, y),
        }
    }

    fn line_to(&self, path: &mut Path, x: f64, y: f64) {
        match self.curve {
            Curve::MonotoneY => path.line_to(y, x),
            _ => path.line_to(x, y),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn bezier(&self, path: &mut Path, x1: f64, y1: f64, x2: f64, y2: f64, x: f64, y: f64) {
        match self.curve {
            Curve::MonotoneY => path.bezier_curve_to(y1, x1, y2, x2, y, x),
            _ => path.bezier_curve_to(x1, y1, x2, y2, x, y),
        }
    }

    /// `_line ? lineTo : moveTo`.
    fn start(&self, path: &mut Path, x: f64, y: f64) {
        if truthy(self.line) {
            self.line_to(path, x, y);
        } else {
            self.move_to(path, x, y);
        }
    }

    /// The common end of a line: close the area's second line, or a line of
    /// one point (`point_count` is the curve's count for that).
    fn close_and_flip(&mut self, path: &mut Path, single: bool) {
        if truthy(self.line) || (self.line != 0.0 && single) {
            path.close_path();
        }
        self.line = 1.0 - self.line;
    }

    pub(crate) fn area_start(&mut self) {
        if !matches!(self.curve, Curve::BasisClosed | Curve::LinearClosed) {
            self.line = 0.0;
        }
    }

    pub(crate) fn area_end(&mut self) {
        if !matches!(self.curve, Curve::BasisClosed | Curve::LinearClosed) {
            self.line = f64::NAN;
        }
    }

    pub(crate) fn line_start(&mut self) {
        match self.curve {
            Curve::Natural => {
                self.xs.clear();
                self.ys.clear();
            }
            Curve::Basis | Curve::BasisOpen => {
                (self.x0, self.x1, self.y0, self.y1) = (f64::NAN, f64::NAN, f64::NAN, f64::NAN);
            }
            Curve::BasisClosed => {
                for v in [
                    &mut self.x0,
                    &mut self.x1,
                    &mut self.x2,
                    &mut self.x3,
                    &mut self.x4,
                    &mut self.y0,
                    &mut self.y1,
                    &mut self.y2,
                    &mut self.y3,
                    &mut self.y4,
                ] {
                    *v = f64::NAN;
                }
            }
            Curve::MonotoneX | Curve::MonotoneY => {
                (self.x0, self.x1, self.y0, self.y1, self.t0) =
                    (f64::NAN, f64::NAN, f64::NAN, f64::NAN, f64::NAN);
            }
            Curve::Step | Curve::StepAfter | Curve::StepBefore => {
                // `_x`, `_y`
                (self.x0, self.y0) = (f64::NAN, f64::NAN);
            }
            Curve::Linear | Curve::LinearClosed | Curve::BumpX | Curve::BumpY => {}
        }
        self.point = 0;
    }

    pub(crate) fn line_end(&mut self, path: &mut Path) {
        match self.curve {
            Curve::Linear | Curve::BumpX | Curve::BumpY => {
                let single = self.point == 1;
                self.close_and_flip(path, single);
            }
            Curve::LinearClosed => {
                if self.point != 0 {
                    path.close_path();
                }
            }
            Curve::Basis => {
                if self.point == 3 {
                    self.basis_point(path, self.x1, self.y1);
                }
                if self.point == 3 || self.point == 2 {
                    self.line_to(path, self.x1, self.y1);
                }
                let single = self.point == 1;
                self.close_and_flip(path, single);
            }
            Curve::BasisClosed => match self.point {
                1 => {
                    self.move_to(path, self.x2, self.y2);
                    path.close_path();
                }
                2 => {
                    self.move_to(
                        path,
                        (self.x2 + 2.0 * self.x3) / 3.0,
                        (self.y2 + 2.0 * self.y3) / 3.0,
                    );
                    self.line_to(
                        path,
                        (self.x3 + 2.0 * self.x2) / 3.0,
                        (self.y3 + 2.0 * self.y2) / 3.0,
                    );
                    path.close_path();
                }
                3 => {
                    let (x2, y2, x3, y3, x4, y4) =
                        (self.x2, self.y2, self.x3, self.y3, self.x4, self.y4);
                    self.point(path, x2, y2);
                    self.point(path, x3, y3);
                    self.point(path, x4, y4);
                }
                _ => {}
            },
            Curve::BasisOpen => {
                let single = self.point == 3;
                self.close_and_flip(path, single);
            }
            Curve::MonotoneX | Curve::MonotoneY => {
                match self.point {
                    2 => self.line_to(path, self.x1, self.y1),
                    3 => {
                        let t0 = self.t0;
                        let t1 = self.slope2(t0);
                        self.hermite(path, t0, t1);
                    }
                    _ => {}
                }
                let single = self.point == 1;
                self.close_and_flip(path, single);
            }
            Curve::Natural => {
                let (x, y) = (std::mem::take(&mut self.xs), std::mem::take(&mut self.ys));
                let n = x.len();
                if n > 0 {
                    self.start(path, x[0], y[0]);
                    if n == 2 {
                        self.line_to(path, x[1], y[1]);
                    } else if n > 2 {
                        // (With one point d3 computes control points it
                        // never uses.)
                        let (px, py) = (control_points(&x), control_points(&y));
                        for i0 in 0..n - 1 {
                            let i1 = i0 + 1;
                            self.bezier(path, px.0[i0], py.0[i0], px.1[i0], py.1[i0], x[i1], y[i1]);
                        }
                    }
                }
                self.close_and_flip(path, n == 1);
            }
            Curve::Step | Curve::StepAfter | Curve::StepBefore => {
                if 0.0 < self.t && self.t < 1.0 && self.point == 2 {
                    self.line_to(path, self.x0, self.y0);
                }
                if truthy(self.line) || (self.line != 0.0 && self.point == 1) {
                    path.close_path();
                }
                if self.line >= 0.0 {
                    self.t = 1.0 - self.t;
                    self.line = 1.0 - self.line;
                }
            }
        }
    }

    pub(crate) fn point(&mut self, path: &mut Path, x: f64, y: f64) {
        match self.curve {
            Curve::Linear => {
                if self.point == 0 {
                    self.point = 1;
                    self.start(path, x, y);
                } else {
                    self.point = 2;
                    self.line_to(path, x, y);
                }
            }
            Curve::LinearClosed => {
                if self.point != 0 {
                    self.line_to(path, x, y);
                } else {
                    self.point = 1;
                    self.move_to(path, x, y);
                }
            }
            Curve::BumpX | Curve::BumpY => {
                if self.point == 0 {
                    self.point = 1;
                    self.start(path, x, y);
                } else {
                    self.point = 2;
                    if self.curve == Curve::BumpX {
                        self.x0 = (self.x0 + x) / 2.0;
                        self.bezier(path, self.x0, self.y0, self.x0, y, x, y);
                    } else {
                        self.y0 = (self.y0 + y) / 2.0;
                        self.bezier(path, self.x0, self.y0, x, self.y0, x, y);
                    }
                }
                (self.x0, self.y0) = (x, y);
            }
            Curve::Basis => {
                match self.point {
                    0 => {
                        self.point = 1;
                        self.start(path, x, y);
                    }
                    1 => self.point = 2,
                    2 => {
                        self.point = 3;
                        self.line_to(
                            path,
                            (5.0 * self.x0 + self.x1) / 6.0,
                            (5.0 * self.y0 + self.y1) / 6.0,
                        );
                        self.basis_point(path, x, y);
                    }
                    _ => self.basis_point(path, x, y),
                }
                self.shift(x, y);
            }
            Curve::BasisClosed => {
                match self.point {
                    0 => {
                        self.point = 1;
                        (self.x2, self.y2) = (x, y);
                    }
                    1 => {
                        self.point = 2;
                        (self.x3, self.y3) = (x, y);
                    }
                    2 => {
                        self.point = 3;
                        (self.x4, self.y4) = (x, y);
                        self.move_to(
                            path,
                            (self.x0 + 4.0 * self.x1 + x) / 6.0,
                            (self.y0 + 4.0 * self.y1 + y) / 6.0,
                        );
                    }
                    _ => self.basis_point(path, x, y),
                }
                self.shift(x, y);
            }
            Curve::BasisOpen => {
                match self.point {
                    0 => self.point = 1,
                    1 => self.point = 2,
                    2 => {
                        self.point = 3;
                        let x0 = (self.x0 + 4.0 * self.x1 + x) / 6.0;
                        let y0 = (self.y0 + 4.0 * self.y1 + y) / 6.0;
                        self.start(path, x0, y0);
                    }
                    _ => {
                        if self.point == 3 {
                            self.point = 4;
                        }
                        self.basis_point(path, x, y);
                    }
                }
                self.shift(x, y);
            }
            Curve::MonotoneX | Curve::MonotoneY => {
                // monotoneY receives (y, x) and reflects its output.
                let (x, y) = if self.curve == Curve::MonotoneY {
                    (y, x)
                } else {
                    (x, y)
                };
                let mut t1 = f64::NAN;
                if x == self.x1 && y == self.y1 {
                    return;
                }
                match self.point {
                    0 => {
                        self.point = 1;
                        self.start(path, x, y);
                    }
                    1 => self.point = 2,
                    2 => {
                        self.point = 3;
                        t1 = self.slope3(x, y);
                        let t0 = self.slope2(t1);
                        self.hermite(path, t0, t1);
                    }
                    _ => {
                        t1 = self.slope3(x, y);
                        self.hermite(path, self.t0, t1);
                    }
                }
                self.shift(x, y);
                self.t0 = t1;
            }
            Curve::Natural => {
                self.xs.push(x);
                self.ys.push(y);
            }
            Curve::Step | Curve::StepAfter | Curve::StepBefore => {
                if self.point == 0 {
                    self.point = 1;
                    self.start(path, x, y);
                } else {
                    self.point = 2;
                    if self.t <= 0.0 {
                        self.line_to(path, self.x0, y);
                        self.line_to(path, x, y);
                    } else {
                        let x1 = self.x0 * (1.0 - self.t) + x * self.t;
                        self.line_to(path, x1, self.y0);
                        self.line_to(path, x1, y);
                    }
                }
                (self.x0, self.y0) = (x, y);
            }
        }
    }

    /// `_x0 = _x1, _x1 = x` and the same for y.
    fn shift(&mut self, x: f64, y: f64) {
        (self.x0, self.x1) = (self.x1, x);
        (self.y0, self.y1) = (self.y1, y);
    }

    /// `point(that, x, y)` of the basis curves.
    fn basis_point(&self, path: &mut Path, x: f64, y: f64) {
        self.bezier(
            path,
            (2.0 * self.x0 + self.x1) / 3.0,
            (2.0 * self.y0 + self.y1) / 3.0,
            (self.x0 + 2.0 * self.x1) / 3.0,
            (self.y0 + 2.0 * self.y1) / 3.0,
            (self.x0 + 4.0 * self.x1 + x) / 6.0,
            (self.y0 + 4.0 * self.y1 + y) / 6.0,
        );
    }

    /// `slope3(that, x2, y2)` of the monotone curves.
    fn slope3(&self, x2: f64, y2: f64) -> f64 {
        let h0 = self.x1 - self.x0;
        let h1 = x2 - self.x1;
        // `h || other < 0 && -0`: h when truthy, else -0 or false (0).
        let divisor = |h: f64, other: f64| {
            if truthy(h) {
                h
            } else if other < 0.0 {
                -0.0
            } else {
                0.0
            }
        };
        let s0 = (self.y1 - self.y0) / divisor(h0, h1);
        let s1 = (y2 - self.y1) / divisor(h1, h0);
        let p = (s0 * h1 + s1 * h0) / (h0 + h1);
        let t = (sign(s0) + sign(s1)) * math::min_of([s0.abs(), s1.abs(), 0.5 * p.abs()]);
        if truthy(t) {
            t
        } else {
            0.0
        }
    }

    /// `slope2(that, t)`.
    fn slope2(&self, t: f64) -> f64 {
        let h = self.x1 - self.x0;
        if truthy(h) {
            (3.0 * (self.y1 - self.y0) / h - t) / 2.0
        } else {
            t
        }
    }

    /// `point(that, t0, t1)` of the monotone curves.
    fn hermite(&self, path: &mut Path, t0: f64, t1: f64) {
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1, self.y1);
        let dx = (x1 - x0) / 3.0;
        self.bezier(path, x0 + dx, y0 + dx * t0, x1 - dx, y1 - dx * t1, x1, y1);
    }
}

/// `controlPoints(x)` of the natural curve: `[a, b]`.
fn control_points(x: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let n = x.len() - 1;
    let (mut a, mut b, mut r) = (vec![0.0; n], vec![0.0; n], vec![0.0; n]);
    a[0] = 0.0;
    b[0] = 2.0;
    r[0] = x[0] + 2.0 * x[1];
    for i in 1..n.saturating_sub(1) {
        a[i] = 1.0;
        b[i] = 4.0;
        r[i] = 4.0 * x[i] + 2.0 * x[i + 1];
    }
    a[n - 1] = 2.0;
    b[n - 1] = 7.0;
    r[n - 1] = 8.0 * x[n - 1] + x[n];
    for i in 1..n {
        let m = a[i] / b[i - 1];
        b[i] -= m;
        r[i] -= m * r[i - 1];
    }
    a[n - 1] = r[n - 1] / b[n - 1];
    for i in (0..n.saturating_sub(1)).rev() {
        a[i] = (r[i] - a[i + 1]) / b[i];
    }
    b[n - 1] = (x[n] + a[n - 1]) / 2.0;
    for i in 0..n.saturating_sub(1) {
        b[i] = 2.0 * x[i + 1] - a[i + 1];
    }
    (a, b)
}
