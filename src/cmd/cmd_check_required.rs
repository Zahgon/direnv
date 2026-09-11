//! `direnv check-required SHELL ENVRC_PATH PATH...`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::consts::DIRENV_REQUIRED;
use crate::cmd::env::Env;
use crate::cmd::rc::{file_hash, path_hash};
use crate::cmd::shell::{detect_shell, ShellExport};
use crate::cmd::shell_bash::bash_escape;
use crate::deps::{goerr, gopath};
use crate::{errorf, Error, Result};

pub fn cmd_check_required() -> Cmd {
    Cmd {
        name: "check-required",
        desc: "Checks if required files have been allowed".to_string(),
        args: &["SHELL", "ENVRC_PATH", "PATH..."],
        aliases: &[],
        private: true,
        action: Action::WithConfig(cmd_check_required_action),
    }
}

fn cmd_check_required_action(_env: &Env, args: &[String], config: &Config) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from("a shell name is required"));
    }
    if args.len() < 3 {
        return Err(Error::from("an envrc path is required"));
    }
    if args.len() < 4 {
        return Err(Error::from("at least one file path is required"));
    }

    let shell_name = args[1].clone();
    let Some(shell) = detect_shell(&shell_name) else {
        return Err(errorf!("unknown target shell '{shell_name}'"));
    };

    let envrc_path = gopath::abs(&args[2])
        .map_err(|err| errorf!("failed to resolve envrc path: {}", goerr::errno_text(&err)))?;
    let envrc_dir = gopath::dir(&envrc_path);
    let envrc_path_hash =
        path_hash(&envrc_path).map_err(|err| errorf!("failed to hash envrc path: {err}"))?;

    let allowed_required_dir = gopath::join(&[&config.allowed_required_dir(), &envrc_path_hash]);

    let mut missing_paths: Vec<String> = Vec::new();

    for rel_path in &args[3..] {
        // Security validation: must be relative
        if gopath::is_abs(rel_path) {
            println!(
                "log_error {};",
                bash_escape(&format!(
                    "require_allowed: path must be relative: {rel_path}"
                ))
            );
            println!("exit 1;");
            return Ok(());
        }

        // Security validation: no parent traversal
        if rel_path.contains("..") {
            println!(
                "log_error {};",
                bash_escape(&format!(
                    "require_allowed: path must not contain '..': {rel_path}"
                ))
            );
            println!("exit 1;");
            return Ok(());
        }

        let abs_path = gopath::join(&[&envrc_dir, rel_path]);
        let Ok(hash) = file_hash(&abs_path) else {
            // File might not exist or be unreadable
            missing_paths.push(rel_path.clone());
            continue;
        };

        // Check if the hash exists in the allowed-required directory
        let allowed_required_file = gopath::join(&[&allowed_required_dir, &hash]);
        match std::fs::metadata(&allowed_required_file) {
            Ok(_) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                missing_paths.push(rel_path.clone())
            }
            Err(err) => {
                return Err(errorf!(
                    "failed to check required file {rel_path}: {}",
                    goerr::path_error("stat", &allowed_required_file, &err)
                ))
            }
        }
    }

    if !missing_paths.is_empty() {
        // Export DIRENV_REQUIRED with the missing paths
        let mut e = ShellExport::new();
        e.add(DIRENV_REQUIRED, &missing_paths.join(":"));

        let export_str = shell.export(&e)?;
        print!("{export_str}");

        // Output error message
        let file_list = if missing_paths.len() == 1 {
            format!("{} requires", missing_paths[0])
        } else {
            format!("{} require", missing_paths.join(" and "))
        };
        println!(
            "log_error {};",
            bash_escape(&format!(
                "{file_list} approval. Run 'direnv allow' to approve."
            ))
        );
        println!("exit 0;");
    }

    Ok(())
}
