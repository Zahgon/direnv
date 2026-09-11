//! The slice of Go's `fmt` verb handling that reaches the user.
//!
//! `logMsg` interpolates the message into the log format and then hands the
//! *result* to `log.Printf` as a format string of its own. A message that
//! happens to contain a percent sign is therefore reformatted, and the
//! artefacts are visible: `direnv log --status "100% done"` prints
//! `direnv: 100%!d(MISSING)one` in the Go build. Reproducing that is the
//! reason this module exists.

/// A value that can be substituted for a verb.
pub enum Arg {
    Str(String),
    /// A Go `[]string`, which `%v` renders as `[a b c]`.
    List(Vec<String>),
}

impl Arg {
    fn type_name(&self) -> &'static str {
        match self {
            Arg::Str(_) => "string",
            Arg::List(_) => "[]string",
        }
    }

    fn to_v(&self) -> String {
        match self {
            Arg::Str(text) => text.clone(),
            Arg::List(items) => format!("[{}]", items.join(" ")),
        }
    }
}

/// Go's `fmt.Sprintf` for the verbs direnv can reach.
pub fn sprintf(format: &str, args: &[Arg]) -> String {
    let bytes = format.as_bytes();
    let mut out = String::with_capacity(format.len());
    let mut i = 0usize;
    let mut arg_num = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'%' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'%' {
                i += 1;
            }
            out.push_str(&format[start..i]);
            continue;
        }
        i += 1; // consume '%'

        // Flags, then width, then precision — parsed and discarded; direnv's
        // own format strings use none of them.
        while i < bytes.len() && matches!(bytes[i], b'#' | b'0' | b'+' | b'-' | b' ') {
            i += 1;
        }
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i < bytes.len() && bytes[i] == b'.' {
            i += 1;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        }

        if i >= bytes.len() {
            out.push_str("%!(NOVERB)");
            break;
        }
        let verb = bytes[i] as char;
        i += 1;

        if verb == '%' {
            out.push('%');
            continue;
        }
        let Some(arg) = args.get(arg_num) else {
            out.push_str(&format!("%!{verb}(MISSING)"));
            continue;
        };
        arg_num += 1;
        match verb {
            's' | 'v' => out.push_str(&arg.to_v()),
            'q' => out.push_str(&format!("{:?}", arg.to_v())),
            other => out.push_str(&format!("%!{other}({}={})", arg.type_name(), arg.to_v())),
        }
    }

    if arg_num < args.len() {
        out.push_str("%!(EXTRA ");
        for (offset, arg) in args[arg_num..].iter().enumerate() {
            if offset > 0 {
                out.push_str(", ");
            }
            out.push_str(&format!("{}={}", arg.type_name(), arg.to_v()));
        }
        out.push(')');
    }

    out
}
