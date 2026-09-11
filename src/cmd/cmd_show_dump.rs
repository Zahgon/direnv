//! `direnv show_dump`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::env::Env;
use crate::deps::gojson;
use crate::{gzenv, Error, Result};

pub fn cmd_show_dump() -> Cmd {
    Cmd {
        name: "show_dump",
        desc: "Show the data inside of a dump for debugging purposes".to_string(),
        args: &["DUMP"],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_show_dump_action),
    }
}

fn cmd_show_dump_action(_env: &Env, args: &[String]) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from("missing DUMP argument"));
    }

    let value = gzenv::unmarshal(&args[1])?;

    // `json.NewEncoder(os.Stdout).SetIndent("", "  ")` plus `Encode`, whose
    // trailing newline is part of the output.
    println!("{}", gojson::marshal_indent(&value, "  "));
    Ok(())
}
