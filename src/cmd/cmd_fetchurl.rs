//! `direnv fetchurl <url> [<integrity-hash>]`

use std::io::{IsTerminal, Write};
use std::os::unix::fs::PermissionsExt;

use crate::cmd::commands::{Action, Cmd};
use crate::cmd::config::Config;
use crate::cmd::env::Env;
use crate::cmd::rc::file_exists;
use crate::deps::{goerr, gopath};
use crate::sri;
use crate::{errorf, Error, Result};

pub fn cmd_fetchurl() -> Cmd {
    Cmd {
        name: "fetchurl",
        desc: "Fetches a given URL into direnv's CAS".to_string(),
        args: &["<url>", "[<integrity-hash>]"],
        aliases: &[],
        private: false,
        action: Action::WithConfig(cmd_fetch_url),
    }
}

fn cmd_fetch_url(_env: &Env, args: &[String], config: &Config) -> Result<()> {
    if args.len() < 2 {
        return Err(Error::from("missing URL argument"));
    }

    let algo = sri::Algo::Sha256;
    let cas_dir = cas_dir(config);
    let is_tty = std::io::stdout().is_terminal();

    let url = args[1].clone();
    let mut integrity_hash = String::new();
    // Validate the SRI hash if it exists
    if args.len() > 2 {
        // Support Base64 where '/' have been replaced by '_'
        integrity_hash = args[2].replace('_', "/");

        let hash = sri::parse(&integrity_hash).map_err(Error)?;

        // Shortcut if the cache already has the file
        let cas_file = cas_path(&cas_dir, &hash);
        if file_exists(&cas_file) {
            println!("{cas_file}");
            return Ok(());
        }
    }

    // Create the CAS directory if it doesn't exist
    std::fs::create_dir_all(&cas_dir)
        .map_err(|err| Error(goerr::path_error("mkdir", &cas_dir, &err)))?;

    // Create a temporary file to copy the content into, before the CAS file
    // location can be calculated.
    let tmp_path = create_temp(&cas_dir, "tmp")?;
    let cleanup = TempFile(tmp_path.clone());

    // Get the URL
    let response = ureq::AgentBuilder::new()
        // Go's http.Client stops after ten redirects.
        .redirects(10)
        .build()
        .get(&url)
        .call();

    let response = match response {
        Ok(response) => response,
        Err(ureq::Error::Status(code, _)) => {
            return Err(errorf!("expected status code 200 but got {code}"))
        }
        Err(err) => return Err(errorf!("Get \"{url}\": {err}")),
    };

    // Abort if we don't get a 200 back
    if response.status() != 200 {
        return Err(errorf!(
            "expected status code 200 but got {}",
            response.status()
        ));
    }

    // While copying the content into the temporary location, also calculate the
    // SRI hash.
    let calculated_hash = {
        let mut tmpfile = std::fs::OpenOptions::new()
            .write(true)
            .open(&tmp_path)
            .map_err(|err| Error(goerr::path_error("open", &tmp_path, &err)))?;
        let mut writer = sri::Writer::new(&mut tmpfile, algo);
        std::io::copy(&mut response.into_reader(), &mut writer)
            .map_err(|err| Error(goerr::errno_text(&err)))?;
        writer.sum()
    };

    // Make the file read-only and executable for later
    std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o500))
        .map_err(|err| Error(goerr::path_error("chmod", &tmp_path, &err)))?;

    // Validate if a comparison hash was given
    if !integrity_hash.is_empty() && calculated_hash.to_string() != integrity_hash {
        return Err(errorf!(
            "hash mismatch. Expected '{integrity_hash}' but got '{calculated_hash}'"
        ));
    }

    // Derive the CAS file location from the SRI hash
    let cas_file = cas_path(&cas_dir, &calculated_hash);

    // Put the file into the CAS store if it's not already there
    if !file_exists(&cas_file) {
        // Move the temporary file to the CAS location.
        std::fs::rename(&tmp_path, &cas_file)
            .map_err(|err| errorf!("rename {tmp_path} {cas_file}: {}", goerr::errno_text(&err)))?;
    }
    drop(cleanup);

    if integrity_hash.is_empty() {
        if is_tty {
            // Print an example for terminal users
            print!(
                "Found hash: {calculated_hash}\n\nInvoke fetchurl again with the hash as an argument to get the disk location:\n\n  direnv fetchurl \"{url}\" \"{calculated_hash}\"\n  #=> {cas_file}\n"
            );
        } else {
            // Only print the hash in scripting mode. Add one extra hurdle on
            // purpose to use fetchurl without the SRI hash.
            println!("{calculated_hash}");
        }
    } else {
        // Print the location to the CAS file
        println!("{cas_file}");
    }
    Ok(())
}

/// Removes the temporary file on drop, standing in for the original's
/// `defer os.Remove(tmpfile.Name())`.
struct TempFile(String);

impl Drop for TempFile {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.0) {
            if err.kind() != std::io::ErrorKind::NotFound {
                eprintln!("Warning: failed to remove temp file {}: {err}", self.0);
            }
        }
    }
}

/// Go's `os.CreateTemp(dir, pattern)`.
fn create_temp(dir: &str, pattern: &str) -> Result<String> {
    for _ in 0..10_000 {
        let mut random = [0u8; 8];
        if getrandom::getrandom(&mut random).is_err() {
            return Err(Error::from("failed to generate a temporary file name"));
        }
        let suffix: String = random.iter().map(|b| format!("{b:02x}")).collect();
        let candidate = gopath::join(&[dir, &format!("{pattern}{suffix}")]);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&candidate)
        {
            Ok(mut file) => {
                let _ = file.flush();
                return Ok(candidate);
            }
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(Error(goerr::path_error("open", &candidate, &err))),
        }
    }
    Err(Error::from("failed to create a temporary file"))
}

use std::os::unix::fs::OpenOptionsExt;

fn cas_dir(c: &Config) -> String {
    gopath::join(&[&c.cache_dir, "cas"])
}

/// The filesystem path for an SRI hash.
fn cas_path(dir: &str, integrity_hash: &sri::Hash) -> String {
    // Use Hex encoding for the filesystem to avoid issues
    let sri_file = integrity_hash.hex();
    gopath::join(&[dir, &sri_file])
}
