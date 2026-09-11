//! The parts of Go's `time` package that direnv puts on screen.
//!
//! Three behaviours are observable: `time.ParseDuration` accepts the
//! `warn_timeout` spelling in `direnv.toml`, `Duration.String` renders it back
//! in `direnv status`, and `Time.MarshalText` renders a watch's mtime as
//! strict RFC 3339 in the local zone.

/// Nanoseconds, the unit of a Go `time.Duration`.
pub const NANOSECOND: i64 = 1;
pub const MICROSECOND: i64 = 1_000 * NANOSECOND;
pub const MILLISECOND: i64 = 1_000 * MICROSECOND;
pub const SECOND: i64 = 1_000 * MILLISECOND;
pub const MINUTE: i64 = 60 * SECOND;
pub const HOUR: i64 = 60 * MINUTE;

/// Go's `time.ParseDuration`.
///
/// A duration is a possibly signed sequence of decimal numbers, each with an
/// optional fraction and a required unit suffix, such as `300ms`, `-1.5h` or
/// `2h45m`. Valid units are `ns`, `us` (or `\u{00b5}s`), `ms`, `s`, `m`, `h`.
pub fn parse_duration(input: &str) -> Result<i64, String> {
    let invalid = || format!("time: invalid duration {}", quote(input));
    let mut s = input;
    let mut neg = false;

    if let Some(first) = s.as_bytes().first() {
        if *first == b'-' || *first == b'+' {
            neg = *first == b'-';
            s = &s[1..];
        }
    }
    // Special case: all that is left is "0".
    if s == "0" {
        return Ok(0);
    }
    if s.is_empty() {
        return Err(invalid());
    }

    let mut total: i64 = 0;
    while !s.is_empty() {
        // The next character must be [0-9.]
        let head = s.as_bytes()[0];
        if !(head == b'.' || head.is_ascii_digit()) {
            return Err(invalid());
        }

        // Consume [0-9]*
        let before = s.len();
        let (mut value, rest) = leading_int(s).ok_or_else(invalid)?;
        s = rest;
        let pre = before != s.len();

        // Consume (\.[0-9]*)?
        let mut frac: i64 = 0;
        let mut scale: f64 = 1.0;
        let mut post = false;
        if let Some(rest) = s.strip_prefix('.') {
            s = rest;
            let before = s.len();
            let (f, sc, rest) = leading_fraction(s);
            frac = f;
            scale = sc;
            s = rest;
            post = before != s.len();
        }
        if !pre && !post {
            return Err(invalid());
        }

        // Consume the unit.
        let unit_end = s
            .as_bytes()
            .iter()
            .position(|c| *c == b'.' || c.is_ascii_digit())
            .unwrap_or(s.len());
        if unit_end == 0 {
            return Err(format!("time: missing unit in duration {}", quote(input)));
        }
        let unit_name = &s[..unit_end];
        s = &s[unit_end..];
        let unit = match unit_name {
            "ns" => NANOSECOND,
            "us" | "\u{00b5}s" | "\u{03bc}s" => MICROSECOND,
            "ms" => MILLISECOND,
            "s" => SECOND,
            "m" => MINUTE,
            "h" => HOUR,
            other => {
                return Err(format!(
                    "time: unknown unit {} in duration {}",
                    quote(other),
                    quote(input)
                ))
            }
        };

        if value > i64::MAX / unit {
            return Err(invalid());
        }
        value *= unit;
        if frac > 0 {
            value += (frac as f64 * (unit as f64 / scale)) as i64;
            if value < 0 {
                return Err(invalid());
            }
        }
        total += value;
        if total < 0 {
            return Err(invalid());
        }
    }

    Ok(if neg { -total } else { total })
}

/// Consume a run of digits, Go's `leadingInt`; `None` on overflow.
fn leading_int(s: &str) -> Option<(i64, &str)> {
    let mut value: i64 = 0;
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        value = value
            .checked_mul(10)?
            .checked_add((bytes[idx] - b'0') as i64)?;
        idx += 1;
    }
    Some((value, &s[idx..]))
}

/// Consume a run of digits as a fraction, Go's `leadingFraction`.
///
/// Overflow is not an error: the remaining digits are consumed and dropped.
fn leading_fraction(s: &str) -> (i64, f64, &str) {
    let mut value: i64 = 0;
    let mut scale = 1.0f64;
    let mut overflow = false;
    let bytes = s.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        if !overflow {
            match value.checked_mul(10) {
                Some(next) if next <= (1 << 62) => {
                    value = next + (bytes[idx] - b'0') as i64;
                    scale *= 10.0;
                }
                _ => overflow = true,
            }
        }
        idx += 1;
    }
    (value, scale, &s[idx..])
}

fn quote(text: &str) -> String {
    format!("{text:?}")
}

/// Go's `time.Duration.String`.
///
/// Renders as a sequence of decimal numbers with unit suffixes, largest unit
/// first, with leading zero units elided: `1h0m0s`, `5s`, `300ms`, `1.0001us`.
/// Zero is `0s`.
pub fn duration_string(d: i64) -> String {
    if d == 0 {
        return "0s".to_string();
    }
    let neg = d < 0;
    let mut u = d.unsigned_abs();
    let mut buf: Vec<u8> = Vec::with_capacity(32);

    if u < SECOND as u64 {
        // Sub-second: use the largest unit that keeps the integer part non-zero.
        let prec;
        buf.push(b's');
        if u < MICROSECOND as u64 {
            prec = 0;
            buf.push(b'n');
        } else if u < MILLISECOND as u64 {
            prec = 3;
            // "µ" is two bytes in UTF-8; the buffer is reversed at the end.
            buf.push(0xb5);
            buf.push(0xc2);
        } else {
            prec = 6;
            buf.push(b'm');
        }
        u = fmt_frac(&mut buf, u, prec);
        fmt_int(&mut buf, u);
    } else {
        buf.push(b's');
        u = fmt_frac(&mut buf, u, 9);
        fmt_int(&mut buf, u % 60);
        u /= 60;
        if u > 0 {
            buf.push(b'm');
            fmt_int(&mut buf, u % 60);
            u /= 60;
            if u > 0 {
                buf.push(b'h');
                fmt_int(&mut buf, u);
            }
        }
    }

    if neg {
        buf.push(b'-');
    }
    buf.reverse();
    String::from_utf8_lossy(&buf).into_owned()
}

/// Append the fractional part of `v` (in reverse) and return the whole part.
fn fmt_frac(buf: &mut Vec<u8>, mut v: u64, prec: usize) -> u64 {
    let mut print = false;
    for _ in 0..prec {
        let digit = v % 10;
        print = print || digit != 0;
        if print {
            buf.push(b'0' + digit as u8);
        }
        v /= 10;
    }
    if print {
        buf.push(b'.');
    }
    v
}

/// Append `v` in reverse; always emits at least one digit.
fn fmt_int(buf: &mut Vec<u8>, mut v: u64) {
    if v == 0 {
        buf.push(b'0');
        return;
    }
    while v > 0 {
        buf.push(b'0' + (v % 10) as u8);
        v /= 10;
    }
}

/// Go's `time.Unix(sec, 0).MarshalText()` — strict RFC 3339 in the local zone.
///
/// The sub-second field is omitted for a whole second, and the offset is `Z`
/// when the local zone is UTC.
pub fn unix_marshal_text(sec: i64) -> String {
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let t = sec as libc::time_t;
    // SAFETY: `tm` is a valid, writable destination and `t` is a valid time_t.
    let ok = unsafe { !libc::localtime_r(&t, &mut tm).is_null() };
    if !ok {
        return "<<???>>".to_string();
    }
    let offset = tm.tm_gmtoff as i64;
    let zone = if offset == 0 {
        "Z".to_string()
    } else {
        let sign = if offset < 0 { '-' } else { '+' };
        let abs = offset.abs();
        format!("{}{:02}:{:02}", sign, abs / 3600, (abs % 3600) / 60)
    };
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
        tm.tm_year as i64 + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        zone
    )
}
