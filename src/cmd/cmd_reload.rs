//! `direnv reload`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::rc::{not_allowed_message, AllowStatus};
use crate::Error;

pub fn cmd_reload() -> Cmd {
    Cmd {
        name: "reload",
        desc: "Triggers an env reload".to_string(),
        args: &[],
        aliases: &[],
        private: false,
        action: Action::WithConfig(|_env, _args, config| {
            let Some(found_rc) = config.find_rc()? else {
                return Err(Error::from(".envrc not found"));
            };

            if found_rc.allowed() == AllowStatus::Denied {
                return Err(Error(not_allowed_message(found_rc.path())));
            }

            found_rc.touch()
        }),
    }
}
