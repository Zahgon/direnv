//! `direnv version`

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::env::Env;
use crate::deps::gosemver;
use crate::{errorf, Result};

pub fn cmd_version() -> Cmd {
    Cmd {
        name: "version",
        desc: "prints the version or checks that direnv is older than VERSION_AT_LEAST."
            .to_string(),
        args: &["[VERSION_AT_LEAST]"],
        aliases: &["--version"],
        private: false,
        action: Action::Simple(cmd_version_action),
    }
}

fn cmd_version_action(_env: &Env, args: &[String]) -> Result<()> {
    let version = crate::cmd::version();
    let sem_version = ensure_v_prefixed(version);
    if args.len() > 1 {
        let at_least = ensure_v_prefixed(&args[1]);
        if !gosemver::is_valid(&at_least) {
            return Err(errorf!("{at_least} is not a valid semver version"));
        }
        let cmp = gosemver::compare(&sem_version, &at_least);
        if cmp < 0 {
            return Err(errorf!(
                "current version {sem_version} is older than the desired version {at_least}"
            ));
        }
    } else {
        println!("{version}");
    }
    Ok(())
}

/// Prefix a version with `v` unless it already has one.
///
/// Public because the original's test drives it directly.
pub fn ensure_v_prefixed(version: &str) -> String {
    if !version.starts_with('v') {
        return format!("v{version}");
    }
    version.to_string()
}
