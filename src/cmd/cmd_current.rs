//! `direnv current PATH`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::consts::DIRENV_WATCHES;
use crate::cmd::env::Env;
use crate::cmd::file_times::FileTimes;
use crate::{Error, Result};

pub fn cmd_current() -> Cmd {
    Cmd {
        name: "current",
        desc: "Reports whether direnv's view of a file is current (or stale)".to_string(),
        args: &["PATH"],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_current_action),
    }
}

fn cmd_current_action(env: &Env, args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from("missing PATH argument"));
    }

    let path = &args[1];
    let mut watches = FileTimes::new();
    if let Some(watch_string) = env.get(DIRENV_WATCHES) {
        watches.unmarshal(watch_string)?;
    }

    watches.check_one(path)
}
