//! `os/exec.LookPath`, except the `PATH` is passed in.
//!
//! direnv resolves a command against the environment it has just built, not
//! against its own, so it cannot use the process `PATH`.

use std::os::unix::fs::PermissionsExt;

/// The error resulting if a path search failed to find an executable file.
pub const ERR_NOT_FOUND: &str = "executable file not found in $PATH";

/// Similar to `os/exec.LookPath` except we pass in the `PATH`.
pub fn look_path(file: &str, pathenv: &str) -> Result<String, String> {
    if file.contains('/') {
        return match find_executable(file) {
            Ok(()) => Ok(file.to_string()),
            Err(err) => Err(err),
        };
    }
    if pathenv.is_empty() {
        return Err(ERR_NOT_FOUND.to_string());
    }
    for dir in pathenv.split(':') {
        // Unix shell semantics: path element "" means "."
        let dir = if dir.is_empty() { "." } else { dir };
        let path = format!("{dir}/{file}");
        if find_executable(&path).is_ok() {
            return Ok(path);
        }
    }
    Err(ERR_NOT_FOUND.to_string())
}

fn find_executable(file: &str) -> Result<(), String> {
    let meta = std::fs::metadata(file)
        .map_err(|err| crate::deps::goerr::path_error("stat", file, &err))?;
    if !meta.is_dir() && meta.permissions().mode() & 0o111 != 0 {
        return Ok(());
    }
    // Go returns the bare `os.ErrPermission` here, not a *PathError.
    Err("permission denied".to_string())
}
