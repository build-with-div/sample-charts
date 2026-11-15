//! `<Text>` (`component/Text.js`): SVG `<text>` with one `<tspan>` per
//! line, the lines found by measuring words (`getWordsByLines`) when the
//! text has a width; with `util/ReduceCSSCalc.js` for the first line's
//! offset and `util/DOMUtils.js`'s `getStringSize` for the measuring.

use std::collections::HashMap;

use js::{number_to_string, parse_float, Value};

use super::svg_props::svg_properties;
use super::Measure;
use crate::chart::ChartError;
use crate::dom::{clsx, Element, Node, Props};

/// A size `getBoundingClientRect` gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f64,
    pub height: f64,
}

/// `createCacheKey(text, style)`: what `getStringSize` caches a
/// measurement under, `String(text)` and six of the style's properties
/// (each `|| ''`).
pub fn string_cache_key(text: &str, style: &Props) -> String {
    let mut key = text.to_owned();
    for name in [
        "fontSize",
        "fontFamily",
        "fontWeight",
        "fontStyle",
        "letterSpacing",
        "textTransform",
    ] {
        key.push('|');
        let value = style.get(name);
        if value.is_truthy() {
            key.push_str(&value.to_js_string());
        }
    }
    key
}

/// `getStringSize` with its cache (`stringCache`, an LRU cache of 2,000
/// entries; a chart that measures fewer distinct strings never evicts).
#[derive(Default)]
pub(super) struct StringSizes {
    cache: HashMap<String, Size>,
}

impl StringSizes {
    /// `getStringSize(text, style)`: nothing to measure for `undefined` or
    /// `null`, else the cached size or the measurement.
    pub(super) fn get(&mut self, measure: &mut dyn Measure, text: &Value, style: &Props) -> Size {
        if text.is_nullish() {
            return Size {
                width: 0.0,
                height: 0.0,
            };
        }
        let text = text.to_js_string();
        let key = string_cache_key(&text, style);
        if let Some(size) = self.cache.get(&key) {
            return *size;
        }
        let size = measure.string_size(&text, style);
        self.cache.insert(key, size);
        size
    }
}

/// `BREAKING_SPACES`: `/[ \f\n\r\t\v  ]+/`.
fn is_breaking_space(c: char) -> bool {
    matches!(
        c,
        ' ' | '\u{c}' | '\n' | '\r' | '\t' | '\u{b}' | '\u{2028}' | '\u{2029}'
    )
}

/// `string.split(BREAKING_SPACES)`: runs of breaking spaces separate the
/// words, so leading and trailing runs give empty words.
fn split_words(text: &str) -> Vec<String> {
    let mut words = vec![String::new()];
    let mut in_space = false;
    for c in text.chars() {
        if is_breaking_space(c) {
            if !in_space {
                words.push(String::new());
                in_space = true;
            }
        } else {
            in_space = false;
            words.last_mut().expect("one word").push(c);
        }
    }
    words
}

/// `string.split('')`: UTF-16 code units, each its own string.
fn split_code_units(text: &str) -> Vec<String> {
    text.encode_utf16()
        .map(|unit| String::from_utf16_lossy(&[unit]))
        .collect()
}

/// A line of words (`{ words, width }`).
#[derive(Debug, Clone)]
struct Line {
    words: Vec<String>,
    width: Option<f64>,
}

/// `calculate(words, lineWidth, spaceWidth, scaleToFit)`.
fn calculate(
    words: &[(String, f64)],
    line_width: &Value,
    space_width: f64,
    scale_to_fit: bool,
) -> Vec<Line> {
    let mut result: Vec<Line> = Vec::new();
    for (word, width) in words {
        match result.last_mut() {
            Some(line)
                if line_width.is_nullish()
                    || scale_to_fit
                    || line.width.unwrap_or(f64::NAN) + width + space_width
                        < line_width.to_number() =>
            {
                line.words.push(word.clone());
                line.width = Some(line.width.unwrap_or(f64::NAN) + width + space_width);
            }
            _ => result.push(Line {
                words: vec![word.clone()],
                width: Some(*width),
            }),
        }
    }
    result
}

/// `getWordsByLines` without `maxLines` (not ported: reported as
/// unsupported by [`text`]).
fn words_by_lines(
    sizes: &mut StringSizes,
    measure: &mut dyn Measure,
    props: &Props,
    scale_to_fit: bool,
) -> Vec<Line> {
    let children = props.get("children");
    let width = props.get("width");
    if width.is_truthy() || scale_to_fit {
        // calculateWordWidths; `children.toString()` of a nullish child
        // makes no words.
        let break_all = props.get("breakAll").is_truthy();
        let style = Props::of(&props.get("style"));
        let words = if children.is_nullish() {
            Vec::new()
        } else if break_all {
            split_code_units(&children.to_js_string())
        } else {
            split_words(&children.to_js_string())
        };
        let with_widths: Vec<(String, f64)> = words
            .into_iter()
            .map(|word| {
                let width = sizes.get(measure, &word.clone().into(), &style).width;
                (word, width)
            })
            .collect();
        let space_width = if break_all {
            0.0
        } else {
            sizes.get(measure, &"\u{a0}".into(), &style).width
        };
        return calculate(&with_widths, &width, space_width, scale_to_fit);
    }
    // getWordsWithoutCalculate
    let words = if children.is_nullish() {
        Vec::new()
    } else {
        split_words(&children.to_js_string())
    };
    vec![Line { words, width: None }]
}

/// `textDefaultProps`.
fn text_default_props() -> Vec<(&'static str, Value)> {
    vec![
        ("angle", Value::Number(0.0)),
        ("breakAll", Value::Bool(false)),
        ("capHeight", "0.71em".into()),
        ("fill", "#808080".into()),
        ("lineHeight", "1em".into()),
        ("scaleToFit", Value::Bool(false)),
        ("textAnchor", "start".into()),
        ("verticalAnchor", "end".into()),
        ("x", Value::Number(0.0)),
        ("y", Value::Number(0.0)),
    ]
}

/// `isNumber`: a number other than `NaN`.
pub(super) fn is_number(value: &Value) -> bool {
    matches!(value, Value::Number(n) if !n.is_nan())
}

/// `isNumOrStr`.
fn is_num_or_str(value: &Value) -> bool {
    is_number(value) || matches!(value, Value::String(_))
}

/// `<Text {...props}>`: the `<text>` element, or nothing when its position
/// is not a finite number.
pub(super) fn text(
    sizes: &mut StringSizes,
    measure: &mut dyn Measure,
    props: &Props,
) -> Result<Option<Element>, ChartError> {
    let props = props.clone().with_defaults(&text_default_props());
    if !props.get("maxLines").is_nullish() {
        return Err(ChartError::Unsupported("text with maxLines"));
    }
    let scale_to_fit = props.get("scaleToFit").is_truthy();
    let lines = words_by_lines(sizes, measure, &props, scale_to_fit);
    let (prop_x, prop_y) = (props.get("x"), props.get("y"));
    if !is_num_or_str(&prop_x) || !is_num_or_str(&prop_y) || lines.is_empty() {
        return Ok(None);
    }
    let offset = |name| match props.get(name) {
        Value::Number(n) if !n.is_nan() => n,
        _ => 0.0,
    };
    let x = prop_x.to_number() + offset("dx");
    let y = prop_y.to_number() + offset("dy");
    if !x.is_finite() || !y.is_finite() {
        return Ok(None);
    }
    let cap_height = props.get("capHeight").to_js_string();
    let line_height = props.get("lineHeight").to_js_string();
    let count = lines.len() as f64;
    let start_dy = match props.get("verticalAnchor").to_js_string().as_str() {
        "start" => reduce_css_calc(&format!("calc({cap_height})")),
        "middle" => reduce_css_calc(&format!(
            "calc({} * -{line_height} + ({cap_height} / 2))",
            number_to_string((count - 1.0) / 2.0)
        )),
        _ => reduce_css_calc(&format!(
            "calc({} * -{line_height})",
            number_to_string(count - 1.0)
        )),
    };

    let mut text_props = props.filter(|key| {
        !matches!(
            key,
            "x" | "y"
                | "lineHeight"
                | "capHeight"
                | "fill"
                | "scaleToFit"
                | "textAnchor"
                | "verticalAnchor"
                | "dx"
                | "dy"
                | "angle"
                | "className"
                | "breakAll"
        )
    });
    let mut transforms = Vec::new();
    if scale_to_fit {
        let width = props.get("width");
        let scale = match (&width, lines[0].width) {
            (Value::Number(w), Some(line)) if !w.is_nan() && !line.is_nan() => w / line,
            _ => 1.0,
        };
        transforms.push(format!("scale({})", number_to_string(scale)));
    }
    let angle = props.get("angle");
    if angle.is_truthy() {
        transforms.push(format!(
            "rotate({}, {}, {})",
            angle.to_js_string(),
            number_to_string(x),
            number_to_string(y)
        ));
    }
    if !transforms.is_empty() {
        text_props.set("transform", transforms.join(" ").into());
    }
    let fill = match props.get("fill") {
        Value::String(fill) if fill.contains("url") => "#808080".to_owned(),
        Value::String(fill) => fill,
        _ => return Err(ChartError::Unsupported("a text fill other than a string")),
    };
    let mut attributes = svg_properties(&text_props);
    attributes.set("x", Value::Number(x));
    attributes.set("y", Value::Number(y));
    attributes.set(
        "className",
        clsx(&["recharts-text".into(), props.get("className")]).into(),
    );
    attributes.set("textAnchor", props.get("textAnchor"));
    attributes.set("fill", fill.into());
    let mut element = Element::svg("text").props(&attributes);
    let separator = if props.get("breakAll").is_truthy() {
        ""
    } else {
        " "
    };
    for (index, line) in lines.iter().enumerate() {
        let mut tspan = Element::svg("tspan");
        tspan.prop("x", &Value::Number(x));
        tspan.prop(
            "dy",
            &if index == 0 {
                start_dy.clone().into()
            } else {
                props.get("lineHeight")
            },
        );
        tspan.prop("children", &line.words.join(separator).into());
        element.children.push(Node::Element(tspan));
    }
    Ok(Some(element))
}

// ------------------------------------------------------------ ReduceCSSCalc

/// `DecimalCSS`: a number with a CSS unit; fixed length units are
/// converted to `px`, unknown units make it `NaN`.
#[derive(Debug, Clone)]
struct DecimalCss {
    num: f64,
    unit: String,
}

fn is_css_length_unit(unit: &str) -> bool {
    matches!(
        unit,
        "px" | "cm"
            | "vh"
            | "vw"
            | "em"
            | "rem"
            | "%"
            | "mm"
            | "in"
            | "pt"
            | "pc"
            | "ex"
            | "ch"
            | "vmin"
            | "vmax"
            | "Q"
    )
}

fn conversion_rate(unit: &str) -> Option<f64> {
    Some(match unit {
        "cm" => 96.0 / 2.54,
        "mm" => 96.0 / 25.4,
        "pt" => 96.0 / 72.0,
        "pc" => 96.0 / 6.0,
        "in" => 96.0,
        "Q" => 96.0 / (2.54 * 40.0),
        "px" => 1.0,
        _ => return None,
    })
}

impl DecimalCss {
    fn new(num: f64, unit: &str) -> DecimalCss {
        let mut value = DecimalCss {
            num,
            unit: unit.to_owned(),
        };
        if num.is_nan() {
            value.unit = String::new();
        }
        if !unit.is_empty() && !is_css_length_unit(unit) {
            value.num = f64::NAN;
            value.unit = String::new();
        }
        if let Some(rate) = conversion_rate(unit) {
            value.num = num * rate;
            value.unit = "px".into();
        }
        value
    }

    fn nan() -> DecimalCss {
        DecimalCss::new(f64::NAN, "")
    }

    /// `DecimalCSS.parse`: the first `(-?\d+(?:\.\d+)?)([a-zA-Z%]+)?`.
    fn parse(text: &str) -> DecimalCss {
        let chars: Vec<char> = text.chars().collect();
        for start in 0..chars.len() {
            if let Some((end, unit_start)) = match_number(&chars, start) {
                let num: String = chars[start..unit_start].iter().collect();
                let unit: String = chars[unit_start..end].iter().collect();
                return DecimalCss::new(parse_float(&num), &unit);
            }
        }
        DecimalCss::nan()
    }

    fn combine(&self, other: &DecimalCss, op: char) -> DecimalCss {
        match op {
            '+' | '-' => {
                if self.unit != other.unit {
                    return DecimalCss::new(f64::NAN, "");
                }
                let num = if op == '+' {
                    self.num + other.num
                } else {
                    self.num - other.num
                };
                DecimalCss::new(num, &self.unit)
            }
            _ => {
                if !self.unit.is_empty() && !other.unit.is_empty() && self.unit != other.unit {
                    return DecimalCss::new(f64::NAN, "");
                }
                let num = if op == '*' {
                    self.num * other.num
                } else {
                    self.num / other.num
                };
                let unit = if self.unit.is_empty() {
                    &other.unit
                } else {
                    &self.unit
                };
                DecimalCss::new(num, unit)
            }
        }
    }

    fn to_css_string(&self) -> String {
        format!("{}{}", number_to_string(self.num), self.unit)
    }
}

/// Matches `-?\d+(?:\.\d+)?[a-zA-Z%]*` at `start`: the end of the match and
/// where its unit starts.
fn match_number(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let mut i = start;
    if chars.get(i) == Some(&'-') {
        i += 1;
    }
    let digits = i;
    while chars.get(i).is_some_and(char::is_ascii_digit) {
        i += 1;
    }
    if i == digits {
        return None;
    }
    if chars.get(i) == Some(&'.') && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
        i += 1;
        while chars.get(i).is_some_and(char::is_ascii_digit) {
            i += 1;
        }
    }
    let unit = i;
    while chars
        .get(i)
        .is_some_and(|c| c.is_ascii_alphabetic() || *c == '%')
    {
        i += 1;
    }
    Some((i, unit))
}

/// The first match of `(operand)([ops])(operand)` (`MULTIPLY_OR_DIVIDE_REGEX`
/// or `ADD_OR_SUBTRACT_REGEX`): start, operator position, end.
fn match_operation(chars: &[char], ops: &[char]) -> Option<(usize, usize, usize)> {
    for start in 0..chars.len() {
        let Some((left_end, _)) = match_number(chars, start) else {
            continue;
        };
        if !chars.get(left_end).is_some_and(|c| ops.contains(c)) {
            continue;
        }
        if let Some((end, _)) = match_number(chars, left_end + 1) {
            return Some((start, left_end, end));
        }
    }
    None
}

/// `/.-\d+(?:\.\d+)?/.test(expr)`: some character before a `-` and a digit.
fn has_subtraction(chars: &[char]) -> bool {
    (1..chars.len()).any(|i| chars[i] == '-' && chars.get(i + 1).is_some_and(char::is_ascii_digit))
}

/// `calculateArithmetic`.
fn calculate_arithmetic(expr: &str) -> String {
    if expr.contains("NaN") {
        return "NaN".into();
    }
    let mut chars: Vec<char> = expr.chars().collect();
    for ops in [&['*', '/'][..], &['+', '-'][..]] {
        loop {
            let pending = if ops[0] == '*' {
                chars.contains(&'*') || chars.contains(&'/')
            } else {
                chars.contains(&'+') || has_subtraction(&chars)
            };
            if !pending {
                break;
            }
            let Some((start, op, end)) = match_operation(&chars, ops) else {
                // The regex finds nothing: both operands parse as NaN.
                return "NaN".into();
            };
            let left: String = chars[start..op].iter().collect();
            let right: String = chars[op + 1..end].iter().collect();
            let result = DecimalCss::parse(&left).combine(&DecimalCss::parse(&right), chars[op]);
            if result.num.is_nan() {
                return "NaN".into();
            }
            let replacement: Vec<char> = result.to_css_string().chars().collect();
            chars.splice(start..end, replacement);
        }
    }
    chars.into_iter().collect()
}

/// `reduceCSSCalc("calc(…)")`: the expression evaluated, or `''` for `NaN`.
pub(super) fn reduce_css_calc(expression: &str) -> String {
    let inner = &expression[5..expression.len() - 1];
    let mut expr: String = inner.chars().filter(|c| !c.is_whitespace()).collect();
    // calculateParentheses: the first match of /\(([^()]*)\)/, a `(` whose
    // next parenthesis is a `)`, until there is none.
    while let Some((open, close)) = expr.char_indices().find_map(|(open, c)| {
        if c != '(' {
            return None;
        }
        let (offset, next) = expr[open + 1..]
            .char_indices()
            .find(|(_, c)| matches!(c, '(' | ')'))?;
        (next == ')').then_some((open, open + 1 + offset))
    }) {
        let value = calculate_arithmetic(&expr[open + 1..close]);
        expr.replace_range(open..=close, &value);
    }
    let result = calculate_arithmetic(&expr);
    if result == "NaN" {
        String::new()
    } else {
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reduce_css_calc_matches_recharts() {
        // recharts@3.8.0's reduceCSSCalc run under Node 24.22.3.
        for (expression, reduced) in [
            ("calc(0.71em)", "0.71em"),
            ("calc(0 * -1em + (0.71em / 2))", "0.355em"),
            ("calc(0 * -1em)", "0em"),
            ("calc(1 * -1em)", "-1em"),
            ("calc(0.5 * -1em + (0.71em / 2))", "-0.14500000000000002em"),
            ("calc(1in + 4px)", "100px"),
            ("calc(1em + 4px)", ""),
            ("calc(1.5 * -1em + (0.71em / 2))", "-1.145em"),
            ("calc(2 * -12px)", "-24px"),
            ("calc(0.5 * -1.2em + (10px / 2))", ""),
        ] {
            assert_eq!(reduce_css_calc(expression), reduced, "{expression}");
        }
    }

    #[test]
    fn words_split_as_javascript_does() {
        assert_eq!(split_words("a  b"), ["a", "b"]);
        assert_eq!(split_words(" a "), ["", "a", ""]);
        assert_eq!(split_words(""), [""]);
    }
}
