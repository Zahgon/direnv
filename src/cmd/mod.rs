//! The direnv command-line interface.

pub mod cmd_allow;
pub mod cmd_apply_dump;
pub mod cmd_check_required;
pub mod cmd_current;
pub mod cmd_deny;
pub mod cmd_dotenv;
pub mod cmd_dump;
pub mod cmd_edit;
pub mod cmd_exec;
pub mod cmd_export;
pub mod cmd_fetchurl;
pub mod cmd_help;
pub mod cmd_hook;
pub mod cmd_log;
pub mod cmd_prune;
pub mod cmd_reload;
pub mod cmd_show_dump;
pub mod cmd_status;
pub mod cmd_stdlib;
pub mod cmd_version;
pub mod cmd_watch;
pub mod cmd_watch_dir;
pub mod cmd_watch_list;
pub mod cmd_watch_print;
pub mod commands;
pub mod config;
pub mod consts;
pub mod env;
pub mod env_diff;
pub mod file_times;
pub mod log;
pub mod look_path;
pub mod printf;
pub mod rc;
pub mod shell;
pub mod shell_bash;
pub mod shell_elvish;
pub mod shell_fish;
pub mod shell_gha;
pub mod shell_gzenv;
pub mod shell_json;
pub mod shell_murex;
pub mod shell_pwsh;
pub mod shell_systemd;
pub mod shell_tcsh;
pub mod shell_vim;
pub mod shell_zsh;
pub mod stdlib;

use std::io::Write;
use std::sync::OnceLock;

use crate::cmd::env::Env;
use crate::cmd::log::{setup_logging, CLEAR_COLOR, ERROR_COLOR};
use crate::Result;

static BASH_PATH: OnceLock<String> = OnceLock::new();
static STDLIB: OnceLock<String> = OnceLock::new();
static VERSION: OnceLock<String> = OnceLock::new();

/// The bash path baked in at build time, if any.
pub fn bash_path() -> &'static str {
    BASH_PATH.get().map(String::as_str).unwrap_or("")
}

/// The embedded `stdlib.sh`.
pub fn stdlib_source() -> &'static str {
    STDLIB.get().map(String::as_str).unwrap_or("")
}

/// The contents of `version.txt`.
pub fn version() -> &'static str {
    VERSION.get().map(String::as_str).unwrap_or("")
}

/// The main entrypoint to direnv.
pub fn main(
    env: &Env,
    args: &[String],
    mod_bash_path: &str,
    mod_stdlib: &str,
    mod_version: &str,
) -> Result<()> {
    // We drop $PWD from caller since it can include symlinks, which will
    // break relative path access when finding .envrc or .env in a parent.
    std::env::remove_var("PWD");

    setup_logging(env);
    let _ = BASH_PATH.set(mod_bash_path.to_string());
    let _ = STDLIB.set(mod_stdlib.to_string());
    let _ = VERSION.set(mod_version.to_string());

    let result = commands::commands_dispatch(env, args);
    if let Err(err) = &result {
        let stderr = std::io::stderr();
        let mut handle = stderr.lock();
        let _ = writeln!(handle, "{ERROR_COLOR}direnv: error {err}{CLEAR_COLOR}");
    }
    result
}
