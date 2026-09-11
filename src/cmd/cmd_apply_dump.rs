//! `direnv apply_dump FILE`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::env::{load_env, Env};
use crate::cmd::shell::BASH;
use crate::deps::goerr;
use crate::{Error, Result};

pub fn cmd_apply_dump() -> Cmd {
    Cmd {
        name: "apply_dump",
        desc: "Accepts a filename containing `direnv dump` output and generates a series of bash export statements to apply the given env".to_string(),
        args: &["FILE"],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_apply_dump_action),
    }
}

fn cmd_apply_dump_action(env: &Env, args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from("not enough arguments"));
    }

    if args.len() > 2 {
        return Err(Error::from("too many arguments"));
    }
    let filename = &args[1];

    let dumped = std::fs::read_to_string(filename)
        .map_err(|err| Error(goerr::path_error("open", filename, &err)))?;

    let dumped_env = load_env(&dumped)?;

    let diff = env.diff(&dumped_env);

    let exports = diff.to_shell(&BASH)?;

    println!("{exports}");

    Ok(())
}
