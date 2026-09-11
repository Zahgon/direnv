//! `direnv watch-dir SHELL PATH`

use std::io::Write;
use std::os::unix::fs::MetadataExt;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::consts::DIRENV_WATCHES;
use crate::cmd::env::Env;
use crate::cmd::file_times::FileTimes;
use crate::cmd::shell::{detect_shell, ShellExport};
use crate::deps::gopath;
use crate::{errorf, Error, Result};

pub fn cmd_watch_dir() -> Cmd {
    Cmd {
        name: "watch-dir",
        desc: "Recursively adds a directory to the list that direnv watches for changes"
            .to_string(),
        args: &["SHELL", "DIR"],
        aliases: &[],
        private: true,
        action: Action::Simple(watch_dir_command),
    }
}

fn watch_dir_command(env: &Env, args: &[String]) -> Result<()> {
    if args.len() < 3 {
        return Err(Error::from(
            "a directory is required to add to the list of watches",
        ));
    }

    let shell_name = args[1].clone();
    let dir = args[2].clone();

    let Some(shell) = detect_shell(&shell_name) else {
        return Err(errorf!("unknown target shell '{shell_name}'"));
    };

    if std::fs::metadata(&dir).is_err() {
        return Err(errorf!("dir '{dir}' does not exist"));
    }

    let mut watches = FileTimes::new();
    if let Some(watch_string) = env.get(DIRENV_WATCHES) {
        watches.unmarshal(watch_string)?;
    }

    let mut walk_err: Option<Error> = None;
    let result = gopath::walk(&dir, &mut |path, info| {
        if let Err(err) = watches.new_time(path, info.mtime(), true) {
            walk_err = Some(err);
            return Err(std::io::Error::other(""));
        }
        Ok(())
    });
    if let Some(err) = walk_err {
        return Err(errorf!("failed to recursively watch dir '{dir}': {err}"));
    }
    if let Err(err) = result {
        return Err(errorf!(
            "failed to recursively watch dir '{dir}': {}",
            crate::deps::goerr::errno_text(&err)
        ));
    }

    let mut e = ShellExport::new();
    e.add(DIRENV_WATCHES, &watches.marshal());

    let export_str = shell.export(&e)?;
    let _ = std::io::stdout().write_all(export_str.as_bytes());

    Ok(())
}
