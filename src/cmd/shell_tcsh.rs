use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::cmd::shell_bash::{
    ACK, AMPERSTAND, BACKSLASH, BACKTICK, CLOSE_BRACKET, CR, DEL, LF, LOWERCASE_Z, NINE,
    OPEN_BRACKET, PLUS, QUESTION, SINGLE_QUOTE, SPACE, TAB, TILDE, UNDERSCORE, UPPERCASE_Z, US,
};
use crate::Result;

/// Adds support for the tickle shell.
pub struct Tcsh;

impl Shell for Tcsh {
    fn hook(&self) -> Result<String> {
        Ok("alias precmd 'eval `{{.SelfPath}} export tcsh`'".to_string())
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
        let mut command = String::from("set path = (");
        for path in value.split(':') {
            command.push(' ');
            command.push_str(&escape(path));
        }
        command.push_str(" );");
        return command;
    }
    format!("setenv {} {} ;", escape(key), escape(value))
}

fn unset(key: &str) -> String {
    format!("unsetenv {} ;", escape(key))
}

/// tcsh's own escaping machine.
///
/// The `<= LOWERCASE_Z` branch sits *before* `<= CLOSE_BRACKET` and
/// `<= BACKTICK`, so bytes 93, 94 and 96..122 are emitted literally and only
/// 123..126 reach the quoting branch. That ordering is the original's and is
/// reproduced rather than tidied.
fn escape(str: &str) -> String {
    if str.is_empty() {
        return "''".to_string();
    }
    let input = str.as_bytes();
    let mut out = String::new();

    for &char in input {
        match char {
            ACK => out.push_str(&format!("\\x{char:02x}")),
            TAB => out.push_str("\\t"),
            LF => out.push_str("\\n"),
            CR => out.push_str("\\r"),
            SPACE => {
                out.push(BACKSLASH as char);
                out.push(SPACE as char);
            }
            c if c <= US => out.push_str(&format!("\\x{c:02x}")),
            c if c <= AMPERSTAND => out.push_str(&format!("\"{}\"", c as char)),
            SINGLE_QUOTE => {
                out.push(BACKSLASH as char);
                out.push(SINGLE_QUOTE as char);
            }
            c if c <= PLUS => out.push_str(&format!("\"{}\"", c as char)),
            c if c <= NINE => out.push(c as char),
            c if c <= QUESTION => out.push_str(&format!("\"{}\"", c as char)),
            c if c <= UPPERCASE_Z => out.push(c as char),
            OPEN_BRACKET => out.push_str(&format!("\"{}\"", OPEN_BRACKET as char)),
            BACKSLASH => {
                out.push(BACKSLASH as char);
                out.push(BACKSLASH as char);
            }
            UNDERSCORE => out.push(UNDERSCORE as char),
            c if c <= LOWERCASE_Z => out.push(c as char),
            c if c <= CLOSE_BRACKET => out.push_str(&format!("\"{}\"", c as char)),
            c if c <= BACKTICK => out.push_str(&format!("\"{}\"", c as char)),
            c if c <= TILDE => out.push_str(&format!("\"{}\"", c as char)),
            DEL => out.push_str(&format!("\\x{DEL:02x}")),
            c => out.push_str(&format!("\\x{c:02x}")),
        }
    }

    out
}
