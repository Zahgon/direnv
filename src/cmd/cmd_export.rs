//! `direnv export $0`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::consts::DIRENV_REQUIRED;
use crate::cmd::env::Env;
use crate::cmd::env_diff::EnvDiff;
use crate::cmd::log::{log_status, log_status_args, PrefixGuard};
use crate::cmd::printf::Arg;
use crate::cmd::rc::find_env_up;
use crate::cmd::shell::{detect_shell, supported_shell_list};
use crate::log_debug;
use crate::{errorf, Result};

fn supported_shell_formatted_string() -> String {
    let mut res = String::from("[");
    for k in supported_shell_list().keys() {
        res.push_str(k);
        res.push_str(", ");
    }
    if let Some(stripped) = res.strip_suffix(", ") {
        res = stripped.to_string();
    }
    res.push(']');
    res
}

pub fn cmd_export() -> Cmd {
    Cmd {
        name: "export",
        desc: format!(
            "Loads an .envrc or .env and prints the diff in terms of exports.\n  Supported SHELL values are: {}",
            supported_shell_formatted_string()
        ),
        args: &["SHELL"],
        aliases: &[],
        private: false,
        action: Action::WithConfigAndWarnTimeout(export_command),
    }
}

fn export_command(current_env: &Env, args: &[String], config: &Config) -> Result<()> {
    let _prefix = PrefixGuard::push("export:");
    log_debug!("start");

    let target = if args.len() > 1 {
        args[1].clone()
    } else {
        String::new()
    };

    let Some(shell) = detect_shell(&target) else {
        return Err(errorf!("unknown target shell '{target}'"));
    };

    log_debug!("loading RCs");
    let loaded_rc = config.loaded_rc();
    let to_load = find_env_up(&config.work_dir, config.load_dotenv);

    if loaded_rc.is_none() && to_load.is_empty() {
        return Ok(());
    }

    log_debug!("updating RC");
    let _update_prefix = PrefixGuard::push("update:");

    log_debug!("Determining action:");
    log_debug!("toLoad: {:?}", to_load);
    log_debug!("loadedRC: {:?}", loaded_rc);

    if to_load.is_empty() {
        log_debug!("no RC found, unloading");
    } else if loaded_rc.is_none() {
        log_debug!("no RC (implies no DIRENV_DIFF),loading");
    } else if loaded_rc.as_ref().map(|rc| rc.path()) != Some(to_load.as_str()) {
        log_debug!("new RC, loading");
    } else if loaded_rc
        .as_ref()
        .map(|rc| rc.times().check().is_err())
        .unwrap_or(false)
    {
        log_debug!("file changed, reloading");
    } else if !current_env.get_or_empty(DIRENV_REQUIRED).is_empty() {
        // Force reload if required files were pending approval.
        // The approval status might have changed even if file times haven't.
        log_debug!("required files pending, reloading");
    } else {
        log_debug!("no update needed");
        return Ok(());
    }

    let previous_env = config.revert(current_env).map_err(|err| {
        let wrapped = errorf!("Revert() failed: {err}");
        log_debug!("err: {}", wrapped);
        wrapped
    })?;

    let new_env: Env;
    let mut load_error = None;
    if to_load.is_empty() {
        log_status(config, "unloading");
        let mut unloaded = previous_env.copy();
        unloaded.clean_context();
        new_env = unloaded;
    } else {
        let (loaded, err) = config.env_from_rc_pair(&to_load, &previous_env);
        if let Some(err) = err {
            log_debug!("err: {}", err);
            // If loading fails, fall through and deliver a diff anyway,
            // but still exit with an error.  This prevents retrying on
            // every prompt.
            load_error = Some(err);
        }
        match loaded {
            Some(loaded) => new_env = loaded,
            // unless of course, the error was in hashing and timestamp loading,
            // in which case we have to abort because we don't know what timestamp
            // to put in the diff!
            None => return load_error.map_or(Ok(()), Err),
        }
    }

    let out = diff_status(&previous_env.diff(&new_env));
    if !out.is_empty() && !config.hide_env_diff {
        log_status_args(config, "export %s", &[Arg::Str(out)]);
    }

    let diff_string = current_env
        .diff(&new_env)
        .to_shell(shell)
        .map_err(|err| errorf!("ToShell() failed: {err}"))?;
    log_debug!("env diff {}", diff_string);
    print!("{diff_string}");

    load_error.map_or(Ok(()), Err)
}

/// Return a string of +/-/~ indicators of an environment diff.
fn diff_status(old_diff: &EnvDiff) -> String {
    if old_diff.any() {
        let mut out: Vec<String> = Vec::new();
        for key in old_diff.prev.keys() {
            if !old_diff.next.contains_key(key) && !direnv_key(key) {
                out.push(format!("-{key}"));
            }
        }

        for key in old_diff.next.keys() {
            if direnv_key(key) {
                continue;
            }
            if old_diff.prev.contains_key(key) {
                out.push(format!("~{key}"));
            } else {
                out.push(format!("+{key}"));
            }
        }

        out.sort();
        return out.join(" ");
    }
    String::new()
}

fn direnv_key(key: &str) -> bool {
    key.starts_with("DIRENV_")
}
