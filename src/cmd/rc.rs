//! The `.envrc` or `.env` file: finding it, gating it, and loading it.

use std::fs::File;
use std::io::Read;
use std::process::{Command, Stdio};

use sha2::{Digest, Sha256};

use crate::cmd::config::Config;
use crate::cmd::consts::{DIRENV_DIFF, DIRENV_DIR, DIRENV_FILE, DIRENV_WATCHES};
use crate::cmd::env::{load_env_json, Env};
use crate::cmd::file_times::FileTimes;
use crate::cmd::shell_bash::bash_escape;
use crate::deps::goerr;
use crate::deps::gopath;
use crate::{errorf, Error, Result};

/// The `.envrc` or `.env` file.
#[derive(Debug, Clone)]
pub struct RC {
    pub(crate) path: String,
    pub(crate) allow_path: String,
    pub(crate) deny_path: String,
    pub(crate) times: FileTimes,
    config: Config,
}

/// The permission status of an RC file.
///
/// The numeric values are user-visible: `direnv status` prints them raw and
/// `direnv status --json` marshals them as numbers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowStatus {
    /// The RC file is permitted to load.
    Allowed = 0,
    /// The RC file has not been granted permission.
    NotAllowed = 1,
    /// The RC file has been explicitly denied.
    Denied = 2,
}

impl std::fmt::Display for AllowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as i32)
    }
}

/// The refusal message, `notAllowed` in the original.
pub fn not_allowed_message(path: &str) -> String {
    format!("{path} is blocked. Run `direnv allow` to approve its content")
}

/// Looks for ".envrc" and ".env" files up in the file hierarchy.
pub fn find_rc(wd: &str, config: &Config) -> Result<Option<RC>> {
    let rc_path = find_env_up(wd, config.load_dotenv);
    if rc_path.is_empty() {
        return Ok(None);
    }

    RC::from_path(&rc_path, config).map(Some)
}

impl RC {
    /// Inits the RC from a given path.
    pub fn from_path(path: &str, config: &Config) -> Result<RC> {
        let file_hash = file_hash(path)?;

        let allow_path = gopath::join(&[&config.allow_dir(), &file_hash]);

        let path_hash = path_hash(path)?;

        let deny_path = gopath::join(&[&config.deny_dir(), &path_hash]);

        let mut times = FileTimes::new();

        times.update(path)?;
        times.update(&allow_path)?;
        times.update(&deny_path)?;

        Ok(RC {
            path: path.to_string(),
            allow_path,
            deny_path,
            times,
            config: config.clone(),
        })
    }

    /// Grants the RC as allowed to load.
    pub fn allow(&mut self) -> Result<()> {
        if self.allow_path.is_empty() {
            return Err(Error::from("cannot allow empty path"));
        }
        let parent = gopath::dir(&self.allow_path);
        std::fs::create_dir_all(&parent)
            .map_err(|err| Error(goerr::path_error("mkdir", &parent, &err)))?;
        allow(&self.path, &self.allow_path)?;
        self.times.update(&self.allow_path)?;
        match std::fs::symlink_metadata(&self.deny_path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error(goerr::path_error("stat", &self.deny_path, &err))),
            Ok(_) => std::fs::remove_file(&self.deny_path)
                .map_err(|err| Error(goerr::path_error("remove", &self.deny_path, &err))),
        }
    }

    /// Revokes the permission of the RC file to load.
    pub fn deny(&self) -> Result<()> {
        let parent = gopath::dir(&self.deny_path);
        std::fs::create_dir_all(&parent)
            .map_err(|err| Error(goerr::path_error("mkdir", &parent, &err)))?;

        // These deny files are not private.
        write_file(&self.deny_path, &format!("{}\n", self.path), 0o644)?;

        match std::fs::metadata(&self.allow_path) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(Error(goerr::path_error("stat", &self.allow_path, &err))),
            Ok(_) => std::fs::remove_file(&self.allow_path)
                .map_err(|err| Error(goerr::path_error("remove", &self.allow_path, &err))),
        }
    }

    /// Checks if the RC file has been granted loading.
    pub fn allowed(&self) -> AllowStatus {
        if std::fs::metadata(&self.deny_path).is_ok() {
            return AllowStatus::Denied;
        }

        // happy path is if this envrc has been explicitly allowed, O(1)ish
        // common case
        if std::fs::metadata(&self.allow_path).is_ok() {
            return AllowStatus::Allowed;
        }

        // when whitelisting we want to be (path) absolutely sure we've not been
        // duped with a symlink
        let Ok(path) = gopath::abs(&self.path) else {
            // seems unlikely that we'd hit this, but have to handle it
            return AllowStatus::NotAllowed;
        };

        // exact whitelists are O(1)ish to check, so look there first
        if self
            .config
            .whitelist_exact
            .get(&path)
            .copied()
            .unwrap_or(false)
        {
            return AllowStatus::Allowed;
        }

        // finally we check if any of our whitelist prefixes match
        for prefix in &self.config.whitelist_prefix {
            if path.starts_with(prefix) {
                return AllowStatus::Allowed;
            }
        }

        AllowStatus::NotAllowed
    }

    /// The path to the RC file.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The recorded watches.
    pub fn times(&self) -> &FileTimes {
        &self.times
    }

    /// The path of this RC's entry in the allow store.
    pub fn allow_path(&self) -> &str {
        &self.allow_path
    }

    /// Updates the mtime of the RC file. This is mainly used to trigger a
    /// reload in direnv.
    pub fn touch(&self) -> Result<()> {
        touch(&self.path)
    }

    /// Evaluates the RC file and returns the new Env, plus any error.
    ///
    /// This function is key to the implementation of direnv.
    ///
    /// Both halves of the pair are meaningful: the original's deferred block
    /// records the directory change even when the load is disallowed or fails,
    /// and `direnv export` relies on getting that environment back *and* the
    /// error, so that a failing `.envrc` still produces a diff and still exits
    /// non-zero instead of retrying on every prompt.
    pub fn load(&self, previous_env: &Env) -> (Env, Option<Error>) {
        let mut new_env = previous_env.copy();
        new_env.insert(DIRENV_WATCHES, &self.times.marshal());

        let outcome = self.load_inner(&mut new_env);

        // Record directory changes even if load is disallowed or fails
        new_env.insert(DIRENV_DIR, &format!("-{}", gopath::dir(&self.path)));
        new_env.insert(DIRENV_FILE, &self.path);
        let diff = previous_env.diff(&new_env).serialize();
        new_env.insert(DIRENV_DIFF, &diff);

        (new_env, outcome.err())
    }

    fn load_inner(&self, new_env: &mut Env) -> Result<()> {
        let config = &self.config;
        let wd = &config.work_dir;
        let direnv = &config.self_path;

        // Abort if the file is not allowed
        match self.allowed() {
            AllowStatus::NotAllowed => {
                return Err(Error(not_allowed_message(self.path())));
            }
            AllowStatus::Allowed => {}
            AllowStatus::Denied => return Ok(()),
        }

        // Allow RC loads to be canceled with SIGINT: direnv installs a handler
        // for the duration of the child's life so that the interrupt reaches
        // bash and terminates the load without taking direnv down with it.
        let _interrupt = InterruptGuard::install();

        // check what type of RC we're processing
        // use different exec method for each
        let fn_name = if gopath::base(&self.path) == ".env" {
            "dotenv"
        } else {
            "source_env"
        };

        let prelude = if config.strict_env {
            "set -euo pipefail && "
        } else {
            ""
        };

        // Non-Windows platforms will already use slashes. However, on Windows
        // backslashes are used by default which can result in unexpected
        // escapes like \b or \r in paths. Force slash usage to avoid issues on
        // Windows.
        let slash_separated_path = gopath::to_slash(self.path());
        let arg = format!(
            r#"{prelude}eval "$("{direnv}" stdlib)" && __main__ {fn_name} {}"#,
            bash_escape(&slash_separated_path)
        );

        let mut command = Command::new(&config.bash_path);
        command
            .arg("-c")
            .arg(&arg)
            .current_dir(wd)
            .env_clear()
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        for entry in new_env.to_go_env() {
            if let Some((key, value)) = entry.split_once('=') {
                command.env(key, value);
            }
        }

        // Set stdin based on the config
        if config.disable_stdin {
            let devnull = File::open("/dev/null")
                .map_err(|err| Error(goerr::path_error("open", "/dev/null", &err)))?;
            command.stdin(Stdio::from(devnull));
        } else {
            command.stdin(Stdio::inherit());
        }

        let output = command
            .output()
            .map_err(|err| Error(goerr::errno_text(&err)))?;

        // The original keeps whichever error came first: a non-zero exit from
        // bash, otherwise a JSON parse failure, otherwise none. An empty stdout
        // from a successful run is not an error - the environment is simply
        // left as it was.
        if !output.status.success() {
            return Err(Error(exit_error(&output.status)));
        }
        if !output.stdout.is_empty() {
            let new_env2 = load_env_json(&output.stdout)?;
            *new_env = new_env2;
        }

        Ok(())
    }
}

/// Go's `*exec.ExitError` message.
pub(crate) fn exit_error(status: &std::process::ExitStatus) -> String {
    use std::os::unix::process::ExitStatusExt;
    if let Some(signal) = status.signal() {
        return format!("signal: {}", signal_name(signal));
    }
    match status.code() {
        Some(code) => format!("exit status {code}"),
        None => "exit status unknown".to_string(),
    }
}

fn signal_name(signal: i32) -> String {
    match signal {
        libc::SIGHUP => "hangup".to_string(),
        libc::SIGINT => "interrupt".to_string(),
        libc::SIGQUIT => "quit".to_string(),
        libc::SIGILL => "illegal instruction".to_string(),
        libc::SIGABRT => "abort trap".to_string(),
        libc::SIGKILL => "killed".to_string(),
        libc::SIGSEGV => "segmentation fault".to_string(),
        libc::SIGPIPE => "broken pipe".to_string(),
        libc::SIGTERM => "terminated".to_string(),
        other => format!("signal {other}"),
    }
}

extern "C" fn ignore_interrupt(_: libc::c_int) {}

/// Installs a no-op SIGINT handler and restores the previous one on drop.
///
/// A handler rather than `SIG_IGN`: an ignored disposition would be inherited
/// across `exec` by the bash child, and the interrupt has to reach it.
struct InterruptGuard {
    previous: libc::sigaction,
}

impl InterruptGuard {
    fn install() -> InterruptGuard {
        // SAFETY: sigaction with a valid handler and a zeroed, writable
        // destination for the previous disposition.
        unsafe {
            let mut action: libc::sigaction = std::mem::zeroed();
            let handler: extern "C" fn(libc::c_int) = ignore_interrupt;
            action.sa_sigaction = handler as *const () as usize;
            let mut previous: libc::sigaction = std::mem::zeroed();
            libc::sigaction(libc::SIGINT, &action, &mut previous);
            InterruptGuard { previous }
        }
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        // SAFETY: restoring a disposition captured by `install`.
        unsafe {
            libc::sigaction(libc::SIGINT, &self.previous, std::ptr::null_mut());
        }
    }
}

/// Inits the RC from the environment.
pub fn rc_from_env(path: &str, marshalled_times: &str, config: &Config) -> Option<RC> {
    let file_hash = file_hash(path).ok()?;

    let allow_path = gopath::join(&[&config.allow_dir(), &file_hash]);

    let mut times = FileTimes::new();
    times.unmarshal(marshalled_times).ok()?;

    let path_hash = path_hash(path).ok()?;

    let deny_path = gopath::join(&[&config.deny_dir(), &path_hash]);

    Some(RC {
        path: path.to_string(),
        allow_path,
        deny_path,
        times,
        config: config.clone(),
    })
}

// Utils

/// Every directory from `path` up to the filesystem root, `path` first.
pub fn each_dir(path: &str) -> Vec<String> {
    let Ok(path) = gopath::abs(path) else {
        return Vec::new();
    };

    let mut paths = vec![path.clone()];

    if path == "/" {
        return paths;
    }

    let mut bytes = path.into_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'/' {
            bytes.truncate(i);
            if bytes.is_empty() {
                bytes = b"/".to_vec();
            }
            paths.push(String::from_utf8_lossy(&bytes).into_owned());
        }
    }

    paths
}

/// Whether `path` names a readable, regular file.
///
/// Some broken filesystems like SSHFS return file information on `stat()` but
/// then cannot open the file, so this opens it.
pub fn file_exists(path: &str) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };

    // Next, check that the file is a regular file.
    let Ok(meta) = file.metadata() else {
        return false;
    };

    meta.is_file()
}

/// `SHA256(abs(path) + "\n" + <file contents>)`, hex-encoded.
pub fn file_hash(path: &str) -> Result<String> {
    let path = gopath::abs(path).map_err(|err| Error(goerr::errno_text(&err)))?;

    let mut fd = File::open(&path).map_err(|err| Error(goerr::path_error("open", &path, &err)))?;

    let mut hasher = Sha256::new();
    hasher.update(format!("{path}\n").as_bytes());
    let mut buffer = Vec::new();
    fd.read_to_end(&mut buffer)
        .map_err(|err| Error(goerr::path_error("read", &path, &err)))?;
    hasher.update(&buffer);

    Ok(hex(&hasher.finalize()))
}

/// `SHA256(abs(path) + "\n")`, hex-encoded.
pub fn path_hash(path: &str) -> Result<String> {
    let path = gopath::abs(path).map_err(|err| Error(goerr::errno_text(&err)))?;

    let mut hasher = Sha256::new();
    hasher.update(format!("{path}\n").as_bytes());

    Ok(hex(&hasher.finalize()))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Sets the file's access and modification times to now, Go's `os.Chtimes`.
fn touch(path: &str) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let times = [
        libc::timeval {
            tv_sec: now as libc::time_t,
            tv_usec: 0,
        },
        libc::timeval {
            tv_sec: now as libc::time_t,
            tv_usec: 0,
        },
    ];
    let c_path =
        std::ffi::CString::new(path).map_err(|_| errorf!("chtimes {path}: invalid argument"))?;
    // SAFETY: `c_path` is a valid NUL-terminated string and `times` is a
    // two-element array of timevals, as utimes requires.
    let rc = unsafe { libc::utimes(c_path.as_ptr(), times.as_ptr()) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        return Err(Error(goerr::path_error("chtimes", path, &err)));
    }
    Ok(())
}

fn allow(path: &str, allow_path: &str) -> Result<()> {
    write_file(allow_path, &format!("{path}\n"), 0o644)
}

/// Go's `os.WriteFile` with an explicit mode.
pub(crate) fn write_file(path: &str, contents: &str, mode: u32) -> Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(mode)
        .open(path)
        .map_err(|err| Error(goerr::path_error("open", path, &err)))?;
    file.write_all(contents.as_bytes())
        .map_err(|err| Error(goerr::path_error("write", path, &err)))?;
    Ok(())
}

/// The first `.envrc` — or `.env`, when `load_dotenv` is on — at or above
/// `search_dir`.
pub fn find_env_up(search_dir: &str, load_dotenv: bool) -> String {
    if load_dotenv {
        return find_up(search_dir, &[".envrc", ".env"]);
    }
    find_up(search_dir, &[".envrc"])
}

fn find_up(search_dir: &str, file_names: &[&str]) -> String {
    if search_dir.is_empty() {
        return String::new();
    }
    for dir in each_dir(search_dir) {
        for file_name in file_names {
            let path = gopath::join(&[&dir, file_name]);
            if file_exists(&path) {
                return path;
            }
        }
    }
    String::new()
}
