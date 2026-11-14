//! Recharts' shapes: the path data of `<Curve>` (lines and areas),
//! `<Rectangle>` (bars) and `<Symbols>` (legend icons, scatter points).

mod curve;
mod rectangle;
mod symbols;

pub use curve::{curve_factory, get_path, BaseLine, CurvePoint};
pub use rectangle::{get_rectangle_path, rectangle, Radius};
pub use symbols::{symbols, SymbolsPath};

/// `upperFirst` of `util/DataUtils.js` for a string: `charAt(0)`
/// upper-cased, then the rest.
pub(crate) fn upper_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if (first as u32) < 0x10000 => {
            format!(
                "{}{}",
                first.to_uppercase().collect::<String>(),
                chars.as_str()
            )
        }
        _ => value.to_owned(),
    }
}
