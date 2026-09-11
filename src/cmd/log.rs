//! direnv's logging, including the parts of Go's `log` package it leans on.
//!
//! Two behaviours here look like bugs and are not to be tidied up: `logError`
//! emits the ANSI colour codes when `LogColor` is **false**, and `logStatus`
//! prefixes the reset code in the same case. Both are reproduced exactly,
//! because the wording and the bytes of direnv's output are the contract.

use std::io::Write;
use std::sync::Mutex;
use std::sync::OnceLock;

use crate::cmd::config::Config;
use crate::cmd::consts::DIRENV_DEBUG;
use crate::cmd::env::Env;
use crate::cmd::printf::{sprintf, Arg};

pub const DEFAULT_LOG_FORMAT: &str = "direnv: %s";
pub const ERROR_COLOR: &str = "\x1b[31m";
pub const CLEAR_COLOR: &str = "\x1b[0m";

/// Go's `log.Ltime`.
const LTIME: u8 = 1 << 0;
/// Go's `log.Lshortfile`.
const LSHORTFILE: u8 = 1 << 1;

struct LoggerState {
    flags: u8,
    prefix: String,
    debugging: bool,
}

fn state() -> &'static Mutex<LoggerState> {
    static STATE: OnceLock<Mutex<LoggerState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(LoggerState {
            flags: 0,
            prefix: String::new(),
            debugging: false,
        })
    })
}

/// Configure logging from the environment, as `main` does before dispatching.
pub fn setup_logging(env: &Env) {
    let mut logger = state().lock().expect("logger");
    logger.flags = 0;
    logger.prefix = String::new();
    if let Some(value) = env.get(DIRENV_DEBUG) {
        if value == "1" || value.eq_ignore_ascii_case("true") {
            logger.debugging = true;
            logger.flags = LTIME;
            logger.prefix = "direnv: ".to_string();
        }
    }
}

/// The current log prefix, Go's `log.Prefix()`.
pub fn prefix() -> String {
    state().lock().expect("logger").prefix.clone()
}

/// Go's `log.SetPrefix`.
pub fn set_prefix(value: &str) {
    state().lock().expect("logger").prefix = value.to_string();
}

/// Restores the log prefix on drop, standing in for Go's
/// `defer log.SetPrefix(log.Prefix())`.
pub struct PrefixGuard(String);

impl PrefixGuard {
    /// Push `extra` onto the current prefix until the guard is dropped.
    pub fn push(extra: &str) -> PrefixGuard {
        let previous = prefix();
        set_prefix(&format!("{previous}{extra}"));
        PrefixGuard(previous)
    }
}

impl Drop for PrefixGuard {
    fn drop(&mut self) {
        set_prefix(&self.0);
    }
}

/// Whether `DIRENV_DEBUG` turned debug logging on.
pub fn debugging() -> bool {
    state().lock().expect("logger").debugging
}

pub fn log_error(config: &Config, message: &str) {
    log_error_args(config, message, &[]);
}

pub fn log_error_args(config: &Config, message: &str, args: &[Arg]) {
    if config.log_color {
        log_msg(DEFAULT_LOG_FORMAT, message, args);
    } else {
        log_msg(
            &format!("{ERROR_COLOR}{DEFAULT_LOG_FORMAT}{CLEAR_COLOR}"),
            message,
            args,
        );
    }
}

pub fn log_status(config: &Config, message: &str) {
    log_status_args(config, message, &[]);
}

pub fn log_status_args(config: &Config, message: &str, args: &[Arg]) {
    let format = &config.log_format;
    let should_log = match &config.log_filter {
        Some(filter) => filter.is_match(message),
        None => true,
    };
    if should_log && !format.is_empty() {
        if config.log_color {
            log_msg(format, message, args);
        } else {
            log_msg(&format!("{CLEAR_COLOR}{format}"), message, args);
        }
    }
}

/// Go's `logMsg`: interpolate the message into the format, then hand the
/// result to `Printf` as a format string in its own right.
fn log_msg(format: &str, message: &str, args: &[Arg]) {
    let interpolated = sprintf(&format!("{format}\n"), &[Arg::Str(message.to_string())]);
    let rendered = sprintf(&interpolated, args);
    write_line(&rendered, false);
}

/// Go's `logDebug`, which adds `Lshortfile` for the duration of the call.
pub fn log_debug_at(file: &str, line: u32, message: &str) {
    if !debugging() {
        return;
    }
    let location = format!("{}:{}: ", file.rsplit('/').next().unwrap_or(file), line);
    write_line(&format!("{location}{message}\n"), true);
}

/// Emit through the shared logger state, honouring prefix and flags.
fn write_line(text: &str, with_flags: bool) {
    let logger = state().lock().expect("logger");
    let mut out = String::new();
    if with_flags {
        out.push_str(&logger.prefix);
        if logger.flags & LTIME != 0 {
            out.push_str(&clock_time());
            out.push(' ');
        }
    }
    let _ = logger.flags & LSHORTFILE;
    out.push_str(text);
    if !out.ends_with('\n') {
        out.push('\n');
    }
    drop(logger);
    let stderr = std::io::stderr();
    let mut handle = stderr.lock();
    let _ = handle.write_all(out.as_bytes());
}

/// `HH:MM:SS`, the rendering of Go's `log.Ltime`.
fn clock_time() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    let t = now as libc::time_t;
    // SAFETY: `tm` is a valid, writable destination and `t` is a valid time_t.
    if unsafe { libc::localtime_r(&t, &mut tm).is_null() } {
        return "00:00:00".to_string();
    }
    format!("{:02}:{:02}:{:02}", tm.tm_hour, tm.tm_min, tm.tm_sec)
}

/// direnv's debug logger. A no-op unless `DIRENV_DEBUG` is set.
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        $crate::cmd::log::log_debug_at(file!(), line!(), &format!($($arg)*))
    };
}
