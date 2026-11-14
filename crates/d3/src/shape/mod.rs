//! d3-shape 3.2.1 (`dist/d3-shape.mjs`): the line and area generators and
//! the curves Recharts' `Curve` uses, the stack layout with the orders and
//! offsets Recharts uses, and the filled symbol types. Coordinates are given
//! already evaluated (what d3's accessors return, after `+`), with `None`
//! where the `defined` accessor is false.

mod curve;
mod line;
pub mod stack;
mod symbol;

pub use curve::Curve;
pub use line::{area, line, AreaPoint};
pub use symbol::{symbol, SymbolType};
