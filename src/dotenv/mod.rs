//! Parsing of the `.env` format.
//!
//! There is no formal definition of the format but it has been introduced by
//! <https://github.com/bkeepers/dotenv> which is thus canonical.

use std::collections::HashMap;
use std::sync::OnceLock;

use regex::Regex;

use crate::deps::goexpand;

/// The regexp matching a single line, before whitespace and comments are
/// stripped out of it at first use.
pub const LINE: &str = r#"
\A
\s*
(?:|#.*|          # comment line
(?:export\s+)?    # optional export
([\w\.]+)         # key
(?:\s*=\s*|:\s+?) # separator
(                 # optional value begin
  '(?:\'|[^'])*'  #   single quoted value
  |               #   or
  "(?:\"|[^"])*"  #   double quoted value
  |               #   or
  [^\s#\n]+       #   unquoted value
)?                # value end
\s*
(?:\#.*)?         # optional comment
)
\z
"#;

fn lines_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"[\r\n]+").expect("lines regexp"))
}

/// `LINE` with its inline comments and every run of literal whitespace removed,
/// which is how the original derives the pattern it actually matches with.
fn line_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let comments = Regex::new(r"\s+# .*").expect("comment regexp");
        let spaces = Regex::new(r"\s+").expect("space regexp");
        let stripped = comments.replace_all(LINE, "");
        let pattern = spaces.replace_all(&stripped, "");
        Regex::new(&pattern).expect("line regexp")
    })
}

fn esc_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\\([^$])").expect("escape regexp"))
}

/// Reads a string in the .env format and returns a map of the extracted
/// key=values.
///
/// Ported from <https://github.com/bkeepers/dotenv/blob/84f33f48107c492c3a99bd41c1059e7b4c1bb67a/lib/dotenv/parser.rb>
pub fn parse(data: &str) -> Result<HashMap<String, String>, String> {
    let mut dotenv: HashMap<String, String> = HashMap::new();
    let lines: Vec<&str> = lines_re().split(data).collect();
    let mut in_multiline = false;
    let mut multiline_value = String::new();
    let mut quote_char = b'\0';

    for line in lines {
        // Continue collecting a multi-line value
        if in_multiline {
            multiline_value.push('\n');
            multiline_value.push_str(line);

            // Check if this line completes the multi-line value
            if line.trim().ends_with(quote_char as char) {
                // Process the completed multi-line value
                if let Some(caps) = line_re().captures(&multiline_value) {
                    let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    if !key.is_empty() {
                        let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                        parse_value(key, value, &mut dotenv);
                    }
                }
                in_multiline = false;
            }
            continue;
        }

        // Check for the beginning of a multi-line value
        if line.contains('=') || line.contains(':') {
            let sep_idx = line.find(['=', ':']).unwrap_or(0);
            if sep_idx > 0 && sep_idx + 1 < line.len() {
                // Extract the part after the separator
                let after_sep = &line[sep_idx + 1..];
                let trimmed_after_sep = after_sep.trim_start_matches([' ', '\t']);

                // Check if value starts with a quote
                let head = trimmed_after_sep.as_bytes().first().copied();
                if head == Some(b'"') || head == Some(b'\'') {
                    quote_char = head.unwrap();
                    // Count quotes to determine if it's multi-line
                    if trimmed_after_sep.matches(quote_char as char).count() == 1 {
                        // Start multi-line collection
                        in_multiline = true;
                        multiline_value = line.to_string();
                        continue;
                    }
                }
            }
        }

        // Normal line processing
        let Some(caps) = line_re().captures(line) else {
            return Err(format!("invalid line: {line}"));
        };
        // commented or empty line
        let key = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if key.is_empty() {
            continue;
        }
        let value = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        parse_value(key, value, &mut dotenv);
    }

    // If we end with an unclosed multi-line value, return an error
    if in_multiline {
        return Err("unclosed quoted value in .env file".to_string());
    }

    Ok(dotenv)
}

/// Works the same as [`parse`] but panics on error.
pub fn must_parse(data: &str) -> HashMap<String, String> {
    match parse(data) {
        Ok(env) => env,
        Err(err) => panic!("{err}"),
    }
}

fn parse_value(key: &str, value: &str, dotenv: &mut HashMap<String, String>) {
    if value.len() <= 1 {
        dotenv.insert(key.to_string(), value.to_string());
        return;
    }

    let bytes = value.as_bytes();
    let first = bytes[0];
    let last = bytes[bytes.len() - 1];
    let mut single_quoted = false;
    let mut value = value.to_string();

    if first == b'\'' && last == b'\'' {
        // single-quoted string, do not expand
        single_quoted = true;
        value = value[1..value.len() - 1].to_string();
    } else if first == b'"' && last == b'"' {
        value = value[1..value.len() - 1].to_string();
        value = expand_new_lines(&value);
        value = unescape_characters(&value);
    }

    if !single_quoted {
        value = expand_env(&value, dotenv);
    }

    dotenv.insert(key.to_string(), value);
}

fn unescape_characters(value: &str) -> String {
    esc_re().replace_all(value, "$1").into_owned()
}

fn expand_new_lines(value: &str) -> String {
    value.replace("\\n", "\n").replace("\\r", "\r")
}

fn expand_env(value: &str, dotenv: &HashMap<String, String>) -> String {
    goexpand::expand(value, |name| {
        let (env_key, default_value, has_default) = split_key_and_default(name, ":-");

        match dotenv.get(env_key) {
            Some(expanded) => expanded.clone(),
            None => get_from_env_or_default(env_key, default_value, has_default),
        }
    })
}

fn split_key_and_default<'a>(value: &'a str, sep: &str) -> (&'a str, &'a str, bool) {
    match value.find(sep) {
        None => (value, "", false),
        Some(i) => (&value[..i], &value[i + sep.len()..], true),
    }
}

fn get_from_env_or_default(env_key: &str, default_value: &str, has_default: bool) -> String {
    let env_value = std::env::var(env_key).unwrap_or_default();

    if env_value.is_empty() && has_default {
        return default_value.to_string();
    }
    env_value
}
