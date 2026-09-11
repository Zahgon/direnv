//! `direnv hook $0`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::env::Env;
use crate::cmd::shell::detect_shell;
use crate::deps::goerr;
use crate::deps::gotemplate::{execute, HookContext};
use crate::{errorf, Error, Result};

pub fn cmd_hook() -> Cmd {
    Cmd {
        name: "hook",
        desc: "Used to setup the shell hook".to_string(),
        args: &["SHELL"],
        aliases: &[],
        private: false,
        action: Action::Simple(cmd_hook_action),
    }
}

fn cmd_hook_action(_env: &Env, args: &[String]) -> Result<()> {
    let target = if args.len() > 1 {
        args[1].clone()
    } else {
        String::new()
    };

    let self_path = std::env::current_exe()
        .map_err(|err| Error(goerr::errno_text(&err)))?
        .to_string_lossy()
        .into_owned();

    // Convert Windows path if needed
    let self_path = self_path.replace('\\', "/");
    let ctx = HookContext { self_path };

    let Some(shell) = detect_shell(&target) else {
        return Err(errorf!("unknown target shell '{target}'"));
    };

    let hook_str = shell.hook()?;

    let rendered = execute("hook", &hook_str, &ctx).map_err(Error)?;
    print!("{rendered}");

    Ok(())
}
