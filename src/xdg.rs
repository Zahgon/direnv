//! A minimal implementation of the XDG specification.
//!
//! <https://standards.freedesktop.org/basedir-spec/basedir-spec-latest.html>

use std::collections::HashMap;

use crate::deps::gopath;

/// Returns the data folder for the application.
pub fn data_dir(env: &HashMap<String, String>, program_name: &str) -> String {
    dir_for(env, "XDG_DATA_HOME", &[".local", "share"], program_name)
}

/// Returns the config folder for the application.
///
/// The `XDG_CONFIG_DIRS` case is not being handled.
pub fn config_dir(env: &HashMap<String, String>, program_name: &str) -> String {
    dir_for(env, "XDG_CONFIG_HOME", &[".config"], program_name)
}

/// Returns the cache directory for the application.
pub fn cache_dir(env: &HashMap<String, String>, program_name: &str) -> String {
    dir_for(env, "XDG_CACHE_HOME", &[".cache"], program_name)
}

fn dir_for(
    env: &HashMap<String, String>,
    xdg_key: &str,
    home_suffix: &[&str],
    program_name: &str,
) -> String {
    if let Some(base) = env.get(xdg_key).filter(|v| !v.is_empty()) {
        return gopath::join(&[base, program_name]);
    }
    if let Some(home) = env.get("HOME").filter(|v| !v.is_empty()) {
        let mut parts: Vec<&str> = vec![home];
        parts.extend_from_slice(home_suffix);
        parts.push(program_name);
        return gopath::join(&parts);
    }
    // In theory we could also read /etc/passwd and look for the home based on
    // the process' UID.
    String::new()
}
