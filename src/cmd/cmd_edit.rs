//! `direnv edit [PATH_TO_RC]`

use std::process::Command;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::env::Env;
use crate::cmd::file_times::FileTimes;
use crate::cmd::log::{log_error, PrefixGuard};
use crate::cmd::look_path::look_path;
use crate::cmd::rc::find_rc;
use crate::cmd::shell_bash::bash_escape;
use crate::deps::{goerr, gopath};
use crate::log_debug;
use crate::{Error, Result};

pub fn cmd_edit() -> Cmd {
    Cmd {
        name: "edit",
        desc: "Opens PATH_TO_RC or the current .envrc or .env into an $EDITOR and allow\n  the file to be loaded afterwards.".to_string(),
        args: &["[PATH_TO_RC]"],
        aliases: &[],
        private: false,
        action: Action::WithConfig(cmd_edit_action),
    }
}

/// A list of known editors and how to start them.
pub const EDITORS: &[&[&str]] = &[
    &["editor"],
    &["subl", "-w"],
    &["mate", "-w"],
    &["open", "-t", "-W"], // Opens with the default text editor on mac
    &["nano"],
    &["vim"],
    &["emacs"],
];

fn cmd_edit_action(env: &Env, args: &[String], config: &Config) -> Result<()> {
    let _prefix = PrefixGuard::push("cmd_edit: ");

    let found_rc = config.find_rc()?;
    let times: Option<FileTimes> = found_rc.as_ref().map(|rc| rc.times().clone());

    let rc_path: String = if args.len() > 1 {
        let mut candidate = args[1].clone();
        if let Ok(fi) = std::fs::metadata(&candidate) {
            if fi.is_dir() {
                candidate = gopath::join(&[&candidate, ".envrc"]);
            }
        }
        candidate
    } else {
        let Some(found_rc) = &found_rc else {
            return Err(Error::from(
                ".envrc or .env not found. Use `direnv edit .` to create a new .envrc in the current directory",
            ));
        };
        found_rc.path().to_string()
    };

    let mut editor = env.get_or_empty("EDITOR").to_string();
    if editor.is_empty() {
        log_error(config, "$EDITOR not found.");
        editor = detect_editor(env.get_or_empty("PATH"));
        if editor.is_empty() {
            return Err(Error::from("could not find a default editor in the PATH"));
        }
    }

    let run = format!("{editor} {}", bash_escape(&rc_path));

    let status = Command::new(&config.bash_path)
        .arg("-c")
        .arg(&run)
        .status()
        .map_err(|err| Error(goerr::errno_text(&err)))?;
    if !status.success() {
        return Err(Error(crate::cmd::rc::exit_error(&status)));
    }

    let found_rc = find_rc(&rc_path, config);
    log_debug!("foundRC: {:?}", found_rc);
    log_debug!("times: {:?}", times);
    if let Some(times) = &times {
        log_debug!("times.Check(): {:?}", times.check());
    }
    if let Some(mut found_rc) = found_rc? {
        if times.as_ref().map(|t| t.check().is_err()).unwrap_or(true) {
            return found_rc.allow();
        }
    }

    Ok(())
}

// Utils

fn detect_editor(pathenv: &str) -> String {
    for editor in EDITORS {
        if look_path(editor[0], pathenv).is_ok() {
            return editor.join(" ");
        }
    }
    String::new()
}
