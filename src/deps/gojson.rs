//! A JSON writer that emits exactly the bytes Go's `encoding/json` emits.
//!
//! Two of Go's choices are visible in direnv's output and are not what a
//! typical Rust JSON writer does:
//!
//! * `<`, `>` and `&` are escaped as `<`, `>` and `&`, and
//!   U+2028 / U+2029 as ` ` / ` `, because `encoding/json` escapes
//!   HTML by default. This shows up in `direnv dump json`, `direnv show_dump`,
//!   `direnv status --json` and inside every gzenv payload.
//! * A map is marshalled with its keys **sorted**, while a struct keeps its
//!   field declaration order. `JsonValue::Object` therefore carries an ordered
//!   pair list and the map case sorts on the way in.
//!
//! Parsing is delegated to `serde_json`, whose `Value` maps objects to a
//! `BTreeMap` — already the sorted shape Go produces for `map[string]any`.

use std::collections::BTreeMap;
use std::collections::HashMap;

/// A JSON document in the shape Go would marshal it.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// Go unmarshals every JSON number into `float64`, and marshals it back
    /// with `strconv.AppendFloat(_, 'f', -1, 64)` unless the magnitude forces
    /// exponent form.
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    /// Key/value pairs in emission order.
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    /// A `map[string]string`: sorted keys.
    pub fn from_string_map(map: &HashMap<String, String>) -> JsonValue {
        let sorted: BTreeMap<&String, &String> = map.iter().collect();
        JsonValue::Object(
            sorted
                .into_iter()
                .map(|(k, v)| (k.clone(), JsonValue::String(v.clone())))
                .collect(),
        )
    }

    /// A `map[string]*string`: sorted keys, `nil` marshals as `null`.
    pub fn from_optional_string_map(map: &HashMap<String, Option<String>>) -> JsonValue {
        let sorted: BTreeMap<&String, &Option<String>> = map.iter().collect();
        JsonValue::Object(
            sorted
                .into_iter()
                .map(|(k, v)| {
                    let value = match v {
                        Some(text) => JsonValue::String(text.clone()),
                        None => JsonValue::Null,
                    };
                    (k.clone(), value)
                })
                .collect(),
        )
    }

    /// Sort an object's keys, as Go does for maps.
    pub fn sorted_object(mut pairs: Vec<(String, JsonValue)>) -> JsonValue {
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        JsonValue::Object(pairs)
    }
}

/// Go's `json.Marshal`.
pub fn marshal(value: &JsonValue) -> String {
    let mut out = String::new();
    write_value(&mut out, value, None, 0);
    out
}

/// Go's `json.MarshalIndent(v, prefix, indent)`.
pub fn marshal_indent(value: &JsonValue, indent: &str) -> String {
    let mut out = String::new();
    write_value(&mut out, value, Some(indent), 0);
    out
}

/// Go's `json.Encoder.Encode` — compact, plus a trailing newline.
pub fn encode(value: &JsonValue) -> String {
    let mut out = marshal(value);
    out.push('\n');
    out
}

fn write_value(out: &mut String, value: &JsonValue, indent: Option<&str>, depth: usize) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(n) => out.push_str(&format_number(*n)),
        JsonValue::String(s) => write_string(out, s),
        JsonValue::Array(items) => {
            if items.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_newline_indent(out, indent, depth + 1);
                write_value(out, item, indent, depth + 1);
            }
            write_newline_indent(out, indent, depth);
            out.push(']');
        }
        JsonValue::Object(pairs) => {
            if pairs.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (key, item)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_newline_indent(out, indent, depth + 1);
                write_string(out, key);
                out.push(':');
                if indent.is_some() {
                    out.push(' ');
                }
                write_value(out, item, indent, depth + 1);
            }
            write_newline_indent(out, indent, depth);
            out.push('}');
        }
    }
}

fn write_newline_indent(out: &mut String, indent: Option<&str>, depth: usize) {
    if let Some(unit) = indent {
        out.push('\n');
        for _ in 0..depth {
            out.push_str(unit);
        }
    }
}

/// Go's `encodeState.string` with `escapeHTML` left on.
fn write_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Go's `encodeState.floatEncoder` for a `float64`.
fn format_number(value: f64) -> String {
    if value == value.trunc() && value.abs() < 1e21 && value != 0.0 {
        return format!("{}", value as i64);
    }
    if value == 0.0 {
        return "0".to_string();
    }
    let abs = value.abs();
    if abs < 1e-6 || abs >= 1e21 {
        // Go trims the exponent's leading zero: 1e+09 rather than 1e+009.
        let text = format!("{value:e}");
        return match text.split_once('e') {
            Some((mantissa, exp)) => {
                if let Some(stripped) = exp.strip_prefix('-') {
                    format!("{mantissa}e-{stripped:0>2}")
                } else {
                    format!("{mantissa}e+{exp:0>2}")
                }
            }
            None => text,
        };
    }
    format!("{value}")
}

/// Parse JSON text into the writer's value shape.
///
/// Errors keep `serde_json`'s wording; Go's parser messages differ, and the
/// only place direnv shows one is the `unmarshal() json parsing: %w` wrapper.
pub fn parse(text: &str) -> Result<JsonValue, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    Ok(from_serde(&value))
}

fn from_serde(value: &serde_json::Value) -> JsonValue {
    match value {
        serde_json::Value::Null => JsonValue::Null,
        serde_json::Value::Bool(b) => JsonValue::Bool(*b),
        serde_json::Value::Number(n) => JsonValue::Number(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => JsonValue::String(s.clone()),
        serde_json::Value::Array(items) => JsonValue::Array(items.iter().map(from_serde).collect()),
        // serde_json's Map is a BTreeMap here, so this is already key-sorted,
        // which is what Go produces for a map.
        serde_json::Value::Object(map) => JsonValue::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), from_serde(v)))
                .collect(),
        ),
    }
}

/// The name `encoding/json` uses for a value in a type error.
pub fn kind(value: &JsonValue) -> &'static str {
    match value {
        JsonValue::Null => "null",
        JsonValue::Bool(_) => "bool",
        JsonValue::Number(_) => "number",
        JsonValue::String(_) => "string",
        JsonValue::Array(_) => "array",
        JsonValue::Object(_) => "object",
    }
}

/// `json: cannot unmarshal <kind> into Go value of type <target>`.
pub fn type_error(value: &JsonValue, target: &str) -> String {
    format!(
        "json: cannot unmarshal {} into Go value of type {target}",
        kind(value)
    )
}

/// `json: cannot unmarshal <kind> into Go struct field <path> of type <target>`.
pub fn field_type_error(value: &JsonValue, path: &str, target: &str) -> String {
    format!(
        "json: cannot unmarshal {} into Go struct field {path} of type {target}",
        kind(value)
    )
}

/// `json: cannot unmarshal <kind> into <path> of type <target>` — the wording
/// `encoding/json` uses for a slice element rather than a struct field.
pub fn element_type_error(value: &JsonValue, path: &str, target: &str) -> String {
    format!(
        "json: cannot unmarshal {} into {path} of type {target}",
        kind(value)
    )
}

/// Decode a JSON object into `map[string]string`, with Go's error wording.
///
/// `named` is how Go spells the destination type: `cmd.Env` at the top level,
/// `map[string]string` for a nested field. `prefix` is the struct-field path
/// Go would report for a bad member.
pub fn decode_string_map(
    value: &JsonValue,
    named: &str,
    prefix: &str,
) -> Result<HashMap<String, String>, String> {
    let mut map = HashMap::new();
    match value {
        // Go leaves the destination untouched for a JSON null.
        JsonValue::Null => Ok(map),
        JsonValue::Object(pairs) => {
            for (key, item) in pairs {
                match item {
                    JsonValue::String(text) => {
                        map.insert(key.clone(), text.clone());
                    }
                    other => {
                        return Err(field_type_error(
                            other,
                            &format!("{prefix}.{key}"),
                            "string",
                        ))
                    }
                }
            }
            Ok(map)
        }
        other => Err(type_error(other, named)),
    }
}

/// Parse a JSON object of string values into a map, Go's
/// `json.Unmarshal(data, &cmd.Env{})`.
pub fn parse_string_map(text: &str) -> Result<HashMap<String, String>, String> {
    let value = parse(text)?;
    decode_string_map(&value, "cmd.Env", "")
}
