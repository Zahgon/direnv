use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::log_debug;
use crate::{Error, Result};

/// Not really a shell, but useful to add support for systemd's
/// [`EnvironmentFile`](https://0pointer.de/public/systemd-man/systemd.exec.html#EnvironmentFile=).
pub struct SystemdShell;

impl Shell for SystemdShell {
    fn hook(&self) -> Result<String> {
        Err(Error::from("this feature is not supported"))
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        let mut out = String::new();
        for (key, value) in e.iter() {
            if let Some(value) = value {
                out.push_str(&export(key, value));
            }
        }
        Ok(out)
    }

    fn dump(&self, env: &Env) -> Result<String> {
        let mut out = String::new();
        for (key, value) in env.iter() {
            out.push_str(&export(key, value));
        }
        Ok(out)
    }
}

/// Strips `encapsulating_value` from both ends of `value_to_test`, reporting
/// whether it was there.
pub fn cut_encapsulated<'a>(value_to_test: &'a str, encapsulating_value: &str) -> (&'a str, bool) {
    if let Some(without_prefix) = value_to_test.strip_prefix(encapsulating_value) {
        if let Some(without_prefix_and_suffix) = without_prefix.strip_suffix(encapsulating_value) {
            return (without_prefix_and_suffix, true);
        }
    }
    (value_to_test, false)
}

/// Quotes a value only when it contains a character systemd would misread.
pub fn sanitize_value(value: &str) -> String {
    let mut contain_special_char = false;
    let special_character_list = ["\n", "\\", "\"", "'"];
    for special_char in special_character_list {
        if value.contains(special_char) {
            contain_special_char = true;
        }
    }

    let mut sanitized_value = value.to_string();

    if contain_special_char {
        // Since the value contains special characters it needs to be quoted
        let (value_without_encapsulation, encapsulated_by_single_quotes) =
            cut_encapsulated(value, "'");

        if encapsulated_by_single_quotes {
            sanitized_value = format!("'{}'", value_without_encapsulation.replace('\'', "\\'"));
        } else {
            let (value_without_encapsulation, encapsulated_by_double_quotes) =
                cut_encapsulated(value, "\"");
            log_debug!(
                "encapsulated by double quotes : {}",
                encapsulated_by_double_quotes
            );
            sanitized_value = format!("\"{}\"", value_without_encapsulation.replace('"', "\\\""));
        }
    }
    // if the value doesn't contain special characters then we don't touch it
    sanitized_value
}

fn export(key: &str, value: &str) -> String {
    format!("{key}={}\n", sanitize_value(value))
}
