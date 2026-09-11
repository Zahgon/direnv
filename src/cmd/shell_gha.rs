use std::sync::OnceLock;

use regex::Regex;

use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::{errorf, Error, Result};

/// The GitHub Actions `$GITHUB_ENV` writer.
pub struct GitHubActions;

fn valid_key_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^[A-Za-z_][A-Za-z0-9_]*$").expect("key pattern"))
}

impl Shell for GitHubActions {
    fn hook(&self) -> Result<String> {
        Err(Error::from("Hook not implemented for GitHub Actions shell"))
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        let mut b = String::new();
        for (key, value) in e.iter() {
            if !valid_key_pattern().is_match(key) {
                // Skip invalid environment variable keys
                eprintln!("direnv: Skipping invalid environment variable key: {key}");
                continue;
            }
            match value {
                None => unset(&mut b, key),
                Some(value) => export(&mut b, key, value)?,
            }
        }
        Ok(b)
    }

    fn dump(&self, env: &Env) -> Result<String> {
        let mut b = String::new();
        for (key, value) in env.iter() {
            if !valid_key_pattern().is_match(key) {
                // Skip invalid environment variable keys
                eprintln!("direnv: Skipping invalid environment variable key: {key}");
                continue;
            }
            export(&mut b, key, value)?;
        }
        Ok(b)
    }
}

fn export(b: &mut String, key: &str, value: &str) -> Result<()> {
    // Generate a random delimiter
    let mut delimiter = generate_delimiter();

    // Check if key or value contains delimiter (should be extremely rare)
    if key.contains(&delimiter) || value.contains(&delimiter) {
        // Log the collision and regenerate delimiter
        eprintln!("direnv: Delimiter collision detected for key {key}, regenerating delimiter");
        delimiter = generate_delimiter();

        // If still colliding (astronomically unlikely), error out
        if key.contains(&delimiter) || value.contains(&delimiter) {
            return Err(errorf!(
                "delimiter collision after regeneration for key {key}"
            ));
        }
    }

    b.push_str(key);
    b.push_str("<<");
    b.push_str(&delimiter);
    b.push('\n');
    b.push_str(value);
    b.push('\n');
    b.push_str(&delimiter);
    b.push('\n');
    Ok(())
}

fn unset(_b: &mut String, _key: &str) {
    // Don't do anything. > $GITHUB_ENV will overwrite the existing env.
}

fn generate_delimiter() -> String {
    // Generate random bytes for delimiter
    let mut random_bytes = [0u8; 16];
    if getrandom::getrandom(&mut random_bytes).is_err() {
        // Fallback to timestamp-based delimiter
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        return format!("ghadelimiter_{nanos}");
    }

    // Convert to hex string
    let hex: String = random_bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!("ghadelimiter_{hex}")
}
