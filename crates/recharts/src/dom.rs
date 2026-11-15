//! The DOM a React component tree renders, as data: elements with their
//! attributes, inline style declarations and children. A chart rendered
//! here is the tree `react-dom` 19.2 builds for the same chart in a page;
//! drawing it is Div's job (inline SVG, docs/framework-gaps.md V1).
//!
//! [`Element::prop`] applies one React prop the way `setProp` and
//! `setValueForStyle` of `react-dom-client.production.js` (react-dom
//! 19.3.1) do for a built-in (not custom) element: which attribute a prop
//! names, when it is left out, and the string its value becomes. Event
//! handlers, refs and `dangerouslySetInnerHTML` set no attribute; the value
//! model (`js::Value`) has no functions, so a prop holding one cannot
//! occur here.
//!
//! Inline style is kept as React writes it, one `element.style[name] =
//! value` (or `setProperty` for a custom property) per declaration, with
//! the IDL attribute name (`backgroundColor`). Turning those into
//! declarations and the `style` attribute's text is the CSS object
//! model's work, so Div's.

use js::{number_to_string, Value};

/// An element's namespace, as the HTML parser or React's `createElement`
/// gives it: SVG inside `<svg>`, HTML otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Namespace {
    Html,
    Svg,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Element(Element),
    Text(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// The local name (`clipPath`, `g`, `div`).
    pub tag: String,
    pub namespace: Namespace,
    /// Attributes in the order React sets them, by qualified name
    /// (`clip-path`, `xlink:href`).
    pub attributes: Vec<(String, String)>,
    /// Inline style: `element.style[name] = value` in the order React
    /// assigns them (custom properties by their `--` name).
    pub style: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn new(namespace: Namespace, tag: &str) -> Element {
        Element {
            tag: tag.to_owned(),
            namespace,
            attributes: Vec::new(),
            style: Vec::new(),
            children: Vec::new(),
        }
    }

    pub fn svg(tag: &str) -> Element {
        Element::new(Namespace::Svg, tag)
    }

    /// The attribute `name`, if set.
    pub fn attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }

    /// Sets each prop in turn, as React does on mount.
    pub fn props(mut self, props: &Props) -> Element {
        for (key, value) in props.entries() {
            self.prop(key, value);
        }
        self
    }

    /// Sets one prop on a newly created element (`setProp` with no previous
    /// value).
    pub fn prop(&mut self, key: &str, value: &Value) {
        match key {
            "children" => {
                // A string or number child is the element's text.
                let text = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => number_to_string(*n),
                    _ => return,
                };
                // `textContent = ""` leaves no text node.
                self.children.retain(|c| !matches!(c, Node::Text(_)));
                if !text.is_empty() {
                    self.children.push(Node::Text(text));
                }
            }
            "className" => self.known_attribute("class", value),
            "tabIndex" => self.known_attribute("tabindex", value),
            "dir" | "role" | "viewBox" | "width" | "height" => self.known_attribute(key, value),
            "style" => self.styles(value),
            "src" | "href" => self.url_attribute(key, value),
            "data" => self.known_attribute("data", value),
            // Form actions render a script URL for functions, which cannot
            // occur here; other values are URLs.
            "action" | "formAction" => match value {
                Value::Null | Value::Undefined | Value::Bool(_) => self.remove(key),
                _ => self.set(key, sanitize_url(&value.to_js_string())),
            },
            "suppressContentEditableWarning"
            | "suppressHydrationWarning"
            | "defaultValue"
            | "defaultChecked"
            | "innerHTML"
            | "ref"
            | "autoFocus"
            | "innerText"
            | "textContent"
            | "dangerouslySetInnerHTML"
            | "multiple"
            | "muted" => {}
            "xlinkHref" => match value {
                Value::Null | Value::Undefined | Value::Bool(_) => self.remove("xlink:href"),
                _ => self.set("xlink:href", sanitize_url(&value.to_js_string())),
            },
            "contentEditable"
            | "spellCheck"
            | "draggable"
            | "value"
            | "autoReverse"
            | "externalResourcesRequired"
            | "focusable"
            | "preserveAlpha" => match value {
                Value::Null | Value::Undefined => self.remove(key),
                _ => self.set(key, value.to_js_string()),
            },
            "inert"
            | "allowFullScreen"
            | "async"
            | "autoPlay"
            | "controls"
            | "credentialless"
            | "default"
            | "defer"
            | "disabled"
            | "disablePictureInPicture"
            | "disableRemotePlayback"
            | "formNoValidate"
            | "hidden"
            | "loop"
            | "noModule"
            | "noValidate"
            | "open"
            | "playsInline"
            | "readOnly"
            | "required"
            | "reversed"
            | "scoped"
            | "seamless"
            | "itemScope" => {
                if value.is_truthy() {
                    self.set(key, String::new());
                } else {
                    self.remove(key);
                }
            }
            "capture" | "download" => match value {
                Value::Bool(true) => self.set(key, String::new()),
                Value::Bool(false) | Value::Null | Value::Undefined => self.remove(key),
                _ => self.set(key, value.to_js_string()),
            },
            "cols" | "rows" | "size" | "span" => {
                if !value.is_nullish() && !value.to_number().is_nan() && value.to_number() >= 1.0 {
                    self.set(key, value.to_js_string());
                } else {
                    self.remove(key);
                }
            }
            "rowSpan" | "start" => {
                if value.is_nullish() || value.to_number().is_nan() {
                    self.remove(key);
                } else {
                    self.set(key, value.to_js_string());
                }
            }
            "popover" | "is" => self.attribute_value(key, value),
            "xlinkActuate" => self.namespaced_attribute("xlink:actuate", value),
            "xlinkArcrole" => self.namespaced_attribute("xlink:arcrole", value),
            "xlinkRole" => self.namespaced_attribute("xlink:role", value),
            "xlinkShow" => self.namespaced_attribute("xlink:show", value),
            "xlinkTitle" => self.namespaced_attribute("xlink:title", value),
            "xlinkType" => self.namespaced_attribute("xlink:type", value),
            "xmlBase" => self.namespaced_attribute("xml:base", value),
            "xmlLang" => self.namespaced_attribute("xml:lang", value),
            "xmlSpace" => self.namespaced_attribute("xml:space", value),
            _ => {
                // Event handlers: `on` and at least one more character.
                let mut chars = key.chars();
                let event = key.chars().count() > 2
                    && matches!(chars.next(), Some('o' | 'O'))
                    && matches!(chars.next(), Some('n' | 'N'));
                if !event {
                    self.attribute_value(alias(key).unwrap_or(key), value);
                }
            }
        }
    }

    /// `setValueForAttribute`.
    fn attribute_value(&mut self, name: &str, value: &Value) {
        if !is_attribute_name_safe(name) {
            return;
        }
        match value {
            Value::Null | Value::Undefined => self.remove(name),
            Value::Bool(_) => {
                let prefix: String = name.to_lowercase().chars().take(5).collect();
                if prefix == "data-" || prefix == "aria-" {
                    self.set(name, value.to_js_string());
                } else {
                    self.remove(name);
                }
            }
            _ => self.set(name, value.to_js_string()),
        }
    }

    /// `setValueForKnownAttribute` (and `setValueForNamespacedAttribute`,
    /// which treats values the same).
    fn known_attribute(&mut self, name: &str, value: &Value) {
        match value {
            Value::Null | Value::Undefined | Value::Bool(_) => self.remove(name),
            _ => self.set(name, value.to_js_string()),
        }
    }

    fn namespaced_attribute(&mut self, name: &str, value: &Value) {
        self.known_attribute(name, value);
    }

    fn url_attribute(&mut self, name: &str, value: &Value) {
        match value {
            Value::String(s) if s.is_empty() => self.remove(name),
            Value::Null | Value::Undefined | Value::Bool(_) => self.remove(name),
            _ => self.set(name, sanitize_url(&value.to_js_string())),
        }
    }

    /// `setValueForStyles` on a new element: `setValueForStyle` for each own
    /// property of the style object.
    fn styles(&mut self, value: &Value) {
        let Value::Object(entries) = value else {
            // null and undefined set nothing; other primitives are React
            // error 62, which the chart components never pass.
            return;
        };
        for (name, value) in entries {
            self.style_value(name, value);
        }
    }

    /// `setValueForStyle`: numbers get `px` unless the property is unitless
    /// or the number is 0; `null`, booleans and `""` clear the property.
    fn style_value(&mut self, name: &str, value: &Value) {
        let cleared = matches!(value, Value::Null | Value::Undefined | Value::Bool(_))
            || matches!(value, Value::String(s) if s.is_empty());
        self.style.retain(|(n, _)| n != name);
        if cleared {
            return;
        }
        let text = if name.starts_with("--") {
            value.to_js_string()
        } else {
            match value {
                Value::Number(n) if *n != 0.0 && !UNITLESS_NUMBERS.contains(&name) => {
                    format!("{}px", number_to_string(*n))
                }
                _ => js_trim(&value.to_js_string()).to_owned(),
            }
        };
        self.style.push((name.to_owned(), text));
    }

    fn set(&mut self, name: &str, value: String) {
        match self.attributes.iter_mut().find(|(n, _)| n == name) {
            Some(entry) => entry.1 = value,
            None => self.attributes.push((name.to_owned(), value)),
        }
    }

    fn remove(&mut self, name: &str) {
        self.attributes.retain(|(n, _)| n != name);
    }
}

/// A props object: own properties in order. Setting a key that exists keeps
/// its place, as assignment to a JavaScript object does.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Props(Vec<(String, Value)>);

impl Props {
    pub fn new() -> Props {
        Props(Vec::new())
    }

    /// The own properties of an object value; nothing for anything else
    /// (`{...value}` of a primitive or `null`).
    pub fn of(value: &Value) -> Props {
        let mut props = Props::new();
        props.spread(value);
        props
    }

    pub fn entries(&self) -> impl Iterator<Item = (&str, &Value)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v))
    }

    /// `props[key]`, `undefined` when absent.
    pub fn get(&self, key: &str) -> Value {
        self.0
            .iter()
            .find(|(k, _)| k == key)
            .map_or(Value::Undefined, |(_, v)| v.clone())
    }

    pub fn has(&self, key: &str) -> bool {
        self.0.iter().any(|(k, _)| k == key)
    }

    pub fn set(&mut self, key: &str, value: Value) {
        match self.0.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = value,
            None => self.0.push((key.to_owned(), value)),
        }
    }

    /// `delete props[key]`.
    pub fn remove(&mut self, key: &str) {
        self.0.retain(|(k, _)| k != key);
    }

    /// `{...this, ...value}`: the own enumerable properties of an object;
    /// an array spreads its indices.
    pub fn spread(&mut self, value: &Value) {
        match value {
            Value::Object(entries) => {
                for (k, v) in entries {
                    self.set(k, v.clone());
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    self.set(&i.to_string(), v.clone());
                }
            }
            _ => {}
        }
    }

    pub fn spread_props(&mut self, other: &Props) {
        for (k, v) in &other.0 {
            self.set(k, v.clone());
        }
    }

    /// The props as an object value.
    pub fn to_value(&self) -> Value {
        Value::Object(self.0.clone())
    }

    /// `resolveDefaultProps(this, defaults)`: every default whose prop is
    /// `undefined` (or absent), in the defaults' order.
    pub fn with_defaults(mut self, defaults: &[(&str, Value)]) -> Props {
        for (key, value) in defaults {
            if matches!(self.get(key), Value::Undefined) && !matches!(value, Value::Undefined) {
                self.set(key, value.clone());
            }
        }
        self
    }

    /// Keeps the props `keep` accepts, in order.
    pub fn filter(&self, keep: impl Fn(&str) -> bool) -> Props {
        Props(self.0.iter().filter(|(k, _)| keep(k)).cloned().collect())
    }
}

/// `clsx(...args)` of the clsx package: the truthy strings and numbers,
/// array items and object keys with truthy values, joined by spaces.
pub fn clsx(args: &[Value]) -> String {
    fn value(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => number_to_string(*n),
            Value::Array(items) => {
                let mut out = String::new();
                for item in items {
                    if item.is_truthy() {
                        let part = value(item);
                        if !part.is_empty() {
                            if !out.is_empty() {
                                out.push(' ');
                            }
                            out.push_str(&part);
                        }
                    }
                }
                out
            }
            Value::Object(entries) => {
                let mut out = String::new();
                for (k, v) in entries {
                    if v.is_truthy() {
                        if !out.is_empty() {
                            out.push(' ');
                        }
                        out.push_str(k);
                    }
                }
                out
            }
            _ => String::new(),
        }
    }
    let mut out = String::new();
    for arg in args {
        if arg.is_truthy() {
            let part = value(arg);
            if !part.is_empty() {
                if !out.is_empty() {
                    out.push(' ');
                }
                out.push_str(&part);
            }
        }
    }
    out
}

/// `String.prototype.trim`: strips WhiteSpace and LineTerminator code
/// points (ECMA-262 12.2, 12.3), which are not Rust's `char::is_whitespace`.
fn js_trim(s: &str) -> &str {
    let space = |c: char| {
        matches!(
            c,
            '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
                ..='\u{200a}'
                    | '\u{2028}'
                    | '\u{2029}'
                    | '\u{202f}'
                    | '\u{205f}'
                    | '\u{3000}'
                    | '\u{feff}'
        )
    };
    s.trim_matches(space)
}

/// `sanitizeURL`: a `javascript:` URL (ignoring leading controls and
/// spaces, and tabs and newlines inside the scheme) becomes a script that
/// throws.
fn sanitize_url(url: &str) -> String {
    let mut chars = url.chars().peekable();
    while chars.next_if(|&c| c <= '\u{1f}' || c == ' ').is_some() {}
    let mut javascript = true;
    for (i, expected) in "javascript".chars().enumerate() {
        if i > 0 {
            while chars
                .next_if(|&c| matches!(c, '\r' | '\n' | '\t'))
                .is_some()
            {}
        }
        if !chars
            .next()
            .is_some_and(|c| c.eq_ignore_ascii_case(&expected))
        {
            javascript = false;
            break;
        }
    }
    if javascript {
        while chars
            .next_if(|&c| matches!(c, '\r' | '\n' | '\t'))
            .is_some()
        {}
        javascript = chars.next() == Some(':');
    }
    if javascript {
        "javascript:throw new Error('React has blocked a javascript: URL as a security precaution.')"
            .to_owned()
    } else {
        url.to_owned()
    }
}

/// `VALID_ATTRIBUTE_NAME_REGEX`.
fn is_attribute_name_safe(name: &str) -> bool {
    let start = |c: char| {
        matches!(c,
            ':' | 'A'..='Z' | '_' | 'a'..='z'
            | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}' | '\u{f8}'..='\u{2ff}'
            | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}' | '\u{200c}'..='\u{200d}'
            | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}'
            | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}')
    };
    let rest = |c: char| {
        start(c)
            || matches!(c, '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}')
    };
    let mut chars = name.chars();
    chars.next().is_some_and(start) && chars.all(rest)
}

/// The `aliases` map: React prop names whose attribute is spelled
/// differently.
fn alias(key: &str) -> Option<&'static str> {
    ALIASES
        .iter()
        .find(|(prop, _)| *prop == key)
        .map(|(_, attribute)| *attribute)
}

const ALIASES: &[(&str, &str)] = &[
    ("acceptCharset", "accept-charset"),
    ("htmlFor", "for"),
    ("httpEquiv", "http-equiv"),
    ("crossOrigin", "crossorigin"),
    ("accentHeight", "accent-height"),
    ("alignmentBaseline", "alignment-baseline"),
    ("arabicForm", "arabic-form"),
    ("baselineShift", "baseline-shift"),
    ("capHeight", "cap-height"),
    ("clipPath", "clip-path"),
    ("clipRule", "clip-rule"),
    ("colorInterpolation", "color-interpolation"),
    ("colorInterpolationFilters", "color-interpolation-filters"),
    ("colorProfile", "color-profile"),
    ("colorRendering", "color-rendering"),
    ("dominantBaseline", "dominant-baseline"),
    ("enableBackground", "enable-background"),
    ("fillOpacity", "fill-opacity"),
    ("fillRule", "fill-rule"),
    ("floodColor", "flood-color"),
    ("floodOpacity", "flood-opacity"),
    ("fontFamily", "font-family"),
    ("fontSize", "font-size"),
    ("fontSizeAdjust", "font-size-adjust"),
    ("fontStretch", "font-stretch"),
    ("fontStyle", "font-style"),
    ("fontVariant", "font-variant"),
    ("fontWeight", "font-weight"),
    ("glyphName", "glyph-name"),
    ("glyphOrientationHorizontal", "glyph-orientation-horizontal"),
    ("glyphOrientationVertical", "glyph-orientation-vertical"),
    ("horizAdvX", "horiz-adv-x"),
    ("horizOriginX", "horiz-origin-x"),
    ("imageRendering", "image-rendering"),
    ("letterSpacing", "letter-spacing"),
    ("lightingColor", "lighting-color"),
    ("markerEnd", "marker-end"),
    ("markerMid", "marker-mid"),
    ("markerStart", "marker-start"),
    ("maskType", "mask-type"),
    ("overlinePosition", "overline-position"),
    ("overlineThickness", "overline-thickness"),
    ("paintOrder", "paint-order"),
    ("panose-1", "panose-1"),
    ("pointerEvents", "pointer-events"),
    ("renderingIntent", "rendering-intent"),
    ("shapeRendering", "shape-rendering"),
    ("stopColor", "stop-color"),
    ("stopOpacity", "stop-opacity"),
    ("strikethroughPosition", "strikethrough-position"),
    ("strikethroughThickness", "strikethrough-thickness"),
    ("strokeDasharray", "stroke-dasharray"),
    ("strokeDashoffset", "stroke-dashoffset"),
    ("strokeLinecap", "stroke-linecap"),
    ("strokeLinejoin", "stroke-linejoin"),
    ("strokeMiterlimit", "stroke-miterlimit"),
    ("strokeOpacity", "stroke-opacity"),
    ("strokeWidth", "stroke-width"),
    ("textAnchor", "text-anchor"),
    ("textDecoration", "text-decoration"),
    ("textRendering", "text-rendering"),
    ("transformOrigin", "transform-origin"),
    ("underlinePosition", "underline-position"),
    ("underlineThickness", "underline-thickness"),
    ("unicodeBidi", "unicode-bidi"),
    ("unicodeRange", "unicode-range"),
    ("unitsPerEm", "units-per-em"),
    ("vAlphabetic", "v-alphabetic"),
    ("vHanging", "v-hanging"),
    ("vIdeographic", "v-ideographic"),
    ("vMathematical", "v-mathematical"),
    ("vectorEffect", "vector-effect"),
    ("vertAdvY", "vert-adv-y"),
    ("vertOriginX", "vert-origin-x"),
    ("vertOriginY", "vert-origin-y"),
    ("wordSpacing", "word-spacing"),
    ("writingMode", "writing-mode"),
    ("xmlnsXlink", "xmlns:xlink"),
    ("xHeight", "x-height"),
];

/// `unitlessNumbers`: style properties whose numbers get no `px`.
const UNITLESS_NUMBERS: &[&str] = &[
    "animationIterationCount",
    "aspectRatio",
    "borderImageOutset",
    "borderImageSlice",
    "borderImageWidth",
    "boxFlex",
    "boxFlexGroup",
    "boxOrdinalGroup",
    "columnCount",
    "columns",
    "flex",
    "flexGrow",
    "flexPositive",
    "flexShrink",
    "flexNegative",
    "flexOrder",
    "gridArea",
    "gridRow",
    "gridRowEnd",
    "gridRowSpan",
    "gridRowStart",
    "gridColumn",
    "gridColumnEnd",
    "gridColumnSpan",
    "gridColumnStart",
    "fontWeight",
    "lineClamp",
    "lineHeight",
    "opacity",
    "order",
    "orphans",
    "scale",
    "tabSize",
    "widows",
    "zIndex",
    "zoom",
    "fillOpacity",
    "floodOpacity",
    "stopOpacity",
    "strokeDasharray",
    "strokeDashoffset",
    "strokeMiterlimit",
    "strokeOpacity",
    "strokeWidth",
    "MozAnimationIterationCount",
    "MozBoxFlex",
    "MozBoxFlexGroup",
    "MozLineClamp",
    "msAnimationIterationCount",
    "msFlex",
    "msZoom",
    "msFlexGrow",
    "msFlexNegative",
    "msFlexOrder",
    "msFlexPositive",
    "msFlexShrink",
    "msGridColumn",
    "msGridColumnSpan",
    "msGridRow",
    "msGridRowSpan",
    "WebkitAnimationIterationCount",
    "WebkitBoxFlex",
    "WebKitBoxFlexGroup",
    "WebkitBoxOrdinalGroup",
    "WebkitColumnCount",
    "WebkitColumns",
    "WebkitFlex",
    "WebkitFlexGrow",
    "WebkitFlexPositive",
    "WebkitFlexShrink",
    "WebkitLineClamp",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn props_become_attributes_as_react_sets_them() {
        let mut path = Element::svg("path");
        path.prop("className", &"a".into());
        path.prop("strokeWidth", &Value::Number(2.5));
        path.prop("name", &Value::Undefined);
        path.prop("radius", &Value::Bool(true));
        path.prop("data-x", &Value::Bool(true));
        path.prop("onClick", &"f".into());
        path.prop(
            "r",
            &Value::Array(vec![10.0.into(), Value::Null, 3.0.into()]),
        );
        path.prop("width", &Value::Bool(false));
        assert_eq!(
            path.attributes,
            vec![
                ("class".to_owned(), "a".to_owned()),
                ("stroke-width".to_owned(), "2.5".to_owned()),
                ("data-x".to_owned(), "true".to_owned()),
                ("r".to_owned(), "10,,3".to_owned()),
            ]
        );
    }

    #[test]
    fn styles_get_px_unless_unitless() {
        let mut div = Element::new(Namespace::Html, "div");
        div.prop(
            "style",
            &Value::Object(vec![
                ("width".into(), 500.5.into()),
                ("opacity".into(), 0.5.into()),
                ("marginTop".into(), 0.0.into()),
                ("cursor".into(), " default ".into()),
                ("height".into(), "".into()),
                ("--x".into(), 3.0.into()),
            ]),
        );
        assert_eq!(
            div.style,
            vec![
                ("width".to_owned(), "500.5px".to_owned()),
                ("opacity".to_owned(), "0.5".to_owned()),
                ("marginTop".to_owned(), "0".to_owned()),
                ("cursor".to_owned(), "default".to_owned()),
                ("--x".to_owned(), "3".to_owned()),
            ]
        );
    }

    #[test]
    fn javascript_urls_are_blocked() {
        assert!(sanitize_url(" \u{1}Java\tScript:alert(1)").starts_with("javascript:throw"));
        assert_eq!(sanitize_url("javascripts:x"), "javascripts:x");
        assert_eq!(sanitize_url("#clip"), "#clip");
    }

    #[test]
    fn clsx_joins_truthy_parts() {
        assert_eq!(
            clsx(&[
                "recharts-bar".into(),
                Value::Number(0.0),
                Value::Array(vec!["a".into(), Value::Null, 7.0.into()]),
                Value::Object(vec![
                    ("b".into(), Value::Bool(true)),
                    ("c".into(), Value::Null)
                ]),
            ]),
            "recharts-bar a 7 b"
        );
    }
}
