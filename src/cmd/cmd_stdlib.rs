//! `direnv stdlib`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::stdlib::get_stdlib;

pub fn cmd_stdlib() -> Cmd {
    Cmd {
        name: "stdlib",
        desc: "Displays the stdlib available in the .envrc execution context".to_string(),
        args: &[],
        aliases: &[],
        private: false,
        action: Action::WithConfig(|_env, _args, config| {
            println!("{}", get_stdlib(config));
            Ok(())
        }),
    }
}
