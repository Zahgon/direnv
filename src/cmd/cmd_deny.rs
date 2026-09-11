//! `direnv block [PATH_TO_RC]`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::env::Env;
use crate::cmd::rc::{find_rc, path_hash};
use crate::deps::{goerr, gopath};
use crate::{errorf, Error, Result};

pub fn cmd_deny() -> Cmd {
    Cmd {
        name: "block",
        desc: "Revokes the authorization of a given .envrc or .env file.".to_string(),
        args: &["[PATH_TO_RC]"],
        aliases: &["deny", "disallow", "revoke"],
        private: false,
        action: Action::WithConfig(cmd_deny_action),
    }
}

fn cmd_deny_action(_env: &Env, args: &[String], config: &Config) -> Result<()> {
    let rc_path: String = if args.len() > 1 {
        let absolute = gopath::abs(&args[1]).map_err(|err| Error(goerr::errno_text(&err)))?;
        gopath::eval_symlinks(&absolute)
            .map_err(|err| Error(goerr::path_error("lstat", &absolute, &err)))?
    } else {
        gopath::getwd().map_err(|err| Error(goerr::errno_text(&err)))?
    };

    let Some(rc) = find_rc(&rc_path, config)? else {
        if config.load_dotenv {
            return Err(Error::from(".envrc or .env file not found"));
        }
        return Err(Error::from(".envrc file not found"));
    };

    // Remove required files for this .envrc
    remove_allowed_required_files(rc.path(), config)?;

    rc.deny()
}

fn remove_allowed_required_files(rc_path: &str, config: &Config) -> Result<()> {
    let envrc_path_hash =
        path_hash(rc_path).map_err(|err| errorf!("failed to hash envrc path: {err}"))?;

    let allowed_required_dir = gopath::join(&[&config.allowed_required_dir(), &envrc_path_hash]);

    // Remove the entire allowed-required directory for this .envrc
    match std::fs::remove_dir_all(&allowed_required_dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(errorf!(
            "failed to remove allowed-required files: {}",
            goerr::path_error("unlinkat", &allowed_required_dir, &err)
        )),
    }
}
