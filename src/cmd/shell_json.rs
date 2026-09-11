use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::deps::gojson::{self, JsonValue};
use crate::{Error, Result};

/// Not really a shell but it fits. Useful to add support to editors and other
/// external tools that understand JSON as a format.
pub struct JsonShell;

impl Shell for JsonShell {
    fn hook(&self) -> Result<String> {
        Err(Error::from("this feature is not supported"))
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        Ok(gojson::marshal_indent(
            &JsonValue::from_optional_string_map(&e.0),
            "  ",
        ))
    }

    fn dump(&self, env: &Env) -> Result<String> {
        Ok(gojson::marshal_indent(
            &JsonValue::from_string_map(&env.0),
            "  ",
        ))
    }
}
