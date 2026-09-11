//! The difference between two environments, and its wire form.

use std::collections::HashMap;

use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::deps::gojson::{self, JsonValue};
use crate::{gzenv, Result};

/// List of keys we don't want to deal with.
pub fn ignored_keys() -> &'static [&'static str] {
    &[
        // direnv env config
        "DIRENV_CONFIG",
        "DIRENV_BASH",
        // should only be available inside of the .envrc or .env
        "DIRENV_IN_ENVRC",
        "COMP_WORDBREAKS", // Avoids segfaults in bash
        "PS1",             // PS1 should not be exported, fixes problem in bash
        // variables that should change freely
        "OLDPWD",
        "PWD",
        "SHELL",
        "SHELLOPTS",
        "SHLVL",
        "_",
    ]
}

/// The diff between two environments.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvDiff {
    /// Serialised as `"p"`.
    pub prev: HashMap<String, String>,
    /// Serialised as `"n"`.
    pub next: HashMap<String, String>,
}

impl EnvDiff {
    /// An empty constructor for `EnvDiff`.
    pub fn new() -> EnvDiff {
        EnvDiff {
            prev: HashMap::new(),
            next: HashMap::new(),
        }
    }

    /// Returns whether the diff contains any changes.
    pub fn any(&self) -> bool {
        !self.prev.is_empty() || !self.next.is_empty()
    }

    /// Applies the env diff as a set of commands that are understood by the
    /// target `shell`. The outputted string is then meant to be evaluated in
    /// the target shell.
    pub fn to_shell(&self, shell: &dyn Shell) -> Result<String> {
        let mut export = ShellExport::new();

        for key in self.prev.keys() {
            if !self.next.contains_key(key) {
                export.remove(key);
            }
        }

        for (key, value) in &self.next {
            export.add(key, value);
        }

        shell.export(&export)
    }

    /// Applies the diff to the given env and returns a new env with the
    /// changes applied.
    pub fn patch(&self, env: &Env) -> Env {
        let mut new_env = env.copy();

        for key in self.prev.keys() {
            new_env.remove(key);
        }

        for (key, value) in &self.next {
            new_env.insert(key, value);
        }

        new_env
    }

    /// Flips the diff so that it applies the other way around.
    pub fn reverse(&self) -> EnvDiff {
        EnvDiff {
            prev: self.next.clone(),
            next: self.prev.clone(),
        }
    }

    /// Marshals the environment diff to the gzenv format.
    pub fn serialize(&self) -> String {
        gzenv::marshal(&self.to_json())
    }

    /// The JSON shape of the Go struct: field order, not sorted.
    fn to_json(&self) -> JsonValue {
        JsonValue::Object(vec![
            ("p".to_string(), JsonValue::from_string_map(&self.prev)),
            ("n".to_string(), JsonValue::from_string_map(&self.next)),
        ])
    }
}

/// Analyses the changes between `e1` and `e2` and builds an `EnvDiff` out of it.
pub fn build_env_diff(e1: &Env, e2: &Env) -> EnvDiff {
    let mut diff = EnvDiff::new();

    for key in e1.0.keys() {
        if ignored_env(key) {
            continue;
        }
        if e2.get_or_empty(key) != e1.get_or_empty(key) || !e2.contains_key(key) {
            diff.prev
                .insert(key.clone(), e1.get_or_empty(key).to_string());
        }
    }

    for key in e2.0.keys() {
        if ignored_env(key) {
            continue;
        }
        if e2.get_or_empty(key) != e1.get_or_empty(key) || !e1.contains_key(key) {
            diff.next
                .insert(key.clone(), e2.get_or_empty(key).to_string());
        }
    }

    diff
}

/// Unmarshals a gzenv string back into an `EnvDiff`.
pub fn load_env_diff(gzenv_str: &str) -> Result<EnvDiff> {
    let value = gzenv::unmarshal(gzenv_str)?;
    let mut diff = EnvDiff::new();
    let wrap = |err: String| crate::Error(format!("unmarshal() json parsing: {err}"));
    match &value {
        // Go leaves the destination untouched for a JSON null.
        JsonValue::Null => {}
        JsonValue::Object(pairs) => {
            for (key, item) in pairs {
                match key.as_str() {
                    "p" => {
                        diff.prev = gojson::decode_string_map(item, "map[string]string", ".p")
                            .map_err(wrap)?
                    }
                    "n" => {
                        diff.next = gojson::decode_string_map(item, "map[string]string", ".n")
                            .map_err(wrap)?
                    }
                    _ => {}
                }
            }
        }
        other => return Err(wrap(gojson::type_error(other, "cmd.EnvDiff"))),
    }
    Ok(diff)
}

/// Returns true if the key should be ignored in environment diffs.
pub fn ignored_env(key: &str) -> bool {
    if key.starts_with("__fish") {
        return true;
    }
    if key.starts_with("BASH_FUNC_") {
        return true;
    }
    ignored_keys().contains(&key)
}
