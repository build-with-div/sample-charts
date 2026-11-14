//! `shape/Symbols.js`: the `<path>` a `<Symbols>` draws (legend icons,
//! scatter points), through d3-shape's symbol generator.
//!
//! Symbols registered at run time (`Symbols.registerSymbol`) are not
//! modelled. The star needs `Math.tan` (its size by diameter) and
//! `Math.sin`/`Math.cos` (its points) as Chromium computes them, not ported
//! yet: it returns [`PathError::Trigonometry`].

use d3::path::PathError;
use d3::shape::{symbol, SymbolType};
use js::number_to_string;

use super::upper_first;

const PI: f64 = std::f64::consts::PI;

/// What `<Symbols>` renders: the path's `transform` and `d` (`None` when
/// the attribute is left out).
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolsPath {
    pub transform: String,
    pub d: Option<String>,
}

/// `getSymbolFactory(type)`: the d3 symbol named `"symbol" + upperFirst(type)`,
/// or the circle.
fn symbol_factory(r#type: &str) -> SymbolType {
    match format!("symbol{}", upper_first(r#type)).as_str() {
        "symbolCross" => SymbolType::Cross,
        "symbolDiamond" => SymbolType::Diamond,
        "symbolSquare" => SymbolType::Square,
        "symbolStar" => SymbolType::Star,
        "symbolTriangle" => SymbolType::Triangle,
        "symbolWye" => SymbolType::Wye,
        _ => SymbolType::Circle,
    }
}

/// `calculateAreaSize(size, sizeType, type)`: `size` is an area unless
/// `sizeType` is something other than `'area'`, when it is a diameter.
fn area_size(size: f64, size_type: &str, r#type: &str) -> Result<f64, PathError> {
    if size_type == "area" {
        return Ok(size);
    }
    let sqrt3 = 3f64.sqrt();
    Ok(match r#type {
        "cross" => 5.0 * size * size / 9.0,
        "diamond" => 0.5 * size * size / sqrt3,
        "square" => size * size,
        // 1.25 * size * size * (Math.tan(angle) - Math.tan(angle * 2) * Math.tan(angle) ** 2)
        "star" => return Err(PathError::Trigonometry),
        "triangle" => sqrt3 * size * size / 4.0,
        "wye" => (21.0 - 10.0 * sqrt3) * size * size / 8.0,
        _ => PI * size * size / 4.0,
    })
}

/// `<Symbols type size sizeType cx cy>`: `type` is `None` when it is not a
/// string (`undefined` included: `'circle'`), `size` and `size_type` `None`
/// when `undefined` (64 and `'area'`), `cx`/`cy` `None` when not numbers.
/// `Ok(None)` when nothing renders (`cx`, `cy` or `size` not a number or
/// NaN).
pub fn symbols(
    r#type: Option<&str>,
    size: Option<f64>,
    size_type: Option<&str>,
    cx: Option<f64>,
    cy: Option<f64>,
) -> Result<Option<SymbolsPath>, PathError> {
    let real_type = r#type.unwrap_or("circle");
    let size = size.unwrap_or(64.0);
    let size_type = size_type.unwrap_or("area");
    let (Some(cx), Some(cy)) = (cx, cy) else {
        return Ok(None);
    };
    if cx.is_nan() || cy.is_nan() || size.is_nan() {
        return Ok(None);
    }
    let area = area_size(size, size_type, real_type)?;
    let d = symbol(symbol_factory(real_type), area)?;
    Ok(Some(SymbolsPath {
        transform: format!(
            "translate({}, {})",
            number_to_string(cx),
            number_to_string(cy)
        ),
        d,
    }))
}
