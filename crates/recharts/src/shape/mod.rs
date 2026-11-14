//! Recharts' shapes (`shape/*.js`): the SVG path data they draw.

mod curve;

pub use curve::{curve_factory, get_path, BaseLine, CurvePoint};
