//! `RechartsScale` (`util/scale/RechartsScale.js`): the d3 scale an axis
//! uses, wrapped with Recharts' `map`, and how `combineConfiguredScale`
//! (`state/selectors/combiners/combineConfiguredScale.js`) builds it.

use d3::scale::{Band, Linear, Log};
use js::{JsError, Value};

use super::ChartError;

/// The scale names `d3-scale` exports, as `scale` + `upperFirst(name)`.
const D3_SCALES: [&str; 27] = [
    "scaleBand",
    "scaleDiverging",
    "scaleDivergingLog",
    "scaleDivergingPow",
    "scaleDivergingSqrt",
    "scaleDivergingSymlog",
    "scaleIdentity",
    "scaleImplicit",
    "scaleLinear",
    "scaleLog",
    "scaleOrdinal",
    "scalePoint",
    "scalePow",
    "scaleQuantile",
    "scaleQuantize",
    "scaleRadial",
    "scaleSequential",
    "scaleSequentialLog",
    "scaleSequentialPow",
    "scaleSequentialQuantile",
    "scaleSequentialSqrt",
    "scaleSequentialSymlog",
    "scaleSqrt",
    "scaleSymlog",
    "scaleThreshold",
    "scaleTime",
    "scaleUtc",
];

/// `upperFirst(value)` of `util/DataUtils.js`: the first UTF-16 code unit
/// upper-cased.
fn upper_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if (first as u32) < 0x10000 => {
            let upper: String = first.to_uppercase().collect();
            format!("{upper}{}", chars.as_str())
        }
        _ => value.to_owned(),
    }
}

/// `"scale" + upperFirst(name)`: the d3 export a scale type names.
pub(crate) fn d3_scale_name(name: &str) -> String {
    format!("scale{}", upper_first(name))
}

/// `isSupportedScaleName` of `combineRealScaleType.js`.
pub(crate) fn is_supported_scale_name(name: &str) -> bool {
    D3_SCALES.contains(&d3_scale_name(name).as_str())
}

/// The configured d3 scale of an axis.
#[derive(Debug, Clone, PartialEq)]
pub enum RechartsScale {
    Linear(Linear),
    Log(Log),
    Band(Band),
}

/// Where in a band `map` places a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandPosition {
    Start,
    Middle,
    End,
}

impl RechartsScale {
    /// `combineConfiguredScaleInternal(realScaleType, axisDomain,
    /// axisRange)`: `getD3ScaleFromType(realScaleType)` given
    /// `domain(axisDomain)` and `range(axisRange)`, `None` (`undefined`)
    /// for a name d3-scale does not export. Linear, log, band and point
    /// scales are ported; the other d3 scales are not yet.
    pub fn configure(
        real_scale_type: &str,
        domain: &[Value],
        range: [f64; 2],
    ) -> Result<Option<RechartsScale>, ChartError> {
        let name = if D3_SCALES.contains(&real_scale_type) {
            real_scale_type.to_owned()
        } else {
            d3_scale_name(real_scale_type)
        };
        match name.as_str() {
            "scaleLinear" => {
                let mut scale = Linear::new();
                scale
                    .set_domain(domain.iter().map(Value::to_number).collect())
                    .set_range(range.to_vec());
                Ok(Some(RechartsScale::Linear(scale)))
            }
            "scaleLog" => {
                let mut scale = Log::new();
                scale
                    .set_domain(domain.iter().map(Value::to_number).collect())
                    .set_range(range.to_vec());
                Ok(Some(RechartsScale::Log(scale)))
            }
            // `scaleImplicit` is d3-scale's unknown-value symbol, not a
            // constructor, so `d3Scales[name]()` throws. The message is the
            // one V8 gives for Recharts' es6 source; a minifying bundler
            // renames `d3Scales` and changes it.
            "scaleImplicit" => Err(ChartError::Js(JsError::new(
                "d3Scales[name] is not a function",
            ))),
            "scaleBand" | "scalePoint" => {
                let mut scale = if name == "scaleBand" {
                    Band::new()
                } else {
                    Band::point()
                };
                scale
                    .set_domain(domain.iter().cloned())
                    .map_err(|_| ChartError::Unsupported("object domain values"))?
                    .set_range(range);
                Ok(Some(RechartsScale::Band(scale)))
            }
            _ if D3_SCALES.contains(&name.as_str()) => Err(ChartError::Unsupported(
                "d3 scales other than linear, log, band and point",
            )),
            _ => Ok(None),
        }
    }

    /// `scale.domain()`.
    pub fn domain(&self) -> Vec<Value> {
        match self {
            RechartsScale::Linear(scale) => {
                scale.domain().iter().map(|n| Value::Number(*n)).collect()
            }
            RechartsScale::Log(scale) => scale.domain().iter().map(|n| Value::Number(*n)).collect(),
            RechartsScale::Band(scale) => scale.domain().to_vec(),
        }
    }

    /// The d3 scale's `range()`.
    pub fn d3_range(&self) -> Vec<f64> {
        match self {
            RechartsScale::Linear(scale) => scale.range().to_vec(),
            RechartsScale::Log(scale) => scale.range().to_vec(),
            RechartsScale::Band(scale) => scale.range().to_vec(),
        }
    }

    /// `scale.bandwidth()`, for band and point scales.
    pub fn bandwidth(&self) -> Option<f64> {
        match self {
            RechartsScale::Linear(_) | RechartsScale::Log(_) => None,
            RechartsScale::Band(scale) => Some(scale.bandwidth()),
        }
    }

    /// `scale.ticks(count)`, for linear and log scales.
    pub fn ticks(&self, count: f64) -> Option<Result<Vec<f64>, js::JsError>> {
        match self {
            RechartsScale::Linear(scale) => Some(scale.ticks(count)),
            RechartsScale::Log(scale) => Some(scale.ticks(count)),
            RechartsScale::Band(_) => None,
        }
    }

    /// `rangeMin()` and `rangeMax()`.
    pub fn range_extent(&self) -> [f64; 2] {
        let range = self.d3_range();
        [
            js::math::min_of(range.iter().copied()),
            js::math::max_of(range.iter().copied()),
        ]
    }

    /// `map(input, { position })`: the d3 scale's output, moved within the
    /// band for band and point scales; `None` where d3 returns `undefined`.
    pub fn map(&self, input: &Value, position: Option<BandPosition>) -> Option<f64> {
        match self {
            RechartsScale::Linear(scale) => {
                if input.is_nullish() {
                    return None;
                }
                scale.apply(input.to_number())
            }
            RechartsScale::Log(scale) => {
                if input.is_nullish() {
                    return None;
                }
                scale.apply(input.to_number())
            }
            RechartsScale::Band(scale) => {
                let base = scale.apply(input)?;
                let bandwidth = scale.bandwidth();
                Some(match position {
                    Some(BandPosition::Middle) => base + bandwidth / 2.0,
                    Some(BandPosition::End) => base + bandwidth,
                    _ => base,
                })
            }
        }
    }
}
