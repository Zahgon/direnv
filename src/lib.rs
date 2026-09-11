//! direnv — unclutter your .profile.
//!
//! The crate is split the way the Go tree was: [`cmd`] holds the command-line
//! interface and everything that talks to the host shell, [`dotenv`], [`sri`],
//! [`gzenv`] and [`xdg`] are the standalone packages it builds on, and
//! [`deps`] holds the narrow reimplementations of Go standard-library
//! behaviour that direnv's output depends on.

pub mod cmd;
pub mod deps;
pub mod dotenv;
pub mod gzenv;
pub mod sri;
pub mod xdg;

use std::fmt;

/// A direnv failure.
///
/// direnv renders every error straight to the user as `direnv: error <err>`,
/// so the message text — not a type — is the contract. Errors therefore carry
/// exactly the string the Go build would have produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<String> for Error {
    fn from(message: String) -> Error {
        Error(message)
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Error {
        Error(message.to_string())
    }
}

/// The result of a direnv operation.
pub type Result<T> = std::result::Result<T, Error>;

/// Build an [`Error`] with `format!` syntax.
#[macro_export]
macro_rules! errorf {
    ($($arg:tt)*) => {
        $crate::Error(format!($($arg)*))
    };
}
