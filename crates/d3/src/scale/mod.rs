//! Port of d3-scale 4.0.3's linear, log, time, band and point scales.

mod band;
mod linear;
mod log;
mod time;

pub use band::Band;
pub use linear::Linear;
pub use log::Log;
pub use time::{domain_number as time_domain_number, Time};
