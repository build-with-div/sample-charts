//! The ECMAScript semantics the sample's npm ports depend on, as V8 (the
//! engine in Chromium) implements them: number conversion and printing,
//! `Math`, the values chart data is made of, and the exceptions the ported
//! code throws.
//!
//! The ports (`d3`, `recharts`) are line-for-line translations of JavaScript,
//! so wherever the original relies on a language rule (`+value`,
//! `String(value)`, `Math.round`, `Map` key equality, property lookup), the
//! translation calls the function here that implements that rule. Each
//! function cites the specification section or V8 source it follows.

mod error;
pub mod math;
mod number;
mod value;

pub use error::JsError;
pub use number::{
    math_round, number_to_string, parse_float, shortest_digits, string_to_number, to_int32,
};
pub use value::{Key, Value};
