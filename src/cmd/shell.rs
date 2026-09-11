//! The interaction with the host shell.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::cmd::env::Env;
use crate::cmd::shell_bash::Bash;
use crate::cmd::shell_elvish::Elvish;
use crate::cmd::shell_fish::Fish;
use crate::cmd::shell_gha::GitHubActions;
use crate::cmd::shell_gzenv::GzEnvShell;
use crate::cmd::shell_json::JsonShell;
use crate::cmd::shell_murex::Murex;
use crate::cmd::shell_pwsh::Pwsh;
use crate::cmd::shell_systemd::SystemdShell;
use crate::cmd::shell_tcsh::Tcsh;
use crate::cmd::shell_vim::Vim;
use crate::cmd::shell_zsh::Zsh;
use crate::deps::gopath;
use crate::Result;

/// The interface that represents the interaction with the host shell.
pub trait Shell: Sync {
    /// The string that gets evaluated into the host shell config and sets
    /// direnv up as a prompt hook.
    fn hook(&self) -> Result<String>;

    /// Outputs the `ShellExport` as an evaluatable string on the host shell.
    fn export(&self, e: &ShellExport) -> Result<String>;

    /// Outputs an evaluatable string that sets the env in the host shell.
    fn dump(&self, env: &Env) -> Result<String>;
}

/// Environment variables to add and remove on the host shell.
///
/// A `None` value marks a removal, which is Go's `map[string]*string` with a
/// nil pointer.
#[derive(Debug, Clone, Default)]
pub struct ShellExport(pub HashMap<String, Option<String>>);

impl ShellExport {
    pub fn new() -> ShellExport {
        ShellExport(HashMap::new())
    }

    /// Represents the addition of a new environment variable.
    pub fn add(&mut self, key: &str, value: &str) {
        self.0.insert(key.to_string(), Some(value.to_string()));
    }

    /// Represents the removal of a given `key` environment variable.
    pub fn remove(&mut self, key: &str) {
        self.0.insert(key.to_string(), None);
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, Option<String>> {
        self.0.iter()
    }
}

pub static BASH: Bash = Bash;
pub static ELVISH: Elvish = Elvish;
pub static FISH: Fish = Fish;
pub static GITHUB_ACTIONS: GitHubActions = GitHubActions;
pub static GZENV: GzEnvShell = GzEnvShell;
pub static JSON: JsonShell = JsonShell;
pub static MUREX: Murex = Murex;
pub static PWSH: Pwsh = Pwsh;
pub static SYSTEMD: SystemdShell = SystemdShell;
pub static TCSH: Tcsh = Tcsh;
pub static VIM: Vim = Vim;
pub static ZSH: Zsh = Zsh;

/// The table of shells `direnv export`, `hook`, `dump` and friends dispatch on.
///
/// A `HashMap` on purpose: the original is a Go map, and `direnv export`'s help
/// text renders this table in whatever order the map iterates in.
pub fn supported_shell_list() -> &'static HashMap<&'static str, &'static dyn Shell> {
    static LIST: OnceLock<HashMap<&'static str, &'static dyn Shell>> = OnceLock::new();
    LIST.get_or_init(|| {
        let mut list: HashMap<&'static str, &'static dyn Shell> = HashMap::new();
        list.insert("bash", &BASH);
        list.insert("elvish", &ELVISH);
        list.insert("fish", &FISH);
        list.insert("gha", &GITHUB_ACTIONS);
        list.insert("gzenv", &GZENV);
        list.insert("json", &JSON);
        list.insert("murex", &MUREX);
        list.insert("tcsh", &TCSH);
        list.insert("vim", &VIM);
        list.insert("zsh", &ZSH);
        list.insert("pwsh", &PWSH);
        list.insert("systemd", &SYSTEMD);
        list
    })
}

/// Returns a `Shell` instance from the given target.
///
/// `target` is usually `$0` and can also be prefixed by `-`.
pub fn detect_shell(target: &str) -> Option<&'static dyn Shell> {
    let base = gopath::base(target);
    let name = match base.strip_prefix('-') {
        Some(rest) => rest.to_string(),
        None => base,
    };
    supported_shell_list().get(name.as_str()).copied()
}
