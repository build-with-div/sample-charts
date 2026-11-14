//! A port of the [d3](https://d3js.org) modules Recharts 3.8.0 uses for its
//! axes: from d3-array 3.2.5 the tick and range helpers, from d3-interpolate
//! 3.0.2 the number interpolators, and from d3-scale 4.0.3 the linear, band
//! and point scales.
//!
//! These are the npm packages a page gets: Recharts' browser build imports
//! `victory-vendor/d3-scale`, whose "import" export re-exports `d3-scale`.
//! (Its CommonJS build uses copies vendored inside victory-vendor, whose tick
//! code is older and gives different ticks for some domains.)
//!
//! Numbers are JavaScript numbers (`f64`) and `Math` calls go through the
//! `js` crate, so results equal Chromium's bit for bit. The tests compare
//! against fixtures written by running the packages
//! (`tools/reference/axis_fixtures.mjs`).
//!
//! Comparisons keep d3's form, `!(x > 0)` and the like, where NaN must
//! take the negated branch.

#![allow(clippy::neg_cmp_op_on_partial_ord)]

pub mod array;
pub mod interpolate;
pub mod path;
pub mod scale;
pub mod shape;
