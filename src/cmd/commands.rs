//! The sub-command table and the dispatcher.

use std::sync::mpsc;
use std::time::Duration;

use crate::cmd::config::{load_config, Config};
use crate::cmd::env::Env;
use crate::cmd::log::log_error_args;
use crate::cmd::printf::Arg;
use crate::cmd::*;
use crate::{errorf, Result};

/// A sub-command's implementation.
///
/// The three variants stand for the original's `actionSimple`,
/// `actionWithConfig` and the `cmdWithWarnTimeout` wrapper.
pub enum Action {
    Simple(fn(&Env, &[String]) -> Result<()>),
    WithConfig(fn(&Env, &[String], &Config) -> Result<()>),
    WithConfigAndWarnTimeout(fn(&Env, &[String], &Config) -> Result<()>),
}

impl Action {
    fn call(&self, env: &Env, args: &[String]) -> Result<()> {
        match self {
            Action::Simple(f) => f(env, args),
            Action::WithConfig(f) => {
                let config = load_config(env)?;
                f(env, args, &config)
            }
            Action::WithConfigAndWarnTimeout(f) => {
                let config = load_config(env)?;
                with_warn_timeout(*f, env, args, &config)
            }
        }
    }
}

/// Warn on stderr when a command takes longer than `warn_timeout`.
fn with_warn_timeout(
    f: fn(&Env, &[String], &Config) -> Result<()>,
    env: &Env,
    args: &[String],
    config: &Config,
) -> Result<()> {
    // Disable warning if WarnTimeout is <= 0
    if config.warn_timeout <= 0 {
        return f(env, args, config);
    }

    let (done_tx, done_rx) = mpsc::channel::<()>();
    let timeout = Duration::from_nanos(config.warn_timeout as u64);
    let warn_config = config.clone();
    let warn_args: Vec<String> = args.to_vec();
    let watcher = std::thread::spawn(move || {
        if done_rx.recv_timeout(timeout).is_err() {
            log_error_args(
                &warn_config,
                "(%v) is taking a while to execute. Use CTRL-C to give up.",
                &[Arg::List(warn_args)],
            );
        }
    });

    let err = f(env, args, config);
    let _ = done_tx.send(());
    let _ = watcher.join();
    err
}

/// A direnv sub-command.
pub struct Cmd {
    pub name: &'static str,
    pub desc: String,
    pub args: &'static [&'static str],
    pub aliases: &'static [&'static str],
    pub private: bool,
    pub action: Action,
}

/// The list of all direnv sub-commands, in `direnv help` order.
pub fn cmd_list() -> Vec<Cmd> {
    vec![
        cmd_allow::cmd_allow(),
        cmd_apply_dump::cmd_apply_dump(),
        cmd_show_dump::cmd_show_dump(),
        cmd_check_required::cmd_check_required(),
        cmd_deny::cmd_deny(),
        cmd_dotenv::cmd_dotenv(),
        cmd_dump::cmd_dump(),
        cmd_edit::cmd_edit(),
        cmd_exec::cmd_exec(),
        cmd_export::cmd_export(),
        cmd_fetchurl::cmd_fetchurl(),
        cmd_help::cmd_help(),
        cmd_hook::cmd_hook(),
        cmd_prune::cmd_prune(),
        cmd_reload::cmd_reload(),
        cmd_status::cmd_status(),
        cmd_stdlib::cmd_stdlib(),
        cmd_version::cmd_version(),
        cmd_watch::cmd_watch(),
        cmd_watch_dir::cmd_watch_dir(),
        cmd_watch_list::cmd_watch_list(),
        cmd_watch_print::cmd_watch_print(),
        cmd_current::cmd_current(),
        cmd_log::cmd_log(),
    ]
}

/// Called by `main` to dispatch to a sub-command.
pub fn commands_dispatch(env: &Env, args: &[String]) -> Result<()> {
    let command_name: String;
    let command_prefix: String;
    let command_args: Vec<String>;

    if args.len() < 2 {
        command_name = "help".to_string();
        command_prefix = args[0].clone();
        command_args = Vec::new();
    } else {
        command_name = args[1].clone();
        command_prefix = args[0..2].join(" ");
        let mut collected = vec![command_prefix.clone()];
        collected.extend_from_slice(&args[2..]);
        command_args = collected;
    }

    let list = cmd_list();
    let mut command: Option<&Cmd> = None;
    for cmd in &list {
        if cmd.name == command_name {
            command = Some(cmd);
            break;
        }
        for alias in cmd.aliases {
            if *alias == command_name {
                command = Some(cmd);
            }
        }
    }

    let Some(command) = command else {
        return Err(errorf!("command \"{command_prefix}\" not found"));
    };

    command.action.call(env, &command_args)
}
