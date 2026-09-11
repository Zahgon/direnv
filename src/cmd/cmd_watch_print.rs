//! `direnv watch-print`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::consts::DIRENV_WATCHES;
use crate::cmd::env::Env;
use crate::cmd::file_times::FileTimes;
use crate::Result;

pub fn cmd_watch_print() -> Cmd {
    Cmd {
        name: "watch-print",
        desc: "prints the watched paths".to_string(),
        args: &["[--null]"],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_watch_print_action),
    }
}

fn cmd_watch_print_action(env: &Env, args: &[String]) -> Result<()> {
    let mut watches = FileTimes::new();
    let separator = if args.len() > 1 && args[1] == "--null" {
        '\0'
    } else {
        '\n'
    };

    if let Some(watch_string) = env.get(DIRENV_WATCHES) {
        watches.unmarshal(watch_string)?;
    }

    for watch in &watches.list {
        print!("{}{}", watch.path, separator);
    }

    Ok(())
}
