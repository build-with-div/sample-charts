//! d3-shape 3.2.1's `symbol()` generator and the filled symbol types
//! (`symbolsFill`): circle, cross, diamond, square, star, triangle and wye.
//! Paths are written by a [`Path`] with three digits, `withPath`'s default.
//!
//! The star's constants and points come from `Math.sin` and `Math.cos`
//! ([`js::math::sin`], [`js::math::cos`]): correctly rounded, so equal to
//! Chromium 147's except on the arguments Chromium does not round correctly
//! (`crates/js/tests/fixtures/chrome_trig_gaps.json`; none is a star's).

use crate::path::{Path, PathError};
use js::math::{cos, sin};

const PI: f64 = std::f64::consts::PI;
const TAU: f64 = 2.0 * PI;

/// A symbol type of `symbolsFill`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    Circle,
    Cross,
    Diamond,
    Square,
    Star,
    Triangle,
    Wye,
}

/// `type.draw(context, size)`.
fn draw(r#type: SymbolType, context: &mut Path, size: f64) -> Result<(), PathError> {
    let sqrt = f64::sqrt;
    match r#type {
        SymbolType::Circle => {
            let r = sqrt(size / PI);
            context.move_to(r, 0.0);
            context.arc(0.0, 0.0, r, 0.0, TAU, false)?;
        }
        SymbolType::Cross => {
            let r = sqrt(size / 5.0) / 2.0;
            context.move_to(-3.0 * r, -r);
            context.line_to(-r, -r);
            context.line_to(-r, -3.0 * r);
            context.line_to(r, -3.0 * r);
            context.line_to(r, -r);
            context.line_to(3.0 * r, -r);
            context.line_to(3.0 * r, r);
            context.line_to(r, r);
            context.line_to(r, 3.0 * r);
            context.line_to(-r, 3.0 * r);
            context.line_to(-r, r);
            context.line_to(-3.0 * r, r);
            context.close_path();
        }
        SymbolType::Diamond => {
            let tan30 = sqrt(1.0 / 3.0);
            let tan30_2 = tan30 * 2.0;
            let y = sqrt(size / tan30_2);
            let x = y * tan30;
            context.move_to(0.0, -y);
            context.line_to(x, 0.0);
            context.line_to(0.0, y);
            context.line_to(-x, 0.0);
            context.close_path();
        }
        SymbolType::Square => {
            let w = sqrt(size);
            let x = -w / 2.0;
            context.rect(x, x, w, w);
        }
        SymbolType::Star => {
            // d3's literal, which carries more digits than an f64 holds.
            #[allow(clippy::excessive_precision)]
            const KA: f64 = 0.89081309152928522810;
            let kr = sin(PI / 10.0) / sin(7.0 * PI / 10.0);
            let kx = sin(TAU / 10.0) * kr;
            let ky = -cos(TAU / 10.0) * kr;
            let r = sqrt(size * KA);
            let x = kx * r;
            let y = ky * r;
            context.move_to(0.0, -r);
            context.line_to(x, y);
            for i in 1..5 {
                let a = TAU * f64::from(i) / 5.0;
                let c = cos(a);
                let s = sin(a);
                context.line_to(s * r, -c * r);
                context.line_to(c * x - s * y, s * x + c * y);
            }
            context.close_path();
        }
        SymbolType::Triangle => {
            let sqrt3 = sqrt(3.0);
            let y = -sqrt(size / (sqrt3 * 3.0));
            context.move_to(0.0, y * 2.0);
            context.line_to(-sqrt3 * y, -y);
            context.line_to(sqrt3 * y, -y);
            context.close_path();
        }
        SymbolType::Wye => {
            let c = -0.5;
            let s = sqrt(3.0) / 2.0;
            let k = 1.0 / sqrt(12.0);
            let a = (k / 2.0 + 1.0) * 3.0;
            let r = sqrt(size / a);
            let (x0, y0) = (r / 2.0, r * k);
            let (x1, y1) = (x0, r * k + r);
            let (x2, y2) = (-x1, y1);
            context.move_to(x0, y0);
            context.line_to(x1, y1);
            context.line_to(x2, y2);
            context.line_to(c * x0 - s * y0, s * x0 + c * y0);
            context.line_to(c * x1 - s * y1, s * x1 + c * y1);
            context.line_to(c * x2 - s * y2, s * x2 + c * y2);
            context.line_to(c * x0 + s * y0, c * y0 - s * x0);
            context.line_to(c * x1 + s * y1, c * y1 - s * x1);
            context.line_to(c * x2 + s * y2, c * y2 - s * x2);
            context.close_path();
        }
    }
    Ok(())
}

/// `symbol().type(type).size(size)()`: the path data, or `None` (`null`)
/// for an empty path. `size` is the number `+size` gives.
pub fn symbol(r#type: SymbolType, size: f64) -> Result<Option<String>, PathError> {
    let mut path = Path::new(Some(3));
    draw(r#type, &mut path, size)?;
    let data = path.data();
    Ok((!data.is_empty()).then(|| data.to_owned()))
}
