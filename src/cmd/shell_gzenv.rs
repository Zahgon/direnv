use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::deps::gojson::JsonValue;
use crate::{gzenv, Error, Result};

/// Not a real shell; used for internal purposes.
pub struct GzEnvShell;

impl Shell for GzEnvShell {
    fn hook(&self) -> Result<String> {
        Err(Error::from("the gzenv shell doesn't support hooking"))
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        Ok(gzenv::marshal(&JsonValue::from_optional_string_map(&e.0)))
    }

    fn dump(&self, env: &Env) -> Result<String> {
        Ok(gzenv::marshal(&JsonValue::from_string_map(&env.0)))
    }
}
