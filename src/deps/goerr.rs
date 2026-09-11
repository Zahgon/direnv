//! Go-shaped error text for OS failures.
//!
//! Go renders a failed syscall as `<op> <path>: <errno>`, where `<errno>` is
//! the C `strerror` string with a lower-case first letter (`open /nope: no
//! such file or directory`). Rust's `io::Error` renders the same failure as
//! `No such file or directory (os error 2)`. direnv prints these straight to
//! the user through `direnv: error %v`, so the wording is part of the contract.

use std::ffi::CStr;
use std::io;

/// The Go spelling of an `io::Error`, without the operation or path prefix.
pub fn errno_text(err: &io::Error) -> String {
    match err.raw_os_error() {
        Some(code) => strerror(code),
        None => err.to_string(),
    }
}

/// `<op> <path>: <errno>` — Go's `*fs.PathError`.
pub fn path_error(op: &str, path: &str, err: &io::Error) -> String {
    format!("{op} {path}: {}", errno_text(err))
}

fn strerror(code: i32) -> String {
    // SAFETY: strerror returns a pointer to a static, NUL-terminated buffer.
    let text = unsafe {
        let ptr = libc::strerror(code);
        if ptr.is_null() {
            return format!("errno {code}");
        }
        CStr::from_ptr(ptr).to_string_lossy().into_owned()
    };
    lower_first(&text)
}

fn lower_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
