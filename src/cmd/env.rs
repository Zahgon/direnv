//! A map representation of environment variables.

use std::collections::HashMap;

use crate::cmd::consts::{
    DIRENV_DIFF, DIRENV_DIR, DIRENV_DUMP_FILE_PATH, DIRENV_FILE, DIRENV_WATCHES,
};
use crate::cmd::env_diff::{build_env_diff, EnvDiff};
use crate::cmd::shell::{Shell, ShellExport};
use crate::deps::gojson::{self, JsonValue};
use crate::{gzenv, Result};

/// A map of environment variable names to values.
///
/// NOTE: We don't support having two variables with the same name. I've never
/// seen it used in the wild but according to POSIX it's allowed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Env(pub HashMap<String, String>);

impl Env {
    /// An empty environment.
    pub fn new() -> Env {
        Env(HashMap::new())
    }

    /// Turns the classic unix environment variables into a map of key->values
    /// which is more handy to work with.
    pub fn from_process() -> Env {
        let mut env = HashMap::new();
        for (key, value) in std::env::vars_os() {
            let key = key.to_string_lossy().into_owned();
            let value = value.to_string_lossy().into_owned();
            env.insert(key, value);
        }
        Env(env)
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    /// The value of `key`, or `""` — Go's zero value for a missing map entry.
    pub fn get_or_empty(&self, key: &str) -> &str {
        self.0.get(key).map(String::as_str).unwrap_or("")
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    pub fn insert(&mut self, key: &str, value: &str) {
        self.0.insert(key.to_string(), value.to_string());
    }

    pub fn remove(&mut self, key: &str) {
        self.0.remove(key);
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, String> {
        self.0.iter()
    }

    /// Removes all the direnv-related environment variables. Call this after
    /// reverting the environment, otherwise direnv will just be amnesic about
    /// the previously-loaded environment.
    pub fn clean_context(&mut self) {
        self.0.remove(DIRENV_DIFF);
        self.0.remove(DIRENV_DIR);
        self.0.remove(DIRENV_FILE);
        self.0.remove(DIRENV_DUMP_FILE_PATH);
        self.0.remove(DIRENV_WATCHES);
    }

    /// Returns a fresh copy of the env. Because the env is a map under the
    /// hood, we want to get a copy whenever we mutate it and want to keep the
    /// original around.
    pub fn copy(&self) -> Env {
        Env(self.0.clone())
    }

    /// Should really be named `to_unix_env`. It turns the env back into a list
    /// of "key=value" strings like the ones returned by `os.Environ()`.
    pub fn to_go_env(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect()
    }

    /// Outputs the environment into an evaluatable string that is understood
    /// by the target shell.
    pub fn to_shell(&self, shell: &dyn Shell) -> Result<String> {
        let mut export = ShellExport::new();
        for (key, value) in &self.0 {
            export.add(key, value);
        }
        shell.export(&export)
    }

    /// Marshals the env into the gzenv format.
    pub fn serialize(&self) -> String {
        gzenv::marshal(&JsonValue::from_string_map(&self.0))
    }

    /// Returns the diff between the current env and the passed env.
    pub fn diff(&self, other: &Env) -> EnvDiff {
        build_env_diff(self, other)
    }

    /// Tries to get the value associated with the given `key`, or returns the
    /// provided default if none is set.
    ///
    /// Note that empty environment variables are considered to be set.
    pub fn fetch(&self, key: &str, default: &str) -> String {
        match self.0.get(key) {
            Some(value) => value.clone(),
            None => default.to_string(),
        }
    }
}

impl From<HashMap<String, String>> for Env {
    fn from(map: HashMap<String, String>) -> Env {
        Env(map)
    }
}

/// Unmarshals the env back from a gzenv string.
pub fn load_env(gzenv_str: &str) -> Result<Env> {
    let value = gzenv::unmarshal(gzenv_str)?;
    let map = gojson::decode_string_map(&value, "cmd.Env", "")
        .map_err(|err| crate::Error(format!("unmarshal() json parsing: {err}")))?;
    Ok(Env(map))
}

/// Unmarshals the env back from a JSON string.
pub fn load_env_json(json_bytes: &[u8]) -> Result<Env> {
    let text = String::from_utf8_lossy(json_bytes);
    let map = gojson::parse_string_map(&text)?;
    Ok(Env(map))
}
