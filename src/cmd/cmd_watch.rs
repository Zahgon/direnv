//! `direnv watch SHELL [PATH...]`

use std::io::Write;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::consts::DIRENV_WATCHES;
use crate::cmd::env::Env;
use crate::cmd::file_times::FileTimes;
use crate::cmd::shell::{detect_shell, ShellExport};
use crate::{errorf, Error, Result};

pub fn cmd_watch() -> Cmd {
    Cmd {
        name: "watch",
        desc: "Adds a path to the list that direnv watches for changes".to_string(),
        args: &["SHELL", "PATH..."],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_watch_action),
    }
}

fn cmd_watch_action(env: &Env, args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from(
            "a path is required to add to the list of watches",
        ));
    }
    let shell_name = args[1].clone();

    let Some(shell) = detect_shell(&shell_name) else {
        return Err(errorf!("unknown target shell '{shell_name}'"));
    };

    let mut watches = FileTimes::new();
    if let Some(watch_string) = env.get(DIRENV_WATCHES) {
        watches.unmarshal(watch_string)?;
    }

    for arg in &args[2..] {
        watches.update(arg)?;
    }

    let mut e = ShellExport::new();
    e.add(DIRENV_WATCHES, &watches.marshal());

    let export_str = shell.export(&e)?;
    let _ = std::io::stdout().write_all(export_str.as_bytes());

    Ok(())
}
