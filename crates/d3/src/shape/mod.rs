//! d3-shape 3.2.1 (`dist/d3-shape.mjs`): the line and area generators and
//! the curves Recharts' `Curve` uses, and the stack layout with the orders
//! and offsets Recharts uses. Coordinates are given already evaluated (what
//! d3's accessors return, after `+`), with `None` where the `defined`
//! accessor is false.

mod curve;
mod line;
pub mod stack;

pub use curve::Curve;
pub use line::{area, line, AreaPoint};
