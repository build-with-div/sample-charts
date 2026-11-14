//! The JavaScript values chart data is made of, with the conversions and
//! comparisons the ported code applies to them.
//!
//! The model covers what JSON describes, plus `undefined`: no functions,
//! dates, symbols or object identity. Property lookup finds own properties
//! only (array indices and `length`, string indices and `length`, object
//! keys); inherited ones such as `toString` read as `undefined` here, where
//! JavaScript would find a function.

use crate::number::{number_to_string, string_to_number};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    /// An object's own enumerable properties, in insertion order.
    Object(Vec<(String, Value)>),
}

impl Value {
    pub fn is_nullish(&self) -> bool {
        matches!(self, Value::Undefined | Value::Null)
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// ToBoolean (ECMA-262 7.1.2): whether `if (value)` takes the branch.
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Undefined | Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => !(n.is_nan() || *n == 0.0),
            Value::String(s) => !s.is_empty(),
            Value::Array(_) | Value::Object(_) => true,
        }
    }

    /// ToNumber (ECMA-262 7.1.4), as in `+value` or `Number(value)`. Arrays
    /// and objects convert through their string form, as
    /// `Array.prototype.toString` and `Object.prototype.toString` give it.
    pub fn to_number(&self) -> f64 {
        match self {
            Value::Undefined => f64::NAN,
            Value::Null => 0.0,
            Value::Bool(b) => f64::from(u8::from(*b)),
            Value::Number(n) => *n,
            Value::String(s) => string_to_number(s),
            Value::Array(_) | Value::Object(_) => string_to_number(&self.to_js_string()),
        }
    }

    /// ToString (ECMA-262 7.1.17), as in `String(value)`.
    pub fn to_js_string(&self) -> String {
        match self {
            Value::Undefined => "undefined".into(),
            Value::Null => "null".into(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => number_to_string(*n),
            Value::String(s) => s.clone(),
            Value::Array(items) => items
                .iter()
                .map(|item| {
                    if item.is_nullish() {
                        String::new()
                    } else {
                        item.to_js_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(","),
            Value::Object(_) => "[object Object]".into(),
        }
    }

    /// `a === b` (IsStrictlyEqual, ECMA-262 7.2.15). Arrays and objects have
    /// no identity in this model, so they are never strictly equal.
    pub fn strict_equals(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Undefined, Value::Undefined) | (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }

    /// `value[key]` for an own property, `undefined` otherwise.
    pub fn get(&self, key: &str) -> Value {
        match self {
            Value::Object(entries) => entries
                .iter()
                .rev()
                .find(|(k, _)| k == key)
                .map_or(Value::Undefined, |(_, v)| v.clone()),
            Value::Array(items) => {
                if key == "length" {
                    return Value::Number(items.len() as f64);
                }
                array_index(key)
                    .and_then(|i| items.get(i))
                    .cloned()
                    .unwrap_or(Value::Undefined)
            }
            Value::String(s) => {
                let units: Vec<u16> = s.encode_utf16().collect();
                if key == "length" {
                    return Value::Number(units.len() as f64);
                }
                array_index(key)
                    .and_then(|i| units.get(i))
                    .map_or(Value::Undefined, |unit| {
                        Value::String(String::from_utf16_lossy(&[*unit]))
                    })
            }
            _ => Value::Undefined,
        }
    }
}

/// `key` as an array index if it is the canonical string of one.
fn array_index(key: &str) -> Option<usize> {
    let index: u32 = key.parse().ok()?;
    (index != u32::MAX && index.to_string() == key).then_some(index as usize)
}

impl From<f64> for Value {
    fn from(n: f64) -> Self {
        Value::Number(n)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.into())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

/// A primitive as a `Map` or `Set` key: equal under SameValueZero
/// (ECMA-262 7.2.11), so NaN equals NaN and -0 equals +0. This is also the
/// key d3's `InternMap` uses for primitives.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Undefined,
    Null,
    Bool(bool),
    /// The bits of the number, with NaN and -0 canonicalised.
    Number(u64),
    String(String),
}

impl Key {
    pub fn number(n: f64) -> Key {
        let canonical = if n.is_nan() {
            f64::NAN
        } else if n == 0.0 {
            0.0
        } else {
            n
        };
        Key::Number(canonical.to_bits())
    }

    /// The key of a primitive value; `None` for arrays and objects, whose
    /// identity this model does not track.
    pub fn of(value: &Value) -> Option<Key> {
        Some(match value {
            Value::Undefined => Key::Undefined,
            Value::Null => Key::Null,
            Value::Bool(b) => Key::Bool(*b),
            Value::Number(n) => Key::number(*n),
            Value::String(s) => Key::String(s.clone()),
            Value::Array(_) | Value::Object(_) => return None,
        })
    }

    pub fn to_value(&self) -> Value {
        match self {
            Key::Undefined => Value::Undefined,
            Key::Null => Value::Null,
            Key::Bool(b) => Value::Bool(*b),
            Key::Number(bits) => Value::Number(f64::from_bits(*bits)),
            Key::String(s) => Value::String(s.clone()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conversions_match_javascript() {
        assert!(Value::Undefined.to_number().is_nan());
        assert_eq!(Value::Null.to_number(), 0.0);
        assert_eq!(Value::Array(vec![]).to_number(), 0.0);
        assert_eq!(Value::Array(vec![Value::from("7")]).to_number(), 7.0);
        assert!(Value::Array(vec![1.0.into(), 2.0.into()])
            .to_number()
            .is_nan());
        assert_eq!(
            Value::Array(vec![
                1.0.into(),
                Value::Null,
                Value::Array(vec![2.0.into(), 3.0.into()])
            ])
            .to_js_string(),
            "1,,2,3"
        );
        assert_eq!(Value::Object(vec![]).to_js_string(), "[object Object]");
    }

    #[test]
    fn property_lookup() {
        let row = Value::Object(vec![("a".into(), 1.0.into())]);
        assert_eq!(row.get("a"), Value::Number(1.0));
        assert_eq!(row.get("b"), Value::Undefined);
        let list = Value::Array(vec!["x".into(), "y".into()]);
        assert_eq!(list.get("1"), Value::from("y"));
        assert_eq!(list.get("01"), Value::Undefined);
        assert_eq!(list.get("length"), Value::Number(2.0));
        assert_eq!(Value::from("héllo").get("1"), Value::from("é"));
    }

    #[test]
    fn keys_use_same_value_zero() {
        assert_eq!(Key::number(-0.0), Key::number(0.0));
        assert_eq!(Key::number(f64::NAN), Key::number(-f64::NAN));
        assert_ne!(Key::number(1.0), Key::String("1".into()));
    }
}
