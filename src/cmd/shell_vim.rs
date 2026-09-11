use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::{Error, Result};

/// Adds support for vim. Not really a shell but it's handy.
pub struct Vim;

impl Shell for Vim {
    fn hook(&self) -> Result<String> {
        Err(Error::from(
            "this feature is not supported. Install the direnv.vim plugin instead",
        ))
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        let mut out = String::new();
        for (key, value) in e.iter() {
            match value {
                None => out.push_str(&unset(key)),
                Some(value) => out.push_str(&export(key, value)),
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

fn export(key: &str, value: &str) -> String {
    format!("call setenv({},{})\n", escape_key(key), escape_value(value))
}

fn unset(key: &str) -> String {
    format!("call setenv({},v:null)\n", escape_key(key))
}

// TODO: support keys with special chars or fail
fn escape_key(str: &str) -> String {
    escape_value(str)
}

// TODO: Make sure this escaping is valid
fn escape_value(str: &str) -> String {
    format!("'{}'", str.replace('\n', "\\n").replace('\'', "''"))
}
