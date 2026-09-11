use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::deps::gojson::{self, JsonValue};
use crate::Result;

/// The shell implementation for the Murex shell.
pub struct Murex;

const MUREX_HOOK: &str = r#"event: onPrompt direnv_hook=before {
	"{{.SelfPath}}" export murex -> set exports
	if { $exports != "" } {
		$exports -> :json: formap key value {
			if { is-null value } then {
				!export "$key"
			} else {
				$value -> export "$key"
			}
		}
	}
}"#;

impl Shell for Murex {
    fn hook(&self) -> Result<String> {
        Ok(MUREX_HOOK.to_string())
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        Ok(gojson::encode(&JsonValue::from_optional_string_map(&e.0)))
    }

    fn dump(&self, env: &Env) -> Result<String> {
        Ok(gojson::encode(&JsonValue::from_string_map(&env.0)))
    }
}
