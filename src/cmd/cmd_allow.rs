//! `direnv allow [PATH_TO_RC]`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::consts::DIRENV_REQUIRED;
use crate::cmd::env::Env;
use crate::cmd::rc::{file_hash, find_rc, path_hash, write_file};
use crate::deps::{goerr, gopath};
use crate::{errorf, Error, Result};

pub fn cmd_allow() -> Cmd {
    Cmd {
        name: "allow",
        desc: "Grants direnv permission to load the given .envrc or .env file.".to_string(),
        args: &["[PATH_TO_RC]"],
        aliases: &["permit", "grant"],
        private: false,
        action: Action::WithConfig(cmd_allow_action),
    }
}

const MIGRATION_MESSAGE: &str = r#"
Migrating the allow data to the new location

The allowed .envrc or .env permissions used to be stored in the XDG_CONFIG_HOME. It's
better to keep that folder for user-editable configuration so the data is
being moved to XDG_DATA_HOME.
"#;

fn cmd_allow_action(env: &Env, args: &[String], config: &Config) -> Result<()> {
    let rc_path: String = if args.len() > 1 {
        let absolute = gopath::abs(&args[1]).map_err(|err| Error(goerr::errno_text(&err)))?;
        gopath::eval_symlinks(&absolute)
            .map_err(|err| Error(goerr::path_error("lstat", &absolute, &err)))?
    } else {
        gopath::getwd().map_err(|err| Error(goerr::errno_text(&err)))?
    };

    if std::fs::metadata(config.allow_dir()).is_err() {
        let old_allow_dir = gopath::join(&[&config.conf_dir, "allow"]);
        if std::fs::metadata(&old_allow_dir).is_ok() {
            println!("{MIGRATION_MESSAGE}");

            println!("moving {} to {}", old_allow_dir, config.allow_dir());
            let parent = gopath::dir(&config.allow_dir());
            std::fs::create_dir_all(&parent)
                .map_err(|err| Error(goerr::path_error("mkdir", &parent, &err)))?;

            std::fs::rename(&old_allow_dir, config.allow_dir()).map_err(|err| {
                errorf!(
                    "rename {} {}: {}",
                    old_allow_dir,
                    config.allow_dir(),
                    goerr::errno_text(&err)
                )
            })?;

            println!(
                "creating a symlink back from {} to {} for back-compat.",
                config.allow_dir(),
                old_allow_dir
            );
            std::os::unix::fs::symlink(config.allow_dir(), &old_allow_dir).map_err(|err| {
                errorf!(
                    "symlink {} {}: {}",
                    config.allow_dir(),
                    old_allow_dir,
                    goerr::errno_text(&err)
                )
            })?;
            println!();
            println!("All done, have a nice day!");
        }
    }

    let Some(mut rc) = find_rc(&rc_path, config)? else {
        if config.load_dotenv {
            return Err(Error::from(".envrc or .env file not found"));
        }
        return Err(Error::from(".envrc file not found"));
    };

    rc.allow()?;

    // Handle required files if DIRENV_REQUIRED is set
    let required_paths = env.get_or_empty(DIRENV_REQUIRED).to_string();
    if !required_paths.is_empty() {
        allow_required_files(rc.path(), &required_paths, config)?;
    }

    Ok(())
}

fn allow_required_files(rc_path: &str, required_paths: &str, config: &Config) -> Result<()> {
    let rc_dir = gopath::dir(rc_path);

    let envrc_path_hash =
        path_hash(rc_path).map_err(|err| errorf!("failed to hash envrc path: {err}"))?;

    let allowed_required_dir = gopath::join(&[&config.allowed_required_dir(), &envrc_path_hash]);

    std::fs::create_dir_all(&allowed_required_dir).map_err(|err| {
        errorf!(
            "failed to create allowed-required directory: {}",
            goerr::path_error("mkdir", &allowed_required_dir, &err)
        )
    })?;

    for rel_path in required_paths.split(':') {
        let abs_path = gopath::join(&[&rc_dir, rel_path]);

        let hash = match file_hash(&abs_path) {
            Ok(hash) => hash,
            Err(err) => {
                if err.0.ends_with("no such file or directory") {
                    return Err(errorf!("required file does not exist: {rel_path}"));
                }
                return Err(errorf!("failed to hash required file {rel_path}: {err}"));
            }
        };

        let allowed_required_file = gopath::join(&[&allowed_required_dir, &hash]);
        write_file(&allowed_required_file, &format!("{rel_path}\n"), 0o644)
            .map_err(|err| errorf!("failed to write allowed-required file entry: {err}"))?;

        println!("direnv: allowing {rel_path}");
    }

    Ok(())
}
