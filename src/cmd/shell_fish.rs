use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::cmd::shell_bash::{BACKSLASH, CR, DEL, LF, SINGLE_QUOTE, TAB, TILDE, US};
use crate::Result;

/// Adds support for the fish shell as a host.
pub struct Fish;

const FISH_HOOK: &str = r#"
    function __direnv_export_eval --on-event fish_prompt;
        "{{.SelfPath}}" export fish | source;

        if test "$direnv_fish_mode" != "disable_arrow";
            function __direnv_cd_hook --on-variable PWD;
                if test "$direnv_fish_mode" = "eval_after_arrow";
                    set -g __direnv_export_again 0;
                else;
                    "{{.SelfPath}}" export fish | source;
                end;
            end;
        end;
    end;

    function __direnv_export_eval_2 --on-event fish_preexec;
        if set -q __direnv_export_again;
            set -e __direnv_export_again;
            "{{.SelfPath}}" export fish | source;
            echo;
        end;

        functions --erase __direnv_cd_hook;
    end;
"#;

impl Shell for Fish {
    fn hook(&self) -> Result<String> {
        Ok(FISH_HOOK.to_string())
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
    if key == "PATH" {
        let mut command = String::from("set -x -g PATH");
        for path in value.split(':') {
            command.push(' ');
            command.push_str(&escape(path));
        }
        command.push(';');
        return command;
    }
    format!("set -x -g {} {};", escape(key), escape(value))
}

fn unset(key: &str) -> String {
    format!("set -e -g {};", escape(key))
}

fn escape(str: &str) -> String {
    let input = str.as_bytes();
    let mut out = String::from("'");

    for &char in input {
        match char {
            TAB => out.push_str("'\\t'"),
            LF => out.push_str("'\\n'"),
            CR => out.push_str("'\\r'"),
            c if c <= US => out.push_str(&format!("'\\X{c:02x}'")),
            SINGLE_QUOTE => {
                out.push(BACKSLASH as char);
                out.push(SINGLE_QUOTE as char);
            }
            BACKSLASH => {
                out.push(BACKSLASH as char);
                out.push(BACKSLASH as char);
            }
            c if c <= TILDE => out.push(c as char),
            DEL => out.push_str(&format!("'\\X{DEL:02x}'")),
            c => out.push_str(&format!("'\\X{c:02x}'")),
        }
    }

    out.push('\'');
    out
}
