//! Port of `recharts/lib/util/scale` (Recharts 3.8.0).

pub mod arithmetic;
pub mod nice_ticks;

pub use arithmetic::get_digit_count;
pub use nice_ticks::{get_nice_tick_values, get_tick_values_fixed_domain, NiceTicks};
