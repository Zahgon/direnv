//! `direnv watch-list`

use std::io::{BufRead, Write};

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::consts::DIRENV_WATCHES;
use crate::cmd::env::Env;
use crate::cmd::file_times::FileTimes;
use crate::cmd::shell::{detect_shell, ShellExport};
use crate::{errorf, Result};

pub fn cmd_watch_list() -> Cmd {
    Cmd {
        name: "watch-list",
        desc: "Pipe pairs of `mtime path` to stdin to build a list of files to watch.".to_string(),
        args: &["[SHELL]"],
        aliases: &[],
        private: true,
        action: Action::Simple(watch_list_command),
    }
}

fn watch_list_command(env: &Env, args: &[String]) -> Result<()> {
    let shell_name = if args.len() >= 2 {
        args[1].clone()
    } else {
        "bash".to_string()
    };

    let Some(shell) = detect_shell(&shell_name) else {
        return Err(errorf!("unknown target shell '{shell_name}'"));
    };

    let mut watches = FileTimes::new();
    if let Some(watch_string) = env.get(DIRENV_WATCHES) {
        watches.unmarshal(watch_string)?;
    }

    // Read `mtime path` lines from stdin
    let stdin = std::io::stdin();
    let mut reader = stdin.lock();

    let mut i = 1usize;
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|err| errorf!("line {i}: {}", crate::deps::goerr::errno_text(&err)))?;
        if read == 0 || !line.ends_with('\n') {
            // Go's ReadString only yields a complete line without an error;
            // anything else ends the loop at EOF.
            break;
        }
        let Some((mtime_text, rest)) = line.split_once(' ') else {
            return Err(errorf!("line {i}: expected to contain two elements"));
        };
        let mtime: i64 = mtime_text.parse().map_err(|_| {
            errorf!("line {i}: strconv.Atoi: parsing {mtime_text:?}: invalid syntax")
        })?;
        let path = &rest[..rest.len() - 1];

        // add to watches
        watches.new_time(path, mtime, true)?;
        i += 1;
    }

    let mut e = ShellExport::new();
    e.add(DIRENV_WATCHES, &watches.marshal());

    let export_str = shell.export(&e)?;
    let _ = std::io::stdout().write_all(export_str.as_bytes());

    Ok(())
}
