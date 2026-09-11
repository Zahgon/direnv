use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::deps::gojson::{self, JsonValue};
use crate::Result;

/// Adds support for the elvish shell.
pub struct Elvish;

impl Shell for Elvish {
    fn hook(&self) -> Result<String> {
        Ok(r#"## hook for direnv
set @edit:before-readline = $@edit:before-readline {
	try {
		var m = [("{{.SelfPath}}" export elvish | from-json)]
		if (> (count $m) 0) {
			set m = (all $m)
			keys $m | each { |k|
				if $m[$k] {
					set-env $k $m[$k]
				} else {
					unset-env $k
				}
			}
		}
	} catch e {
		echo $e
	}
}
"#
        .to_string())
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        Ok(gojson::encode(&JsonValue::from_optional_string_map(&e.0)))
    }

    fn dump(&self, env: &Env) -> Result<String> {
        Ok(gojson::encode(&JsonValue::from_string_map(&env.0)))
    }
}
