//! direnv's configuration and state.

use std::collections::BTreeMap;

use regex::Regex;

use crate::cmd::consts::{DIRENV_BASH, DIRENV_CONFIG, DIRENV_DIFF, DIRENV_FILE, DIRENV_WATCHES};
use crate::cmd::env::Env;
use crate::cmd::env_diff::load_env_diff;
use crate::cmd::log::{log_error, DEFAULT_LOG_FORMAT};
use crate::cmd::look_path::look_path;
use crate::cmd::rc::{find_rc, rc_from_env, RC};
use crate::deps::{gopath, goregexp, gotime};
use crate::log_debug;
use crate::{errorf, xdg, Error, Result};

/// direnv's configuration and state.
#[derive(Debug, Clone)]
pub struct Config {
    pub env: Env,
    /// Current directory.
    pub work_dir: String,
    pub conf_dir: String,
    pub cache_dir: String,
    pub data_dir: String,
    pub self_path: String,
    pub bash_path: String,
    pub rc_file: String,
    pub toml_path: String,
    pub hide_env_diff: bool,
    pub disable_stdin: bool,
    pub strict_env: bool,
    pub load_dotenv: bool,
    pub log_format: String,
    pub log_filter: Option<Regex>,
    pub log_color: bool,
    /// Nanoseconds, matching Go's `time.Duration`.
    pub warn_timeout: i64,
    pub whitelist_prefix: Vec<String>,
    pub whitelist_exact: BTreeMap<String, bool>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            env: Env::new(),
            work_dir: String::new(),
            conf_dir: String::new(),
            cache_dir: String::new(),
            data_dir: String::new(),
            self_path: String::new(),
            bash_path: String::new(),
            rc_file: String::new(),
            toml_path: String::new(),
            hide_env_diff: false,
            disable_stdin: false,
            strict_env: false,
            load_dotenv: false,
            log_format: String::new(),
            log_filter: None,
            log_color: false,
            warn_timeout: 0,
            whitelist_prefix: Vec::new(),
            whitelist_exact: BTreeMap::new(),
        }
    }
}

/// The `[global]` section, which is also accepted at the top level for
/// backward compatibility.
#[derive(Debug, Default)]
struct TomlGlobal {
    bash_path: String,
    disable_stdin: bool,
    strict_env: bool,
    /// Deprecated, use `load_dotenv`.
    skip_dotenv: bool,
    load_dotenv: bool,
    warn_timeout: Option<i64>,
    hide_env_diff: bool,
    log_format: String,
    log_filter: String,
}

/// Expand a path string prefixed with `~/` to the current user's home
/// directory.
///
/// Example: if the current user is user1 with home directory in /home/user1,
/// then `~/project` -> `/home/user1/project`. It's useful to allow paths with
/// `~/`, so that direnv.toml can be reused via dotfiles repos across systems
/// with different standard home paths (compare Linux /home and macOS /Users).
fn expand_tilde_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(homedir) = std::env::var("HOME") {
            if !homedir.is_empty() {
                return gopath::join(&[&homedir, rest]);
            }
        }
    }
    path.to_string()
}

/// Opens up the direnv configuration from the `Env`.
pub fn load_config(env: &Env) -> Result<Config> {
    let mut config = Config {
        env: env.copy(),
        ..Config::default()
    };

    config.conf_dir = env.get_or_empty(DIRENV_CONFIG).to_string();
    if config.conf_dir.is_empty() {
        config.conf_dir = xdg::config_dir(&env.0, "direnv");
    }
    if config.conf_dir.is_empty() {
        return Err(Error::from(
            "couldn't find a configuration directory for direnv",
        ));
    }

    let exe_path = std::env::current_exe().map_err(|err| {
        errorf!(
            "LoadConfig() os.Executable() failed: {}",
            crate::deps::goerr::errno_text(&err)
        )
    })?;
    // Fix for mingsys
    config.self_path = exe_path.to_string_lossy().replace('\\', "/");

    match gopath::getwd() {
        Ok(wd) => config.work_dir = wd,
        // handled by `find_env_up`
        Err(_) => return Ok(config),
    }

    // Default Warn Timeout
    config.warn_timeout = 5 * gotime::SECOND;

    // Default log format
    config.log_format = DEFAULT_LOG_FORMAT.to_string();

    config.rc_file = env.get_or_empty(DIRENV_FILE).to_string();

    // Load the TOML config
    config.toml_path = gopath::join(&[&config.conf_dir, "direnv.toml"]);
    if std::fs::metadata(&config.toml_path).is_err() {
        config.toml_path = gopath::join(&[&config.conf_dir, "config.toml"]);
        if std::fs::metadata(&config.toml_path).is_err() {
            config.toml_path = String::new();
        }
    }

    if !config.toml_path.is_empty() {
        let (global, whitelist_prefix, whitelist_exact) = decode_toml(&config.toml_path)?;

        config.log_color = std::env::var("TERM").unwrap_or_default() != "dumb";

        if let Some(format) = env.get("DIRENV_LOG_FORMAT") {
            config.log_format = format.clone();
        } else if !global.log_format.is_empty() {
            let mut log_fmt = global.log_format.clone();
            if log_fmt == "-" {
                log_fmt = String::new();
            }
            config.log_format = log_fmt;
        }

        if !global.log_filter.is_empty() {
            let pattern = goregexp::compile_pattern(&global.log_filter)
                .map_err(|err| errorf!("error in log filter: {err}"))?;
            let filter_regexp =
                Regex::new(&pattern).map_err(|err| errorf!("error in log filter: {err}"))?;
            config.log_filter = Some(filter_regexp);
        }

        config.hide_env_diff = global.hide_env_diff;

        for path in &whitelist_prefix {
            config.whitelist_prefix.push(expand_tilde_path(path));
        }

        for path in &whitelist_exact {
            let mut path = path.clone();
            if !path.ends_with("/.envrc") && !path.ends_with("/.env") {
                path = gopath::join(&[&path, ".envrc"]);
            }
            config
                .whitelist_exact
                .insert(expand_tilde_path(&path), true);
        }

        if global.skip_dotenv {
            log_error(&config, "skip_dotenv has been inverted to load_dotenv.");
        }

        config.bash_path = global.bash_path.clone();
        config.disable_stdin = global.disable_stdin;
        config.load_dotenv = global.load_dotenv;
        config.strict_env = global.strict_env;
        if let Some(warn_timeout) = global.warn_timeout {
            config.warn_timeout = warn_timeout;
        }
    }

    let ts = env.fetch("DIRENV_WARN_TIMEOUT", "");
    if !ts.is_empty() {
        match gotime::parse_duration(&ts) {
            Ok(timeout) => config.warn_timeout = timeout,
            Err(err) => log_error(&config, &format!("invalid DIRENV_WARN_TIMEOUT: {err}")),
        }
    }

    if config.bash_path.is_empty() {
        if !env.get_or_empty(DIRENV_BASH).is_empty() {
            config.bash_path = env.get_or_empty(DIRENV_BASH).to_string();
        } else if !crate::cmd::bash_path().is_empty() {
            config.bash_path = crate::cmd::bash_path().to_string();
        } else {
            let path = std::env::var("PATH").unwrap_or_default();
            match look_path("bash", &path) {
                Ok(found) => config.bash_path = found,
                Err(err) => return Err(errorf!("can't find bash: {err}")),
            }
        }
    }

    if config.cache_dir.is_empty() {
        config.cache_dir = xdg::cache_dir(&env.0, "direnv");
    }
    if config.cache_dir.is_empty() {
        return Err(Error::from("couldn't find a cache directory for direnv"));
    }

    if config.data_dir.is_empty() {
        config.data_dir = xdg::data_dir(&env.0, "direnv");
    }
    if config.data_dir.is_empty() {
        return Err(Error::from("couldn't find a data directory for direnv"));
    }

    Ok(config)
}

/// Decode `direnv.toml`.
///
/// The original declares `global` once and shares it between the top-level and
/// the `[global]` key, so a setting is honoured in either position. Top-level
/// keys are applied first and `[global]` wins where both are present.
fn decode_toml(path: &str) -> Result<(TomlGlobal, Vec<String>, Vec<String>)> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        errorf!(
            "LoadConfig() failed to parse {path}: {}",
            crate::deps::goerr::path_error("open", path, &err)
        )
    })?;
    let document: toml::Value = toml::from_str(&text)
        .map_err(|err| errorf!("LoadConfig() failed to parse {path}: {err}"))?;

    let mut global = TomlGlobal::default();
    apply_global(&mut global, &document, path, &text, "")?;
    if let Some(section) = document.get("global") {
        apply_global(&mut global, section, path, &text, "global")?;
    }

    let mut prefix = Vec::new();
    let mut exact = Vec::new();
    if let Some(whitelist) = document.get("whitelist") {
        prefix = string_list(whitelist.get("prefix"));
        exact = string_list(whitelist.get("exact"));
    }

    Ok((global, prefix, exact))
}

fn apply_global(
    global: &mut TomlGlobal,
    table: &toml::Value,
    path: &str,
    text: &str,
    section: &str,
) -> Result<()> {
    if let Some(value) = table.get("bash_path").and_then(toml::Value::as_str) {
        global.bash_path = value.to_string();
    }
    if let Some(value) = table.get("disable_stdin").and_then(toml::Value::as_bool) {
        global.disable_stdin = value;
    }
    if let Some(value) = table.get("strict_env").and_then(toml::Value::as_bool) {
        global.strict_env = value;
    }
    if let Some(value) = table.get("skip_dotenv").and_then(toml::Value::as_bool) {
        global.skip_dotenv = value;
    }
    if let Some(value) = table.get("load_dotenv").and_then(toml::Value::as_bool) {
        global.load_dotenv = value;
    }
    if let Some(value) = table.get("hide_env_diff").and_then(toml::Value::as_bool) {
        global.hide_env_diff = value;
    }
    if let Some(value) = table.get("log_format").and_then(toml::Value::as_str) {
        global.log_format = value.to_string();
    }
    if let Some(value) = table.get("log_filter").and_then(toml::Value::as_str) {
        global.log_filter = value.to_string();
    }
    if let Some(value) = table.get("warn_timeout").and_then(toml::Value::as_str) {
        let duration = gotime::parse_duration(value).map_err(|err| {
            errorf!(
                "LoadConfig() failed to parse {path}: toml: line {} (last key \"{}warn_timeout\"): {err}",
                key_line(text, section, "warn_timeout"),
                if section.is_empty() {
                    String::new()
                } else {
                    format!("{section}.")
                }
            )
        })?;
        global.warn_timeout = Some(duration);
    }
    Ok(())
}

/// The 1-based line of `key` inside `section`, for the decoder's error report.
///
/// The TOML decoder reports which line a failing value came from; the Rust
/// crate surfaces that only for syntax errors, so the line is recovered from
/// the document text.
fn key_line(text: &str, section: &str, key: &str) -> usize {
    let mut current = String::new();
    for (index, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
            current = name.trim().to_string();
            continue;
        }
        if current == section {
            let head = trimmed.split('=').next().unwrap_or("").trim();
            if head == key {
                return index + 1;
            }
        }
    }
    1
}

fn string_list(value: Option<&toml::Value>) -> Vec<String> {
    match value.and_then(toml::Value::as_array) {
        Some(items) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        None => Vec::new(),
    }
}

impl Config {
    /// The folder where all the "allow" files are stored.
    pub fn allow_dir(&self) -> String {
        gopath::join(&[&self.data_dir, "allow"])
    }

    /// The folder where all the "deny" files are stored.
    pub fn deny_dir(&self) -> String {
        gopath::join(&[&self.data_dir, "deny"])
    }

    /// The folder where all the "allowed required" files are stored.
    pub fn allowed_required_dir(&self) -> String {
        gopath::join(&[&self.data_dir, "allowed-required"])
    }

    /// Returns an RC file if any has been loaded.
    pub fn loaded_rc(&self) -> Option<RC> {
        if self.env.get_or_empty(DIRENV_FILE).is_empty() {
            log_debug!("RCFile is blank - loadedRC is nil");
            return None;
        }
        let rc_path = self.env.get_or_empty(DIRENV_FILE).to_string();
        let times_string = self.env.get_or_empty(DIRENV_WATCHES).to_string();

        rc_from_env(&rc_path, &times_string, self)
    }

    /// Loads an RC from a specified path and returns the new environment.
    ///
    /// Both halves are returned because `RC::load` produces an environment even
    /// when it fails; see [`RC::load`].
    pub fn env_from_rc_pair(&self, path: &str, previous_env: &Env) -> (Option<Env>, Option<Error>) {
        match RC::from_path(path, self) {
            Err(err) => (None, Some(err)),
            Ok(rc) => {
                let (env, err) = rc.load(previous_env);
                (Some(env), err)
            }
        }
    }

    /// Looks for an RC file in the config environment.
    pub fn find_rc(&self) -> Result<Option<RC>> {
        find_rc(&self.work_dir, self)
    }

    /// Undoes the recorded changes (if any) to the supplied environment,
    /// returning a new environment.
    pub fn revert(&self, env: &Env) -> Result<Env> {
        if self.env.get_or_empty(DIRENV_DIFF).is_empty() {
            return Ok(env.copy());
        }
        let diff = load_env_diff(self.env.get_or_empty(DIRENV_DIFF))?;
        Ok(diff.reverse().patch(env))
    }
}

/// Go's `%v` for a `[]string`.
pub fn format_string_slice(items: &[String]) -> String {
    format!("[{}]", items.join(" "))
}

/// Go's `%v` for a `map[string]bool`, whose keys `fmt` prints sorted.
pub fn format_string_bool_map(map: &BTreeMap<String, bool>) -> String {
    let rendered: Vec<String> = map
        .iter()
        .map(|(key, value)| format!("{key}:{value}"))
        .collect();
    format!("map[{}]", rendered.join(" "))
}
