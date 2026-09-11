use crate::cmd::env::Env;
use crate::cmd::shell::{Shell, ShellExport};
use crate::cmd::shell_bash::{
    CLOSE_BRACKET, CLOSE_CURLY_BRACE, COLON, EQUALS, OPEN_BRACKET, OPEN_CURLY_BRACE, QUESTION,
    SINGLE_QUOTE, STAR,
};
use crate::Result;

/// The PowerShell shell instance.
pub struct Pwsh;

const PWSH_HOOK: &str = r#"using namespace System;
using namespace System.Management.Automation;

if ($PSVersionTable.PSVersion.Major -lt 7 -or ($PSVersionTable.PSVersion.Major -eq 7 -and $PSVersionTable.PSVersion.Minor -lt 2)) {
    throw "direnv: PowerShell version $($PSVersionTable.PSVersion) does not meet the minimum required version 7.2!"
}

$hook = [EventHandler[LocationChangedEventArgs]] {
  param([object] $source, [LocationChangedEventArgs] $eventArgs)
  end {
    $export = ({{.SelfPath}} export pwsh) -join [Environment]::NewLine;
    if ($export) {
      Invoke-Expression -Command $export;
    }
  }
};
$currentAction = $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction;
if ($currentAction) {
  $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction = [Delegate]::Combine($currentAction, $hook);
}
else {
  $ExecutionContext.SessionState.InvokeCommand.LocationChangedAction = $hook;
};

"#;

impl Shell for Pwsh {
    fn hook(&self) -> Result<String> {
        Ok(PWSH_HOOK.to_string())
    }

    fn export(&self, e: &ShellExport) -> Result<String> {
        let mut out = String::new();
        for (key, value) in e.iter() {
            if !key.is_empty() {
                match value {
                    None => out.push_str(&unset(key)),
                    Some(value) => out.push_str(&export(key, value)),
                }
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
    format!(
        "${{env:{}}}='{}';",
        powershell_escape_env_key(key),
        powershell_escape_verbatim_string(value)
    )
}

fn unset(key: &str) -> String {
    format!(
        "Remove-Item -LiteralPath 'env:/{}';",
        powershell_escape_verbatim_env_key(key)
    )
}

/// Escapes environment variable keys for PowerShell.
pub fn powershell_escape_env_key(str: &str) -> String {
    if str.is_empty() {
        return "__DiReNv_UnReAcHaBlE__".to_string();
    }
    // Byte-wise like the original: a byte that is not special is passed
    // through untouched, so a multi-byte UTF-8 sequence survives intact.
    let mut out: Vec<u8> = Vec::with_capacity(str.len());
    for &char in str.as_bytes() {
        match char {
            STAR | COLON | EQUALS | QUESTION | OPEN_BRACKET | CLOSE_BRACKET => {
                out.extend_from_slice(format!("\\x{char:02x}").as_bytes())
            }
            OPEN_CURLY_BRACE => out.extend_from_slice(b"`{"),
            CLOSE_CURLY_BRACE => out.extend_from_slice(b"`}"),
            _ => out.push(char),
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Escapes environment variable keys using verbatim strings for PowerShell.
pub fn powershell_escape_verbatim_env_key(str: &str) -> String {
    if str.is_empty() {
        return "__DiReNv_UnReAcHaBlE__".to_string();
    }
    double_single_quotes(str)
}

/// Escapes strings using verbatim string literals for PowerShell.
pub fn powershell_escape_verbatim_string(str: &str) -> String {
    if str.is_empty() {
        return String::new();
    }
    double_single_quotes(str)
}

fn double_single_quotes(str: &str) -> String {
    let mut out: Vec<u8> = Vec::with_capacity(str.len());
    for &char in str.as_bytes() {
        if char == SINGLE_QUOTE {
            out.extend_from_slice(b"''");
        } else {
            out.push(char);
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/*
   1. Minimal handling required for verbatim strings:
   Characters in a verbatim string (e.g.: 'a single quoted string') don't require escaping
   except for the single quote character itself which is escaped by doubling it
   (i.e.: '''' -eq "'" -and ''''.Length -eq 1).

   2. Handling any exported newline or carriage return characters from the PowerShell hook:
   Newline or carriage return characters in any part of the output of `direnv export pwsh`
   will produce an array of strings when imported into PowerShell. To join all parts of the
   array into a single string, the following is done in the PowerShell hook:

   `(direnv export pwsh) -join [Environment]::NewLine`

   3. Allowing PowerShell variable names with special characters:
   PowerShell environment variable names may contain "special characters" when enclosed in
   curly braces like this:

   ${env:name-with-special-chars-like-dashes} = 'value'

   The following special characters may NOT be used in such names: *, ?, :, =, [, ]
   These invalid special characters are mapped to hex codes (e.g.: "*" -> "\x2A").

   Curly braces may be used, if escaped with a backtick: `{, `}

   For more info on Pwsh variable names that include special characters see:
   https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_variables#variable-names-that-include-special-characters

   4. Paranoid handling of paths when removing environment variables:
   Use `Remove-Item -LiteralPath <PATH>` instead of `Remove-Item -Path <PATH>` to avoid any
   potential wildcard interpretations.

   5. Paranoid handling of potentially empty key names:
   I'm not sure if `Remove-Item -LiteralPath 'env:'` or `${env:} = 'value'` could be abused so two
   overlapping steps are taken to avoid this issue:
     a. Empty key names are skipped.
     b. Empty key names are replaced with "__DiReNv_UnReAcHaBlE__".
*/
