//! `direnv dump`

use std::io::Write;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::consts::DIRENV_DUMP_FILE_PATH;
use crate::cmd::env::Env;
use crate::cmd::shell::detect_shell;
use crate::deps::goerr;
use crate::{errorf, Error, Result};

pub fn cmd_dump() -> Cmd {
    Cmd {
        name: "dump",
        desc: "Used to export the inner bash state at the end of execution".to_string(),
        args: &["[SHELL]", "[FILE]"],
        aliases: &[],
        private: true,
        action: Action::Simple(cmd_dump_action),
    }
}

fn cmd_dump_action(env: &Env, args: &[String]) -> Result<()> {
    let mut target = "gzenv".to_string();

    if args.len() > 1 {
        target = args[1].clone();
    }

    let file_path = if args.len() > 2 {
        args[2].clone()
    } else {
        std::env::var(DIRENV_DUMP_FILE_PATH).unwrap_or_default()
    };

    let mut writer: Box<dyn Write> = Box::new(std::io::stdout());
    if !file_path.is_empty() {
        writer = match file_path.parse::<i32>() {
            // A bare number names an already-open file descriptor.
            Ok(num) => {
                use std::os::unix::io::FromRawFd;
                // SAFETY: the caller passed this descriptor deliberately; the
                // original does the same with os.NewFile.
                Box::new(unsafe { std::fs::File::from_raw_fd(num) })
            }
            Err(_) => Box::new(
                std::fs::OpenOptions::new()
                    .write(true)
                    .open(&file_path)
                    .map_err(|err| Error(goerr::path_error("open", &file_path, &err)))?,
            ),
        };
    }

    let Some(shell) = detect_shell(&target) else {
        return Err(errorf!("unknown target shell '{target}'"));
    };

    let dump_str = shell.dump(env)?;
    writeln!(writer, "{dump_str}").map_err(|err| Error(goerr::errno_text(&err)))?;

    Ok(())
}
