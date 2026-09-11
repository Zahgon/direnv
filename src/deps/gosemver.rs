//! `golang.org/x/mod/semver`, as `direnv version VERSION_AT_LEAST` uses it.
//!
//! This is not the semver.org grammar: the leading `v` is mandatory, and the
//! minor and patch fields are optional (`v2` and `v2.37` are both valid and
//! both mean `.0` for the missing fields). Build metadata is parsed but
//! ignored when comparing, and a pre-release sorts *before* its release.

struct Parsed<'a> {
    major: &'a str,
    minor: &'a str,
    patch: &'a str,
    prerelease: &'a str,
}

/// Reports whether `v` is a valid semantic version string in Go's dialect.
pub fn is_valid(v: &str) -> bool {
    parse(v).is_some()
}

/// Compares two versions: -1, 0 or +1. An invalid version is lower than a
/// valid one, and two invalid versions compare equal.
pub fn compare(v: &str, w: &str) -> i32 {
    let pv = parse(v);
    let pw = parse(w);
    let (pv, pw) = match (pv, pw) {
        (None, None) => return 0,
        (None, Some(_)) => return -1,
        (Some(_), None) => return 1,
        (Some(a), Some(b)) => (a, b),
    };
    let c = compare_int(pv.major, pw.major);
    if c != 0 {
        return c;
    }
    let c = compare_int(pv.minor, pw.minor);
    if c != 0 {
        return c;
    }
    let c = compare_int(pv.patch, pw.patch);
    if c != 0 {
        return c;
    }
    compare_prerelease(pv.prerelease, pw.prerelease)
}

fn parse(v: &str) -> Option<Parsed<'_>> {
    let rest = v.strip_prefix('v')?;
    let (major, rest) = parse_int(rest)?;
    if rest.is_empty() {
        return Some(Parsed {
            major,
            minor: "0",
            patch: "0",
            prerelease: "",
        });
    }
    let rest = rest.strip_prefix('.')?;
    let (minor, rest) = parse_int(rest)?;
    if rest.is_empty() {
        return Some(Parsed {
            major,
            minor,
            patch: "0",
            prerelease: "",
        });
    }
    let rest = rest.strip_prefix('.')?;
    let (patch, mut rest) = parse_int(rest)?;

    let mut prerelease = "";
    if rest.starts_with('-') {
        let (pre, remainder) = parse_prerelease(rest)?;
        prerelease = pre;
        rest = remainder;
    }
    if rest.starts_with('+') {
        let (_, remainder) = parse_build(rest)?;
        rest = remainder;
    }
    if !rest.is_empty() {
        return None;
    }
    Some(Parsed {
        major,
        minor,
        patch,
        prerelease,
    })
}

/// A run of digits with no redundant leading zero.
fn parse_int(v: &str) -> Option<(&str, &str)> {
    let bytes = v.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }
    let mut i = 1;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    if bytes[0] == b'0' && i != 1 {
        return None;
    }
    Some((&v[..i], &v[i..]))
}

fn parse_prerelease(v: &str) -> Option<(&str, &str)> {
    let bytes = v.as_bytes();
    if bytes.is_empty() || bytes[0] != b'-' {
        return None;
    }
    let mut i = 1;
    let mut start = 1;
    while i < bytes.len() && bytes[i] != b'+' {
        if !is_ident_char(bytes[i]) && bytes[i] != b'.' {
            return None;
        }
        if bytes[i] == b'.' {
            if start == i || is_bad_num(&v[start..i]) {
                return None;
            }
            start = i + 1;
        }
        i += 1;
    }
    if start == i || is_bad_num(&v[start..i]) {
        return None;
    }
    Some((&v[..i], &v[i..]))
}

fn parse_build(v: &str) -> Option<(&str, &str)> {
    let bytes = v.as_bytes();
    if bytes.is_empty() || bytes[0] != b'+' {
        return None;
    }
    let mut i = 1;
    let mut start = 1;
    while i < bytes.len() {
        if !is_ident_char(bytes[i]) && bytes[i] != b'.' {
            return None;
        }
        if bytes[i] == b'.' {
            if start == i {
                return None;
            }
            start = i + 1;
        }
        i += 1;
    }
    if start == i {
        return None;
    }
    Some((&v[..i], &v[i..]))
}

fn is_ident_char(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'-'
}

/// An all-digit identifier with a redundant leading zero.
fn is_bad_num(v: &str) -> bool {
    let bytes = v.as_bytes();
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    i == bytes.len() && i > 1 && bytes[0] == b'0'
}

/// Go's `isNum`: every byte is a digit (vacuously true for the empty string).
fn is_num(v: &str) -> bool {
    v.bytes().all(|c| c.is_ascii_digit())
}

fn compare_int(x: &str, y: &str) -> i32 {
    if x == y {
        return 0;
    }
    if x.len() < y.len() {
        return -1;
    }
    if x.len() > y.len() {
        return 1;
    }
    if x < y {
        -1
    } else {
        1
    }
}

fn compare_prerelease(mut x: &str, mut y: &str) -> i32 {
    if x == y {
        return 0;
    }
    // A version with a pre-release sorts below the same version without one.
    if x.is_empty() {
        return 1;
    }
    if y.is_empty() {
        return -1;
    }
    while !x.is_empty() && !y.is_empty() {
        x = &x[1..]; // skip the leading '-' or '.'
        y = &y[1..];
        let (dx, restx) = next_ident(x);
        let (dy, resty) = next_ident(y);
        x = restx;
        y = resty;
        if dx != dy {
            let ix = is_num(dx);
            let iy = is_num(dy);
            if ix != iy {
                return if ix { -1 } else { 1 };
            }
            if ix {
                if dx.len() < dy.len() {
                    return -1;
                }
                if dx.len() > dy.len() {
                    return 1;
                }
            }
            return if dx < dy { -1 } else { 1 };
        }
    }
    if x.is_empty() {
        -1
    } else {
        1
    }
}

fn next_ident(x: &str) -> (&str, &str) {
    let i = x.find('.').unwrap_or(x.len());
    (&x[..i], &x[i..])
}
