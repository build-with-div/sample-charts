//! A port of the [Recharts](https://recharts.org) npm package, version 3.8.0,
//! onto Div, following the sample's porting principle: the package's own
//! logic is ported line for line, with its numeric semantics, and the web
//! platform features it runs on are Div's job, not this crate's.
//!
//! Every module names the JavaScript file it ports. The pinned packages
//! (`recharts`, `decimal.js-light`, `victory-vendor`) are installed by
//! `tools/reference` and run to produce the fixtures the tests compare
//! against exactly.
//!
//! What is here so far: the axis tick algorithm (`scale::nice_ticks`) and
//! the parts of `decimal.js-light` it uses.

pub mod decimal;
pub mod js;
pub mod scale;

pub use js::JsError;
