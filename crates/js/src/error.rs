use std::fmt;

use crate::number_to_string;

/// An exception the original JavaScript throws, with its message. The ports
/// return it where the package would throw, so callers see the same
/// failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsError {
    pub message: String,
}

impl JsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// decimal.js-light's `invalidArgument` error, thrown by
    /// `new Decimal(value)` for `NaN` and infinities.
    pub fn decimal_invalid_argument(value: f64) -> Self {
        Self::new(format!(
            "[DecimalError] Invalid argument: {}",
            number_to_string(value)
        ))
    }

    /// A `RangeError` from `new Array(length)` with a negative length.
    pub fn invalid_array_length() -> Self {
        Self::new("Invalid array length")
    }

    /// V8's `RangeError` when recursion exhausts the stack.
    pub fn stack_overflow() -> Self {
        Self::new("Maximum call stack size exceeded")
    }
}

impl fmt::Display for JsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for JsError {}
