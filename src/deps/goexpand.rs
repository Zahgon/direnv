//! Go's `os.Expand`.
//!
//! `pkg/dotenv` uses it for `$VAR` / `${VAR}` substitution, and its exact
//! treatment of the awkward cases is pinned by `TestVariableExpansion` and
//! `TestVariableExpansionWithDefaults`: `${}` and a dangling `${` are eaten,
//! a `$` that is not followed by a name is left alone, and the shell's
//! single-character special names (`$*`, `$1`, `$-`, ...) are recognised.

/// Replace `${name}` or `$name` in `text` according to `mapping`.
pub fn expand<F>(text: &str, mapping: F) -> String
where
    F: Fn(&str) -> String,
{
    let bytes = text.as_bytes();
    let mut out: Option<String> = None;
    let mut i = 0usize;
    let mut j = 0usize;
    while j < bytes.len() {
        if bytes[j] == b'$' && j + 1 < bytes.len() {
            let buf = out.get_or_insert_with(|| String::with_capacity(2 * text.len()));
            buf.push_str(&text[i..j]);
            let (name, width) = shell_name(&text[j + 1..]);
            if name.is_empty() && width > 0 {
                // Valid but empty syntax such as "${}": eat the characters.
            } else if name.is_empty() {
                // A "$" that is not followed by a name stays as it is.
                buf.push('$');
            } else {
                buf.push_str(&mapping(name));
            }
            j += width;
            i = j + 1;
        }
        j += 1;
    }
    match out {
        None => text.to_string(),
        Some(mut buf) => {
            buf.push_str(&text[i..]);
            buf
        }
    }
}

/// Go's `getShellName`: the name at the start of `s` and how many bytes of `s`
/// it occupies.
fn shell_name(s: &str) -> (&str, usize) {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return ("", 0);
    }
    if bytes[0] == b'{' {
        if bytes.len() > 2 && is_shell_special_var(bytes[1]) && bytes[2] == b'}' {
            return (&s[1..2], 3);
        }
        // Scan to the closing brace.
        for i in 1..bytes.len() {
            if bytes[i] == b'}' {
                if i == 1 {
                    return ("", 2); // Bad syntax; eat "${}"
                }
                return (&s[1..i], i + 1);
            }
        }
        return ("", 1); // Bad syntax; eat "${"
    }
    if is_shell_special_var(bytes[0]) {
        return (&s[0..1], 1);
    }
    let mut i = 0usize;
    while i < bytes.len() && is_alpha_num(bytes[i]) {
        i += 1;
    }
    (&s[..i], i)
}

fn is_shell_special_var(c: u8) -> bool {
    matches!(
        c,
        b'*' | b'#' | b'$' | b'@' | b'!' | b'?' | b'-' | b'0'..=b'9'
    )
}

fn is_alpha_num(c: u8) -> bool {
    c == b'_' || c.is_ascii_digit() || c.is_ascii_lowercase() || c.is_ascii_uppercase()
}
