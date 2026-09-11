//! `direnv exec DIR <COMMAND> ...`

use std::os::unix::process::CommandExt;
use std::process::Command;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::env::Env;
use crate::cmd::look_path::look_path;
use crate::cmd::rc::find_env_up;
use crate::deps::{goerr, gopath};
use crate::{errorf, Error, Result};

pub fn cmd_exec() -> Cmd {
    Cmd {
        name: "exec",
        desc: "Executes a command after loading the first .envrc or .env found in DIR".to_string(),
        args: &["DIR", "COMMAND", "[...ARGS]"],
        aliases: &[],
        private: false,
        action: Action::WithConfig(cmd_exec_action),
    }
}

fn cmd_exec_action(env: &Env, args: &[String], config: &Config) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from("missing DIR and COMMAND arguments"));
    }

    let mut rc_path = gopath::clean(&args[1]);
    let fi = std::fs::metadata(&rc_path)
        .map_err(|err| Error(goerr::path_error("stat", &rc_path, &err)))?;

    let command: String;
    let argv: Vec<String>;
    if fi.is_dir() {
        if args.len() < 3 {
            return Err(Error::from("missing COMMAND argument"));
        }
        command = args[2].clone();
        argv = args[2..].to_vec();
    } else {
        command = rc_path.clone();
        rc_path = gopath::dir(&rc_path);
        argv = args[1..].to_vec();
    }

    // Restore pristine environment if needed
    let mut previous_env = config.revert(env)?;
    previous_env.clean_context();

    // Load the rc
    let new_env: Env;
    let to_load = find_env_up(&rc_path, config.load_dotenv);
    if !to_load.is_empty() {
        let (loaded, err) = config.env_from_rc_pair(&to_load, &previous_env);
        if let Some(err) = err {
            return Err(err);
        }
        match loaded {
            Some(loaded) => new_env = loaded,
            None => return Ok(()),
        }
    } else {
        new_env = previous_env;
    }

    let command_path = look_path(&command, new_env.get_or_empty("PATH")).map_err(|_| {
        errorf!(
            "command '{command}' not found on PATH '{}'",
            new_env.get_or_empty("PATH")
        )
    })?;

    let mut process = Command::new(&command_path);
    process.arg0(&argv[0]).args(&argv[1..]).env_clear();
    for entry in new_env.to_go_env() {
        if let Some((key, value)) = entry.split_once('=') {
            process.env(key, value);
        }
    }
    // `exec` only returns on failure.
    let err = process.exec();
    Err(Error(goerr::errno_text(&err)))
}
