//! The record of files direnv is watching for changes.

use std::fs::Metadata;
use std::io;
use std::os::unix::fs::MetadataExt;

use crate::deps::gojson::{self, JsonValue};
use crate::deps::{gopath, gotime};
use crate::log_debug;
use crate::{errorf, gzenv, Error, Result};

/// A single recorded file status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTime {
    pub path: String,
    pub modtime: i64,
    pub exists: bool,
}

impl FileTime {
    /// Verifies that the file is good and hasn't changed.
    pub fn check(&self) -> Result<()> {
        match get_latest_stat(&self.path) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                if self.exists {
                    log_debug!("Stat Check: {}: gone", self.path);
                    return Err(errorf!("File {:?} is missing (Stat)", self.path));
                }
            }
            Err(err) => {
                log_debug!("Stat Check: {}: ERR: {}", self.path, err);
                return Err(Error(crate::deps::goerr::path_error(
                    "stat", &self.path, &err,
                )));
            }
            Ok(stat) => {
                if !self.exists {
                    log_debug!("Check: {}: appeared", self.path);
                    return Err(errorf!("File {:?} newly created", self.path));
                }
                if stat.mtime() != self.modtime {
                    log_debug!(
                        "Check: {}: stale (stat: {}, lastcheck: {})",
                        self.path,
                        stat.mtime(),
                        self.modtime
                    );
                    return Err(errorf!("File {:?} has changed", self.path));
                }
            }
        }
        log_debug!("Check: {}: up to date", self.path);
        Ok(())
    }

    /// Shows the time in a user-friendly format.
    pub fn formatted(&self, rel_dir: &str) -> String {
        let time_bytes = gotime::unix_marshal_text(self.modtime);
        let path = gopath::rel(rel_dir, &self.path).unwrap_or_else(|_| self.path.clone());
        format!("{path:?} - {time_bytes}")
    }

    /// The JSON shape of the Go struct: field order, not sorted.
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(vec![
            ("path".to_string(), JsonValue::String(self.path.clone())),
            (
                "modtime".to_string(),
                JsonValue::Number(self.modtime as f64),
            ),
            ("exists".to_string(), JsonValue::Bool(self.exists)),
        ])
    }
}

/// A record of all the known files and times.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FileTimes {
    /// The recorded watches, in insertion order.
    ///
    /// Public because the original's tests reach into the slice directly, and
    /// because `direnv status` and `direnv watch-print` iterate it in order.
    pub list: Vec<FileTime>,
}

impl FileTimes {
    /// Creates a new empty `FileTimes`.
    pub fn new() -> FileTimes {
        FileTimes { list: Vec::new() }
    }

    /// Gets the latest stats on the path and updates the record.
    pub fn update(&mut self, path: &str) -> Result<()> {
        let mut modtime: i64 = 0;
        let exists;

        match get_latest_stat(path) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => exists = false,
            Err(err) => {
                return Err(Error(crate::deps::goerr::path_error("stat", path, &err)));
            }
            Ok(stat) => {
                exists = true;
                modtime = stat.mtime();
            }
        }

        self.new_time(path, modtime, exists)
    }

    /// Adds the file on `path`, with `modtime` and the `exists` flag, to the
    /// list of known files.
    pub fn new_time(&mut self, path: &str, modtime: i64, exists: bool) -> Result<()> {
        let path = gopath::abs(path).map_err(|err| Error(crate::deps::goerr::errno_text(&err)))?;
        let path = gopath::clean(&path);

        match self.list.iter_mut().find(|time| time.path == path) {
            Some(time) => {
                time.modtime = modtime;
                time.exists = exists;
            }
            None => self.list.push(FileTime {
                path,
                modtime,
                exists,
            }),
        }

        Ok(())
    }

    /// Validates all the recorded file times.
    pub fn check(&self) -> Result<()> {
        if self.list.is_empty() {
            return Err(Error::from("Times list is empty"));
        }
        for time in &self.list {
            time.check()?;
        }
        Ok(())
    }

    /// Compares notes between the given path and the recorded times.
    pub fn check_one(&self, path: &str) -> Result<()> {
        let path = gopath::abs(path).map_err(|err| Error(crate::deps::goerr::errno_text(&err)))?;
        for time in &self.list {
            if time.path == path {
                return time.check();
            }
        }
        Err(errorf!("File {path:?} is unknown"))
    }

    /// Dumps the times into gzenv format.
    pub fn marshal(&self) -> String {
        gzenv::marshal(&JsonValue::Array(
            self.list.iter().map(FileTime::to_json).collect(),
        ))
    }

    /// Loads the watches back from gzenv.
    ///
    /// The payload comes out of the user's own environment, so a wrong shape is
    /// reachable; the type errors are worded the way `encoding/json` words them
    /// when decoding into `[]cmd.FileTime`.
    pub fn unmarshal(&mut self, from: &str) -> Result<()> {
        let value = gzenv::unmarshal(from)?;
        let wrap = |err: String| Error(format!("unmarshal() json parsing: {err}"));
        let mut list = Vec::new();
        match value {
            // Go leaves the slice untouched for a JSON null.
            JsonValue::Null => {}
            JsonValue::Array(items) => {
                for (index, item) in items.into_iter().enumerate() {
                    let JsonValue::Object(pairs) = &item else {
                        return Err(wrap(gojson::element_type_error(
                            &item,
                            &format!(".{index}"),
                            "cmd.FileTime",
                        )));
                    };
                    let mut time = FileTime {
                        path: String::new(),
                        modtime: 0,
                        exists: false,
                    };
                    for (key, value) in pairs {
                        match (key.as_str(), value) {
                            ("path", JsonValue::String(text)) => time.path = text.clone(),
                            ("path", other) => {
                                return Err(wrap(gojson::field_type_error(
                                    other,
                                    &format!(".{index}.path"),
                                    "string",
                                )))
                            }
                            ("modtime", JsonValue::Number(n)) => time.modtime = *n as i64,
                            ("modtime", other) => {
                                return Err(wrap(gojson::field_type_error(
                                    other,
                                    &format!(".{index}.modtime"),
                                    "int64",
                                )))
                            }
                            ("exists", JsonValue::Bool(b)) => time.exists = *b,
                            ("exists", other) => {
                                return Err(wrap(gojson::field_type_error(
                                    other,
                                    &format!(".{index}.exists"),
                                    "bool",
                                )))
                            }
                            // Unknown members are ignored, as in Go.
                            _ => {}
                        }
                    }
                    list.push(time);
                }
                self.list = list;
            }
            other => {
                return Err(wrap(gojson::type_error(&other, "[]cmd.FileTime")));
            }
        }
        Ok(())
    }
}

/// `lstat` and `stat` the path and keep whichever reports the later mtime, so
/// that re-pointing a symlink counts as a change even when the target has not
/// moved.
fn get_latest_stat(path: &str) -> io::Result<Metadata> {
    let lstat = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) => {
            log_debug!("getLatestStat,Lstat: {}: error: {}", path, err);
            return Err(err);
        }
    };
    let lstat_mod_time = lstat.mtime();

    let stat = match std::fs::metadata(path) {
        Ok(meta) => meta,
        Err(err) => {
            log_debug!(
                "getLatestStat,Stat: {}: error: {} (Lstat time: {})",
                path,
                err,
                lstat_mod_time
            );
            return Err(err);
        }
    };
    let stat_mod_time = stat.mtime();

    if lstat_mod_time > stat_mod_time {
        log_debug!(
            "getLatestStat: {}: Lstat: {}, Stat: {} -> preferring Lstat",
            path,
            lstat_mod_time,
            stat_mod_time
        );
        return Ok(lstat);
    }
    log_debug!(
        "getLatestStat: {}: Lstat: {}, Stat: {} -> preferring Stat",
        path,
        lstat_mod_time,
        stat_mod_time
    );
    Ok(stat)
}
