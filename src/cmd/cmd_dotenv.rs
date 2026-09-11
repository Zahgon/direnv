//! `direnv dotenv [SHELL [PATH_TO_DOTENV]]`
//!
//! Transforms a .env file to evaluatable `export KEY=PAIR` statements.
//!
//! See: <https://github.com/bkeepers/dotenv> and <https://github.com/ddollar/foreman>

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::env::Env;
use crate::cmd::shell::{detect_shell, Shell, BASH};
use crate::deps::{goerr, gopath};
use crate::{dotenv, Error, Result};

pub fn cmd_dotenv() -> Cmd {
    Cmd {
        name: "dotenv",
        desc: "Transforms a .env file to evaluatable `export KEY=PAIR` statements".to_string(),
        args: &["[SHELL]", "[PATH_TO_DOTENV]"],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_dotenv_action),
    }
}

fn cmd_dotenv_action(_env: &Env, args: &[String]) -> Result<()> {
    let shell: Option<&dyn Shell> = if args.len() > 1 {
        detect_shell(&args[1])
    } else {
        Some(&BASH)
    };

    let mut target = if args.len() > 2 {
        args[2].clone()
    } else {
        String::new()
    };

    if target.is_empty() {
        target = ".env".to_string();
    }

    let data = std::fs::read_to_string(&target)
        .map_err(|err| Error(goerr::path_error("open", &target, &err)))?;

    // Set PWD env var to the directory the .env file resides in. This results
    // in the least amount of surprise, as a dotenv file is most often defined
    // in the same directory it's loaded from, so referring to PWD should match
    // the directory of the .env file.
    let path = gopath::abs(&target).map_err(|err| Error(goerr::errno_text(&err)))?;
    std::env::set_var("PWD", gopath::dir(&path));

    let newenv: Env = dotenv::parse(&data)?.into();

    // The original does not check for an unknown shell here: `DetectShell`
    // returns a nil interface and `ToShell` dereferences it, so
    // `direnv dotenv <unknown>` crashes. The crash is preserved rather than
    // quietly turned into an error; see truth.md for the exit-code difference.
    let Some(shell) = shell else {
        panic!("runtime error: invalid memory address or nil pointer dereference");
    };
    let str = newenv.to_shell(shell)?;
    println!("{str}");

    Ok(())
}
