use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::cmd::shell_bash::bash_escape;
use crate::Result;

/// Adds support for the venerable Z shell.
pub struct Zsh;

const ZSH_HOOK: &str = r#"
_direnv_hook() {
  vars="$("{{.SelfPath}}" export zsh)"
  trap -- '' SIGINT
  eval "$vars"
  trap - SIGINT
}
typeset -ag precmd_functions
if (( ! ${precmd_functions[(I)_direnv_hook]} )); then
  precmd_functions=(_direnv_hook $precmd_functions)
fi
typeset -ag chpwd_functions
if (( ! ${chpwd_functions[(I)_direnv_hook]} )); then
  chpwd_functions=(_direnv_hook $chpwd_functions)
fi
"#;

impl Shell for Zsh {
    fn hook(&self) -> Result<String> {
        Ok(ZSH_HOOK.to_string())
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
    format!("export {}={};", bash_escape(key), bash_escape(value))
}

fn unset(key: &str) -> String {
    format!("unset {};", bash_escape(key))
}
