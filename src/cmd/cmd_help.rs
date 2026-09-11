//! `direnv help`

use crate::cmd::commands::{cmd_list, Action, Cmd};
use crate::cmd::env::Env;
use crate::Result;

pub fn cmd_help() -> Cmd {
    Cmd {
        name: "help",
        desc: "Shows this help".to_string(),
        args: &["[SHOW_PRIVATE]"],
        aliases: &["--help"],
        private: false,
        action: Action::Simple(cmd_help_action),
    }
}

fn cmd_help_action(_env: &Env, args: &[String]) -> Result<()> {
    let show_private = args.len() > 1;
    print!(
        "direnv v{}\nUsage: direnv COMMAND [...ARGS]\n\nAvailable commands\n------------------\n",
        crate::cmd::version()
    );
    for cmd in cmd_list() {
        let opts = if !cmd.args.is_empty() {
            format!(" {}", cmd.args.join(" "))
        } else {
            String::new()
        };
        if cmd.private {
            if show_private {
                println!("*{}{}:\n  {}", cmd.name, opts, cmd.desc);
            }
        } else {
            println!("{}{}:", cmd.name, opts);
            let aliases: Vec<&str> = cmd
                .aliases
                .iter()
                .filter(|alias| !alias.starts_with('-'))
                .copied()
                .collect();
            if !aliases.is_empty() {
                println!("  aliases: {}", aliases.join(", "));
            }
            println!("  {}", cmd.desc);
        }
    }

    if show_private {
        println!("* = private commands");
    }
    Ok(())
}
