use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::Result;

pub struct Bash;

const BASH_HOOK: &str = r#"
_direnv_hook() {
  local previous_exit_status=$?;
  vars="$("{{.SelfPath}}" export bash)";
  trap -- '' SIGINT;
  eval "$vars";
  trap - SIGINT;
  return $previous_exit_status;
};
if [[ ";${PROMPT_COMMAND[*]:-};" != *";_direnv_hook;"* ]]; then
  if [[ "$(declare -p PROMPT_COMMAND 2>&1)" == "declare -a"* ]]; then
    PROMPT_COMMAND=(_direnv_hook "${PROMPT_COMMAND[@]}")
  else
    PROMPT_COMMAND="_direnv_hook${PROMPT_COMMAND:+;$PROMPT_COMMAND}"
  fi
fi
"#;

impl Shell for Bash {
    fn hook(&self) -> Result<String> {
        Ok(BASH_HOOK.to_string())
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

/*
 * Escaping
 */

pub(crate) const ACK: u8 = 6;
pub(crate) const TAB: u8 = 9;
pub(crate) const LF: u8 = 10;
pub(crate) const CR: u8 = 13;
pub(crate) const US: u8 = 31;
pub(crate) const SPACE: u8 = 32;
pub(crate) const AMPERSTAND: u8 = 38;
pub(crate) const SINGLE_QUOTE: u8 = 39;
pub(crate) const STAR: u8 = 42;
pub(crate) const PLUS: u8 = 43;
pub(crate) const NINE: u8 = 57;
pub(crate) const COLON: u8 = 58;
pub(crate) const EQUALS: u8 = 61;
pub(crate) const QUESTION: u8 = 63;
pub(crate) const UPPERCASE_Z: u8 = 90;
pub(crate) const OPEN_BRACKET: u8 = 91;
pub(crate) const BACKSLASH: u8 = 92;
pub(crate) const UNDERSCORE: u8 = 95;
pub(crate) const CLOSE_BRACKET: u8 = 93;
pub(crate) const BACKTICK: u8 = 96;
pub(crate) const LOWERCASE_Z: u8 = 122;
pub(crate) const OPEN_CURLY_BRACE: u8 = 123;
pub(crate) const CLOSE_CURLY_BRACE: u8 = 125;
pub(crate) const TILDE: u8 = 126;
pub(crate) const DEL: u8 = 127;

/// Escapes strings for safe use in Bash.
///
/// Based on <https://github.com/solidsnack/shell-escape/blob/master/Text/ShellEscape/Bash.hs>
///
/// A Bash escaped string. The strings are wrapped in `$'...'` if any bytes
/// within them must be escaped; otherwise, they are left as is. Newlines and
/// other control characters are represented as ANSI escape sequences. High
/// bytes are represented as hex codes. Thus Bash escaped strings will always
/// fit on one line and never contain non-ASCII bytes.
///
/// The branch order below overlaps and is significant; it is the order of the
/// original's switch.
pub fn bash_escape(str: &str) -> String {
    if str.is_empty() {
        return "''".to_string();
    }
    let input = str.as_bytes();
    let mut out = String::new();
    let mut escape = false;

    for &char in input {
        match char {
            ACK => {
                escape = true;
                out.push_str(&format!("\\x{char:02x}"));
            }
            TAB => {
                escape = true;
                out.push_str("\\t");
            }
            LF => {
                escape = true;
                out.push_str("\\n");
            }
            CR => {
                escape = true;
                out.push_str("\\r");
            }
            c if c <= US => {
                escape = true;
                out.push_str(&format!("\\x{c:02x}"));
            }
            c if c <= AMPERSTAND => {
                escape = true;
                out.push(c as char);
            }
            SINGLE_QUOTE => {
                escape = true;
                out.push(BACKSLASH as char);
                out.push(SINGLE_QUOTE as char);
            }
            c if c <= PLUS => {
                escape = true;
                out.push(c as char);
            }
            c if c <= NINE => out.push(c as char),
            c if c <= QUESTION => {
                escape = true;
                out.push(c as char);
            }
            c if c <= UPPERCASE_Z => out.push(c as char),
            OPEN_BRACKET => {
                escape = true;
                out.push(OPEN_BRACKET as char);
            }
            BACKSLASH => {
                escape = true;
                out.push(BACKSLASH as char);
                out.push(BACKSLASH as char);
            }
            UNDERSCORE => out.push(UNDERSCORE as char),
            c if c <= CLOSE_BRACKET => {
                escape = true;
                out.push(c as char);
            }
            c if c <= BACKTICK => {
                escape = true;
                out.push(c as char);
            }
            c if c <= TILDE => {
                escape = true;
                out.push(c as char);
            }
            DEL => {
                escape = true;
                out.push_str(&format!("\\x{DEL:02x}"));
            }
            c => {
                escape = true;
                out.push_str(&format!("\\x{c:02x}"));
            }
        }
    }

    if escape {
        out = format!("$'{out}'");
    }

    out
}
