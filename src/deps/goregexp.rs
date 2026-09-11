//! The one place Go's `regexp` accepts a pattern the `regex` crate rejects.
//!
//! `log_filter` in `direnv.toml` is compiled with Go's `regexp`, which treats a
//! `{` that does not begin a valid `{n}`, `{n,}` or `{n,m}` repetition as a
//! literal brace. The `regex` crate rejects it. A filter such as `\{` written
//! as `{` therefore works before the migration and stops working after it,
//! which is a regression a user would feel, so the leniency is reproduced by
//! escaping those braces before compiling.
//!
//! The reverse case is here too: Go inherits Perl's refusal to stack repetition
//! operators, so `a**` is a syntax error rather than a doubled star, while the
//! `regex` crate accepts it. Rejecting it keeps a bad filter from silently
//! meaning something different after the migration.
//!
//! Three more differences are closed here because they change *matching*
//! rather than parsing, and would therefore be silent:
//!
//! * Go's `\d`, `\s`, `\w` (and their negations) are ASCII-only Perl classes,
//!   while the `regex` crate makes them Unicode-aware, so `\w` matches `é` in
//!   Rust and not in Go. They are expanded to their ASCII definitions.
//! * `\b` and `\B` are ASCII word boundaries in Go; they are scoped with
//!   `(?-u:...)` to get the same behaviour.
//! * Go accepts a repeated capture-group name and the `regex` crate rejects it.
//!   direnv only ever calls `MatchString`, so group names carry no meaning and
//!   named groups are flattened to plain ones.
//!
//! This is not a port of RE2's grammar. Anything else the two engines disagree
//! about is left to them; see truth.md.

/// Translate a Go pattern for the `regex` crate, or report Go's error.
pub fn compile_pattern(pattern: &str) -> Result<String, String> {
    if let Some(text) = nested_repeat(pattern) {
        return Err(format!(
            "error parsing regexp: invalid nested repetition operator: `{text}`"
        ));
    }
    Ok(translate(pattern))
}

/// The text Go would quote for `ErrInvalidRepeatOp`, if the pattern stacks two
/// repetition operators.
fn nested_repeat(pattern: &str) -> Option<String> {
    let bytes = pattern.as_bytes();
    let mut i = 0usize;
    let mut in_class = false;
    // Where the previous repetition token started, once one has been seen.
    let mut last_repeat: Option<usize> = None;

    while i < bytes.len() {
        let c = bytes[i];
        if c == b'\\' {
            i += 1 + if i + 1 < bytes.len() {
                char_len(bytes, i + 1)
            } else {
                0
            };
            last_repeat = None;
            continue;
        }
        if in_class {
            if c == b']' {
                in_class = false;
            }
            i += char_len(bytes, i);
            continue;
        }
        if c == b'[' {
            in_class = true;
            i += 1;
            last_repeat = None;
            continue;
        }

        let width = match c {
            b'*' | b'+' | b'?' => Some(1),
            b'{' => parse_repeat_len(&pattern[i..]),
            _ => None,
        };
        match width {
            Some(width) => {
                if let Some(start) = last_repeat {
                    return Some(pattern[start..i + width].to_string());
                }
                let start = i;
                i += width;
                // Perl's non-greedy marker binds to the operator just parsed.
                if i < bytes.len() && bytes[i] == b'?' {
                    i += 1;
                }
                last_repeat = Some(start);
            }
            None => {
                last_repeat = None;
                i += char_len(bytes, i);
            }
        }
    }
    None
}

/// Escape every `{` that Go would treat as a literal.
pub fn translate(pattern: &str) -> String {
    let bytes = pattern.as_bytes();
    let mut out = String::with_capacity(pattern.len());
    let mut i = 0usize;
    let mut in_class = false;

    while i < bytes.len() {
        let c = bytes[i];
        match c {
            b'\\' => {
                i += 1;
                if i >= bytes.len() {
                    out.push('\\');
                    continue;
                }
                let escaped = bytes[i];
                if let Some(expansion) = perl_class(escaped, in_class) {
                    out.push_str(expansion);
                    i += 1;
                    continue;
                }
                // Octal escapes: `\0`, `\012`, `\12`. The `regex` crate has none.
                if let Some((value, width)) = octal_escape(&pattern[i..]) {
                    out.push_str(&format!("\\x{{{value:x}}}"));
                    i += width;
                    continue;
                }
                // Perl's literal-text quoting, which the `regex` crate lacks.
                if escaped == b'Q' {
                    let body = &pattern[i + 1..];
                    let end = body.find("\\E").unwrap_or(body.len());
                    for ch in body[..end].chars() {
                        push_quoted(&mut out, ch);
                    }
                    i += 1 + end + if end < body.len() { 2 } else { 0 };
                    continue;
                }
                out.push('\\');
                out.push_str(&pattern[i..i + char_len(bytes, i)]);
                i += char_len(bytes, i);
                // `\p{Greek}`, `\P{Greek}` and `\x{1F600}` carry their own braces.
                if matches!(escaped, b'p' | b'P' | b'x') && i < bytes.len() && bytes[i] == b'{' {
                    if let Some(end) = pattern[i..].find('}') {
                        out.push_str(&pattern[i..i + end + 1]);
                        i += end + 1;
                    }
                }
                continue;
            }
            b'[' if !in_class => {
                in_class = true;
                out.push('[');
            }
            b']' if in_class => {
                in_class = false;
                out.push(']');
            }
            b'(' if !in_class => {
                // Flatten a named group; Go allows duplicate names, the `regex`
                // crate does not, and direnv never reads the names.
                if let Some(width) = named_group_len(&pattern[i..]) {
                    out.push('(');
                    i += width;
                    continue;
                }
                out.push('(');
            }
            b'{' if !in_class => {
                if parse_repeat(&pattern[i..]).is_some() {
                    out.push('{');
                } else {
                    // Go's parser falls back to a literal brace here.
                    out.push_str("\\{");
                }
            }
            _ => out.push_str(&pattern[i..i + char_len(bytes, i)]),
        }
        i += char_len(bytes, i);
    }

    out
}

/// Go's octal escape: one to three octal digits, where a lone non-zero digit is
/// a back-reference rather than an escape and so is not one.
fn octal_escape(s: &str) -> Option<(u32, usize)> {
    let bytes = s.as_bytes();
    let first = *bytes.first()?;
    if !(b'0'..=b'7').contains(&first) {
        return None;
    }
    if first != b'0' && !matches!(bytes.get(1), Some(b'0'..=b'7')) {
        return None;
    }
    let mut value = (first - b'0') as u32;
    let mut width = 1;
    while width < 3 {
        match bytes.get(width) {
            Some(d @ b'0'..=b'7') => {
                value = value * 8 + (d - b'0') as u32;
                width += 1;
            }
            _ => break,
        }
    }
    Some((value, width))
}

/// Emit `ch` so that it matches itself, whatever it is.
fn push_quoted(out: &mut String, ch: char) {
    if ch.is_ascii_alphanumeric() || ch == '_' || !ch.is_ascii() {
        out.push(ch);
    } else {
        out.push('\\');
        out.push(ch);
    }
}

/// Go's ASCII-only spelling of the Perl classes and word boundaries.
fn perl_class(escaped: u8, in_class: bool) -> Option<&'static str> {
    Some(match (escaped, in_class) {
        (b'd', false) => "[0-9]",
        (b'd', true) => "0-9",
        (b'D', false) => "[^0-9]",
        (b'D', true) => "[^0-9]",
        (b's', false) => "[\\t\\n\\x0c\\r ]",
        (b's', true) => "\\t\\n\\x0c\\r ",
        (b'S', false) => "[^\\t\\n\\x0c\\r ]",
        (b'S', true) => "[^\\t\\n\\x0c\\r ]",
        (b'w', false) => "[0-9A-Za-z_]",
        (b'w', true) => "0-9A-Za-z_",
        (b'W', false) => "[^0-9A-Za-z_]",
        (b'W', true) => "[^0-9A-Za-z_]",
        (b'b', false) => "(?-u:\\b)",
        (b'B', false) => "(?-u:\\B)",
        _ => return None,
    })
}

/// The byte length of a `(?P<name>` or `(?<name>` prefix, if `s` starts with one.
fn named_group_len(s: &str) -> Option<usize> {
    let rest = s.strip_prefix("(?P<").or_else(|| s.strip_prefix("(?<"))?;
    let end = rest.find('>')?;
    if !rest[..end]
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || end == 0
    {
        return None;
    }
    Some(s.len() - rest.len() + end + 1)
}

fn char_len(bytes: &[u8], i: usize) -> usize {
    let b = bytes[i];
    if b < 0x80 {
        1
    } else if b >= 0xf0 {
        4
    } else if b >= 0xe0 {
        3
    } else if b >= 0xc0 {
        2
    } else {
        1
    }
}

/// Go's `parseRepeat`: `{n}`, `{n,}` or `{n,m}`.
fn parse_repeat(s: &str) -> Option<()> {
    parse_repeat_len(s).map(|_| ())
}

/// The byte length of the `{...}` repetition at the start of `s`, if it is one.
fn parse_repeat_len(s: &str) -> Option<usize> {
    let rest = s.strip_prefix('{')?;
    let (_, rest) = parse_int(rest)?;
    let rest = match rest.strip_prefix(',') {
        None => rest,
        Some(after) => {
            if after.starts_with('}') {
                after
            } else {
                let (_, after) = parse_int(after)?;
                after
            }
        }
    };
    let rest = rest.strip_prefix('}')?;
    Some(s.len() - rest.len())
}

fn parse_int(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    Some((&s[..i], &s[i..]))
}
