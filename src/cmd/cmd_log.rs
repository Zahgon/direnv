//! `direnv log [--status | --error] <message>`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::env::Env;
use crate::cmd::log::{log_error, log_status};
use crate::{errorf, Error, Result};

pub fn cmd_log() -> Cmd {
    Cmd {
        name: "log",
        desc: "Logs a given message".to_string(),
        args: &["[--status | --error]", "<message>"],
        aliases: &[],
        private: false,
        action: Action::WithConfig(cmd_log_action),
    }
}

fn cmd_log_action(_env: &Env, args: &[String], c: &Config) -> Result<()> {
    if args.len() != 3 {
        return Err(Error::from("invalid arguments"));
    }
    let log_type = &args[1];
    let message = &args[2];
    match log_type.as_str() {
        "--status" | "-status" => log_status(c, message),
        "--error" | "-error" => log_error(c, message),
        other => return Err(errorf!("invalid log-type '{other}'")),
    }
    Ok(())
}
