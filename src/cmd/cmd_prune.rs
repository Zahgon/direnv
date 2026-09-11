//! `direnv prune`

use std::collections::HashMap;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::env::Env;
use crate::cmd::rc::{file_exists, file_hash, path_hash};
use crate::deps::{goerr, gopath};
use crate::{Error, Result};

pub fn cmd_prune() -> Cmd {
    Cmd {
        name: "prune",
        desc: "Removes old allowed and required files".to_string(),
        args: &[],
        aliases: &[],
        private: false,
        action: Action::WithConfig(cmd_prune_action),
    }
}

fn cmd_prune_action(_env: &Env, _args: &[String], config: &Config) -> Result<()> {
    // Track valid envrc paths for pruning required directory
    let mut valid_envrcs: HashMap<String, String> = HashMap::new(); // pathHash -> envrcPath

    let allowed = config.allow_dir();
    let dir_list = read_dir_names(&allowed)?;

    for hash in dir_list {
        let filename = gopath::join(&[&allowed, &hash]);
        let fi = std::fs::metadata(&filename)
            .map_err(|err| Error(goerr::path_error("stat", &filename, &err)))?;

        if !fi.is_dir() {
            let envrc = std::fs::read_to_string(&filename)
                .map_err(|err| Error(goerr::path_error("open", &filename, &err)))?;
            let envrc_str = envrc.trim().to_string();

            // skip old files, w/o path inside
            if envrc_str.is_empty() {
                continue;
            }
            if !file_exists(&envrc_str) {
                let _ = std::fs::remove_file(&filename);
            } else {
                // remove outdated hashes
                let h = file_hash(&envrc_str)?;
                if h != hash {
                    let _ = std::fs::remove_file(&filename);
                } else {
                    // This envrc is still valid, track it
                    if let Ok(ph) = path_hash(&envrc_str) {
                        valid_envrcs.insert(ph, envrc_str.clone());
                    }
                }
            }
        }
    }

    // Prune orphaned and outdated allowed-required files
    prune_allowed_required_dir(config, &valid_envrcs)
}

fn prune_allowed_required_dir(
    config: &Config,
    valid_envrcs: &HashMap<String, String>,
) -> Result<()> {
    let allowed_required_dir = config.allowed_required_dir();
    let dir_list = match read_dir_names(&allowed_required_dir) {
        Ok(names) => names,
        Err(err) if err.0.ends_with("no such file or directory") => return Ok(()),
        Err(err) => return Err(err),
    };

    for envrc_path_hash in dir_list {
        let Some(envrc_path) = valid_envrcs.get(&envrc_path_hash) else {
            // Remove allowed-required directories that don't have a valid
            // allowed envrc
            let _ =
                std::fs::remove_dir_all(gopath::join(&[&allowed_required_dir, &envrc_path_hash]));
            continue;
        };

        // Prune outdated allowed-required files within valid directories
        let envrc_dir = gopath::dir(envrc_path);
        let subdir = gopath::join(&[&allowed_required_dir, &envrc_path_hash]);
        prune_allowed_required_files(&subdir, &envrc_dir)?;
    }

    Ok(())
}

fn prune_allowed_required_files(allowed_required_subdir: &str, envrc_dir: &str) -> Result<()> {
    let files = match read_dir_names(allowed_required_subdir) {
        Ok(names) => names,
        Err(err) if err.0.ends_with("no such file or directory") => return Ok(()),
        Err(err) => return Err(err),
    };

    for hash in files {
        let filename = gopath::join(&[allowed_required_subdir, &hash]);
        let Ok(content) = std::fs::read_to_string(&filename) else {
            continue;
        };
        let rel_path = content.trim();

        let abs_path = gopath::join(&[envrc_dir, rel_path]);
        if !file_exists(&abs_path) {
            let _ = std::fs::remove_file(&filename);
        } else {
            // Check if hash is still valid
            match file_hash(&abs_path) {
                Ok(h) if h == hash => {}
                _ => {
                    let _ = std::fs::remove_file(&filename);
                }
            }
        }
    }

    Ok(())
}

/// Go's `os.File.Readdirnames(0)`.
fn read_dir_names(path: &str) -> Result<Vec<String>> {
    let entries =
        std::fs::read_dir(path).map_err(|err| Error(goerr::path_error("open", path, &err)))?;
    let mut names = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|err| Error(goerr::path_error("readdirent", path, &err)))?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}
